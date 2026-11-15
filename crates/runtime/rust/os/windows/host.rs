//! Native process, mapping, disk and clock operations.
use std::ffi::{c_int, c_void};
pub(crate) fn setup() {
    use windows_sys::Win32::System::Diagnostics::Debug::*;
    unsafe {
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX);
    }
}
pub(crate) fn exit(code: u32) {
    unsafe {
        windows_sys::Win32::System::Threading::ExitProcess(code);
    }
}
pub(crate) unsafe fn lock(pointer: *mut c_void, size: usize, unlock: c_int) -> c_int {
    unsafe {
        use windows_sys::Win32::System::Memory::{VirtualLock, VirtualUnlock};
        (if unlock != 0 {
            VirtualUnlock(pointer, size)
        } else {
            VirtualLock(pointer, size)
        } != 0) as c_int
    }
}
pub(crate) unsafe fn disk(drive: u32, available: *mut u64, total: *mut u64) -> c_int {
    unsafe {
        use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExA;
        if drive > 26 {
            return 0;
        }
        let root = [b'A' + drive.saturating_sub(1) as u8, b':', b'\\', 0];
        (GetDiskFreeSpaceExA(
            if drive == 0 {
                std::ptr::null()
            } else {
                root.as_ptr()
            },
            available,
            total,
            std::ptr::null_mut(),
        ) != 0) as c_int
    }
}
pub(crate) unsafe fn time(parts: *mut u32) {
    let fields = unsafe {
        use windows_sys::Win32::Foundation::SYSTEMTIME;
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        let mut time = std::mem::MaybeUninit::<SYSTEMTIME>::zeroed();
        GetLocalTime(time.as_mut_ptr());
        let time = time.assume_init();
        [
            u32::from(time.wHour),
            u32::from(time.wMinute),
            u32::from(time.wSecond),
            u32::from(time.wMilliseconds / 10),
        ]
    };
    unsafe {
        parts.copy_from_nonoverlapping(fields.as_ptr(), 4);
    }
}
