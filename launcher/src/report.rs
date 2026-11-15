//! Crash reports: one timestamped ZIP in the log folder's `crash-reports` folder with what is needed to look into a game
//! that crashed or froze: the run's log, its last screenshot, system details, the launcher
//! settings, the player's INI files and newest save. Game files are never included.
use crate::font::named;
use crate::settings::Settings;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::SimpleFileOptions;

/// Where the report's inputs are.
pub struct Sources<'a> {
    pub logs: Option<PathBuf>,
    pub game: Option<&'a Path>,
    pub user: Option<PathBuf>,
    pub settings: &'a Settings,
}

/// How the game ended, for a report made after it stopped; `None` when the player asks for one.
pub struct Exit {
    pub pid: u32,
    pub status: String,
}

/// Write `Maiko-crash-<UTC date and time>.zip` (or `Maiko-report-...` without `exit`) into the
/// `crash-reports` folder inside the log folder, falling back to `fallback` when that cannot be
/// written.
pub fn create(fallback: &Path, sources: &Sources, exit: Option<&Exit>) -> io::Result<PathBuf> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (date, time) = utc(now);
    let kind = if exit.is_some() { "crash" } else { "report" };
    let name = format!("Maiko-{kind}-{date}_{}.zip", time.replace(':', "-"));
    let bytes = archive(sources, exit, &format!("{date} {time} UTC"))?;
    let mut last = None;
    let reports = sources.logs.as_ref().map(|logs| logs.join("crash-reports"));
    for directory in [reports.as_deref(), Some(fallback)].into_iter().flatten() {
        let path = directory.join(&name);
        match std::fs::create_dir_all(directory).and_then(|()| std::fs::write(&path, &bytes)) {
            Ok(()) => return Ok(path),
            Err(error) => last = Some(error),
        }
    }
    Err(last.unwrap_or_else(|| io::ErrorKind::NotFound.into()))
}

fn archive(sources: &Sources, exit: Option<&Exit>, created: &str) -> io::Result<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut add = |name: &str, data: &[u8]| -> io::Result<()> {
        zip.start_file(name, options).map_err(io::Error::other)?;
        zip.write_all(data)
    };
    let log = sources
        .logs
        .as_deref()
        .and_then(|logs| session_log(logs, exit.map(|exit| exit.pid)));
    add(
        "system.txt",
        system(sources, exit, log.as_deref(), created).as_bytes(),
    )?;
    if let Some(log) = &log {
        let name = log.file_name().unwrap_or_default().to_string_lossy();
        add(&format!("logs/{name}"), &std::fs::read(log)?)?;
        if let Ok(screenshot) = std::fs::read(log.with_extension("bmp")) {
            add("screenshot.bmp", &screenshot)?;
        }
    }
    if let Some(renderer) = sources.game.and_then(renderer_log) {
        add("logs/OpenGLid.log", renderer.as_bytes())?;
    }
    add(
        "launcher.json",
        &serde_json::to_vec_pretty(sources.settings).map_err(io::Error::other)?,
    )?;
    if let Some(user) = &sources.user {
        for path in files(user) {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.to_ascii_uppercase().ends_with(".INI") {
                add(&format!("user/{name}"), &std::fs::read(&path)?)?;
            }
        }
        if let Some(slot) = named(user, "SAVEGAME").and_then(|saves| newest_slot(&saves)) {
            let slot_name = slot.file_name().unwrap_or_default().to_string_lossy();
            for path in files(&slot) {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                add(
                    &format!("user/SAVEGAME/{slot_name}/{name}"),
                    &std::fs::read(&path)?,
                )?;
            }
        }
    }
    Ok(zip.finish().map_err(io::Error::other)?.into_inner())
}

/// The game names its log `maiko-<nanoseconds>-<pid>.log`; without a pid, the newest one.
fn session_log(logs: &Path, pid: Option<u32>) -> Option<PathBuf> {
    files(logs)
        .into_iter()
        .filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("maiko-")
                && match pid {
                    Some(pid) => name.ends_with(&format!("-{pid}.log")),
                    None => name.ends_with(".log"),
                }
        })
        .max()
}

