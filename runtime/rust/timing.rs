use std::cell::RefCell;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const STARTUP_STEP: Duration = Duration::from_nanos(1_000_000_000 / 12);

#[derive(Default)]
struct StartupPacer {
    last_step: Option<Instant>,
}

impl StartupPacer {
    fn delay(&mut self, remaining: u32, now: Instant) -> Option<Duration> {
        if remaining == 0 {
            self.last_step = None;
            return None;
        }
        Some(self.last_step.map_or(STARTUP_STEP, |last| {
            (last + STARTUP_STEP).saturating_duration_since(now)
        }))
    }
}

/// Frames after the warm-up ends during which the original's frame-time smoothing is switched off.
const UNSMOOTHED_FRAMES: u32 = 60;

/// What to do with the original's smoothing flag (guest word at loc_1A4654) this frame.
#[derive(Debug, PartialEq, Eq)]
pub enum Smoothing {
    Keep,
    Disable,
    Restore(u32),
}

/// The warm-up is paced at 12 Hz, so the original's smoothed time step starts out believing the
/// game runs at 12 frames per second. A faster renderer then moves everything about three times too
/// fast until the smoothing catches up (seconds on a fast machine). Switching the smoothing off for
/// a short stretch after the warm-up makes the first real frames measure their own duration.
#[derive(Default)]
pub struct SmoothingRelease {
    warming: bool,
    saved: Option<u32>,
    frames_left: u32,
}

impl SmoothingRelease {
    pub fn step(&mut self, remaining: u32, flag: u32) -> Smoothing {
        if remaining != 0 {
            self.warming = true;
            self.frames_left = 0;
            return self
                .saved
                .take()
                .map_or(Smoothing::Keep, Smoothing::Restore);
        }
        if self.warming {
            self.warming = false;
            self.saved = Some(flag);
            self.frames_left = UNSMOOTHED_FRAMES;
            return Smoothing::Disable;
        }
        if self.frames_left > 0 {
            self.frames_left -= 1;
            if self.frames_left == 0 {
                return self
                    .saved
                    .take()
                    .map_or(Smoothing::Keep, Smoothing::Restore);
            }
        }
        Smoothing::Keep
    }
}

/// The game's own frame-time measurement (guest words, see `docs/engine/player.md` "FPS-derived
/// frame scale"): FPS is measured over windows of 18 BIOS ticks and smoothed, and the movement scale
/// is `normal_frame_rate * 256 / FPS`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GuestTiming {
    pub smoothed: i32,
    pub measured: i32,
    pub frames: i32,
    pub baseline: u32,
    pub remainder: i32,
    pub scale: i32,
}

const STALL_MS: u32 = 250;
const SETTLED_FRAMES: u32 = 60;
const CALIBRATION_FRAMES: usize = 5;
const CALIBRATION_GIVE_UP: u32 = 60;
const MIN_FPS: u32 = 6;
const MAX_FPS: u32 = 300;

/// A load or a menu stalls the game for seconds. The original's one-second window then holds a
/// handful of frames over a long time, so it measures a tiny FPS and every movement runs several times
/// too fast until the window refreshes. After the warm-up that follows each load the window also still
/// holds 12 Hz frames. Both are corrected here: when a stall or a warm-up ends, the last measurement from
/// steady play stands in at once (if there is one), then the median of the next few real frame times
/// replaces it, and the game's own window starts clean.
#[derive(Default)]
pub struct FrameGuard {
    last_ms: Option<u32>,
    was_warm: bool,
    settled: u32,
    steady_since: u32,
    last_good: Option<GuestTiming>,
    calibrating: Option<Calibration>,
}

struct Calibration {
    intervals: Vec<u32>,
    previous: u32,
    calls: u32,
}

