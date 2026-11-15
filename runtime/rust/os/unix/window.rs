//! The window handle `rg_input_window` returns to the C host and tests.
pub(crate) fn host_window_handle(
    window: &sdl2::video::Window,
) -> Result<*mut std::ffi::c_void, String> {
    Ok(window.raw().cast())
}
