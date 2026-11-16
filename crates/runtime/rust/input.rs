//! The game window (winit) and DOS keyboard/mouse state shared by every host. The translated game
//! owns its main loop, so window events are pumped from it instead of from a winit event loop.
use crate::rg_log;
#[path = "controller.rs"]
mod controller;
#[path = "overlay.rs"]
pub mod overlay;
use controller::Controllers;
use overlay::Routed;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::{c_int, c_void};
use std::mem::ManuallyDrop;
use std::sync::Arc;
use std::time::Duration;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Fullscreen, Window, WindowId};

/// Collects window events while the loop is pumped.
#[derive(Default)]
struct Pending(Vec<WindowEvent>);

impl ApplicationHandler for Pending {
    fn resumed(&mut self, _: &ActiveEventLoop) {}

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.0.push(event);
    }
}

struct Input {
    events: Option<EventLoop<()>>,
    window: Option<Arc<Window>>,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
    buttons: u32,
    cursor: bool,
    known: bool,
    focused: bool,
    closed: bool,
    queue: VecDeque<u8>,
    held: [bool; 512],
    keyboard_held: [bool; 512],
    controller_held: [bool; 512],
    controllers: Option<Controllers>,
    controllers_initialized: bool,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            events: None,
            window: None,
            width: 640,
            height: 480,
            x: 0,
            y: 0,
            dx: 0,
            dy: 0,
            buttons: 0,
            cursor: true,
            known: false,
            focused: false,
            closed: false,
            queue: VecDeque::with_capacity(256),
            held: [false; 512],
            keyboard_held: [false; 512],
            controller_held: [false; 512],
            controllers: None,
            controllers_initialized: false,
        }
    }
}

// The window and the renderer's surface share process-lifetime state; their destructors must not
// run before the CRT's exit callbacks or under the Windows loader lock.
thread_local! { static INPUT: RefCell<ManuallyDrop<Input>> = RefCell::new(ManuallyDrop::new(Input::default())); }

/// The DOS (set 1) scan code of a physical key; extended keys carry bit 8. 0 for keys DOS lacks.
fn dos_scan(key: KeyCode) -> u16 {
    use KeyCode::*;
    match key {
        KeyA => 0x1e,
        KeyB => 0x30,
        KeyC => 0x2e,
        KeyD => 0x20,
        KeyE => 0x12,
        KeyF => 0x21,
        KeyG => 0x22,
        KeyH => 0x23,
        KeyI => 0x17,
        KeyJ => 0x24,
        KeyK => 0x25,
        KeyL => 0x26,
        KeyM => 0x32,
        KeyN => 0x31,
        KeyO => 0x18,
        KeyP => 0x19,
        KeyQ => 0x10,
        KeyR => 0x13,
        KeyS => 0x1f,
        KeyT => 0x14,
        KeyU => 0x16,
        KeyV => 0x2f,
        KeyW => 0x11,
        KeyX => 0x2d,
        KeyY => 0x15,
        KeyZ => 0x2c,
        Digit1 => 2,
        Digit2 => 3,
        Digit3 => 4,
        Digit4 => 5,
        Digit5 => 6,
        Digit6 => 7,
        Digit7 => 8,
        Digit8 => 9,
        Digit9 => 0xa,
        Digit0 => 0xb,
        F1 => 0x3b,
        F2 => 0x3c,
        F3 => 0x3d,
        F4 => 0x3e,
        F5 => 0x3f,
        F6 => 0x40,
        F7 => 0x41,
        F8 => 0x42,
        F9 => 0x43,
        F10 => 0x44,
        F11 => 0x57,
        F12 => 0x58,
        Enter => 0x1c,
        Escape => 1,
        Space => 0x39,
        Tab => 0xf,
        Backspace => 0xe,
        Minus => 0xc,
        Equal => 0xd,
        BracketLeft => 0x1a,
        BracketRight => 0x1b,
        Semicolon => 0x27,
        Quote => 0x28,
        Backquote => 0x29,
        Backslash => 0x2b,
        Comma => 0x33,
        Period => 0x34,
        Slash => 0x35,
        ShiftLeft => 0x2a,
        ShiftRight => 0x36,
        ControlLeft => 0x1d,
        ControlRight => 0x11d,
        AltLeft => 0x38,
        AltRight => 0x138,
        CapsLock => 0x3a,
        NumLock => 0x45,
        ScrollLock => 0x46,
        ArrowUp => 0x148,
        ArrowDown => 0x150,
        ArrowLeft => 0x14b,
        ArrowRight => 0x14d,
        Home => 0x147,
        End => 0x14f,
        PageUp => 0x149,
        PageDown => 0x151,
        Insert => 0x152,
        Delete => 0x153,
        Numpad0 => 0x52,
        Numpad1 => 0x4f,
        Numpad2 => 0x50,
        Numpad3 => 0x51,
        Numpad4 => 0x4b,
        Numpad5 => 0x4c,
        Numpad6 => 0x4d,
        Numpad7 => 0x47,
        Numpad8 => 0x48,
        Numpad9 => 0x49,
        NumpadDecimal => 0x53,
        NumpadAdd => 0x4e,
        NumpadSubtract => 0x4a,
        NumpadMultiply => 0x37,
        NumpadDivide => 0x135,
        NumpadEnter => 0x11c,
        IntlBackslash => 0x56,
        _ => 0,
    }
}

