//! Read-only ISO root files exposed to the original DOS movie player.
use crate::guest::GuestFrame;
use crate::memory::guest_ptr;
use crate::rg_log;
use std::ffi::{CStr, c_char, c_int};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

const SECTOR_DATA: u64 = 2048;
const FIRST_HANDLE: u32 = 512;
const HANDLE_COUNT: usize = 64;

struct Entry {
    name: Vec<u8>,
    extent: u32,
    size: u32,
}

#[derive(Clone, Copy)]
struct Handle {
    entry: usize,
    position: u32,
}

struct Media {
    file: File,
    length: u64,
    sector: u64,
    header: u64,
    entries: Vec<Entry>,
    handles: [Option<Handle>; HANDLE_COUNT],
}

static MEDIA: Mutex<Option<Media>> = Mutex::new(None);

fn both32(bytes: &[u8]) -> Option<u32> {
    let value = u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?);
    (value == u32::from_be_bytes(bytes.get(4..8)?.try_into().ok()?)).then_some(value)
}

impl Media {
    fn read(&mut self, offset: u64, out: &mut [u8]) -> Option<()> {
        if offset.checked_add(out.len() as u64)? > self.length {
            return None;
        }
        self.file.seek(SeekFrom::Start(offset)).ok()?;
        self.file.read_exact(out).ok()
    }

    fn valid_extent(&self, extent: u32, size: u32, volume: u32) -> bool {
        let end = u64::from(extent) + u64::from(size).div_ceil(SECTOR_DATA);
        end <= u64::from(volume) && end * self.sector <= self.length
    }

    fn open(path: &Path) -> Option<Self> {
        let file = File::open(path).ok()?;
        let length = file.metadata().ok()?.len();
        let mut media = Self {
            file,
            length,
            sector: 2048,
            header: 0,
            entries: Vec::new(),
            handles: [None; HANDLE_COUNT],
        };
        let mut pvd = [0; 2048];
        if media.read(16 * 2048, &mut pvd).is_none() || &pvd[..7] != b"\x01CD001\x01" {
            media.sector = 2352;
            media.header = 16;
            media.read(16 * 2352 + 16, &mut pvd)?;
            if &pvd[..7] != b"\x01CD001\x01" {
                return None;
            }
        }
        let volume = both32(&pvd[80..88])?;
        let root = both32(&pvd[158..166])?;
        let size = both32(&pvd[166..174])?;
        if pvd[128..132] != [0, 8, 8, 0]
            || pvd[156] < 34
            || pvd[181] & 2 == 0
            || pvd[157] != 0
            || u64::from(volume) * media.sector > media.length
            || size == 0
            || size > 16 * 1024 * 1024
            || !media.valid_extent(root, size, volume)
        {
            return None;
        }
        for block in 0..u64::from(size).div_ceil(SECTOR_DATA) {
            let mut data = [0; 2048];
            media.read(
                (u64::from(root) + block) * media.sector + media.header,
                &mut data,
            )?;
            let limit = (u64::from(size) - block * SECTOR_DATA).min(SECTOR_DATA) as usize;
            let mut offset = 0;
            while offset < limit && data[offset] != 0 {
                let length = usize::from(data[offset]);
                let record = data.get(offset..offset.checked_add(length)?)?;
                offset += length;
                if length < 34 || offset > limit || 33 + usize::from(record[32]) > length {
                    return None;
                }
                if record[25] & 2 != 0 {
                    continue;
                }
                if record[1] != 0
                    || record[26] != 0
                    || record[27] != 0
                    || record[25] & 0x80 != 0
                    || record[32] == 0
                    || record[32] >= 64
                    || media.entries.len() == 512
                {
                    return None;
                }
                let name: Vec<_> = record[33..33 + usize::from(record[32])]
                    .iter()
                    .copied()
                    .take_while(|&b| b != b';')
                    .collect();
                if name.is_empty()
                    || name
                        .iter()
                        .any(|&b| !(32..=126).contains(&b) || b"/\\:".contains(&b))
                    || media
                        .entries
                        .iter()
                        .any(|e| e.name.eq_ignore_ascii_case(&name))
                {
                    return None;
                }
                let extent = both32(&record[2..10])?;
                let size = both32(&record[10..18])?;
                if !media.valid_extent(extent, size, volume) {
                    return None;
                }
                media.entries.push(Entry { name, extent, size });
            }
        }
        Some(media)
    }

