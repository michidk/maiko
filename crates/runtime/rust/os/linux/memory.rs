//! Native memory queries.
use std::ffi::c_int;
pub(crate) fn readable(address: usize, end: usize) -> std::ffi::c_int {
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
pub(crate) fn memory() -> Option<(u64, u64)> {
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
}
pub(crate) fn guest_space() -> Option<(u64, u64)> {
    let base = crate::memory::guest_base() as u64;
    #[cfg(feature = "llasm")]
    let limit = base.checked_add(0x8000_0000)?;
    #[cfg(not(feature = "llasm"))]
    let limit = 0xe000_0000;
    let (mut cursor, mut total, mut largest) = (base.checked_add(0x2000_0000)?, 0, 0);

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
    Some((total, largest.min(0x10000000)))
}
