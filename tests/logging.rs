//! Exercise redirection and abrupt termination in child processes only.
use std::process::Command;

#[test]
fn logging_child() {
    let Some(directory) = std::env::var_os("MAIKO_TEST_LOG_DIRECTORY") else {
        return;
    };
    maiko_runtime::logging::initialize_in(std::path::Path::new(&directory)).unwrap();
    maiko_runtime::rg_log!("test Rust console and file marker");
    maiko_runtime::logging::game_output(false, b"\"game quote ").unwrap();
    maiko_runtime::logging::game_output(false, b"marker\"\r\n").unwrap();
    unsafe {
        libc::write(1, b"native stdout marker\n".as_ptr().cast(), 21);
    }
    unsafe {
        libc::write(2, b"native stderr marker\n".as_ptr().cast(), 21);
    }
    if std::env::var_os("MAIKO_TEST_FATAL").is_some() {
        maiko_runtime::crash::rg_crash_install();
        #[cfg(unix)]
        unsafe {
            libc::raise(libc::SIGABRT);
        }
    }
    maiko_runtime::logging::exit("test-request", 17);
    std::process::exit(17);
}

#[test]
fn captures_native_output_exit_details_and_retains_seven_logs() {
    let directory = tempfile::tempdir().unwrap();
    for index in 0..9 {
        std::fs::write(
            directory.path().join(format!("maiko-{index:020}-1.log")),
            b"old",
        )
        .unwrap();
    }
    std::fs::write(directory.path().join("keep.txt"), b"unrelated").unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "logging_child", "--nocapture"])
        .env("MAIKO_TEST_LOG_DIRECTORY", directory.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(17));
    let console = String::from_utf8_lossy(&output.stderr);
    assert!(console.contains("test Rust console and file marker"));
    assert!(console.contains("game: \"game quote marker\"\r\n"));
    let files = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "log"))
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 7);
    assert!(directory.path().join("keep.txt").is_file());
    let text = files
        .iter()
        .map(|path| std::fs::read_to_string(path).unwrap())
        .find(|text| text.contains("event=startup"))
        .unwrap();
    for expected in [
        "os=",
        "arch=",
        "backend=",
        "executable=",
        "test Rust console and file marker",
        "native stdout marker",
        "native stderr marker",
        "game: \"game quote marker\"\r\n",
        "reason=test-request status=17",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
}

#[cfg(unix)]
#[test]
fn fatal_report_reaches_file_before_immediate_exit() {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "logging_child", "--nocapture"])
        .env("MAIKO_TEST_LOG_DIRECTORY", directory.path())
        .env("MAIKO_TEST_FATAL", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let path = std::fs::read_dir(directory.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains("event=startup") && text.contains("BLOCKED"));
}