    // Guest strings and buffers are owned by the original single-threaded DOS
    // caller. Native adapters must provide mapped memory for those addresses.
    unsafe fn service(&mut self, frame: &mut GuestFrame) -> bool {
        let op = ((frame.eax >> 8) & 255) as u8;
        let handle = frame.ebx as u32;
        if matches!(op, 0x3d | 0x43 | 0x39 | 0x3a | 0x3c | 0x41 | 0x56 | 0x4e) {
            let path = unsafe { cd_path(frame.edx as u32) };
            if op == 0x56 && path.is_none() && unsafe { cd_path(frame.edi as u32) }.is_some() {
                frame.error(5);
                return true;
            }
            let Some(path) = path else { return false };
            if !matches!(op, 0x3d | 0x43) {
                frame.error(if op == 0x4e { 18 } else { 5 });
                return true;
            }
            if (op == 0x3d && frame.eax & 3 != 0) || (op == 0x43 && frame.eax & 255 != 0) {
                frame.error(5);
                return true;
            }
            let name = path;
            // DOS permits leading separators, but only root files are exposed.
            let name = &name[name
                .iter()
                .take_while(|&&b| b == b'/' || b == b'\\')
                .count()..];
            if name.is_empty() && op == 0x43 {
                frame.ecx = 17;
                frame.success(frame.eax as u32);
                return true;
            }
            if name.is_empty()
                || name.iter().any(|b| b"/\\:".contains(b))
                || name == b"."
                || name == b".."
            {
                frame.error(3);
                return true;
            }
            let Some(entry) = self
                .entries
                .iter()
                .position(|e| e.name.eq_ignore_ascii_case(name))
            else {
                frame.error(2);
                return true;
            };
            if op == 0x43 {
                frame.ecx = 1;
                frame.success(frame.eax as u32);
            } else if let Some(slot) = self.handles.iter().position(Option::is_none) {
                self.handles[slot] = Some(Handle { entry, position: 0 });
                frame.success(FIRST_HANDLE + slot as u32);
                rg_log!(
                    "OPEN D:\\{} -> {} (CD image)",
                    String::from_utf8_lossy(&self.entries[entry].name),
                    FIRST_HANDLE + slot as u32
                );
            } else {
                frame.error(4);
            }
            return true;
        }
        if !(FIRST_HANDLE..FIRST_HANDLE + HANDLE_COUNT as u32).contains(&handle)
            || !matches!(op, 0x3e | 0x3f | 0x40 | 0x42 | 0x44)
        {
            return false;
        }
        let slot = (handle - FIRST_HANDLE) as usize;
        let Some(mut handle) = self.handles[slot] else {
            frame.error(6);
            return true;
        };
        match op {
            0x3e => {
                self.handles[slot] = None;
                frame.success(0);
            }
            0x40 => frame.error(5),
            0x44 => {
                if frame.eax & 255 != 0 {
                    frame.error(1);
                } else {
                    frame.edx = 0;
                    frame.success(0);
                }
            }
            0x42 => {
                let origin = frame.eax & 255;
                let distance = (((frame.ecx & 65535) << 16) | (frame.edx & 65535)) as u32 as i32;
                let result = i64::from(distance)
                    + match origin {
                        1 => i64::from(handle.position),
                        2 => i64::from(self.entries[handle.entry].size),
                        _ => 0,
                    };
                if origin > 2 || !(0..=i64::from(u32::MAX)).contains(&result) {
                    frame.error(1);
                } else {
                    handle.position = result as u32;
                    self.handles[slot] = Some(handle);
                    frame.success(result as u32 & 65535);
                    frame.edx = result as u64 >> 16;
                }
            }
            0x3f => {
                let entry = &self.entries[handle.entry];
                let extent = entry.extent;
                let count = (frame.ecx as u32).min(entry.size.saturating_sub(handle.position));
                let mut done = 0;
                while done < count {
                    let part = (2048 - handle.position % 2048).min(count - done);
                    let offset = (u64::from(extent) + u64::from(handle.position / 2048))
                        * self.sector
                        + self.header
                        + u64::from(handle.position % 2048);
                    let out = unsafe {
                        std::slice::from_raw_parts_mut(
                            guest_ptr::<u8>(frame.edx as u32).add(done as usize),
                            part as usize,
                        )
                    };
                    if self.read(offset, out).is_none() {
                        frame.error(5);
                        return true;
                    }
                    done += part;
                    handle.position += part;
                }
                self.handles[slot] = Some(handle);
                frame.success(count);
            }
            _ => unreachable!(),
        }
        true
    }
}

