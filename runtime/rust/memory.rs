//! Guest-addressable allocation tracking around the pinned external SR heap.
use std::ffi::{c_int, c_void};
use std::sync::Mutex;

const MAX_ALLOCATIONS: usize = 49152;

unsafe extern "C" {
    fn x86_init_malloc() -> c_int;
    fn x86_malloc(size: u32) -> *mut c_void;
    fn x86_free(pointer: *mut c_void);
}

struct Heap {
    initialized: bool,
    allocations: Vec<(usize, usize)>,
}

static HEAP: Mutex<Heap> = Mutex::new(Heap {
    initialized: false,
    allocations: Vec::new(),
});

/// # Safety
/// `out` is writable for one u32. The pointer is tested numerically, not read.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_guest_pointer(
    pointer: *const c_void,
    size: usize,
    out: *mut u32,
) -> c_int {
    let address = pointer as usize;
    if address == 0
        || address > u32::MAX as usize
        || size as u64 > (1u64 << 32) - address as u64
        || out.is_null()
    {
        return 0;
    }
    unsafe {
        *out = address as u32;
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_low_alloc(size: usize) -> *mut c_void {
    if size == 0 || size > 0x10000000 {
        return std::ptr::null_mut();
    }
    let mut heap = HEAP.lock().unwrap_or_else(|e| e.into_inner());
    if !heap.initialized {
        if unsafe { x86_init_malloc() } != 0 {
            return std::ptr::null_mut();
        }
        heap.initialized = true;
    }
    if heap.allocations.len() == MAX_ALLOCATIONS {
        return std::ptr::null_mut();
    }
    let pointer = unsafe { x86_malloc(size as u32) };
    if pointer.is_null() {
        return pointer;
    }
    // The pinned SR allocator's header is two i32 fields: inverted size,
    // then block size including its 8-byte header. Its C API patch changes
    // linkage only. The actual allocator and capacity policy stay upstream.
    let block_size = unsafe { pointer.cast::<i32>().sub(1).read() };
    let Some(capacity) = usize::try_from(block_size)
        .ok()
        .and_then(|n| n.checked_sub(8))
    else {
        unsafe {
            x86_free(pointer);
        }
        return std::ptr::null_mut();
    };
    if capacity < size
        || (pointer as usize)
            .checked_add(capacity)
            .is_none_or(|end| end > 0x80000000)
    {
        unsafe {
            x86_free(pointer);
        }
        return std::ptr::null_mut();
    }
    unsafe {
        pointer.cast::<u8>().write_bytes(0, capacity);
    }
    heap.allocations.push((pointer as usize, capacity));
    pointer
}

/// # Safety
/// Caller has relinquished all references to this guest allocation. Unknown
/// and already freed addresses are rejected without touching their memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_low_free(pointer: *mut c_void) -> c_int {
    if pointer.is_null() {
        return 1;
    }
    let mut heap = HEAP.lock().unwrap_or_else(|e| e.into_inner());
    let Some(slot) = heap
        .allocations
        .iter()
        .position(|&(address, _)| address == pointer as usize)
    else {
        return 0;
    };
    unsafe {
        x86_free(pointer);
    }
    heap.allocations.swap_remove(slot);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_low_size(pointer: *const c_void) -> usize {
    HEAP.lock()
        .unwrap_or_else(|e| e.into_inner())
        .allocations
        .iter()
        .find(|&&(address, _)| address == pointer as usize)
        .map_or(0, |&(_, size)| size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_pointer_range_includes_last_byte_but_rejects_wraparound() {
        let mut out = 0;
        let cases = [
            (0usize, 0usize, false),
            (1, 0, true),
            (0xffff_ffff, 1, true),
            (0xffff_ffff, 2, false),
            (0x1_0000_0000, 0, false),
            (0x1000, usize::MAX, false),
        ];
        for (address, size, valid) in cases {
            assert_eq!(
                unsafe { rg_guest_pointer(address as *const c_void, size, &mut out) } != 0,
                valid
            );
            if valid {
                assert_eq!(out, address as u32);
            }
        }
        assert_eq!(
            unsafe { rg_guest_pointer(std::ptr::dangling::<c_void>(), 0, std::ptr::null_mut()) },
            0
        );
    }
}
