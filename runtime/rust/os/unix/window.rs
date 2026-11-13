//! Window handle contract expected by the separately linked renderer.
pub(crate) fn renderer_window(
    window: &sdl2::video::Window,
) -> Result<*mut std::ffi::c_void, String> {
    Ok(window.raw().cast())
}
