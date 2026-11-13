//! Window handle contract expected by the separately linked renderer.
pub(crate) fn renderer_window(
    window: &sdl2::video::Window,
) -> Result<*mut std::ffi::c_void, String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().map_err(|e| e.to_string())?.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as *mut std::ffi::c_void),
        _ => Err("SDL did not provide a Windows window handle".into()),
    }
}
