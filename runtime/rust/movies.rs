use std::ffi::{CStr, c_char, c_int, c_void};

pub(crate) fn leaf(request: &[u8]) -> Option<&[u8]> {
    if request.len() < 2 || !request[0].eq_ignore_ascii_case(&b'D') || request[1] != b':' {
        return None;
    }
    let name = &request[2..];
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

/// # Safety
/// `requested` points to a live NUL-terminated DOS pathname.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_movie_leaf(requested: *const c_char) -> *const c_char {
    if requested.is_null() {
        return std::ptr::null();
    }
    let request = unsafe { CStr::from_ptr(requested) }.to_bytes();
    leaf(request).map_or(std::ptr::null(), |name| name.as_ptr().cast())
}

/// # Safety
/// `requested` is a live C string. `lookup` accepts a temporary NUL-terminated
/// relative pathname and does not retain it; `context` meets its contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_movie_find(
    requested: *const c_char,
    lookup: unsafe extern "C" fn(*const c_char, *mut c_void) -> c_int,
    context: *mut c_void,
) -> c_int {
    if requested.is_null() {
        return 0;
    }
    let request = unsafe { CStr::from_ptr(requested) }.to_bytes();
    let Some(name) = leaf(request) else {
        return 0;
    };
    let mut relative = [0u8; 4096];
    for folder in [b"".as_slice(), b"MOVIES/", b"ANIMS/", b"VIDEO/"] {
        let length = folder.len() + name.len();
        if length >= relative.len() {
            continue;
        }
        relative[..folder.len()].copy_from_slice(folder);
        relative[folder.len()..length].copy_from_slice(name);
        relative[length] = 0;
        if unsafe { lookup(relative.as_ptr().cast(), context) } != 0 {
            return 1;
        }
    }
    -1
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
        for name in [
            b"".as_slice(),
            b"D:",
            b"D:.SMK",
            b"E:INTRO.SMK",
            b"D:../INTRO.SMK",
            b"D:*.SMK",
            b"D:INTRO.TXT",
        ] {
            assert_eq!(leaf(name), None);
        }
    }
}
