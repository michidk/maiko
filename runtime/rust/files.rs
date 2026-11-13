//! Original DOS file handles and DTA searches, with one host-independent policy.
use crate::guest::GuestFrame;
use crate::media::rg_cd_service;
use crate::memory::guest_ptr;
use crate::rg_log;
use crate::storage::{self, STORAGE, Storage};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::OpenOptions;
use std::ffi::c_int;
use std::fs::{File, Metadata};
use std::io::{self, IsTerminal, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Mutex;

struct Handle {
    file: File,
    writable: bool,
}
struct Entry {
    name: String,
    size: u32,
    attributes: u8,
    time: u16,
    date: u16,
}
struct Search {
    id: u32,
    entries: std::vec::IntoIter<Entry>,
}
struct Files {
    handles: [Option<Handle>; 256],
    searches: [Option<Search>; 64],
    dta: usize,
    sequence: u32,
}
static FILES: Mutex<Option<Files>> = Mutex::new(None);
impl Default for Files {
    fn default() -> Self {
        Self {
            handles: std::array::from_fn(|_| None),
            searches: std::array::from_fn(|_| None),
            dta: 0,
            sequence: 0,
        }
    }
}

fn dos_error(error: io::Error) -> u32 {
    match error.kind() {
        io::ErrorKind::NotFound => 2,
        io::ErrorKind::NotADirectory => 3,
        _ if crate::os::too_many_open_files(&error) => 4,
        _ => 5,
    }
}

fn wildcard(pattern: &[u8], name: &[u8]) -> bool {
    let pattern = if pattern == b"*.*" { b"*" } else { pattern };
    let mut row = vec![false; name.len() + 1];
    row[0] = true;
    for &p in pattern {
        let mut next = vec![false; row.len()];
        next[0] = p == b'*' && row[0];
        for (i, &n) in name.iter().enumerate() {
            next[i + 1] = if p == b'*' {
                row[i + 1] || next[i]
            } else {
                (p == b'?' || p.eq_ignore_ascii_case(&n)) && row[i]
            };
        }
        row = next;
    }
    row[name.len()]
}

fn dos_name(name: &str) -> bool {
    if name == "." || name == ".." {
        return true;
    }
    let mut parts = name.split('.');
    let stem = parts.next().unwrap_or("");
    let ext = parts.next();
    name.is_ascii()
        && !stem.is_empty()
        && stem.len() <= 8
        && ext.is_none_or(|e| e.len() <= 3)
        && parts.next().is_none()
}

fn dos_time(meta: &Metadata) -> (u16, u16) {
    let Some(seconds) = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
    else {
        return (0, 0);
    };
    crate::os::dos_timestamp(seconds)
}

fn attributes(meta: &Metadata, name: &str) -> u8 {
    crate::os::file_attributes(meta, name)
}

fn open_file(
    storage: Option<&Storage>,
    request: &str,
    access: u32,
    create: bool,
) -> io::Result<File> {
    if let Some(storage) = storage {
        if crate::movies::leaf(request.as_bytes()).is_some() {
            if access != 0 || create {
                return Err(storage::denied());
            }
            return File::open(storage.resolved(request)?);
        }
        let relative = storage.relative(request)?;
        let user = storage.user.is_some() && storage::user_namespace(&relative);
        let (dir, name, path) = storage.parent(&relative, access != 0, create)?;
        if user {
            let mut options = OpenOptions::new();
            options
                .read(access != 1)
                .write(access != 0)
                .create(create)
                .truncate(create)
                .follow(FollowSymlinks::No);
            let file = dir.open_with(name, &options)?.into_std();
            if !file.metadata()?.is_file() {
                return Err(storage::denied());
            }
            Ok(file)
        } else {
            if access != 0 {
                return Err(storage::denied());
            }
            let file = File::open(path)?;
            if !file.metadata()?.is_file() {
                return Err(storage::denied());
            }
            Ok(file)
        }
    } else {
        if access != 0 || create {
            return Err(storage::denied());
        }
        File::open(host_asset_path(request)?)
    }
}

fn host_asset_path(request: &str) -> io::Result<PathBuf> {
    let path = PathBuf::from(request);
    match std::fs::metadata(&path) {
        Ok(_) => return Ok(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let leaf = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| name.is_ascii())
        .ok_or_else(storage::absent)?;
    let (stem, extension) = leaf.split_once('.').unwrap_or((leaf, ""));
    let short = format!(
        "{}{}",
        &stem[..stem.len().min(8)],
        if extension.is_empty() {
            String::new()
        } else {
            format!(".{}", &extension[..extension.len().min(3)])
        }
    );
    Ok(path.with_file_name(short))
}

impl Files {
    fn open(
        &mut self,
        frame: &mut GuestFrame,
        request: &str,
        storage: Option<&Storage>,
        create: bool,
    ) {
        let access = if create { 2 } else { frame.eax as u32 & 3 };
        if access == 3 {
            frame.error(12);
            return;
        }
        match open_file(storage, request, access, create) {
            Ok(file) => {
                if let Some(slot) = (5..256).find(|&i| self.handles[i].is_none()) {
                    self.handles[slot] = Some(Handle {
                        file,
                        writable: access != 0,
                    });
                    frame.success(slot as u32);
                    rg_log!("OPEN {request} -> {slot}");
                } else {
                    frame.error(4)
                }
            }
            Err(e) => frame.error(dos_error(e)),
        }
    }

    fn mutate(
        &mut self,
        frame: &mut GuestFrame,
        op: u32,
        request: &str,
        storage: Option<&Storage>,
    ) {
        let result = (|| {
            let storage = storage.ok_or_else(storage::denied)?;
            let relative = storage.relative(request)?;
            let (parent, leaf, _) = storage.parent(&relative, true, true)?;
            match op {
                0x39 => parent.create_dir(leaf),
                0x3a => parent.remove_dir(leaf),
                0x41 => {
                    if !parent
                        .symlink_metadata(&leaf)
                        .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
                    {
                        return Err(storage::denied());
                    }
                    parent.remove_file(leaf)
                }
                0x56 => {
                    let target = unsafe { storage::path_from_c(guest_ptr::<_>(frame.edi as u32)) }?;
                    let relative = storage.relative(&target.to_string_lossy())?;
                    let (destination, name, _) = storage.parent(&relative, true, true)?;
                    if !parent
                        .symlink_metadata(&leaf)
                        .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
                    {
                        return Err(storage::denied());
                    }
                    storage::rename_noreplace(&parent, &leaf, &destination, &name)
                }
                _ => Err(storage::denied()),
            }
        })();
        match result {
            Ok(()) => frame.success(0),
            Err(e) => frame.error(dos_error(e)),
        }
        rg_log!(
            "FILE mutation={op:02x} path={request} result={} carry={}",
            frame.eax,
            frame.flags & 1
        );
    }

    fn search(&mut self, frame: &mut GuestFrame, op: u32, storage: Option<&Storage>) -> bool {
        if self.dta == 0 {
            return false;
        }
        let dta = unsafe { &mut *(self.dta as *mut [u8; 43]) };
        let slot = if op == 0x4e {
            let Some(slot) = self.searches.iter().position(Option::is_none) else {
                frame.error(4);
                return true;
            };
            let request = unsafe { storage::path_from_c(guest_ptr::<_>(frame.edx as u32)) };
            let entries =
                request.and_then(|p| entries(storage, &p.to_string_lossy(), frame.ecx as u32));
            let entries = match entries {
                Ok(entries) => entries,
                Err(e) => {
                    frame.error(dos_error(e));
                    return true;
                }
            };
            self.sequence = self.sequence.wrapping_add(1);
            self.searches[slot] = Some(Search {
                id: self.sequence,
                entries: entries.into_iter(),
            });
            slot
        } else {
            let magic = u32::from_le_bytes(dta[..4].try_into().unwrap());
            let id = u32::from_le_bytes(dta[4..8].try_into().unwrap());
            let Some(slot) = self
                .searches
                .iter()
                .position(|s| magic == 0x52474454 && s.as_ref().is_some_and(|s| s.id == id))
            else {
                frame.error(18);
                return true;
            };
            slot
        };
        let search = self.searches[slot].as_mut().unwrap();
        if let Some(entry) = search.entries.next() {
            dta.fill(0);
            dta[..4].copy_from_slice(&0x52474454u32.to_le_bytes());
            dta[4..8].copy_from_slice(&search.id.to_le_bytes());
            dta[21] = entry.attributes;
            dta[22..24].copy_from_slice(&entry.time.to_le_bytes());
            dta[24..26].copy_from_slice(&entry.date.to_le_bytes());
            dta[26..30].copy_from_slice(&entry.size.to_le_bytes());
            let name = entry.name.to_ascii_uppercase();
            dta[30..30 + name.len()].copy_from_slice(name.as_bytes());
            frame.success(0);
        } else {
            self.searches[slot] = None;
            frame.error(18);
        }
        true
    }

    unsafe fn service(&mut self, frame: &mut GuestFrame) -> bool {
        let op = (frame.eax as u32 >> 8) & 255;
        let handle = frame.ebx as u32 as usize;
        let storage = STORAGE.lock().unwrap_or_else(|e| e.into_inner());
        let storage = storage.as_ref();
        match op {
            0x1a => {
                self.dta = guest_ptr::<u8>(frame.edx as u32) as usize;
                return true;
            }
            0x4e | 0x4f => return self.search(frame, op, storage),
            0x39 | 0x3a | 0x3c | 0x3d | 0x41 | 0x43 | 0x56 => {
                let request =
                    match unsafe { storage::path_from_c(guest_ptr::<_>(frame.edx as u32)) } {
                        Ok(p) => p,
                        Err(e) => {
                            frame.error(dos_error(e));
                            return true;
                        }
                    };
                let request = request.to_string_lossy();
                match op {
                    0x3c | 0x3d => self.open(frame, &request, storage, op == 0x3c),
                    0x43 => {
                        if frame.eax & 255 != 0 {
                            frame.error(5);
                            return true;
                        }
                        let result = if let Some(storage) = storage {
                            storage.resolved(&request)
                        } else {
                            host_asset_path(&request)
                        };
                        match result.and_then(|p| std::fs::metadata(&p)) {
                            Ok(meta) => {
                                frame.ecx = attributes(&meta, "").into();
                                frame.success(frame.eax as u32);
                            }
                            Err(e) => frame.error(dos_error(e)),
                        }
                    }
                    _ => self.mutate(frame, op, &request, storage),
                }
                return true;
            }
            0x19 => {
                let drive = crate::os::current_drive(storage.is_some());
                frame.eax = (frame.eax & !255) | drive;
                return true;
            }
            0x0e => {
                let count = crate::os::select_drive((frame.edx & 255) as u8, storage.is_some());
                frame.eax = (frame.eax & !255) | count;
                return true;
            }
            0x40 if (handle == 1 || handle == 2) && frame.ecx as u32 == 0 => {
                frame.success(0);
                return true;
            }
            0x40 if handle == 1 || handle == 2 => {
                let data = unsafe {
                    std::slice::from_raw_parts(
                        guest_ptr::<u8>(frame.edx as u32),
                        frame.ecx as u32 as usize,
                    )
                };
                let result = if handle == 1 {
                    io::stdout().write(data)
                } else {
                    io::stderr().write(data)
                };
                match result {
                    Ok(n) => frame.success(n as u32),
                    Err(e) => frame.error(dos_error(e)),
                }
                return true;
            }
            0x44 if handle == 1 || handle == 2 => {
                if frame.eax & 255 != 0 {
                    return false;
                }
                frame.edx = if if handle == 1 {
                    io::stdout().is_terminal()
                } else {
                    io::stderr().is_terminal()
                } {
                    0x80
                } else {
                    0
                };
                frame.success(frame.edx as u32);
                return true;
            }
            0x3e | 0x3f | 0x40 | 0x42 | 0x44 => {}
            _ => return false,
        }
        let Some(open) = self.handles.get_mut(handle).and_then(Option::as_mut) else {
            frame.error(6);
            return true;
        };
        match op {
            0x3e => {
                self.handles[handle] = None;
                frame.success(0);
            }
            0x3f if frame.ecx as u32 == 0 => frame.success(0),
            0x3f => {
                let data = unsafe {
                    std::slice::from_raw_parts_mut(
                        guest_ptr::<u8>(frame.edx as u32),
                        frame.ecx as u32 as usize,
                    )
                };
                match open.file.read(data) {
                    Ok(n) => frame.success(n as u32),
                    Err(e) => frame.error(dos_error(e)),
                }
            }
            0x40 => {
                if !open.writable {
                    frame.error(5);
                    return true;
                }
                let result = if frame.ecx as u32 == 0 {
                    open.file
                        .stream_position()
                        .and_then(|p| open.file.set_len(p))
                        .map(|_| 0)
                } else {
                    let data = unsafe {
                        std::slice::from_raw_parts(
                            guest_ptr::<u8>(frame.edx as u32),
                            frame.ecx as u32 as usize,
                        )
                    };
                    open.file.write(data)
                };
                match result {
                    Ok(n) => frame.success(n as u32),
                    Err(e) => frame.error(dos_error(e)),
                }
            }
            0x42 => {
                let distance = (((frame.ecx & 65535) << 16) | (frame.edx & 65535)) as u32 as i32;
                let from = match frame.eax & 255 {
                    0 if distance >= 0 => SeekFrom::Start(distance as u64),
                    1 => SeekFrom::Current(i64::from(distance)),
                    2 => SeekFrom::End(i64::from(distance)),
                    _ => {
                        frame.error(1);
                        return true;
                    }
                };
                match open.file.seek(from) {
                    Ok(n) if n <= u64::from(u32::MAX) => {
                        frame.success(n as u32 & 65535);
                        frame.edx = n >> 16;
                    }
                    Ok(_) => frame.error(5),
                    Err(e) => frame.error(dos_error(e)),
                }
            }
            0x44 => {
                if frame.eax & 255 != 0 {
                    return false;
                }
                frame.edx = if open.file.is_terminal() { 0x80 } else { 0 };
                frame.success(frame.edx as u32);
            }
            _ => unreachable!(),
        }
        true
    }
}

fn entries(storage: Option<&Storage>, request: &str, filter: u32) -> io::Result<Vec<Entry>> {
    rg_log!("FIND {request} attributes={:02x}", filter & 255);
    let normalized = request.replace('\\', "/");
    let (parent, pattern) = normalized.rsplit_once('/').unwrap_or((".", &normalized));
    let parent = if parent.is_empty() { "C:/" } else { parent };
    let path = if let Some(storage) = storage {
        storage.resolved(parent)?
    } else {
        PathBuf::from(parent)
    };
    let relative = storage.and_then(|s| s.relative(parent).ok());
    let overlay = storage
        .filter(|s| {
            s.user.is_some()
                && relative
                    .as_deref()
                    .is_some_and(|r| r.eq_ignore_ascii_case("MAPS"))
        })
        .and_then(|s| s.user.as_ref());
    let mut result = Vec::new();
    for (phase, directory) in
        std::iter::once((false, path)).chain(overlay.map(|u| (true, u.path.join("MAPS"))))
    {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if overlay.is_some() && name.to_ascii_uppercase().ends_with(".TSG") != phase {
                continue;
            }
            if !dos_name(name) || !wildcard(pattern.as_bytes(), name.as_bytes()) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let attr = attributes(&meta, name);
            if u32::from(attr) & (2 | 4 | 16) & !filter != 0 {
                continue;
            }
            let (time, date) = dos_time(&meta);
            result.push(Entry {
                name: name.to_owned(),
                size: meta.len() as u32,
                attributes: attr,
                time,
                date,
            });
        }
    }
    result.sort_by_key(|e| e.name.to_ascii_uppercase());
    Ok(result)
}

/// # Safety
/// Frame addresses and buffers must be mapped and valid for the DOS operation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_file_service(frame: *mut GuestFrame) -> c_int {
    if unsafe { rg_cd_service(frame) } != 0 {
        return 1;
    }
    let Some(frame) = (unsafe { frame.as_mut() }) else {
        return 0;
    };
    let mut files = FILES.lock().unwrap_or_else(|e| e.into_inner());
    (unsafe { files.get_or_insert_with(Files::default).service(frame) }) as c_int
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wildcard_and_dos_names_reject_host_metadata() {
        assert!(wildcard(b"*.TSG", b"island.tsg"));
        assert!(wildcard(b"*.*", b"README"));
        assert!(wildcard(b"A?C.*", b"abc.tsg"));
        assert!(!wildcard(b"A?C.*", b"abbc.tsg"));
        assert!(dos_name("SAVEGAME.099"));
        assert!(!dos_name("goggame-123456789.info"));
        assert!(!dos_name("a.b.c"));
    }
}
