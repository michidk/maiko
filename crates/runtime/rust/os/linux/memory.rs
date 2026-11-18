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
pub(crate) fn free_ranges(start: u64, limit: u64) -> Option<Vec<(u64, u64)>> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    let mut used = Vec::new();
    for line in maps.lines() {
        let (begin, end) = line.split_whitespace().next()?.split_once('-')?;
        used.push((
            u64::from_str_radix(begin, 16).ok()?,
            u64::from_str_radix(end, 16).ok()?,
        ));
    }
    Some(crate::memory::gaps(used, start, limit))
}
