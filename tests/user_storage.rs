//! User-storage and DOS file-service behaviour on Unix: saves and settings are imported once into a
//! separate user directory, the installed game stays untouched, native saves can be created, written,
//! seeked, truncated, renamed, read and deleted through the DOS file service, and path traversal and
//! symlink escapes are refused. Replaces the old C test of the same name.
#![cfg(unix)]
use maiko_runtime::{
    files::rg_file_service,
    guest::GuestFrame,
    storage::{rg_game_root, rg_path_root, rg_user_storage_init},
};
use std::ffi::CString;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

/// The services take 32-bit guest addresses, so the string buffer must live in the low 4 GiB.
struct Guest(*mut u8);
impl Guest {
    fn new() -> Self {
        let p = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                8192,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_32BIT,
                -1,
                0,
            )
        };
        assert_ne!(p, libc::MAP_FAILED);
        Guest(p.cast())
    }
    fn address(&self, offset: usize) -> u64 {
        self.0 as u64 + offset as u64
    }
    fn put(&self, offset: usize, text: &str) {
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), self.0.add(offset), text.len());
            *self.0.add(offset + text.len()) = 0;
        }
    }
    fn bytes(&self, offset: usize, length: usize) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.0.add(offset), length) }
    }
    fn cstr(&self, offset: usize) -> String {
        let bytes = self.bytes(offset, 256);
        String::from_utf8_lossy(&bytes[..bytes.iter().position(|b| *b == 0).unwrap()]).into_owned()
    }
    /// A DOS call whose path is at offset 0 and whose function is `ax`.
    fn request(&self, ax: u64, path: &str) -> GuestFrame {
        self.put(0, path);
        GuestFrame {
            eax: ax,
            edx: self.address(0),
            ..Default::default()
        }
    }
}

fn call(f: &mut GuestFrame) -> bool {
    assert_eq!(unsafe { rg_file_service(f) }, 1, "service not handled");
    f.flags & 1 == 0
}

fn ok(f: &mut GuestFrame) {
    assert!(call(f), "unexpected DOS error {}", f.eax);
}

fn fails(f: &mut GuestFrame, code: Option<u64>) {
    assert!(!call(f), "call unexpectedly succeeded");
    if let Some(code) = code {
        assert_eq!(f.eax, code);
    }
}

fn expect(path: &str, value: &str) {
    assert_eq!(fs::read_to_string(path).unwrap(), value, "{path}");
}

fn c(path: &str) -> CString {
    CString::new(path).unwrap()
}

