//! Guest-addressable heap: free-list blocks mapped within 2 GiB of the guest base.
use crate::{os, rg_log};
use std::alloc::Layout;
use std::ffi::{c_int, c_void};
use std::ptr::NonNull;
use std::sync::Mutex;
#[cfg(feature = "llasm")]
use std::sync::OnceLock;

const MAX_ALLOCATIONS: usize = 49152;
const BLOCK: usize = 1 << 20;
/// Windows' allocation granularity; also a multiple of every Unix page size.
const GRANULE: u64 = 0x10000;
/// The guest's lowest 1 MiB + 64 KiB stays unmapped, as DOS reserved it.
const LOW_START: u64 = 0x11_0000;
const GUEST_WINDOW: u64 = 0x8000_0000;
const ALIGN: usize = 16;

#[cfg(feature = "llasm")]
static POINTER_OFFSET: OnceLock<Option<usize>> = OnceLock::new();

pub fn guest_base() -> usize {
    #[cfg(feature = "llasm")]
    {
        POINTER_OFFSET.get().copied().flatten().unwrap_or(0)
    }
    #[cfg(not(feature = "llasm"))]
    {
        0
    }
}

#[cfg(feature = "llasm")]
unsafe extern "C" {
    /// Set by the translated engine; receives the base its guest pointers are relative to.
    static ptr_initialize_pointers: Option<unsafe extern "C" fn(u64)>;
}

/// Choose the guest base: the highest free gap below the engine module, reserved inaccessible.
#[cfg(feature = "llasm")]
fn initialize_pointer_offset() -> Option<usize> {
    let mut info = std::mem::MaybeUninit::<libc::Dl_info>::zeroed();
    if unsafe {
        libc::dladdr(
            (&raw const ptr_initialize_pointers).cast(),
            info.as_mut_ptr(),
        )
    } == 0
    {
        return None;
    }
    let module = unsafe { info.assume_init() }.dli_fbase as u64;
    if module == 0 {
        return None;
    }
    let offset = if module <= 2 * LOW_START {
        0
    } else {
        let (_, end) = os::free_ranges(0, module)?
            .into_iter()
            .rev()
            .find(|(start, end)| end - start >= LOW_START)?;
        let offset = end - LOW_START;
        // Best effort: the offset stays valid even if another mapping took the gap meanwhile.
        os::map_at(offset, LOW_START as usize, false);
        offset
    };
    if let Some(initialize) = unsafe { ptr_initialize_pointers } {
        unsafe { initialize(offset) };
    }
    Some(offset as usize)
}

/// Free address ranges `[start, end)` between `start` and `limit`, given sorted used ranges.
pub(crate) fn gaps(
    used: impl IntoIterator<Item = (u64, u64)>,
    start: u64,
    limit: u64,
) -> Vec<(u64, u64)> {
    let mut cursor = start;
    let mut free = Vec::new();
    for (begin, end) in used {
        if cursor >= limit || begin >= limit {
            break;
        }
        if begin > cursor {
            free.push((cursor, begin));
        }
        cursor = cursor.max(end);
    }
    if cursor < limit {
        free.push((cursor, limit));
    }
    free
}

/// Free guest address space and its largest run, as reported to the original memory checks.
pub(crate) fn guest_space() -> Option<(u64, u64)> {
    let base = guest_base() as u64;
    #[cfg(feature = "llasm")]
    let limit = base.checked_add(GUEST_WINDOW)?;
    #[cfg(not(feature = "llasm"))]
    let limit = 0xe000_0000;
    let free = os::free_ranges(base.checked_add(0x2000_0000)?, limit)?;
    let total = free.iter().map(|(start, end)| end - start).sum();
    let largest = free
        .iter()
        .map(|(start, end)| end - start)
        .max()
        .unwrap_or(0);
    Some((total, largest.min(0x1000_0000)))
}

/// Map `size` bytes of read-write memory in the guest window.
fn map_low(size: usize) -> Option<usize> {
    let base = guest_base() as u64;
    for (start, end) in os::free_ranges(base + LOW_START, base + GUEST_WINDOW)? {
        let address = start.next_multiple_of(GRANULE);
        if address
            .checked_add(size as u64)
            .is_some_and(|stop| stop <= end)
            && os::map_at(address, size, true)
        {
            return Some(address as usize);
        }
    }
    None
}

/// Decode a guest address. Zero is a null pointer, including with an offset base.
pub fn guest_ptr<T>(address: u32) -> *mut T {
    if address == 0 {
        std::ptr::null_mut()
    } else {
        (guest_base() + address as usize) as *mut T
    }
}

/// Encode a pointer to translated code or guest storage. Host-only allocations
/// must never silently truncate into a guest address.
pub fn guest_address<T>(pointer: *const T) -> u32 {
    if pointer.is_null() {
        return 0;
    }
    let mut address = 0;
    if unsafe { rg_guest_pointer(pointer.cast(), 0, &mut address) } == 0 {
        rg_log!("BLOCKED host pointer outside guest address window: {pointer:p}");
        std::process::abort();
    }
    address
}

struct Block {
    start: usize,
    size: usize,
    free: linked_list_allocator::Heap,
}

struct Heap {
    blocks: Vec<Block>,
    allocations: Vec<(usize, usize)>,
}

