//! The original DOS machine inventory and interrupt adapters on native hosts.
use crate::guest::GuestFrame;
use crate::input::rg_mouse_service;
use crate::memory::rg_low_alloc;
use crate::memory::{guest_address, guest_ptr};
use crate::rg_log;
use std::ffi::{c_char, c_int};
use std::sync::Mutex;

unsafe extern "C" {
    fn rg_host_cwd(capacity: u32, buffer: *mut c_char) -> u32;
}
static INVENTORY: Mutex<usize> = Mutex::new(0);

use crate::os::{guest_space, memory};

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
            rg_log!("Native guest memory largest={largest} available_pages={pages}");
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
    for drive in 0..26 {
        if crate::os::logical_drives() & (1 << drive) != 0 {
            info[0x26 + drive] = 1;
            info[0xd0] += 1;
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
    let platform = crate::os::NAME;
    info[0x17c..0x17c + platform.len()].copy_from_slice(platform.as_bytes());
    frame.eax = guest_address(pointer as *const u8).into();
    rg_log!("Native platform inventory: {platform}; DOS/BIOS features absent");
}

/// # Safety
/// `frame` points to a valid guest register frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_video_service(frame: *const GuestFrame) -> c_int {
    if unsafe { frame.as_ref() }.is_none_or(|f| f.eax & 65535 != 3) {
        return 0;
    }
    rg_log!("Original text-mode3 restore uses existing native console");
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