unsafe fn cd_path<'a>(pointer: u32) -> Option<&'a [u8]> {
    if pointer == 0 {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(guest_ptr::<c_char>(pointer)) }.to_bytes();
    crate::movies::cd_drive(bytes)
}

/// Mount a disc image (ISO or raw 2352-byte sectors) read-only as the CD drive. Call during
/// installation selection.
pub(crate) fn mount(path: &Path) -> bool {
    let mut media = MEDIA.lock().unwrap_or_else(|e| e.into_inner());
    *media = None;
    let Some(opened) = Media::open(path) else {
        rg_log!("CD image unavailable or invalid: {}", path.display());
        return false;
    };
    rg_log!(
        "Native CD image mounted read-only as the CD drive: {} layout={}/{} root_files={}",
        path.display(),
        opened.sector,
        opened.header,
        opened.entries.len()
    );
    *media = Some(opened);
    true
}

/// # Safety
/// `path` points to a live NUL-terminated host path. Call during installation selection.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_cd_mount(path: *const c_char) -> c_int {
    if path.is_null() {
        rg_cd_close();
        return 0;
    }
    match unsafe { crate::os::path_from_c(path) } {
        Ok(path) => mount(&path) as c_int,
        Err(_) => {
            rg_cd_close();
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_cd_close() {
    *MEDIA.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// # Safety
/// `frame` and any guest memory referenced by a DOS request must be valid for
/// that request, with writable buffers for reads. Calls serialize image access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_cd_service(frame: *mut GuestFrame) -> c_int {
    let Some(frame) = (unsafe { frame.as_mut() }) else {
        return 0;
    };
    let mut media = MEDIA.lock().unwrap_or_else(|e| e.into_inner());
    media
        .as_mut()
        .is_some_and(|media| unsafe { media.service(frame) }) as c_int
}

#[cfg(test)]
mod tests {
    use super::*;

    fn both(bytes: &mut [u8], value: u32) {
        bytes[..4].copy_from_slice(&value.to_le_bytes());
        bytes[4..8].copy_from_slice(&value.to_be_bytes());
    }

    fn fixture() -> Vec<u8> {
        let mut bytes = vec![0; 24 * 2048];
        let pvd = &mut bytes[16 * 2048..17 * 2048];
        pvd[..7].copy_from_slice(b"\x01CD001\x01");
        both(&mut pvd[80..88], 24);
        pvd[128..132].copy_from_slice(&[0, 8, 8, 0]);
        pvd[156] = 34;
        pvd[181] = 2;
        both(&mut pvd[158..166], 20);
        both(&mut pvd[166..174], 2048);
        let record = &mut bytes[20 * 2048..21 * 2048];
        record[0] = 44;
        record[32] = 11;
        record[33..44].copy_from_slice(b"INTRO.SMK;1");
        both(&mut record[2..10], 21);
        both(&mut record[10..18], 5000);
        bytes
    }

    #[test]
    fn malformed_directory_records_never_panic_or_escape_the_image() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("synthetic.iso");
        let original = fixture();
        fs_write(&path, &original);
        assert_eq!(Media::open(&path).unwrap().entries.len(), 1);
        // Every record length, identifier length and flag value crosses the
        // parser boundary, including records that end beyond the sector.
        for offset in [0, 1, 25, 26, 27, 32] {
            for value in 0..=255 {
                let mut bytes = original.clone();
                bytes[20 * 2048 + offset] = value;
                fs_write(&path, &bytes);
                let _ = Media::open(&path);
            }
        }
        for size in [
            0,
            1,
            2047,
            2048,
            16 * 2048,
            20 * 2048 + 43,
            original.len() - 1,
        ] {
            fs_write(&path, &original[..size]);
            assert!(Media::open(&path).is_none());
        }
    }

    fn fs_write(path: &Path, bytes: &[u8]) {
        std::fs::write(path, bytes).unwrap();
    }
}
