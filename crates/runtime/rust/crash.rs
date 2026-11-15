//! Fatal reporting without allocation, locks, unwinding or exit cleanup.
use std::fmt::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
#[unsafe(no_mangle)]
pub static rg_call_head: AtomicU32 = AtomicU32::new(0);
#[unsafe(no_mangle)]
pub static rg_call_ring: [AtomicU32; 64] = [const { AtomicU32::new(0) }; 64];
pub(crate) static REPORTING: AtomicBool = AtomicBool::new(false);
pub(crate) struct Report {
    bytes: [u8; 4096],
    length: usize,
}
impl Report {
    pub(crate) fn new() -> Self {
        Self {
            bytes: [0; 4096],
            length: 0,
        }
    }
    pub(crate) fn calls(&mut self) {
        let head = rg_call_head.load(Relaxed);
        let count = head.min(32);
        let mut saved = [0; 32];
        for (i, value) in saved.iter_mut().take(count as usize).enumerate() {
            *value = rg_call_ring[(head.wrapping_sub(count).wrapping_add(i as u32) & 63) as usize]
                .load(Relaxed);
        }
        for value in saved.into_iter().take(count as usize) {
            let _ = writeln!(self, "CALLSITE {value:016x}");
        }
    }
    pub(crate) fn output(&self) {
        crate::os::write_stderr(&self.bytes[..self.length]);
        crate::logging::fatal_console(&self.bytes[..self.length]);
    }
}
impl Write for Report {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > self.bytes.len() - self.length {
            return Err(fmt::Error);
        }
        self.bytes[self.length..self.length + s.len()].copy_from_slice(s.as_bytes());
        self.length += s.len();
        Ok(())
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn rg_dump_calls() {
    let mut report = Report::new();
    report.calls();
    report.output();
}
/// Install native fatal-error handlers with a fixed, allocation-free report buffer.
#[unsafe(no_mangle)]
pub extern "C" fn rg_crash_install() {
    crate::os::install_crash();
    let _ = std::thread::Builder::new()
        .name("stall watchdog".into())
        .spawn(watch_for_stalls);
}

/// Seconds without a presented frame after which the game counts as stalled.
const STALL: u64 = 15;
/// How often the kept screenshot is refreshed.
const SNAPSHOT_EVERY: u64 = 5_000;
static START: OnceLock<Instant> = OnceLock::new();
/// Milliseconds since START when a frame was last presented (0: none yet).
static LAST_FRAME: AtomicU64 = AtomicU64::new(0);
static LAST_SNAPSHOT: AtomicU64 = AtomicU64::new(0);
/// The last presented frame as a complete BMP file, for crash and stall reports.
static SNAPSHOT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

fn now() -> u64 {
    START.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1
}

/// Call once per presented frame. True when the kept screenshot is due for a refresh.
pub fn frame_presented() -> bool {
    let now = now();
    LAST_FRAME.store(now, Relaxed);
    if now - LAST_SNAPSHOT.load(Relaxed) < SNAPSHOT_EVERY {
        return false;
    }
    LAST_SNAPSHOT.store(now, Relaxed);
    true
}

/// Keep `width` x `height` RGB565 pixels (`pixel(x, y)`) as the screenshot for crash reports.
pub fn store_snapshot(width: u32, height: u32, pixel: impl Fn(u32, u32) -> u16) {
    let Ok(mut bmp) = SNAPSHOT.try_lock() else {
        return;
    };
    *bmp = encode_bmp(width, height, pixel);
}

fn encode_bmp(width: u32, height: u32, pixel: impl Fn(u32, u32) -> u16) -> Vec<u8> {
    let row = (width * 3).next_multiple_of(4);
    let size = 54 + row * height;
    let mut bmp = Vec::with_capacity(size as usize);
    bmp.extend_from_slice(b"BM");
    for value in [size, 0, 54, 40, width, height] {
        bmp.extend_from_slice(&value.to_le_bytes());
    }
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    bmp.extend_from_slice(&[0; 24]);
    for y in (0..height).rev() {
        for x in 0..width {
            let p = u32::from(pixel(x, y));
            bmp.extend_from_slice(&[
                ((p & 31) * 255 / 31) as u8,
                ((p >> 5 & 63) * 255 / 63) as u8,
                ((p >> 11) * 255 / 31) as u8,
            ]);
        }
        bmp.resize(bmp.len() + (row - width * 3) as usize, 0);
    }
    bmp
}

/// Write the kept screenshot beside the session log (`maiko-....bmp`), where the launcher's crash
/// report looks for it. Never waits for the frame being stored.
pub(crate) fn write_snapshot() {
    // shortcut: from a fault handler this allocates for the file name; a fault inside the
    // allocator could stall here. Pre-open the file if that ever shows up in reports.
    let Some(path) = crate::logging::session_path().map(|log| log.with_extension("bmp")) else {
        return;
    };
    if let Ok(bmp) = SNAPSHOT.try_lock()
        && !bmp.is_empty()
    {
        let _ = std::fs::write(path, &*bmp);
    }
}

/// Log a stall (no frame for `STALL` seconds) once, with the recent guest calls and the last
/// screenshot, so a frozen game that the player has to kill still leaves evidence.
fn watch_for_stalls() {
    let mut reported = 0;
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let last = LAST_FRAME.load(Relaxed);
        if last == 0 || last == reported || now() - last < STALL * 1000 {
            continue;
        }
        reported = last;
        crate::rg_log!(
            "event=stall seconds={STALL} (no frame presented; recent guest calls follow)"
        );
        let mut report = Report::new();
        report.calls();
        report.output();
        write_snapshot();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn snapshots_are_bottom_up_bgr_bitmaps() {
        // 2x1: red, then white. Rows pad to four bytes.
        let bmp = super::encode_bmp(2, 1, |x, _| if x == 0 { 0xf800 } else { 0xffff });
        assert_eq!(&bmp[..2], b"BM");
        assert_eq!(u32::from_le_bytes(bmp[2..6].try_into().unwrap()), 62);
        assert_eq!(&bmp[54..62], &[0, 0, 255, 255, 255, 255, 0, 0]);
    }
}
