//! Anonymous mappings at chosen addresses.

/// Map `size` bytes at exactly `address`; false if that range is taken.
pub(crate) fn map_at(address: u64, size: usize, writable: bool) -> bool {
    let (protection, reserve) = if writable {
        (libc::PROT_READ | libc::PROT_WRITE, 0)
    } else {
        (libc::PROT_NONE, libc::MAP_NORESERVE)
    };
    // A hint, not MAP_FIXED: the kernel never replaces an existing mapping.
    let mapped = unsafe {
        libc::mmap(
            address as usize as *mut _,
            size,
            protection,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | reserve,
            -1,
            0,
        )
    };
    if mapped as u64 == address {
        return true;
    }
    if mapped != libc::MAP_FAILED {
        unsafe { libc::munmap(mapped, size) };
    }
    false
}

pub(crate) fn unmap(address: u64, size: usize) {
    unsafe { libc::munmap(address as usize as *mut _, size) };
}