/// Display choice from the command line: `--fullscreen` or `--window-size WIDTHxHEIGHT`.
/// The game keeps rendering at its own resolution; the renderer scales it to the window.
static DISPLAY: std::sync::Mutex<(bool, Option<(u32, u32)>)> = std::sync::Mutex::new((false, None));

pub fn set_display(fullscreen: bool, size: Option<(u32, u32)>) {
    let mut display = DISPLAY.lock().unwrap_or_else(|e| e.into_inner());
    *display = (fullscreen || display.0, size.or(display.1));
}

pub fn parse_size(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.split_once(['x', 'X'])?;
    let (w, h): (u32, u32) = (w.parse().ok()?, h.parse().ok()?);
    ((320..=8192).contains(&w) && (240..=8192).contains(&h)).then_some((w, h))
}

impl Input {
    fn scan(&mut self, code: u16, release: bool) {
        let needed = if code & 256 != 0 { 2 } else { 1 };
        if self.queue.len() + needed > 256 {
            rg_log!("BLOCKED native keyboard queue overflow");
            self.closed = true;
            return;
        }
        if code & 256 != 0 {
            self.queue.push_back(0xe0);
        }
        self.queue
            .push_back((code as u8 & 127) | if release { 128 } else { 0 });
        self.held[usize::from(code)] = !release;
    }

