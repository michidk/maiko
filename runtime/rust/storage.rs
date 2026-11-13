//! DOS paths and isolated writable storage shared by native hosts.
//! User operations stay relative to opened capability directories. Each parent
//! is opened without following links; installation data is never writable.
use crate::os::{canonical, inside, path_cstring, sync_directory};
pub(crate) use crate::os::{path_from_c, rename_noreplace};
use crate::rg_log;
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
#[cfg(unix)]
use std::ffi::CString;
use std::ffi::{OsStr, OsString, c_char, c_int};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

pub(crate) struct Root {
    pub path: PathBuf,
    pub dir: Dir,
    #[cfg(unix)]
    c_path: CString,
}
pub(crate) struct Storage {
    pub game: Root,
    pub user: Option<Root>,
}
pub(crate) static STORAGE: Mutex<Option<Storage>> = Mutex::new(None);
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[cfg(unix)]
static EMPTY: &[u8] = b"\0";

pub(crate) fn denied() -> io::Error {
    io::Error::from(io::ErrorKind::PermissionDenied)
}
pub(crate) fn absent() -> io::Error {
    io::Error::from(io::ErrorKind::NotFound)
}

pub(crate) fn user_namespace(relative: &str) -> bool {
    let upper = relative.to_ascii_uppercase();
    if upper == "SAVEGAME" || upper.starts_with("SAVEGAME/") {
        return true;
    }
    if let Some(leaf) = upper.strip_prefix("MAPS/") {
        return !leaf.contains('/') && leaf.ends_with(".TSG");
    }
    !upper.contains('/') && (upper == "LOGBOOK.TXT" || upper.ends_with(".INI"))
}

fn normalize(request: &str, initial: &str) -> io::Result<String> {
    let mut parts: Vec<_> = initial
        .split('/')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect();
    for part in request.split('/').filter(|p| !p.is_empty()) {
        match part {
            "." => {}
            ".." => {
                parts.pop().ok_or_else(denied)?;
            }
            _ => {
                if part.bytes().any(|b| b":<>|\"".contains(&b)) || part.ends_with([' ', '.']) {
                    return Err(denied());
                }
                parts.push(part.to_owned());
            }
        }
    }
    let relative = parts.join("/");
    if relative.len() >= 4096 {
        return Err(denied());
    }
    Ok(relative)
}

pub(crate) fn case_name(dir: &Dir, name: &OsStr, missing: bool) -> io::Result<OsString> {
    let mut found = None;
    for entry in dir.entries()? {
        let entry = entry?;
        let candidate = entry.file_name();
        if candidate
            .to_string_lossy()
            .eq_ignore_ascii_case(&name.to_string_lossy())
        {
            if found.is_some() {
                return Err(io::Error::from(io::ErrorKind::AlreadyExists));
            }
            found = Some(candidate);
        }
    }
    found
        .or_else(|| missing.then(|| name.to_owned()))
        .ok_or_else(absent)
}

fn short_leaf(leaf: &str) -> Option<String> {
    let (name, ext) = leaf
        .split_once('.')
        .map_or((leaf, None), |(n, e)| (n, Some(e)));
    if !leaf.is_ascii() || name.len() <= 8 && ext.is_none_or(|e| e.len() <= 3) {
        return None;
    }
    Some(format!(
        "{}{}",
        &name[..name.len().min(8)],
        ext.map_or(String::new(), |e| format!(".{}", &e[..e.len().min(3)]))
    ))
}

impl Root {
    fn open(path: PathBuf) -> io::Result<Self> {
        #[cfg(unix)]
        let c_path = path_cstring(&path)?;
        let dir = Dir::open_ambient_dir(&path, ambient_authority())?;
        Ok(Self {
            path,
            dir,
            #[cfg(unix)]
            c_path,
        })
    }
}

