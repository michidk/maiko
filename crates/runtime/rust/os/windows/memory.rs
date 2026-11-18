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
pub(crate) fn free_ranges(start: u64, limit: u64) -> Option<Vec<(u64, u64)>> {
    use windows_sys::Win32::System::Memory::*;
    let (mut cursor, mut used) = (start, Vec::new());
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
        if info.State != MEM_FREE {
            used.push((cursor, end));
        }
        cursor = end;
    }
    Some(crate::memory::gaps(used, start, limit))
}
/// Map `size` bytes at exactly `address`; false if that range is taken.
pub(crate) fn map_at(address: u64, size: usize, writable: bool) -> bool {
    use windows_sys::Win32::System::Memory::*;
    let (kind, protect) = if writable {
        (MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE)
    } else {
        (MEM_RESERVE, PAGE_NOACCESS)
    };
    let mapped = unsafe { VirtualAlloc(address as usize as *const _, size, kind, protect) };
    if mapped as u64 == address {
        return true;
    }
    if !mapped.is_null() {
        unsafe { VirtualFree(mapped, 0, MEM_RELEASE) };
    }
    false
}
pub(crate) fn unmap(address: u64, _size: usize) {
    use windows_sys::Win32::System::Memory::*;
    unsafe { VirtualFree(address as usize as *mut _, 0, MEM_RELEASE) };
}