#[test]
fn user_storage_and_dos_file_service() {
    let root = std::env::temp_dir().join(format!("rg-user-files-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("game/Savegame/SAVEGAME.001")).unwrap();
    fs::create_dir_all(root.join("game/Maps")).unwrap();
    std::env::set_current_dir(&root).unwrap();
    fs::write("game/Savegame/SAVEGAME.001/SAVEGAME.SAV", "original save").unwrap();
    fs::write("game/Maps/ISLAND.TSG", "original map").unwrap();
    fs::write("game/Maps/ISLAND.RGM", "asset").unwrap();
    fs::write("game/SYSTEM.INI", "original settings").unwrap();

    unsafe {
        assert_eq!(rg_path_root(c("game").as_ptr()), 1);
        assert_eq!(rg_user_storage_init(c("game/must-not-create").as_ptr()), 0);
        assert!(!Path::new("game/must-not-create").exists());
        assert_eq!(rg_user_storage_init(c(".").as_ptr()), 0);
        assert_eq!(rg_user_storage_init(c("user").as_ptr()), 1);
    }
    expect("user/SAVEGAME/SAVEGAME.001/SAVEGAME.SAV", "original save");
    expect("user/MAPS/ISLAND.TSG", "original map");
    expect("user/SYSTEM.INI", "original settings");
    std::env::set_current_dir(
        unsafe { std::ffi::CStr::from_ptr(rg_game_root()) }
            .to_str()
            .unwrap(),
    )
    .unwrap();

    let g = Guest::new();
    // Create a native save directory and file, write, seek, read back, truncate.
    ok(&mut g.request(0x3900, "C:\\SAVEGAME\\SAVEGAME.099"));
    let mut f = g.request(0x3c00, "C:\\SAVEGAME\\SAVEGAME.099\\SAVEGAME.SAV");
    ok(&mut f);
    let handle = f.eax;
    g.put(4096, "native save");
    f.eax = 0x4000;
    f.ebx = handle;
    f.edx = g.address(4096);
    f.ecx = 11;
    ok(&mut f);
    assert_eq!(f.eax, 11);
    f.eax = 0x4200;
    f.ecx = 0;
    f.edx = 7;
    ok(&mut f);
    f.eax = 0x4200;
    f.ebx = handle;
    f.ecx = 0;
    f.edx = 0;
    ok(&mut f);
    f.eax = 0x3f00;
    f.edx = g.address(4096);
    f.ecx = 11;
    ok(&mut f);
    assert_eq!((f.eax, g.bytes(4096, 11)), (11, &b"native save"[..]));
    f.eax = 0x4200;
    f.ecx = 0;
    f.edx = 7;
    ok(&mut f);
    f.eax = 0x4000;
    f.ecx = 0;
    ok(&mut f);
    assert_eq!(f.eax, 0);
    f.eax = 0x3e00;
    ok(&mut f);

    // Rename, reopen and read the truncated prefix.
    let mut f = g.request(0x5600, "C:\\SAVEGAME\\SAVEGAME.099\\SAVEGAME.SAV");
    g.put(2048, "C:\\SAVEGAME\\SAVEGAME.099\\RENAME.SAV");
    f.edi = g.address(2048);
    ok(&mut f);
    let mut f = g.request(0x3d00, "C:\\SAVEGAME\\SAVEGAME.099\\RENAME.SAV");
    ok(&mut f);
    f.ebx = f.eax;
    f.eax = 0x3f00;
    f.edx = g.address(4096);
    f.ecx = 32;
    ok(&mut f);
    assert_eq!((f.eax, g.bytes(4096, 7)), (7, &b"native "[..]));
    f.eax = 0x3e00;
    ok(&mut f);

    // Imported files can be deleted, then stay deleted; new files can be created and closed.
    ok(&mut g.request(0x4100, "C:\\MAPS\\ISLAND.TSG"));
    fails(&mut g.request(0x3d00, "C:\\MAPS\\ISLAND.TSG"), None);
    let mut f = g.request(0x3c00, "C:\\MAPS\\NATIVE.TSG");
    ok(&mut f);
    f.ebx = f.eax;
    f.eax = 0x3e00;
    ok(&mut f);
    f.eax = 0x1a00;
    f.edx = g.address(6000);
    assert_eq!(unsafe { rg_file_service(&mut f) }, 1);
    // Directory searches repeat without leaking handles; the deleted imported map stays hidden.
    for _ in 0..70 {
        let mut f = g.request(0x4e00, "C:\\MAPS\\*.TSG");
        ok(&mut f);
        assert_eq!(g.cstr(6030), "NATIVE.TSG");
        f.eax = 0x4f00;
        fails(&mut f, Some(18));
    }
    // Installed assets are read-only; traversal and symlink escapes are refused.
    fails(&mut g.request(0x3c00, "C:\\MAPS\\ISLAND.RGM"), Some(5));
    fails(
        &mut g.request(0x3c00, "C:\\SAVEGAME\\..\\..\\escape.ini"),
        None,
    );
    fs::create_dir(root.join("outside")).unwrap();
    symlink(
        root.join("outside"),
        root.join("user/SAVEGAME/SAVEGAME.666"),
    )
    .unwrap();
    fails(
        &mut g.request(0x3c00, "C:\\SAVEGAME\\SAVEGAME.666\\ESCAPE.SAV"),
        None,
    );

    // Re-importing never resurrects deleted files or touches the install.
    std::env::set_current_dir(&root).unwrap();
    assert_eq!(unsafe { rg_user_storage_init(c("user").as_ptr()) }, 1);
    assert!(!Path::new("user/MAPS/ISLAND.TSG").exists());
    assert!(!Path::new("outside/ESCAPE.SAV").exists());
    expect("game/Savegame/SAVEGAME.001/SAVEGAME.SAV", "original save");
    expect("game/Maps/ISLAND.TSG", "original map");
    expect("game/Maps/ISLAND.RGM", "asset");
    expect("game/SYSTEM.INI", "original settings");
    let _ = fs::remove_dir_all(&root);
}
