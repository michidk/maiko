//! Operating-system services beneath the shared DOS runtime.
use std::ffi::{c_int, c_void};

#[unsafe(no_mangle)]
pub extern "C" fn rg_host_setup() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Diagnostics::Debug::*;
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX);
    }
}

/// Terminate normal game execution through the CRT so external SDL/SR cleanup
/// callbacks and C ABI fixture callbacks run on Windows as well as Linux.
pub fn exit_cleanly(code: i32) -> ! {
    unsafe { libc::exit(code) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_host_exit(code: u32) {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Threading::ExitProcess(code);
    }
    #[cfg(unix)]
    std::process::exit(code as i32);
}

#[unsafe(no_mangle)]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "VirtualQuery inspects arbitrary addresses without dereferencing them"
)]
pub extern "C" fn rg_host_readable(pointer: *const c_void, size: usize) -> c_int {
    let address = pointer as usize;
    let Some(end) = address.checked_add(size).filter(|_| address != 0) else {
        return 0;
    };
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Memory::*;
        let mut info = std::mem::MaybeUninit::<MEMORY_BASIC_INFORMATION>::zeroed();
        if unsafe {
            VirtualQuery(
                pointer,
                info.as_mut_ptr(),
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        } == 0
        {
            return 0;
        }
        let info = unsafe { info.assume_init() };
        (info.State == MEM_COMMIT
            && info.Protect & (PAGE_NOACCESS | PAGE_GUARD) == 0
            && (info.BaseAddress as usize)
                .checked_add(info.RegionSize)
                .is_some_and(|region_end| end <= region_end)) as c_int
    }
    #[cfg(unix)]
    {
        use std::io::{BufRead, BufReader};
        let Ok(file) = std::fs::File::open("/proc/self/maps") else {
            return 0;
        };
        BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .any(|line| {
                let mut fields = line.split_whitespace();
                let Some(range) = fields.next() else {
                    return false;
                };
                let Some(mode) = fields.next() else {
                    return false;
                };
                let Some((start, stop)) = range.split_once('-') else {
                    return false;
                };
                let (Ok(start), Ok(stop)) = (
                    usize::from_str_radix(start, 16),
                    usize::from_str_radix(stop, 16),
                ) else {
                    return false;
                };
                mode.starts_with('r') && address >= start && end <= stop
            }) as c_int
    }
}

/// # Safety
/// `pointer` and `size` describe the live mapping to pin or unpin.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_host_lock(pointer: *mut c_void, size: usize, unlock: c_int) -> c_int {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Memory::{VirtualLock, VirtualUnlock};
        (if unlock != 0 {
            VirtualUnlock(pointer, size)
        } else {
            VirtualLock(pointer, size)
        } != 0) as c_int
    }
    #[cfg(unix)]
    unsafe {
        (if unlock != 0 {
            libc::munlock(pointer, size)
        } else {
            libc::mlock(pointer, size)
        } == 0) as c_int
    }
}

/// # Safety
/// Both output pointers are writable u64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_host_disk(drive: u32, available: *mut u64, total: *mut u64) -> c_int {
    if available.is_null() || total.is_null() {
        return 0;
    }
    #[cfg(windows)]
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
    #[cfg(unix)]
    unsafe {
        if drive != 0 && drive != 3 {
            return 0;
        }
        let mut info = std::mem::MaybeUninit::<libc::statvfs>::zeroed();
        if libc::statvfs(c".".as_ptr(), info.as_mut_ptr()) != 0 {
            return 0;
        }
        let info = info.assume_init();
        *available = info.f_bavail * info.f_frsize;
        *total = info.f_blocks * info.f_frsize;
        1
    }
}

/// # Safety
/// `parts` is writable for four u32 fields: hour, minute, second, centisecond.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_host_time(parts: *mut u32) {
    if parts.is_null() {
        return;
    }
    #[cfg(windows)]
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
    #[cfg(unix)]
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
