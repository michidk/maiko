//! Darwin memory queries. Region ports are released even when a query fails.
use std::mem::{MaybeUninit, size_of};

// mach/vm_region.h uses four-byte packing for this MIG message on both CPUs.
#[repr(C, packed(4))]
struct Region {
    protection: i32,
    max_protection: i32,
    inheritance: u32,
    shared: i32,
    reserved: i32,
    offset: u64,
    behavior: i32,
    user_wired_count: u16,
}

unsafe extern "C" {
    static mach_task_self_: u32;
    fn mach_host_self() -> u32;
    fn mach_port_deallocate(task: u32, port: u32) -> i32;
    fn mach_vm_region(
        task: u32,
        address: *mut u64,
        size: *mut u64,
        flavor: i32,
        info: *mut i32,
        count: *mut u32,
        object: *mut u32,
    ) -> i32;
    fn host_page_size(host: u32, page: *mut u32) -> i32;
}

fn region(cursor: u64) -> Result<Option<(u64, u64, i32)>, ()> {
    let mut address = cursor;
    let mut length = 0;
    let mut object = 0;
    let mut info = MaybeUninit::<Region>::zeroed();
    let mut count = (size_of::<Region>() / size_of::<i32>()) as u32;
    let result = unsafe {
        mach_vm_region(
            mach_task_self_,
            &mut address,
            &mut length,
            9, // VM_REGION_BASIC_INFO_64
            info.as_mut_ptr().cast(),
            &mut count,
            &mut object,
        )
    };
    if object != 0 {
        unsafe { mach_port_deallocate(mach_task_self_, object) };
    }
    match result {
        1 => Ok(None), // KERN_INVALID_ADDRESS means no following mapping.
        0 if length != 0 && address.checked_add(length).is_some() => {
            Ok(Some((address, address + length, unsafe {
                info.assume_init().protection
            })))
        }
        _ => Err(()),
    }
}

pub fn readable(begin: usize, end: usize) -> bool {
    let mut cursor = begin as u64;
    loop {
        let Ok(Some((start, stop, protection))) = region(cursor) else {
            return false;
        };
        if start > cursor || stop <= cursor || protection & libc::VM_PROT_READ == 0 {
            return false;
        }
        if stop >= end as u64 {
            return true;
        }
        cursor = stop;
    }
}

pub fn memory() -> Option<(u64, u64)> {
    let mut total: u64 = 0;
    let mut length = size_of::<u64>();
    if unsafe {
        libc::sysctlbyname(
            c"hw.memsize".as_ptr(),
            (&raw mut total).cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    } != 0
        || length != size_of::<u64>()
    {
        return None;
    }
    let host = unsafe { mach_host_self() };
    let mut page = 0;
    let mut info = MaybeUninit::<libc::vm_statistics64_data_t>::zeroed();
    let mut count = libc::HOST_VM_INFO64_COUNT;
    let page_result = unsafe { host_page_size(host, &mut page) };
    let stats_result = unsafe {
        libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            info.as_mut_ptr().cast(),
            &mut count,
        )
    };
    unsafe { mach_port_deallocate(mach_task_self_, host) };
    if page_result != 0 || stats_result != 0 || page == 0 {
        return None;
    }
    let info = unsafe { info.assume_init() };
    let available = (u64::from(info.free_count) + u64::from(info.inactive_count)) * u64::from(page);
    Some((available.min(total), total))
}

pub fn guest_space() -> Option<(u64, u64)> {
    let base = crate::memory::guest_base() as u64;
    let (mut cursor, mut total, mut largest) = (base.checked_add(0x2000_0000)?, 0, 0);
    let end = base.checked_add(0x8000_0000)?;
    while cursor < end {
        let next = region(cursor).ok()?;
        let (start, stop) = next.map_or((end, end), |(start, stop, _)| (start.min(end), stop));
        if start > cursor {
            let gap = start - cursor;
            total += gap;
            largest = largest.max(gap);
        }
        if stop <= cursor {
            return None;
        }
        cursor = stop;
    }
    Some((total, largest.min(0x1000_0000)))
}

/// # Safety
/// Both outputs are writable for one u64.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_mac_memory(available: *mut u64, total: *mut u64) -> i32 {
    let Some((a, t)) = memory() else {
        return 0;
    };
    unsafe {
        *available = a;
        *total = t;
    }
    1
}

/// # Safety
/// Both outputs are writable for one u64.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_mac_guest_space(available: *mut u64, largest: *mut u64) -> i32 {
    let Some((a, l)) = guest_space() else {
        return 0;
    };
    unsafe {
        *available = a;
        *largest = l;
    }
    1
}
