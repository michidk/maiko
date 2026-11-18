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
/// Free and total bytes of the current drive (DOS drive 0, or C: as 3).
pub(crate) fn disk(drive: u32) -> Option<(u64, u64)> {
    if drive != 0 && drive != 3 {
        return None;
    }
    let mut info = std::mem::MaybeUninit::<libc::statvfs>::zeroed();
    if unsafe { libc::statvfs(c".".as_ptr(), info.as_mut_ptr()) } != 0 {
        return None;
    }
    {
        let info = unsafe { info.assume_init() };
        #[cfg(target_os = "macos")]
        let (available_blocks, total_blocks, block_size) = (
            u64::from(info.f_bavail),
            u64::from(info.f_blocks),
            info.f_frsize,
        );
        #[cfg(not(target_os = "macos"))]
        let (available_blocks, total_blocks, block_size) =
            (info.f_bavail, info.f_blocks, info.f_frsize);
        Some((
            available_blocks.saturating_mul(block_size),
            total_blocks.saturating_mul(block_size),
        ))
    }
}
/// Local hour, minute, second and centisecond; zero if the clock cannot be read.
pub(crate) fn time() -> [u32; 4] {
    unsafe {
        let mut clock = std::mem::MaybeUninit::<libc::timespec>::zeroed();
        if libc::clock_gettime(libc::CLOCK_REALTIME, clock.as_mut_ptr()) != 0 {
            return [0; 4];
        }
        let clock = clock.assume_init();
        let mut local = std::mem::MaybeUninit::<libc::tm>::zeroed();
        if libc::localtime_r(&clock.tv_sec, local.as_mut_ptr()).is_null() {
            return [0; 4];
        }
        let local = local.assume_init();
        [
            local.tm_hour as u32,
            local.tm_min as u32,
            local.tm_sec as u32,
            (clock.tv_nsec / 10000000) as u32,
        ]
    }
}
