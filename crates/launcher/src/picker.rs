//! The system's folder picker. `None` when the user cancels or the system has none, in which case the
//! launcher asks for the path in its own window.
use std::path::PathBuf;

const PROMPT: &str = "Select your Redguard folder (the one containing RGFX.EXE)";

/// The trimmed first line a picker program printed, if it succeeded.
#[cfg(unix)]
fn printed_path(output: std::io::Result<std::process::Output>) -> Option<PathBuf> {
    let output = output.ok().filter(|output| output.status.success())?;
    let text = String::from_utf8(output.stdout).ok()?;
    let line = text.lines().next()?.trim_end_matches(['\r', '\n']);
    (!line.is_empty()).then(|| PathBuf::from(line))
}

#[cfg(target_os = "macos")]
pub fn choose_folder() -> Option<PathBuf> {
    printed_path(
        std::process::Command::new("/usr/bin/osascript")
            .args([
                "-e",
                &format!("POSIX path of (choose folder with prompt \"{PROMPT}\")"),
            ])
            .output(),
    )
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn choose_folder() -> Option<PathBuf> {
    use std::process::Command;
    // GNOME and most desktops have zenity, KDE has kdialog.
    let pickers = [
        Command::new("zenity")
            .args(["--file-selection", "--directory", "--title", PROMPT])
            .output(),
        Command::new("kdialog")
            .args(["--getexistingdirectory", ".", "--title", PROMPT])
            .output(),
    ];
    pickers.into_iter().find_map(printed_path)
}

#[cfg(windows)]
pub fn choose_folder() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::{System::Com::*, UI::Shell::*};
    unsafe {
        if CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) < 0 {
            return None;
        }
        let title: Vec<u16> = PROMPT.encode_utf16().chain([0]).collect();
        let mut options: BROWSEINFOW = std::mem::zeroed();
        options.lpszTitle = title.as_ptr();
        options.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;
        let selected = SHBrowseForFolderW(&options);
        let mut chosen = [0u16; 32768];
        let valid = !selected.is_null()
            && SHGetPathFromIDListEx(selected, chosen.as_mut_ptr(), chosen.len() as u32, 0) != 0;
        CoTaskMemFree(selected.cast());
        CoUninitialize();
        if !valid {
            return None;
        }
        let length = chosen
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(chosen.len());
        Some(std::ffi::OsString::from_wide(&chosen[..length]).into())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn a_picker_path_keeps_spaces_and_cancel_gives_none() {
        let output = |status, bytes: &[u8]| {
            Ok(std::process::Output {
                status: std::process::ExitStatus::from_raw(status),
                stdout: bytes.to_vec(),
                stderr: Vec::new(),
            })
        };
        assert_eq!(
            printed_path(output(0, b"/home/p/GOG Games/Redguard/\n")),
            Some(PathBuf::from("/home/p/GOG Games/Redguard/"))
        );
        assert_eq!(printed_path(output(256, b"")), None);
        assert_eq!(printed_path(output(0, b"\n")), None);
        assert_eq!(printed_path(Err(std::io::ErrorKind::NotFound.into())), None);
    }
}
