//! The original DOS machine inventory and interrupt adapters on native hosts.
use crate::guest::GuestFrame;
use crate::input::rg_mouse_service;
use crate::memory::rg_low_alloc;
use crate::memory::{guest_address, guest_ptr};
use std::ffi::{c_char, c_int};
use std::sync::Mutex;

unsafe extern "C" {
    fn rg_host_cwd(capacity: u32, buffer: *mut c_char) -> u32;
}
static INVENTORY: Mutex<usize> = Mutex::new(0);

fn memory() -> Option<(u64, u64)> {
    #[cfg(target_os = "macos")]
    {
        crate::darwin::memory()
    }
    #[cfg(target_os = "linux")]
    unsafe {
        let mut info = std::mem::MaybeUninit::<libc::sysinfo>::zeroed();
        if libc::sysinfo(info.as_mut_ptr()) != 0 {
            return None;
        }
        let info = info.assume_init();
        Some((
            info.freeram * u64::from(info.mem_unit),
            info.totalram * u64::from(info.mem_unit),
        ))
    }
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        let mut info: MEMORYSTATUSEX = std::mem::zeroed();
        info.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        (GlobalMemoryStatusEx(&mut info) != 0).then_some((info.ullAvailPhys, info.ullTotalPhys))
    }
}

fn guest_space() -> Option<(u64, u64)> {
    #[cfg(target_os = "macos")]
    return crate::darwin::guest_space();
    #[cfg(not(target_os = "macos"))]
    {
        let base = crate::memory::guest_base() as u64;
        #[cfg(feature = "llasm")]
        let limit = base.checked_add(0x8000_0000)?;
        #[cfg(not(feature = "llasm"))]
        let limit = 0xe000_0000;
        let (mut cursor, mut total, mut largest) = (base.checked_add(0x2000_0000)?, 0, 0);
        #[cfg(unix)]
        {
            let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
            for line in maps.lines() {
                let (start, end) = line.split_whitespace().next()?.split_once('-')?;
                let (start, end) = (
                    u64::from_str_radix(start, 16).ok()?,
                    u64::from_str_radix(end, 16).ok()?,
                );
                if end <= cursor {
                    continue;
                }
                let start = start.min(limit);
                if start > cursor {
                    let bytes = start - cursor;
                    total += bytes;
                    largest = largest.max(bytes);
                }
                cursor = cursor.max(end);
                if cursor >= limit {
                    break;
                }
            }
        }
        #[cfg(windows)]
        {
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
        }
        Some((total, largest.min(0x10000000)))
    }
}

/// # Safety
/// The frame and the input/output register blocks point to mapped game memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_int386(frame: *mut GuestFrame) -> c_int {
    let Some(frame) = (unsafe { frame.as_mut() }) else {
        return 0;
    };
    let mut registers = unsafe { (guest_ptr::<[u32; 7]>(frame.edx as u32)).read_unaligned() };
    match frame.eax & 255 {
        0x33 => {
            if unsafe { rg_mouse_service(registers.as_mut_ptr()) } == 0 {
                return 0;
            }
        }
        0x31 if registers[0] == 0x500 => {
            let Some((available, total)) = memory() else {
                return 0;
            };
            let Some((space, largest)) = guest_space() else {
                return 0;
            };
            if space == 0 {
                return 0;
            }
            let pages = (space / 4096) as u32;
            let mut info = [0u32; 12];
            info[0] = largest as u32;
            info[1] = pages;
            info[2] = 0xc0000000 / 4096;
            info[3] = pages;
            info[4] = (available / 4096).min(u64::from(pages)) as u32;
            info[5] = info[4];
            info[6] = (total / 4096).min(u64::from(info[2])) as u32;
            info[7] = pages;
            unsafe {
                (guest_ptr::<[u32; 12]>(registers[5])).write_unaligned(info);
            }
            eprintln!("Native guest memory largest={largest} available_pages={pages}");
        }
        _ => return 0,
    }
    registers[6] = 0;
    unsafe {
        (guest_ptr::<[u32; 7]>(frame.ebx as u32)).write_unaligned(registers);
    }
    frame.eax = registers[0].into();
    1
}

/// # Safety
/// The frame's optional buffer has room for the original 444-byte inventory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_platform_info(frame: *mut GuestFrame) {
    let Some(frame) = (unsafe { frame.as_mut() }) else {
        return;
    };
    let mut cached = INVENTORY.lock().unwrap_or_else(|e| e.into_inner());
    let pointer = if frame.eax as u32 == 0 {
        if *cached == 0 {
            *cached = rg_low_alloc(0x1bc) as usize;
        }
        *cached
    } else {
        guest_ptr::<u8>(frame.eax as u32) as usize
    };
    if pointer == 0 {
        frame.eax = 0;
        return;
    }
    let info = unsafe { &mut *(pointer as *mut [u8; 0x1bc]) };
    info.fill(0);
    info[0xd] = 1;
    if let Some((available, _)) = memory() {
        info[0xe..0x12].copy_from_slice(&(available.min(0xc0000000) as u32).to_le_bytes());
    }
    if unsafe { rg_host_cwd(128, info.as_mut_ptr().add(0x40).cast()) } != 0 {
        info[0x3f] = info[0x40];
    }
    #[cfg(unix)]
    {
        info[0x28] = 1;
        info[0xd0] = 1;
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::GetLogicalDrives;
        let drives = unsafe { GetLogicalDrives() };
        for drive in 3..26 {
            if drives & (1 << (drive - 1)) != 0 {
                info[0x25 + drive] = 1;
                info[0xd0] += 1;
            }
        }
    }
    for (offset, text) in [
        (
            0xfc,
            &b"Native x64 host; legacy CPU calibration disabled"[..],
        ),
        (0x13c, &b"Native Glide backend not initialized"[..]),
    ] {
        info[offset..offset + text.len()].copy_from_slice(text);
    }
    let platform = if cfg!(windows) {
        "Windows native x64"
    } else if cfg!(target_os = "macos") {
        "macOS native"
    } else {
        "Linux native x64"
    };
    info[0x17c..0x17c + platform.len()].copy_from_slice(platform.as_bytes());
    frame.eax = guest_address(pointer as *const u8).into();
    eprintln!("Native platform inventory: {platform}; DOS/BIOS features absent");
}

/// # Safety
/// `frame` points to a valid guest register frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_video_service(frame: *const GuestFrame) -> c_int {
    if unsafe { frame.as_ref() }.is_none_or(|f| f.eax & 65535 != 3) {
        return 0;
    }
    eprintln!("Original text-mode3 restore uses existing native console");
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guest_space_reports_allocator_limits() {
        let (available, largest) = guest_space().unwrap();
        assert!(available > 0 && available <= 0xc0000000);
        assert!(largest > 0 && largest <= 0x10000000 && largest <= available);
    }
}