impl Storage {
    pub(crate) fn select(path: &Path) -> io::Result<Self> {
        Ok(Self {
            game: Root::open(canonical(path)?)?,
            user: None,
        })
    }
    pub(crate) fn relative(&self, request: &str) -> io::Result<String> {
        let request = request.replace('\\', "/");
        #[cfg(windows)]
        for root in std::iter::once(&self.game).chain(self.user.iter()) {
            if let Some(tail) = inside(Path::new(&request), &root.path) {
                return normalize(&tail.to_string_lossy().replace('\\', "/"), "");
            }
        }
        if request.as_bytes().get(1) == Some(&b':') {
            if !request.as_bytes()[0].eq_ignore_ascii_case(&b'C') {
                return Err(absent());
            }
            return normalize(&request[2..], "");
        }
        if request.starts_with('/') {
            return normalize(&request, "");
        }
        let cwd = std::env::current_dir()?;
        let tail = std::iter::once(&self.game)
            .chain(self.user.iter())
            .find_map(|root| inside(&cwd, &root.path))
            .ok_or_else(denied)?;
        normalize(&request, &tail.to_string_lossy().replace('\\', "/"))
    }

    pub(crate) fn parent(
        &self,
        relative: &str,
        writing: bool,
        missing: bool,
    ) -> io::Result<(Dir, OsString, PathBuf)> {
        let user = self.user.is_some() && user_namespace(relative);
        if writing && !user {
            return Err(denied());
        }
        let root = if user {
            self.user.as_ref().unwrap()
        } else {
            &self.game
        };
        let mut dir = root.dir.try_clone()?;
        let mut path = root.path.clone();
        let mut parts = relative.split('/').filter(|p| !p.is_empty()).peekable();
        while let Some(part) = parts.next() {
            let last = parts.peek().is_none();
            let name = case_name(&dir, OsStr::new(part), last && missing).or_else(|error| {
                if error.kind() != io::ErrorKind::NotFound || !last || missing {
                    return Err(error);
                }
                case_name(&dir, OsStr::new(&short_leaf(part).ok_or(error)?), false)
            })?;
            path.push(&name);
            if last {
                return Ok((dir, name, path));
            }
            dir = if user {
                dir.open_dir_nofollow(&name)?
            } else {
                Dir::open_ambient_dir(&path, ambient_authority())?
            };
        }
        Ok((dir, OsString::from("."), path))
    }

    pub(crate) fn resolved(&self, request: &str) -> io::Result<PathBuf> {
        if let Some(leaf) = crate::movies::leaf(request.as_bytes()) {
            let leaf = std::str::from_utf8(leaf).map_err(|_| absent())?;
            for folder in ["", "MOVIES/", "ANIMS/", "VIDEO/"] {
                if let Ok((_, _, path)) = self.parent(&format!("{folder}{leaf}"), false, false)
                    && std::fs::metadata(&path).is_ok_and(|m| m.is_file())
                {
                    rg_log!("MOVIE loose {request} -> {}", path.display());
                    return Ok(path);
                }
            }
            rg_log!(
                "MOVIE missing {request}; searched GAME_DIR, MOVIES, ANIMS, VIDEO; no installed file found"
            );
            return Err(absent());
        }
        self.parent(&self.relative(request)?, false, false)
            .map(|(_, _, path)| path)
    }