impl FrameGuard {
    /// Called once per game update, before the game measures it. Returns the timing to write back.
    pub fn step(
        &mut self,
        warm: bool,
        now_ms: u32,
        ticks: u32,
        now: GuestTiming,
    ) -> Option<GuestTiming> {
        let gap = self.last_ms.map_or(0, |last| now_ms.wrapping_sub(last));
        self.last_ms = Some(now_ms);
        let clean = |timing: GuestTiming| GuestTiming {
            baseline: ticks,
            remainder: 0,
            frames: 0,
            ..timing
        };
        if warm {
            self.was_warm = true;
            self.settled = 0;
            self.calibrating = None;
            return None;
        }
        let restarted = std::mem::take(&mut self.was_warm) || gap >= STALL_MS;
        if restarted {
            self.settled = 0;
            self.calibrating = Some(Calibration {
                intervals: Vec::new(),
                previous: now_ms,
                calls: 0,
            });
            return self.last_good.map(clean);
        }
        if let Some(calibration) = self.calibrating.as_mut() {
            calibration.calls += 1;
            let interval = now_ms.wrapping_sub(calibration.previous);
            calibration.previous = now_ms;
            if interval < STALL_MS {
                calibration.intervals.push(interval);
            }
            if calibration.intervals.len() < CALIBRATION_FRAMES {
                if calibration.calls >= CALIBRATION_GIVE_UP {
                    // shortcut: a machine under 4 fps keeps the game's own measurement.
                    self.calibrating = None;
                }
                return None;
            }
            calibration.intervals.sort_unstable();
            let median = calibration.intervals[calibration.intervals.len() / 2].max(1);
            let fps = (1000 / median).clamp(MIN_FPS, MAX_FPS) as i32;
            self.calibrating = None;
            return Some(clean(GuestTiming {
                smoothed: fps,
                measured: fps,
                ..now
            }));
        }
        if self.settled == 0 {
            self.steady_since = now_ms;
        }
        self.settled += 1;
        if self.settled >= SETTLED_FRAMES {
            // Only a measurement that matches the frame rate seen here is worth restoring: the game
            // also measures slow phases (movies, the menu book) that say nothing about play.
            let real = i64::from(self.settled) * 1000
                / i64::from(now_ms.wrapping_sub(self.steady_since).max(1));
            let guest = i64::from(now.smoothed.min(now.measured));
            if guest * 2 >= real && guest <= real * 2 {
                self.last_good = Some(now);
            }
        }
        None
    }
}

thread_local! {
    pub static FRAME_GUARD: RefCell<FrameGuard> = RefCell::new(FrameGuard::default());
    pub static SMOOTHING: RefCell<SmoothingRelease> = RefCell::new(SmoothingRelease::default());
    static STARTUP_PACER: RefCell<StartupPacer> = RefCell::new(StartupPacer::default());
}

/// Match wall time to the original fixed 12-Hz clock steps during warm-up.
/// Repeated counter resets during loading fades remain paced. Slow frames
/// never create a backlog of steps to catch up on later.
pub fn pace_startup_clock(remaining: u32) {
    STARTUP_PACER.with(|pacer| {
        let mut pacer = pacer.borrow_mut();
        if let Some(delay) = pacer.delay(remaining, Instant::now()) {
            thread::sleep(delay);
            pacer.last_step = Some(Instant::now());
        }
    });
}

#[unsafe(no_mangle)]
pub static rg_bios_ticks: AtomicU32 = AtomicU32::new(0);

struct Clock {
    began: Instant,
    stop: Arc<AtomicBool>,
    worker: JoinHandle<()>,
}
static CLOCK: Mutex<Option<Clock>> = Mutex::new(None);