/// OpenGLide appends its graphics-driver details to `OpenGLid.log` in the game folder on every
/// start; keep the last block, capped so a long-grown file stays small.
fn renderer_log(game: &Path) -> Option<String> {
    let text =
        String::from_utf8_lossy(&std::fs::read(named(game, "OpenGLid.log")?).ok()?).into_owned();
    let start = text
        .rfind("** OpenGL Information **")
        .map_or(0, |at| text[..at].rfind('\n').map_or(0, |line| line + 1));
    let start = start.max(text.len().saturating_sub(64 * 1024));
    let start = (start..=text.len()).find(|&at| text.is_char_boundary(at))?;
    Some(text[start..].to_owned())
}

fn newest_slot(saves: &Path) -> Option<PathBuf> {
    std::fs::read_dir(saves)
        .ok()?
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .max_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok())
        .map(|entry| entry.path())
}

/// The regular files directly in `directory`, sorted.
fn files(directory: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.path())
        .collect();
    files.sort();
    files
}

fn system(sources: &Sources, exit: Option<&Exit>, log: Option<&Path>, created: &str) -> String {
    let unknown = || "unknown".to_owned();
    let path = |path: Option<&Path>| path.map_or_else(unknown, |p| p.display().to_string());
    let mut text = format!(
        "Maiko report\nCreated: {created}\nReason: {}\nMaiko launcher: {}\nOS: {}\nArchitecture: {}\nCPU: {} ({} threads)\nGame folder: {}\nPlayer files: {}\nLog: {}\nLaunch arguments: {:?}\n",
        exit.map_or_else(
            || "created from Technical Support".to_owned(),
            |exit| format!(
                "the game (process {}) stopped with {}",
                exit.pid, exit.status
            )
        ),
        env!("CARGO_PKG_VERSION"),
        os().unwrap_or_else(unknown),
        std::env::consts::ARCH,
        cpu().unwrap_or_else(unknown),
        std::thread::available_parallelism().map_or(0, |n| n.get()),
        path(sources.game),
        path(sources.user.as_deref()),
        path(log),
        match (sources.game, &sources.user) {
            (Some(game), Some(user)) => sources.settings.arguments(game, user),
            _ => Vec::new(),
        },
    );
    if let Some(game) = sources.game {
        text.push_str("\nGame folder contents (name, bytes):\n");
        listing(game, game, 0, &mut text);
    }
    text
}

/// File names and sizes two folders deep; enough to tell editions and missing files apart.
fn listing(root: &Path, directory: &Path, depth: usize, text: &mut String) {
    let mut entries: Vec<_> = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path).display();
        // Follow links: some installs link their files from elsewhere.
        match std::fs::metadata(&path) {
            Ok(meta) if meta.is_dir() => {
                text.push_str(&format!("  {relative}/\n"));
                if depth < 1 {
                    listing(root, &path, depth + 1, text);
                }
            }
            Ok(meta) => text.push_str(&format!("  {relative}  {}\n", meta.len())),
            Err(_) => {}
        }
    }
}

/// A command's trimmed standard output, without flashing a console window on Windows.
fn output(program: &str, args: &[&str]) -> Option<String> {
    let mut command = std::process::Command::new(program);
    command.args(args);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, 0x0800_0000);
    let out = command.output().ok().filter(|out| out.status.success())?;
    let text = String::from_utf8_lossy(&out.stdout)
        .trim()
        .replace('\n', "; ");
    (!text.is_empty()).then_some(text)
}

fn os() -> Option<String> {
    if cfg!(windows) {
        output("cmd", &["/c", "ver"])
    } else if cfg!(target_os = "macos") {
        output("sw_vers", &[])
    } else {
        let release = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|text| {
                text.lines()
                    .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                    .map(|name| name.trim_matches('"').to_owned())
            });
        let kernel = output("uname", &["-sr"]);
        Some(format!(
            "{} ({})",
            release.as_deref().unwrap_or("Linux"),
            kernel.as_deref().unwrap_or("unknown kernel")
        ))
    }
}

