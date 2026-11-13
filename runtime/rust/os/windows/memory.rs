//! Native memory queries.
use std::ffi::c_int;
pub(crate) fn readable(address: usize, end: usize) -> std::ffi::c_int {
    use windows_sys::Win32::System::Memory::*;
    let mut info = std::mem::MaybeUninit::<MEMORY_BASIC_INFORMATION>::zeroed();
    if unsafe {
        VirtualQuery(
            address as *const std::ffi::c_void,
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
pub(crate) fn memory() -> Option<(u64, u64)> {
    unsafe {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        let mut info: MEMORYSTATUSEX = std::mem::zeroed();
        info.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        (GlobalMemoryStatusEx(&mut info) != 0).then_some((info.ullAvailPhys, info.ullTotalPhys))
    }
}
pub(crate) fn guest_space() -> Option<(u64, u64)> {
    let base = crate::memory::guest_base() as u64;
    #[cfg(feature = "llasm")]
    let limit = base.checked_add(0x8000_0000)?;
    #[cfg(not(feature = "llasm"))]
    let limit = 0xe000_0000;
    let (mut cursor, mut total, mut largest) = (base.checked_add(0x2000_0000)?, 0, 0);

    use windows_sys::Win32::System::Memory::*;
    while cursor < limit {
        let mut info = std::mem::MaybeUninit::<MEMORY_BASIC_INFORMATION>::zeroed();
        if unsafe {
            VirtualQuery(
                cursor as usize as *const _,
                info.as_mut_ptr(),
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        } == 0
        {
            return None;
        }
        let info = unsafe { info.assume_init() };
        let end = (info.BaseAddress as u64 + info.RegionSize as u64).min(limit);
        if end <= cursor {
            return None;
        }
        if info.State == MEM_FREE {
            let bytes = end - cursor;
            total += bytes;
            largest = largest.max(bytes);
        }
        cursor = end;
    }
    Some((total, largest.min(0x10000000)))
}
