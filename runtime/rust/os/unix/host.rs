//! Native process, mapping, disk and clock operations.
use std::ffi::{c_int, c_void};
pub(crate) fn setup() {}
pub(crate) fn exit(code: u32) {
    std::process::exit(code as i32);
}
pub(crate) unsafe fn lock(pointer: *mut c_void, size: usize, unlock: c_int) -> c_int {
    unsafe {
        (if unlock != 0 {
            libc::munlock(pointer, size)
        } else {
            libc::mlock(pointer, size)
        } == 0) as c_int
    }
}
pub(crate) unsafe fn disk(drive: u32, available: *mut u64, total: *mut u64) -> c_int {
    unsafe {
        if drive != 0 && drive != 3 {
            return 0;
        }
        let mut info = std::mem::MaybeUninit::<libc::statvfs>::zeroed();
        if libc::statvfs(c".".as_ptr(), info.as_mut_ptr()) != 0 {
            return 0;
        }
        let info = info.assume_init();
        #[cfg(target_os = "macos")]
        let (available_blocks, total_blocks, block_size) = (
            u64::from(info.f_bavail),
            u64::from(info.f_blocks),
            u64::from(info.f_frsize),
        );
        #[cfg(not(target_os = "macos"))]
        let (available_blocks, total_blocks, block_size) =
            (info.f_bavail, info.f_blocks, info.f_frsize);
        *available = available_blocks.saturating_mul(block_size);
        *total = total_blocks.saturating_mul(block_size);
        1
    }
}
pub(crate) unsafe fn time(parts: *mut u32) {
    let fields = unsafe {
        let mut clock = std::mem::MaybeUninit::<libc::timespec>::zeroed();
        if libc::clock_gettime(libc::CLOCK_REALTIME, clock.as_mut_ptr()) != 0 {
            return;
        }
        let clock = clock.assume_init();
        let mut local = std::mem::MaybeUninit::<libc::tm>::zeroed();
        if libc::localtime_r(&clock.tv_sec, local.as_mut_ptr()).is_null() {
            return;
        }
        let local = local.assume_init();
        [
            local.tm_hour as u32,
            local.tm_min as u32,
            local.tm_sec as u32,
            (clock.tv_nsec / 10000000) as u32,
        ]
    };
    unsafe {
        parts.copy_from_nonoverlapping(fields.as_ptr(), 4);
    }
}
