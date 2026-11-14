//! Per-run diagnostics. Native stdout/stderr and fatal writes share the log file.
use std::ffi::OsString;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const KEPT_LOGS: usize = 7;
struct Session {
    _file: File,
    console: Option<File>,
    started: Instant,
    path: PathBuf,
}
static SESSION: OnceLock<Session> = OnceLock::new();
/// Serializes records; also holds whether each original-game stream (stdout,
/// stderr) is at the start of a line.
static RECORD: Mutex<[bool; 2]> = Mutex::new([true, true]);

#[macro_export]
macro_rules! rg_log {
    ($($arg:tt)*) => { $crate::logging::write(format_args!($($arg)*)) };
}

pub fn initialize() -> io::Result<PathBuf> {
    let preferred = std::env::var_os("MAIKO_LOG_DIR")
        .map(PathBuf::from)
        .map_or_else(crate::os::log_directory, Ok);
    let error = match preferred.and_then(|directory| initialize_in(&directory)) {
        Ok(path) => return Ok(path),
        Err(error) => error,
    };
    let path = initialize_in(&std::env::temp_dir().join("Maiko-logs"))?;
    crate::rg_log!("event=log-directory-fallback error={error}");
    Ok(path)
}

/// Initialize once, before game threads start. A directory override is useful
/// for launch tools and subprocess fixtures; it never changes the game root.
pub fn initialize_in(directory: &Path) -> io::Result<PathBuf> {
    if let Some(session) = SESSION.get() {
        return Ok(session.path.clone());
    }
    std::fs::create_dir_all(directory)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let path = directory.join(format!(
        "maiko-{:020}-{}.log",
        stamp.as_nanos(),
        std::process::id()
    ));
    let file = OpenOptions::new()
        .append(true)
        .create_new(true)
        .open(&path)?;
    let console = crate::os::capture_output(&file)?;
    let session = Session {
        _file: file,
        console,
        started: Instant::now(),
        path: path.clone(),
    };
    SESSION
        .set(session)
        .map_err(|_| io::Error::from(io::ErrorKind::AlreadyExists))?;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let record = format!(
            "event=panic {info}\n{}\n",
            std::backtrace::Backtrace::force_capture()
        );
        crate::os::write_stderr(record.as_bytes());
        fatal_console(record.as_bytes());
        previous(info);
    }));
    // Only our own regular log files are eligible for retention pruning.
    let mut logs = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("maiko-") && name.ends_with(".log"))
        })
        .collect::<Vec<_>>();
    logs.sort();
    let old = logs.len().saturating_sub(KEPT_LOGS);
    for path in logs.into_iter().take(old) {
        let _ = std::fs::remove_file(path);
    }
    crate::rg_log!(
        "event=startup version={} revision={} os={} os_version={:?} arch={} backend={} pid={} executable={:?} args={:?} log={}",
        env!("CARGO_PKG_VERSION"),
        env!("MAIKO_REVISION"),
        std::env::consts::OS,
        os_version(),
        std::env::consts::ARCH,
        if cfg!(feature = "llasm") {
            "llasm"
        } else {
            "native"
        },
        std::process::id(),
        std::env::current_exe().ok(),
        std::env::args_os().collect::<Vec<OsString>>(),
        path.display()
    );
    Ok(path)
}

#[cfg(unix)]
fn os_version() -> String {
    let mut name: libc::utsname = unsafe { std::mem::zeroed() };
    if unsafe { libc::uname(&mut name) } != 0 {
        return "unknown".into();
    }
    let field = |value: &[libc::c_char]| {
        let bytes = value
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect::<Vec<_>>();
        String::from_utf8_lossy(&bytes).into_owned()
    };
    format!(
        "{} {} {}",
        field(&name.sysname),
        field(&name.release),
        field(&name.version)
    )
}

#[cfg(windows)]
fn os_version() -> String {
    use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
    use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;
    let mut info: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
    info.dwOSVersionInfoSize = size_of::<OSVERSIONINFOW>() as u32;
    if unsafe { RtlGetVersion(&mut info) } != 0 {
        return "unknown".into();
    }
    format!(
        "Windows {}.{}.{}",
        info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
    )
}

/// Normal records may allocate and lock. Fatal handlers use raw OS writes.
pub fn write(arguments: fmt::Arguments<'_>) {
    let _guard = RECORD.lock().unwrap_or_else(|error| error.into_inner());
    let elapsed = SESSION
        .get()
        .map_or(0, |session| session.started.elapsed().as_millis());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let record = format!(
        "time={now} elapsed_ms={elapsed} thread={:?} {arguments}\n",
        std::thread::current().id()
    );
    let _ = io::stderr().write_all(record.as_bytes());
    if let Some(console) = SESSION.get().and_then(|session| session.console.as_ref()) {
        let _ = (&*console).write_all(record.as_bytes());
    }
}

/// Original-game console output (DOS handles 1 and 2): every line is tagged
/// `game: ` and goes to the session log and the terminal, as on DOS.
pub fn game_output(stderr: bool, data: &[u8]) -> io::Result<()> {
    let mut starts = RECORD.lock().unwrap_or_else(|error| error.into_inner());
    let tagged = tag_game_lines(&mut starts[usize::from(stderr)], data);
    if stderr {
        io::stderr().write_all(&tagged)?;
    } else {
        let mut stdout = io::stdout();
        stdout.write_all(&tagged)?;
        stdout.flush()?;
    }
    if let Some(console) = SESSION.get().and_then(|session| session.console.as_ref()) {
        let _ = (&*console).write_all(&tagged);
    }
    Ok(())
}

fn tag_game_lines(line_start: &mut bool, data: &[u8]) -> Vec<u8> {
    let mut tagged = Vec::with_capacity(data.len() + 6);
    for line in data.split_inclusive(|&byte| byte == b'\n') {
        if *line_start {
            tagged.extend_from_slice(b"game: ");
        }
        tagged.extend_from_slice(line);
        *line_start = line.ends_with(b"\n");
    }
    tagged
}

pub(crate) fn fatal_console(bytes: &[u8]) {
    if let Some(console) = SESSION.get().and_then(|session| session.console.as_ref()) {
        crate::os::write_console(console, bytes);
    }
}

pub fn exit(reason: &str, status: i32) {
    crate::rg_log!("event=exit reason={reason} status={status}");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn game_lines_are_tagged_across_partial_writes() {
        let mut start = true;
        let mut out = tag_game_lines(&mut start, b"RedGuard version\r\n\r\n\"[Charlie] ");
        out.extend(tag_game_lines(&mut start, b"Bye\"\r\n"));
        assert_eq!(
            out,
            b"game: RedGuard version\r\ngame: \r\ngame: \"[Charlie] Bye\"\r\n"
        );
        assert!(start);
    }
}