#[unsafe(no_mangle)]
pub extern "C" fn rg_clock_convert(elapsed: u64, frequency: u64) -> u32 {
    if frequency == 0 || frequency > 1_000_000_000_000 {
        return 0;
    }
    (u128::from(elapsed) * 1_193_182 / u128::from(frequency) / 65_536) as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_clock_start() -> i32 {
    let mut clock = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    if clock.is_some() {
        return 1;
    }
    let began = Instant::now();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = Arc::clone(&stop);
    rg_bios_ticks.store(0, Ordering::Relaxed);
    let Ok(worker) = thread::Builder::new()
        .name("Redguard BIOS clock".into())
        .spawn(move || {
            while !stopping.load(Ordering::Acquire) {
                let nanos = began.elapsed().as_nanos();
                rg_bios_ticks.store(
                    (nanos * 1_193_182 / 1_000_000_000 / 65_536) as u32,
                    Ordering::Relaxed,
                );
                thread::park_timeout(Duration::from_millis(1));
            }
        })
    else {
        return 0;
    };
    *clock = Some(Clock {
        began,
        stop,
        worker,
    });
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_clock_stop() {
    let mut clock = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(clock) = clock.take() {
        clock.stop.store(true, Ordering::Release);
        clock.worker.thread().unpark();
        let _ = clock.worker.join();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_clock_milliseconds() -> u32 {
    CLOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map_or(0, |clock| clock.began.elapsed().as_millis() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing(fps: i32) -> GuestTiming {
        GuestTiming {
            smoothed: fps,
            measured: fps,
            frames: 7,
            baseline: 1,
            remainder: 3,
            scale: 3072 / fps,
        }
    }

    fn steady(guard: &mut FrameGuard, ms: &mut u32, fps: i32) {
        for _ in 0..SETTLED_FRAMES {
            *ms += 16;
            assert_eq!(guard.step(false, *ms, 0, timing(fps)), None);
        }
    }

    fn run_frames(
        guard: &mut FrameGuard,
        ms: &mut u32,
        every: u32,
        count: usize,
    ) -> Option<GuestTiming> {
        let mut result = None;
        for _ in 0..count {
            *ms += every;
            result = result.or(guard.step(false, *ms, 40, timing(6)));
        }
        result
    }

    #[test]
    fn a_stall_stands_in_with_the_old_measurement_then_measures_the_new_scene() {
        let mut guard = FrameGuard::default();
        let mut ms = 0;
        steady(&mut guard, &mut ms, 60);
        ms += 3_000; // a menu
        let stand_in = guard.step(false, ms, 55, timing(6)).unwrap();
        assert_eq!(
            (stand_in.smoothed, stand_in.measured, stand_in.scale),
            (60, 60, 51)
        );
        assert_eq!(
            (stand_in.frames, stand_in.remainder, stand_in.baseline),
            (0, 0, 55)
        );
        // the new scene runs at 25 fps: the median of the next frames replaces the stand-in
        let measured = run_frames(&mut guard, &mut ms, 40, CALIBRATION_FRAMES).unwrap();
        assert_eq!((measured.smoothed, measured.measured), (25, 25));
        assert_eq!(run_frames(&mut guard, &mut ms, 40, 3), None);
    }

    #[test]
    fn one_slow_frame_does_not_spoil_the_measurement() {
        let mut guard = FrameGuard::default();
        let mut ms = 0;
        steady(&mut guard, &mut ms, 60);
        ms += 3_000;
        guard.step(false, ms, 0, timing(6));
        let mut result = None;
        for every in [10, 10, 200, 10, 10, 10] {
            ms += every;
            result = result.or(guard.step(false, ms, 0, timing(6)));
        }
        assert_eq!(result.unwrap().measured, 100);
    }

    #[test]
    fn the_first_load_has_no_stand_in_and_measures_the_real_frame_rate() {
        let mut guard = FrameGuard::default();
        for n in 0..12 {
            assert_eq!(guard.step(true, n * 83, 0, timing(12)), None);
        }
        let mut ms = 1000;
        assert_eq!(guard.step(false, ms, 20, timing(12)), None);
        let measured = run_frames(&mut guard, &mut ms, 10, CALIBRATION_FRAMES).unwrap();
        assert_eq!(measured.smoothed, 100);
    }

    #[test]
    fn a_measurement_far_from_the_real_frame_rate_is_not_kept() {
        let mut guard = FrameGuard::default();
        let mut ms = 0;
        steady(&mut guard, &mut ms, 2); // the game's own measurement said 2 fps while frames took 16 ms
        ms += 12 * 83;
        for n in 0..12 {
            assert_eq!(guard.step(true, n * 83, 0, timing(12)), None);
        }
        ms += 16;
        assert_eq!(guard.step(false, ms, 9, timing(12)), None); // no stand-in: it measures instead
    }

    #[test]
    fn a_later_warmup_stands_in_with_the_earlier_measurement() {
        let mut guard = FrameGuard::default();
        let mut ms = 0;
        steady(&mut guard, &mut ms, 60);
        for _ in 0..12 {
            ms += 83;
            assert_eq!(guard.step(true, ms, 0, timing(12)), None);
        }
        ms += 16;
        assert_eq!(guard.step(false, ms, 9, timing(12)).unwrap().measured, 60);
    }

    #[test]
    fn smoothing_is_released_after_warmup_and_restored_once() {
        let mut release = SmoothingRelease::default();
        assert_eq!(release.step(0, 1), Smoothing::Keep);
        assert_eq!(release.step(12, 1), Smoothing::Keep);
        assert_eq!(release.step(1, 1), Smoothing::Keep);
        assert_eq!(release.step(0, 1), Smoothing::Disable);
        for _ in 0..UNSMOOTHED_FRAMES - 1 {
            assert_eq!(release.step(0, 0), Smoothing::Keep);
        }
        assert_eq!(release.step(0, 0), Smoothing::Restore(1));
        assert_eq!(release.step(0, 1), Smoothing::Keep);
    }

    #[test]
    fn a_new_warmup_restores_the_flag_immediately() {
        let mut release = SmoothingRelease::default();
        release.step(3, 1);
        assert_eq!(release.step(0, 1), Smoothing::Disable);
        assert_eq!(release.step(5, 0), Smoothing::Restore(1));
    }

    #[test]
    fn startup_steps_follow_twelve_hz_on_a_fast_renderer() {
        let mut pacer = StartupPacer::default();
        let start = Instant::now();
        let mut now = start;
        for remaining in (1..=12).rev() {
            let delay = pacer.delay(remaining, now).unwrap();
            now += delay;
            pacer.last_step = Some(now);
            if remaining != 1 {
                now += Duration::from_millis(10);
            }
        }
        assert_eq!(now.duration_since(start), STARTUP_STEP * 12);
        assert_eq!(pacer.delay(0, now), None);
    }

    #[test]
    fn loading_fade_resets_do_not_disable_pacing() {
        let mut pacer = StartupPacer::default();
        let start = Instant::now();
        let mut now = start;
        for remaining in [12, 12, 12, 11, 12, 11, 10] {
            now += pacer.delay(remaining, now).unwrap();
            pacer.last_step = Some(now);
        }
        assert_eq!(now.duration_since(start), STARTUP_STEP * 7);
    }

    #[test]
    fn slow_frames_do_not_accumulate_catchup_steps() {
        let mut pacer = StartupPacer::default();
        let start = Instant::now();
        pacer.last_step = Some(start);
        let resumed = start + Duration::from_secs(2);
        assert_eq!(pacer.delay(5, resumed), Some(Duration::ZERO));
        pacer.last_step = Some(resumed);
        assert_eq!(pacer.delay(4, resumed), Some(STARTUP_STEP));
        assert_eq!(pacer.delay(0, resumed), None);
        assert_eq!(pacer.delay(12, resumed), Some(STARTUP_STEP));
    }
    #[test]
    fn bios_rate_and_long_uptime_without_overflow() {
        for (elapsed, frequency, ticks) in [
            (0, 1193182, 0),
            (65535, 1193182, 0),
            (65536, 1193182, 1),
            (1193182, 1193182, 18),
            (3600 * 10000000, 10000000, 65543),
            (30 * 86400 * 10000000, 10000000, 47191280),
        ] {
            assert_eq!(rg_clock_convert(elapsed, frequency), ticks);
        }
        assert_eq!(rg_clock_convert(1, 0), 0);
        assert_eq!(rg_clock_convert(1, 1_000_000_000_001), 0);
    }
    #[test]
    fn shared_clock_stops_restarts_and_measures_elapsed_time() {
        assert_eq!(rg_clock_start(), 1);
        assert_eq!(rg_clock_start(), 1);
        thread::sleep(Duration::from_millis(160));
        assert!(rg_bios_ticks.load(Ordering::Relaxed) > 0);
        assert!(rg_clock_milliseconds() >= 150);
        rg_clock_stop();
        let stopped = rg_bios_ticks.load(Ordering::Relaxed);
        thread::sleep(Duration::from_millis(70));
        assert_eq!(rg_bios_ticks.load(Ordering::Relaxed), stopped);
        assert_eq!(rg_clock_start(), 1);
        rg_clock_stop();
        assert_eq!(rg_clock_milliseconds(), 0);
    }
}
