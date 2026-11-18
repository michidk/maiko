//! Stable host-service ABI used by the shared DOS runtime.
use crate::os;
use std::ffi::{c_int, c_void};
#[unsafe(no_mangle)]
pub extern "C" fn rg_host_setup() {
    os::setup();
}
/// Run registered CRT cleanup callbacks on every host.
pub fn exit_cleanly(code: i32) -> ! {
    crate::logging::exit("crt-cleanup", code);
    unsafe { libc::exit(code) }
}
pub fn rg_host_exit(code: u32) {
    crate::logging::exit("host-termination", code as i32);
    os::exit(code);
}
#[unsafe(no_mangle)]
pub extern "C" fn rg_host_readable(pointer: *const c_void, size: usize) -> c_int {
    let address = pointer as usize;
    let Some(end) = address.checked_add(size).filter(|_| address != 0) else {
        return 0;
    };
    os::readable(address, end)
}
/// # Safety
/// `pointer` and `size` describe the live mapping to pin or unpin.
pub unsafe fn rg_host_lock(pointer: *mut c_void, size: usize, unlock: c_int) -> c_int {
    unsafe { os::lock(pointer, size, unlock) }
}
/// Free and total bytes of a DOS drive number.
pub fn rg_host_disk(drive: u32) -> Option<(u64, u64)> {
    os::disk(drive)
}
/// Local hour, minute, second and centisecond.
pub fn rg_host_time() -> [u32; 4] {
    os::time()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapped_ranges_overflow_and_disk_time_contract() {
        let data = [42u8; 32];
        assert_eq!(rg_host_readable(data.as_ptr().cast(), data.len()), 1);
        assert_eq!(rg_host_readable(std::ptr::null(), 0), 0);
        assert_eq!(rg_host_readable(usize::MAX as *const c_void, 2), 0);
        let parts = rg_host_time();
        assert!(parts[0] < 24 && parts[1] < 60 && parts[2] < 61 && parts[3] < 100);
        let (available, total) = rg_host_disk(0).unwrap();
        assert!(available <= total && total > 0);
        assert_eq!(rg_host_disk(27), None);
    }
}