    fn window(&mut self, width: u32, height: u32) -> Result<*mut c_void, String> {
        if width == 0 || height == 0 || width > 8192 || height > 8192 {
            return Err("Invalid window dimensions".into());
        }
        self.width = width;
        self.height = height;
        if self.window.is_none() {
            let events = EventLoop::new().map_err(|e| e.to_string())?;
            let (fullscreen, size) = *DISPLAY.lock().unwrap_or_else(|e| e.into_inner());
            let (w, h) = size.unwrap_or((width, height));
            let mut attributes = Window::default_attributes()
                .with_title("Maiko")
                .with_inner_size(winit::dpi::LogicalSize::new(w, h));
            if fullscreen {
                attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
            }
            // The game pumps its own loop, so the window is made outside a winit callback.
            #[allow(deprecated)]
            let window = events
                .create_window(attributes)
                .map_err(|e| e.to_string())?;
            self.window = Some(Arc::new(window));
            self.events = Some(events);
        } else if *DISPLAY.lock().unwrap_or_else(|e| e.into_inner()) == (false, None)
            && let Some(window) = &self.window
        {
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(width, height));
        }
        self.focused = true;
        let window = self.window.as_ref().ok_or("the window was not created")?;
        Ok(host_window_handle(window))
    }

    fn release_game_input(&mut self) {
        for code in 1..512 {
            if self.held[code] {
                self.scan(code as u16, true);
            }
        }
        self.keyboard_held.fill(false);
        self.controller_held.fill(false);
        self.buttons = 0;
        self.dx = 0;
        self.dy = 0;
        self.known = false;
    }

    fn show_cursor(&self, visible: bool) {
        if let Some(window) = &self.window {
            window.set_cursor_visible(visible);
        }
    }

    fn overlay_toggled(&mut self, open: bool) {
        if open {
            self.release_game_input();
        } else {
            self.controller_keys();
        }
        self.show_cursor(open || self.cursor);
    }

    fn controller_keys(&mut self) {
        let keys = if self.focused && !overlay::is_open() {
            self.controllers
                .as_ref()
                .map_or([false; 512], Controllers::keys)
        } else {
            [false; 512]
        };
        self.apply_controller(keys);
    }

    /// Takes the pads' held keys; releases old directions before pressing new ones, including
    /// reversals.
    fn apply_controller(&mut self, keys: [bool; 512]) {
        self.controller_held = keys;
        for down in [false, true] {
            for code in 1..512 {
                let held = self.keyboard_held[code] || self.controller_held[code];
                if held == down && self.held[code] != held {
                    self.scan(code as u16, !held);
                }
            }
        }
    }

    /// A key from the keyboard; the pad may hold the same key.
    fn keyboard(&mut self, code: u16, down: bool, repeat: bool) {
        if !self.focused || code == 0 {
            return;
        }
        let index = usize::from(code);
        self.keyboard_held[index] = down;
        let held = self.keyboard_held[index] || self.controller_held[index];
        if self.held[index] != held || (down && repeat) {
            self.scan(code, !held);
        }
    }

    fn focus(&mut self, focused: bool) {
        self.focused = focused;
        if focused {
            self.known = false;
            if let Some(controllers) = &mut self.controllers {
                controllers.refresh();
            }
            self.controller_keys();
        } else {
            self.release_game_input();
        }
    }

    /// The pointer moved to window pixel (`x`, `y`): the game's position and motion.
    fn cursor(&mut self, x: f64, y: f64) {
        let Some(size) = self.window.as_ref().map(|window| window.inner_size()) else {
            return;
        };
        if !self.focused || size.width == 0 || size.height == 0 {
            return;
        }
        let (px, py, pw, ph) = picture((size.width, size.height), (self.width, self.height));
        let game = |at: f64, start: i64, size: i64, units: u32| {
            ((at as i64 - start) * i64::from(units) / size).clamp(0, i64::from(units) - 1) as i32
        };
        let nx = game(x, px, pw, self.width);
        let ny = game(y, py, ph, self.height);
        if self.known {
            self.dx = self.dx.wrapping_add(nx.wrapping_sub(self.x));
            self.dy = self.dy.wrapping_add(ny.wrapping_sub(self.y));
        }
        self.x = nx;
        self.y = ny;
        self.known = true;
    }

    fn mouse_button(&mut self, mask: u32, pressed: bool) {
        if !pressed {
            self.buttons &= !mask;
        } else if self.focused {
            self.buttons |= mask;
        }
    }

    fn pump(&mut self) -> bool {
        if !self.controllers_initialized && self.window.is_some() {
            // Controller failure must not prevent keyboard-only play or retry each frame.
            self.controllers_initialized = true;
            match Controllers::new() {
                Ok(controllers) => self.controllers = Some(controllers),
                Err(error) => rg_log!("Native controllers unavailable: {error}"),
            }
            self.controller_keys();
        }
        if self.controllers.as_mut().is_some_and(Controllers::poll) {
            self.controller_keys();
        }
        let mut pending = Pending::default();
        if let Some(events) = &mut self.events
            && let PumpStatus::Exit(_) = events.pump_app_events(Some(Duration::ZERO), &mut pending)
        {
            self.closed = true;
        }
        let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor());
        for event in pending.0 {
            match overlay::route(&event, scale) {
                Routed::Game => {}
                Routed::Consumed => continue,
                Routed::Toggled(open) => {
                    self.overlay_toggled(open);
                    continue;
                }
            }
            match event {
                WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                    rg_log!("event=input-exit-request source={event:?}");
                    self.closed = true;
                }
                WindowEvent::Focused(focused) => self.focus(focused),
                WindowEvent::KeyboardInput {
                    event,
                    is_synthetic,
                    ..
                } => {
                    let PhysicalKey::Code(key) = event.physical_key else {
                        continue;
                    };
                    let down = event.state == ElementState::Pressed;
                    // Keys still held when focus returns (Alt+Tab) are not new presses: a
                    // synthetic Tab would toggle walk mode.
                    if is_synthetic && down {
                        continue;
                    }
                    if matches!(key, KeyCode::Escape | KeyCode::F10) && down && !event.repeat {
                        rg_log!(
                            "event=input-navigation key={key:?} focused={} overlay={}",
                            self.focused,
                            overlay::is_open()
                        );
                    }
                    self.keyboard(dos_scan(key), down, event.repeat);
                }
                WindowEvent::CursorMoved { position, .. } => self.cursor(position.x, position.y),
                WindowEvent::MouseInput { state, button, .. } => {
                    let mask = match button {
                        MouseButton::Left => 1,
                        MouseButton::Right => 2,
                        _ => 0,
                    };
                    self.mouse_button(mask, state == ElementState::Pressed);
                }
                _ => {}
            }
        }
        !self.closed
    }
}