    pub(crate) fn initialize_user(&mut self, path: &Path) -> io::Result<()> {
        let target = if path.exists() {
            canonical(path)?
        } else {
            let absolute = std::path::absolute(path)?;
            let parent = absolute.parent().ok_or_else(denied)?;
            canonical(parent)?.join(absolute.file_name().ok_or_else(denied)?)
        };
        if inside(&target, &self.game.path).is_some() || inside(&self.game.path, &target).is_some()
        {
            return Err(denied());
        }
        match std::fs::create_dir(&target) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
        let parent =
            Dir::open_ambient_dir(target.parent().ok_or_else(denied)?, ambient_authority())?;
        let dir = parent.open_dir_nofollow(target.file_name().ok_or_else(denied)?)?;
        match dir.symlink_metadata(".redguard-storage-v1") {
            Ok(meta) => {
                if !meta.is_file() || meta.file_type().is_symlink() {
                    return Err(denied());
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let save = directory(&dir, OsStr::new("SAVEGAME"))?;
                if let Ok(name) = case_name(&self.game.dir, OsStr::new("SAVEGAME"), false) {
                    copy_tree(&self.game.path.join(name), &save, 0)?;
                }
                let maps = directory(&dir, OsStr::new("MAPS"))?;
                if let Ok(name) = case_name(&self.game.dir, OsStr::new("MAPS"), false) {
                    let source = self.game.path.join(name);
                    for entry in std::fs::read_dir(&source)? {
                        let entry = entry?;
                        if entry
                            .file_name()
                            .to_string_lossy()
                            .to_ascii_uppercase()
                            .ends_with(".TSG")
                        {
                            copy_file(&source, &maps, &entry.file_name())?;
                        }
                    }
                }
                for entry in self.game.dir.entries()? {
                    let name = entry?.file_name();
                    let upper = name.to_string_lossy().to_ascii_uppercase();
                    if upper == "LOGBOOK.TXT" || upper.ends_with(".INI") {
                        copy_file(&self.game.path, &dir, &name)?;
                    }
                }
                let mut options = OpenOptions::new();
                options
                    .write(true)
                    .create_new(true)
                    .follow(FollowSymlinks::No);
                dir.open_with(".redguard-storage-v1", &options)?
                    .sync_all()?;
                sync_directory(&dir)?;
            }
            Err(e) => return Err(e),
        }
        self.user = Some(Root {
            #[cfg(unix)]
            c_path: path_cstring(&target)?,
            path: target,
            dir,
        });
        rg_log!(
            "Isolated native user storage: {}",
            self.user.as_ref().unwrap().path.display()
        );
        Ok(())
    }
}

fn directory(parent: &Dir, name: &OsStr) -> io::Result<Dir> {
    match parent.create_dir(name) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    parent.open_dir_nofollow(name)
}

fn copy_file(source: &Path, destination: &Dir, name: &OsStr) -> io::Result<()> {
    match destination.symlink_metadata(name) {
        Ok(meta) => {
            return if meta.is_file() && !meta.file_type().is_symlink() {
                Ok(())
            } else {
                Err(denied())
            };
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let mut input = std::fs::File::open(source.join(name))?;
    if !input.metadata()?.is_file() {
        return Err(denied());
    }
    let temporary = format!(
        ".rg-import-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Relaxed)
    );
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    let mut output = destination.open_with(&temporary, &options)?;
    let result = (|| {
        let mut buffer = [0; 16384];
        loop {
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            output.write_all(&buffer[..count])?;
        }
        output.sync_all()?;
        drop(output);
        if let Err(e) = rename_noreplace(destination, OsStr::new(&temporary), destination, name)
            && (e.kind() != io::ErrorKind::AlreadyExists
                || !destination
                    .symlink_metadata(name)
                    .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink()))
        {
            return Err(e);
        }
        sync_directory(destination)
    })();
    let _ = destination.remove_file(temporary);
    result
}

fn copy_tree(source: &Path, destination: &Dir, depth: usize) -> io::Result<()> {
    if depth > 8 {
        return Err(denied());
    }
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let meta = std::fs::metadata(source.join(&name))?;
        if meta.is_file() {
            copy_file(source, destination, &name)?;
        } else if meta.is_dir() {
            copy_tree(
                &source.join(&name),
                &directory(destination, &name)?,
                depth + 1,
            )?;
        } else {
            return Err(denied());
        }
    }
    Ok(())
}

pub(crate) unsafe fn copy_path(
    path: &Path,
    out: *mut c_char,
    capacity: usize,
) -> io::Result<usize> {
    let text = path_cstring(path)?;
    let bytes = text.as_bytes_with_nul();
    if out.is_null() || bytes.len() > capacity {
        return Err(denied());
    }
    unsafe {
        out.cast::<u8>()
            .copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
    }
    Ok(bytes.len() - 1)
}

pub(crate) fn select_game(path: &Path) -> io::Result<()> {
    let selected = Storage::select(path)?;
    *STORAGE.lock().unwrap_or_else(|e| e.into_inner()) = Some(selected);
    Ok(())
}

/// # Safety
/// `path` is a live NUL-terminated native directory path.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_path_root(path: *const c_char) -> c_int {
    unsafe { path_from_c(path) }
        .and_then(|p| select_game(&p))
        .is_ok() as c_int
}

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn rg_game_root() -> *const c_char {
    STORAGE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map_or(EMPTY.as_ptr().cast(), |s| s.game.c_path.as_ptr())
}

