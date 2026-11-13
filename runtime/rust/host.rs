//! Stable host-service ABI used by the shared DOS runtime.
use crate::os;
use std::ffi::{c_int, c_void};
#[unsafe(no_mangle)]
pub extern "C" fn rg_host_setup() {
    os::setup();
}
/// Run registered CRT cleanup callbacks on every host.
pub fn exit_cleanly(code: i32) -> ! {
    unsafe { libc::exit(code) }
}
#[unsafe(no_mangle)]
pub extern "C" fn rg_host_exit(code: u32) {
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_host_lock(pointer: *mut c_void, size: usize, unlock: c_int) -> c_int {
    unsafe { os::lock(pointer, size, unlock) }
}
/// # Safety
/// Both output pointers are writable u64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_host_disk(drive: u32, available: *mut u64, total: *mut u64) -> c_int {
    if available.is_null() || total.is_null() {
        return 0;
    }
    unsafe { os::disk(drive, available, total) }
}
/// # Safety
/// `parts` is writable for four u32 fields: hour, minute, second, centisecond.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_host_time(parts: *mut u32) {
    if parts.is_null() {
        return;
    }
    unsafe { os::time(parts) }
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
        let mut parts = [u32::MAX; 4];
        unsafe {
            rg_host_time(parts.as_mut_ptr());
        }
        assert!(parts[0] < 24 && parts[1] < 60 && parts[2] < 61 && parts[3] < 100);
        let (mut available, mut total) = (0, 0);
        assert_eq!(unsafe { rg_host_disk(0, &mut available, &mut total) }, 1);
        assert!(available <= total && total > 0);
        assert_eq!(unsafe { rg_host_disk(27, &mut available, &mut total) }, 0);
    }
}