fn cpu() -> Option<String> {
    if cfg!(windows) {
        std::env::var("PROCESSOR_IDENTIFIER").ok()
    } else if cfg!(target_os = "macos") {
        output("sysctl", &["-n", "machdep.cpu.brand_string"])
    } else {
        std::fs::read_to_string("/proc/cpuinfo")
            .ok()?
            .lines()
            .find_map(|line| {
                let (key, value) = line.split_once(':')?;
                (key.trim() == "model name").then(|| value.trim().to_owned())
            })
    }
}

/// `YYYY-MM-DD` and `HH:MM:SS` in UTC for seconds since 1970.
fn utc(seconds: u64) -> (String, String) {
    let (days, rest) = ((seconds / 86_400) as i64, seconds % 86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (
        format!("{year:04}-{month:02}-{day:02}"),
        format!("{:02}:{:02}:{:02}", rest / 3600, rest / 60 % 60, rest % 60),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_utc_calendar_dates() {
        assert_eq!(utc(0), ("1970-01-01".into(), "00:00:00".into()));
        assert_eq!(utc(951_782_400), ("2000-02-29".into(), "00:00:00".into()));
        assert_eq!(utc(1_794_737_389), ("2026-11-15".into(), "10:09:49".into()));
    }

    #[test]
    fn a_crash_report_holds_the_runs_log_screenshot_and_player_files() {
        let root = tempfile::tempdir().unwrap();
        let (logs, game, user, out) = (
            root.path().join("logs"),
            root.path().join("game"),
            root.path().join("user"),
            root.path().join("launcher"),
        );
        for directory in [&logs, &game, &out, &user.join("SAVEGAME/SAVEGAME.002")] {
            std::fs::create_dir_all(directory).unwrap();
        }
        std::fs::write(logs.join("maiko-01-7.log"), "crashed run").unwrap();
        std::fs::write(logs.join("maiko-01-7.bmp"), "BM").unwrap();
        std::fs::write(logs.join("maiko-02-8.log"), "later run").unwrap();
        std::fs::write(game.join("RGFX.EXE"), "game file").unwrap();
        std::fs::write(
            game.join("OpenGLid.log"),
            "-----\n** OpenGL Information **\nRenderer: old\n-----\n** OpenGL Information **\nRenderer: x\n",
        )
        .unwrap();
        std::fs::write(user.join("SYSTEM.INI"), "ini").unwrap();
        std::fs::write(user.join("SAVEGAME/SAVEGAME.002/SAVEGAME.SAV"), "save").unwrap();
        let settings = Settings::default();
        let sources = Sources {
            logs: Some(logs),
            game: Some(&game),
            user: Some(user),
            settings: &settings,
        };
        let exit = Exit {
            pid: 7,
            status: "exit status: 3".into(),
        };
        let path = create(&out, &sources, Some(&exit)).unwrap();
        assert!(path.starts_with(sources.logs.as_ref().unwrap().join("crash-reports")));
        assert!(
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("Maiko-crash-")
        );
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        let mut read = |name: &str| {
            let mut text = String::new();
            io::Read::read_to_string(&mut zip.by_name(name).unwrap(), &mut text).unwrap();
            text
        };
        assert_eq!(read("logs/maiko-01-7.log"), "crashed run");
        assert_eq!(read("screenshot.bmp"), "BM");
        assert_eq!(
            read("logs/OpenGLid.log"),
            "** OpenGL Information **\nRenderer: x\n"
        );
        assert_eq!(read("user/SYSTEM.INI"), "ini");
        assert_eq!(read("user/SAVEGAME/SAVEGAME.002/SAVEGAME.SAV"), "save");
        let system = read("system.txt");
        assert!(system.contains("stopped with exit status: 3"), "{system}");
        assert!(system.contains("RGFX.EXE  9"), "{system}");
        assert!(zip.by_name("RGFX.EXE").is_err(), "game files stay out");
    }
}