static HEAP: Mutex<Heap> = Mutex::new(Heap {
    blocks: Vec::new(),
    allocations: Vec::new(),
});

fn layout(capacity: usize) -> Layout {
    Layout::from_size_align(capacity, ALIGN).expect("capacity is at most 256 MiB")
}

/// # Safety
/// `out` is writable for one u32. The pointer is tested numerically, not read.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_guest_pointer(
    pointer: *const c_void,
    size: usize,
    out: *mut u32,
) -> c_int {
    let Some(address) = (pointer as usize).checked_sub(guest_base()) else {
        return 0;
    };
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
    #[cfg(feature = "llasm")]
    if POINTER_OFFSET
        .get_or_init(initialize_pointer_offset)
        .is_none()
    {
        return std::ptr::null_mut();
    }
    if heap.allocations.len() == MAX_ALLOCATIONS {
        return std::ptr::null_mut();
    }
    let capacity = size.next_multiple_of(ALIGN);
    let found = heap
        .blocks
        .iter_mut()
        .find_map(|block| block.free.allocate_first_fit(layout(capacity)).ok());
    let pointer = match found {
        Some(pointer) => pointer,
        None => {
            let block_size = capacity.next_multiple_of(BLOCK);
            let Some(start) = map_low(block_size) else {
                return std::ptr::null_mut();
            };
            let mut free = linked_list_allocator::Heap::empty();
            // SAFETY: the block was just mapped read-write for `block_size` bytes and is owned here.
            unsafe { free.init(start as *mut u8, block_size) };
            let pointer = free.allocate_first_fit(layout(capacity));
            heap.blocks.push(Block {
                start,
                size: block_size,
                free,
            });
            let Ok(pointer) = pointer else {
                return std::ptr::null_mut();
            };
            pointer
        }
    };
    let pointer = pointer.as_ptr();
    // SAFETY: the allocator handed out `capacity` bytes at `pointer`.
    unsafe { pointer.write_bytes(0, capacity) };
    heap.allocations.push((pointer as usize, capacity));
    pointer.cast()
}

/// # Safety
/// Caller has relinquished all references to this guest allocation. Unknown
/// and already freed addresses are rejected without touching their memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_low_free(pointer: *mut c_void) -> c_int {
    let Some(pointer) = NonNull::new(pointer.cast::<u8>()) else {
        return 1;
    };
    let address = pointer.as_ptr() as usize;
    let mut heap = HEAP.lock().unwrap_or_else(|e| e.into_inner());
    let Some(slot) = heap.allocations.iter().position(|&(a, _)| a == address) else {
        return 0;
    };
    let (_, capacity) = heap.allocations.swap_remove(slot);
    let index = heap
        .blocks
        .iter()
        .position(|b| (b.start..b.start + b.size).contains(&address))
        .expect("every allocation lies in a block");
    let block = &mut heap.blocks[index];
    // SAFETY: the allocation came from this block with this layout and is freed once.
    unsafe { block.free.deallocate(pointer, layout(capacity)) };
    // Return empty blocks, keeping one so small churn does not remap.
    if block.free.used() == 0 && heap.blocks.len() > 1 {
        let block = heap.blocks.swap_remove(index);
        os::unmap(block.start as u64, block.size);
    }
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

    #[test]
    fn gaps_cover_unused_space_inside_the_window() {
        let used = [
            (0x0, 0x2000),
            (0x3000, 0x4000),
            (0x3800, 0x5000),
            (0x9000, 0xa000),
        ];
        assert_eq!(
            gaps(used, 0x1000, 0x9800),
            [(0x2000, 0x3000), (0x5000, 0x9000)]
        );
        assert_eq!(gaps([], 0x1000, 0x2000), [(0x1000, 0x2000)]);
        assert_eq!(gaps(used, 0x9000, 0xa000), []);
    }

    #[test]
    fn allocations_are_zeroed_low_and_reusable() {
        let sizes = [1, 24, 4096, BLOCK + 1, 3 * BLOCK];
        let pointers = sizes.map(|size| {
            let pointer = rg_low_alloc(size).cast::<u8>();
            assert!(!pointer.is_null());
            let capacity = rg_low_size(pointer.cast());
            assert!(capacity >= size && capacity.is_multiple_of(ALIGN));
            let end = pointer as usize + capacity - guest_base();
            assert!((pointer as usize).is_multiple_of(ALIGN) && end <= GUEST_WINDOW as usize);
            let bytes = unsafe { std::slice::from_raw_parts_mut(pointer, capacity) };
            assert!(bytes.iter().all(|&b| b == 0));
            bytes.fill(0xa5);
            pointer
        });
        for pointer in pointers {
            assert_eq!(unsafe { rg_low_free(pointer.cast()) }, 1);
            assert_eq!(unsafe { rg_low_free(pointer.cast()) }, 0, "double free");
        }
        let again = rg_low_alloc(24).cast::<u8>();
        assert!(
            unsafe { std::slice::from_raw_parts(again, 32) }
                .iter()
                .all(|&b| b == 0)
        );
        assert_eq!(unsafe { rg_low_free(again.cast()) }, 1);
        assert_eq!(unsafe { rg_low_free(std::ptr::null_mut()) }, 1);
        assert_eq!(rg_low_alloc(0), std::ptr::null_mut());
    }
}