/// # Safety
/// `path` is a C path; `out` is writable for `capacity` bytes.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_resolve_path(
    path: *const c_char,
    out: *mut c_char,
    capacity: usize,
) -> c_int {
    let result = (|| {
        let path = unsafe { path_from_c(path) }?;
        let storage = STORAGE.lock().unwrap_or_else(|e| e.into_inner());
        let storage = storage.as_ref().ok_or_else(denied)?;
        let resolved = storage.resolved(&path.to_string_lossy())?;
        unsafe { copy_path(&resolved, out, capacity) }?;
        Ok::<_, io::Error>(())
    })();
    result.is_ok() as c_int
}

/// # Safety
/// `path` is a live native user directory pathname.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_user_storage_init(path: *const c_char) -> c_int {
    let result = (|| {
        let path = unsafe { path_from_c(path) }?;
        let mut storage = STORAGE.lock().unwrap_or_else(|e| e.into_inner());
        storage.as_mut().ok_or_else(denied)?.initialize_user(&path)
    })();
    result.is_ok() as c_int
}

/// # Safety
/// `game` and `user` are live native directory pathnames.
#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_user_storage_init(game: *const c_char, user: *const c_char) -> c_int {
    let result = (|| {
        let game = unsafe { path_from_c(game) }?;
        let user = unsafe { path_from_c(user) }?;
        let mut selected = Storage::select(&game)?;
        selected.initialize_user(&user)?;
        *STORAGE.lock().unwrap_or_else(|e| e.into_inner()) = Some(selected);
        Ok::<_, io::Error>(())
    })();
    if let Err(error) = &result {
        rg_log!("Native user storage initialization failed: {error}");
    }
    result.is_ok() as c_int
}

/// # Safety
/// `path` points to a native directory pathname.
#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_storage_game_root(path: *const c_char) -> c_int {
    unsafe { path_from_c(path) }
        .and_then(|p| select_game(&p))
        .is_ok() as c_int
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "C" fn rg_storage_game_selected() -> c_int {
    STORAGE.lock().unwrap_or_else(|e| e.into_inner()).is_some() as c_int
}

/// # Safety
/// `path` is a native directory and `out` has the legacy MAX_PATH capacity.
#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_storage_canonical_directory(
    path: *const c_char,
    out: *mut c_char,
) -> c_int {
    unsafe { path_from_c(path) }
        .and_then(|p| canonical(&p))
        .and_then(|p| unsafe { copy_path(&p, out, 260) })
        .is_ok() as c_int
}

/// # Safety
/// `out` is writable for `capacity` bytes, or null when asking for the size.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_host_cwd(capacity: u32, out: *mut c_char) -> u32 {
    let result = (|| {
        let storage = STORAGE.lock().unwrap_or_else(|e| e.into_inner());
        let cwd = std::env::current_dir()?;
        let path = if let Some(storage) = storage.as_ref() {
            let relative = std::iter::once(&storage.game)
                .chain(storage.user.iter())
                .find_map(|r| inside(&cwd, &r.path))
                .ok_or_else(denied)?;
            PathBuf::from(format!(
                "C:\\{}",
                relative.to_string_lossy().replace('/', "\\")
            ))
        } else {
            cwd
        };
        let length = path_cstring(&path)?.as_bytes_with_nul().len();
        if capacity == 0 {
            return Ok(length as u32);
        }
        unsafe { copy_path(&path, out, capacity as usize) }.map(|n| n as u32)
    })();
    result.unwrap_or(0)
}

