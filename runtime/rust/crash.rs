//! Fatal reporting without allocation, locks, unwinding or exit cleanup.
use std::fmt::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
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
}