/// The handle `rg_input_window` gives the C host: the HWND on Windows, where the host's tests post
/// messages to it, and an opaque non-null pointer elsewhere.
fn host_window_handle(window: &Arc<Window>) -> *mut c_void {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::Win32(handle)) => handle.hwnd.get() as *mut c_void,
        _ => Arc::as_ptr(window).cast_mut().cast(),
    }
}

/// The game cannot run without its window: say why and exit instead of letting the original
/// code treat the failed mouse reset or Glide open as an unsupported call.
fn window_failed(error: &str) -> ! {
    rg_log!(
        "Native window failed: {error}{}",
        window_hint(
            cfg!(target_os = "linux"),
            std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty()),
            std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
        )
    );
    rg_log!("FATAL Redguard could not open its game window and will exit.");
    crate::host::exit_cleanly(1)
}

fn window_hint(linux: bool, display: bool, wayland: bool) -> &'static str {
    match (linux, display || wayland) {
        (false, _) => "",
        (true, false) => {
            " (neither DISPLAY nor WAYLAND_DISPLAY is set: Maiko needs a desktop session)"
        }
        (true, true) => " (Maiko needs a Vulkan or OpenGL driver)",
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_input_window(width: u32, height: u32) -> *mut c_void {
    INPUT.with(|input| {
        input
            .borrow_mut()
            .window(width, height)
            .unwrap_or_else(|error| window_failed(&error))
    })
}

/// Where the game picture sits in a window of `window` points: as large as fits, centred, at the
/// game's aspect, as the renderer scales it. Returns x, y, width, height.
fn picture(window: (u32, u32), game: (u32, u32)) -> (i64, i64, i64, i64) {
    let (w, h) = (i64::from(window.0), i64::from(window.1));
    let (gw, gh) = (i64::from(game.0.max(1)), i64::from(game.1.max(1)));
    let (mut pw, mut ph) = (w, w * gh / gw);
    if ph > h {
        ph = h;
        pw = h * gw / gh;
    }
    ((w - pw) / 2, (h - ph) / 2, pw.max(1), ph.max(1))
}

/// Let the player resize the game window (not in fullscreen).
pub fn set_resizable(resizable: bool) {
    INPUT.with(|input| {
        if let Some(window) = &input.borrow().window
            && !DISPLAY.lock().unwrap_or_else(|e| e.into_inner()).0
        {
            window.set_resizable(resizable);
            if resizable {
                window.set_min_inner_size(Some(winit::dpi::LogicalSize::new(320, 240)));
            }
        }
    })
}

/// For the host's C tests: feeds one window event through the paths real events take. `kind` 0
/// presses and 1 releases the DOS scan code `value`, 2 gains and 3 loses focus, 4 asks to close,
/// 5 moves the pointer to window pixel (`value & 0xffff`, `value >> 16`), 6 presses and 7
/// releases mouse buttons `value` (1 left, 2 right). Returns 0 for an unknown kind.
#[unsafe(no_mangle)]
pub extern "C" fn rg_input_test_event(kind: u32, value: u32) -> c_int {
    INPUT.with(|input| {
        let mut input = input.borrow_mut();
        match kind {
            0 | 1 => input.keyboard(value as u16 & 511, kind == 0, false),
            2 | 3 => input.focus(kind == 2),
            4 => input.closed = true,
            5 => input.cursor(f64::from(value & 0xffff), f64::from(value >> 16)),
            6 | 7 => input.mouse_button(value & 3, kind == 6),
            _ => return 0,
        }
        1
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_input_pump() -> c_int {
    INPUT.with(|input| input.borrow_mut().pump()) as c_int
}

/// Paints the overlay over the presented frame, when it is open.
pub fn draw_overlay(target: overlay::Target<'_>) {
    INPUT.with(|input| {
        if let Some(window) = &input.borrow().window {
            let pixels = window.inner_size();
            let points = pixels.to_logical::<u32>(window.scale_factor());
            overlay::draw(
                target,
                (points.width, points.height),
                (pixels.width, pixels.height),
            );
        }
    })
}

/// The game window's raw handles and drawable size, for the renderer.
pub fn renderer_window() -> Option<(
    raw_window_handle::RawWindowHandle,
    raw_window_handle::RawDisplayHandle,
    (u32, u32),
)> {
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
    INPUT.with(|input| {
        let input = input.borrow();
        let window = input.window.as_ref()?;
        let handle = window.window_handle().ok()?.as_raw();
        let display = window.display_handle().ok()?.as_raw();
        let size = window.inner_size();
        Some((handle, display, (size.width, size.height)))
    })
}

/// The game window's drawable size in pixels.
pub fn drawable_size() -> Option<(u32, u32)> {
    INPUT.with(|input| {
        let size = input.borrow().window.as_ref()?.inner_size();
        Some((size.width, size.height))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_input_take_scan() -> c_int {
    INPUT.with(|input| input.borrow_mut().queue.pop_front().map_or(-1, c_int::from))
}

/// # Safety
/// `registers` points to seven writable u32 values. All input/window calls
/// belong to the game thread that opened the window.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_mouse_service(registers: *mut u32) -> c_int {
    if registers.is_null() {
        return 0;
    }
    let regs = unsafe { &mut *registers.cast::<[u32; 7]>() };
    INPUT.with(|input| {
        let mut input = input.borrow_mut();
        if !input.pump() {
            return 0;
        }
        match regs[0] & 65535 {
            0 => {
                if let Err(error) = input.window(640, 480) {
                    window_failed(&error);
                }
                input.buttons = 0;
                input.dx = 0;
                input.dy = 0;
                input.known = false;
                regs[0] = 65535;
                regs[1] = 2;
            }
            1 | 2 => {
                input.cursor = regs[0] & 65535 == 1;
                if !overlay::is_open() {
                    input.show_cursor(input.cursor);
                }
            }
            3 => {
                regs[1] = input.buttons;
                regs[2] = u32::from(input.x.wrapping_mul(2) as u16);
                regs[3] = u32::from(input.y as u16);
            }
            4 => {
                let Some(window) = input.window.clone() else {
                    return 0;
                };
                input.x = i32::from(regs[2] as u16) / 2;
                input.y = i32::from(regs[3] as u16);
                let size = window.inner_size();
                let (px, py, pw, ph) =
                    picture((size.width, size.height), (input.width, input.height));
                if input.focused && !overlay::is_open() {
                    let _ = window.set_cursor_position(winit::dpi::PhysicalPosition::new(
                        (px + i64::from(input.x) * pw / i64::from(input.width.max(1))) as i32,
                        (py + i64::from(input.y) * ph / i64::from(input.height.max(1))) as i32,
                    ));
                }
                input.dx = 0;
                input.dy = 0;
                input.known = true;
            }
            0xb => {
                regs[2] = u32::from(input.dx as u16);
                regs[3] = u32::from(input.dy as u16);
                input.dx = 0;
                input.dy = 0;
            }
            _ => return 0,
        }
        1
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_hint_names_the_missing_display() {
        assert_eq!(window_hint(false, false, true), "");
        assert!(window_hint(true, false, false).contains("DISPLAY"));
        assert!(window_hint(true, true, false).contains("OpenGL"));
        assert!(window_hint(true, false, true).contains("OpenGL"));
    }

    #[test]
    fn the_picture_keeps_its_aspect_centred_in_any_window() {
        assert_eq!(picture((640, 480), (640, 480)), (0, 0, 640, 480));
        assert_eq!(picture((1920, 1080), (640, 480)), (240, 0, 1440, 1080));
        assert_eq!(picture((800, 1000), (640, 480)), (0, 200, 800, 600));
    }

    #[test]
    fn window_size_accepts_only_sane_dimensions() {
        assert_eq!(parse_size("1600x1200"), Some((1600, 1200)));
        assert_eq!(parse_size("1280X720"), Some((1280, 720)));
        for bad in ["", "1600", "0x0", "100x100", "axb", "9000x9000", "1600x"] {
            assert_eq!(parse_size(bad), None, "{bad}");
        }
    }

    #[test]
    fn keypad_function_and_extended_keys_have_their_dos_codes() {
        use KeyCode::*;
        for (key, code) in [
            (Numpad0, 0x52),
            (Numpad1, 0x4f),
            (Numpad5, 0x4c),
            (Numpad9, 0x49),
            (NumpadDecimal, 0x53),
            (NumpadAdd, 0x4e),
            (NumpadSubtract, 0x4a),
            (NumpadMultiply, 0x37),
            (NumpadDivide, 0x135),
            (NumpadEnter, 0x11c),
            (F11, 0x57),
            (F12, 0x58),
            (IntlBackslash, 0x56),
            (ArrowUp, 0x148),
            (ControlRight, 0x11d),
            (KeyA, 0x1e),
            (KeyZ, 0x2c),
            (Digit0, 0xb),
        ] {
            assert_eq!(dos_scan(key), code, "{key:?}");
        }
        assert_eq!(dos_scan(KeyCode::MediaPlayPause), 0);
    }

    #[test]
    fn losing_focus_releases_held_keys_and_ignores_new_ones() {
        let mut input = Input {
            focused: true,
            ..Input::default()
        };
        input.keyboard(0x148, true, false);
        assert_eq!(input.queue.drain(..).collect::<Vec<_>>(), [0xe0, 0x48]);
        input.focus(false);
        assert_eq!(input.queue.drain(..).collect::<Vec<_>>(), [0xe0, 0xc8]);
        input.keyboard(0x1e, true, false);
        assert!(input.queue.is_empty());
        input.mouse_button(1, true);
        assert_eq!(input.buttons, 0, "an unfocused window takes no clicks");
    }

    #[test]
    fn full_key_queue_requests_close_without_exiting_or_partial_extended_scan() {
        let mut input = Input::default();
        for _ in 0..128 {
            input.scan(0x148, false);
        }
        assert_eq!(input.queue.len(), 256);
        input.scan(0x11d, false);
        assert!(input.closed);
        assert_eq!(input.queue.len(), 256);
        assert!(!input.held[0x11d]);
    }

    #[test]
    fn dos_key_queue_keeps_extended_releases_distinct() {
        let mut input = Input::default();
        for key in [
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::ArrowUp,
            KeyCode::Numpad8,
            KeyCode::Space,
        ] {
            let code = dos_scan(key);
            input.scan(code, false);
            input.scan(code, true);
            let mut expected = vec![(code & 127) as u8, (code & 127) as u8 | 128];
            if code & 256 != 0 {
                expected = vec![0xe0, expected[0], 0xe0, expected[1]];
            }
            assert_eq!(input.queue.drain(..).collect::<Vec<_>>(), expected);
            assert!(!input.held[usize::from(code)]);
        }
    }
}

#[cfg(test)]
#[path = "input_controller_tests.rs"]
mod controller_tests;
