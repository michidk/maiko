/// The path after the drive of a request for the CD drive. The original reads movies from
/// `animation_drive` in SYSTEM.INI: `D:\` in the GOG release, the drive the disc was in when the
/// CD release was installed. The game itself only uses `C:`, so any other letter means the disc.
pub(crate) fn cd_drive(request: &[u8]) -> Option<&[u8]> {
    (request.len() >= 2
        && request[0].is_ascii_alphabetic()
        && !request[0].eq_ignore_ascii_case(&b'C')
        && request[1] == b':')
        .then(|| &request[2..])
}

pub(crate) fn leaf(request: &[u8]) -> Option<&[u8]> {
    let name = cd_drive(request)?;
    let name = &name[name
        .iter()
        .take_while(|&&c| c == b'/' || c == b'\\')
        .count()..];
    if name.len() <= 4
        || !name[name.len() - 4..].eq_ignore_ascii_case(b".SMK")
        || name.iter().any(|c| b"\\/:*?<>|\"".contains(c))
    {
        return None;
    }
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dos_movie_names_preserve_bytes_and_path_boundaries() {
        assert_eq!(leaf(b"d:\\INTRO.smk"), Some(b"INTRO.smk".as_slice()));
        assert_eq!(leaf(b"D://\\INTRO.SMK"), Some(b"INTRO.SMK".as_slice()));
        assert_eq!(leaf(b"D:\\ leading.SMK"), Some(b" leading.SMK".as_slice()));
        assert_eq!(leaf(b"D: leading.SMK"), Some(b" leading.SMK".as_slice()));
        // A CD release installed from another drive asks for that drive.
        assert_eq!(leaf(b"e:\\SCENE3.SMK"), Some(b"SCENE3.SMK".as_slice()));
        for name in [
            b"".as_slice(),
            b"D:",
            b"D:.SMK",
            b"C:INTRO.SMK",
            b"1:INTRO.SMK",
            b"D:../INTRO.SMK",
            b"D:*.SMK",
            b"D:INTRO.TXT",
        ] {
            assert_eq!(leaf(name), None);
        }
    }
}
