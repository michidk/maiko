use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