/// # Safety
/// `path` is a live NUL-terminated DOS directory name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_host_chdir(path: *const c_char) -> c_int {
    let result = (|| {
        let path = unsafe { path_from_c(path) }?;
        let storage = STORAGE.lock().unwrap_or_else(|e| e.into_inner());
        let target = if let Some(storage) = storage.as_ref() {
            storage.resolved(&path.to_string_lossy())?
        } else {
            path
        };
        std::env::set_current_dir(target)
    })();
    result.is_ok() as c_int
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn namespaces_and_parent_traversal_are_shared() {
        for path in [
            "SAVEGAME/SAVEGAME.099/SAVEGAME.SAV",
            "maps/ISLAND.tsg",
            "SYSTEM.INI",
            "logbook.txt",
        ] {
            assert!(user_namespace(path));
        }
        for path in [
            "MAPS/ISLAND.RGM",
            "MAPS/nested/ISLAND.TSG",
            "FXART/SYSTEM.INI",
            "LOGBOOK.TXT/x",
        ] {
            assert!(!user_namespace(path));
        }
        assert!(normalize("../escape.ini", "").is_err());
        assert_eq!(
            normalize("SAVEGAME/../MAPS/ISLAND.TSG", "").unwrap(),
            "MAPS/ISLAND.TSG"
        );
    }
    #[test]
    fn native_rename_preserves_existing_destination_and_moves_without_a_link() {
        let root = tempfile::tempdir().unwrap();
        let dir = Dir::open_ambient_dir(root.path(), ambient_authority()).unwrap();
        dir.write("source", b"source").unwrap();
        dir.write("existing", b"existing").unwrap();
        assert!(
            rename_noreplace(&dir, OsStr::new("source"), &dir, OsStr::new("existing")).is_err()
        );
        assert_eq!(dir.read("source").unwrap(), b"source");
        assert_eq!(dir.read("existing").unwrap(), b"existing");
        rename_noreplace(&dir, OsStr::new("source"), &dir, OsStr::new("moved")).unwrap();
        assert!(!dir.exists("source"));
        assert_eq!(dir.read("moved").unwrap(), b"source");
    }

    #[test]
    fn capability_storage_preserves_existing_files_and_refuses_links() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let user = root.path().join("user");
        std::fs::create_dir_all(game.join("Savegame/SAVEGAME.001")).unwrap();
        std::fs::write(game.join("Savegame/SAVEGAME.001/SAVEGAME.SAV"), b"original").unwrap();
        std::fs::write(game.join("SYSTEM.INI"), b"settings").unwrap();
        let mut storage = Storage::select(&game).unwrap();
        assert!(storage.initialize_user(&game.join("bad")).is_err());
        assert!(!game.join("bad").exists());
        storage.initialize_user(&user).unwrap();
        let saved = user.join("SAVEGAME/SAVEGAME.001/SAVEGAME.SAV");
        assert_eq!(std::fs::read(&saved).unwrap(), b"original");
        std::fs::write(&saved, b"user").unwrap();
        storage.initialize_user(&user).unwrap();
        assert_eq!(std::fs::read(&saved).unwrap(), b"user");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.path(), user.join("SAVEGAME/escape")).unwrap();
            assert!(
                storage
                    .parent("SAVEGAME/escape/evil.ini", true, true)
                    .is_err()
            );
        }
        assert_eq!(std::fs::read(game.join("SYSTEM.INI")).unwrap(), b"settings");
    }
}
