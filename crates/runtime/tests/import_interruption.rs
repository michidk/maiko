//! A first import that is killed part-way never publishes a partial save: the temporary copy has a
//! `.rg-import-` name, the final file appears only when complete, and a restart imports the full
//! original bytes. Replaces the old C test of the same name.
#![cfg(unix)]
use maiko_runtime::storage::{rg_path_root, rg_user_storage_init};
use std::ffi::CString;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const SIZE: u64 = 128 * 1024 * 1024;
const TAIL: &[u8] = b"owned fixture";

fn c(path: &str) -> CString {
    CString::new(path).unwrap()
}

/// Runs only as the child process of `killed_import_never_publishes_a_partial_save`.
#[test]
fn import_child() {
    if let Ok(root) = std::env::var("RG_IMPORT_CHILD") {
        std::env::set_current_dir(root).unwrap();
        unsafe {
            assert_eq!(rg_path_root(c("game").as_ptr()), 1);
            std::process::exit(if rg_user_storage_init(c("user").as_ptr()) == 1 {
                0
            } else {
                1
            });
        }
    }
}

#[test]
fn killed_import_never_publishes_a_partial_save() {
    let root = std::env::temp_dir().join(format!("rg-import-interruption-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("game/SAVEGAME")).unwrap();
    let mut source = fs::File::create(root.join("game/SAVEGAME/LARGE.SAV")).unwrap();
    source.set_len(SIZE).unwrap();
    source
        .seek(SeekFrom::Start(SIZE - TAIL.len() as u64))
        .unwrap();
    source.write_all(TAIL).unwrap();
    drop(source);

    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "import_child", "--nocapture"])
        .env("RG_IMPORT_CHILD", &root)
        .spawn()
        .unwrap();
    let mut interrupted = false;
    for _ in 0..10_000 {
        if let Ok(entries) = fs::read_dir(root.join("user/SAVEGAME"))
            && entries
                .flatten()
                .any(|e| e.file_name().to_string_lossy().starts_with(".rg-import-"))
        {
            child.kill().unwrap();
            interrupted = true;
            break;
        }
        std::thread::sleep(Duration::from_micros(100));
    }
    let status = child.wait().unwrap();
    assert!(
        interrupted,
        "never saw the temporary import file; the copy finished too fast"
    );
    assert_eq!(status.signal(), Some(libc::SIGKILL));
    assert!(
        !root.join("user/SAVEGAME/LARGE.SAV").exists(),
        "partial save was published"
    );

    std::env::set_current_dir(&root).unwrap();
    unsafe {
        assert_eq!(rg_path_root(c("game").as_ptr()), 1);
        assert_eq!(rg_user_storage_init(c("user").as_ptr()), 1);
    }
    let mut output = fs::File::open(root.join("user/SAVEGAME/LARGE.SAV")).unwrap();
    assert_eq!(output.metadata().unwrap().len(), SIZE);
    output
        .seek(SeekFrom::Start(SIZE - TAIL.len() as u64))
        .unwrap();
    let mut tail = vec![0; TAIL.len()];
    output.read_exact(&mut tail).unwrap();
    assert_eq!(tail, TAIL);
    let _ = fs::remove_dir_all(Path::new(&root));
}
