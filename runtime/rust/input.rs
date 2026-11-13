//! SDL window and DOS keyboard/mouse state shared by the Windows/Linux hosts.
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Scancode;
use sdl2::mouse::MouseButton;
use sdl2::video::{GLProfile, Window};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::{c_int, c_void};
use std::mem::ManuallyDrop;

struct Input {
    sdl: Option<sdl2::Sdl>,
    events: Option<sdl2::EventPump>,
    window: Option<Window>,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
    buttons: u32,
    known: bool,
    focused: bool,
    closed: bool,
    queue: VecDeque<u8>,
    held: [bool; 512],
}

impl Default for Input {
    fn default() -> Self {
        Self {
            sdl: None,
            events: None,
            window: None,
            width: 640,
            height: 480,
            x: 0,
            y: 0,
            dx: 0,
            dy: 0,
            buttons: 0,
            known: false,
            focused: false,
            closed: false,
            queue: VecDeque::with_capacity(256),
            held: [false; 512],
        }
    }
}

// SDL and OpenGLide share process-lifetime window state. Do not run SDL
// destructors before CRT audio callbacks or under the Windows loader lock.
thread_local! { static INPUT: RefCell<ManuallyDrop<Input>> = RefCell::new(ManuallyDrop::new(Input::default())); }

fn dos_scan(key: Scancode) -> u16 {
    use Scancode::*;
    const LETTERS: [u16; 26] = [
        0x1e, 0x30, 0x2e, 0x20, 0x12, 0x21, 0x22, 0x23, 0x17, 0x24, 0x25, 0x26, 0x32, 0x31, 0x18,
        0x19, 0x10, 0x13, 0x1f, 0x14, 0x16, 0x2f, 0x11, 0x2d, 0x15, 0x2c,
    ];
    if (A as i32..=Z as i32).contains(&(key as i32)) {
        return LETTERS[(key as i32 - A as i32) as usize];
    }
    if (Num1 as i32..=Num9 as i32).contains(&(key as i32)) {
        return (2 + key as i32 - Num1 as i32) as u16;
    }
    if (F1 as i32..=F10 as i32).contains(&(key as i32)) {
        return (0x3b + key as i32 - F1 as i32) as u16;
    }
    match key {
        Num0 => 0xb,
        Return => 0x1c,
        Escape => 1,
        Space => 0x39,
        Tab => 0xf,
        Backspace => 0xe,
        Minus => 0xc,
        Equals => 0xd,
        LeftBracket => 0x1a,
        RightBracket => 0x1b,
        Semicolon => 0x27,
        Apostrophe => 0x28,
        Grave => 0x29,
        Backslash => 0x2b,
        Comma => 0x33,
        Period => 0x34,
        Slash => 0x35,
        LShift => 0x2a,
        RShift => 0x36,
        LCtrl => 0x1d,
        RCtrl => 0x11d,
        LAlt => 0x38,
        RAlt => 0x138,
        CapsLock => 0x3a,
        NumLockClear => 0x45,
        ScrollLock => 0x46,
        Up => 0x148,
        Down => 0x150,
        Left => 0x14b,
        Right => 0x14d,
        Home => 0x147,
        End => 0x14f,
        PageUp => 0x149,
        PageDown => 0x151,
        Insert => 0x152,
        Delete => 0x153,
        Kp0 => 0x52,
        Kp1 => 0x4f,
        Kp2 => 0x50,
        Kp3 => 0x51,
        Kp4 => 0x4b,
        Kp5 => 0x4c,
        Kp6 => 0x4d,
        Kp7 => 0x47,
        Kp8 => 0x48,
        Kp9 => 0x49,
        KpPeriod => 0x53,
        KpPlus => 0x4e,
        KpMinus => 0x4a,
        KpMultiply => 0x37,
        KpDivide => 0x135,
        KpEnter => 0x11c,
        F11 => 0x57,
        F12 => 0x58,
        NonUsBackslash => 0x56,
        _ => 0,
    }
}

/// Display choice from the command line: `--fullscreen` or `--window-size WIDTHxHEIGHT`.
/// The game keeps rendering at its own resolution; SDL scales it to the window.
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
            eprintln!("BLOCKED native keyboard queue overflow");
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
            sdl2::hint::set("SDL_NO_SIGNAL_HANDLERS", "1");
            let sdl = sdl2::init()?;
            let video = sdl.video()?;
            let gl = video.gl_attr();
            gl.set_context_profile(GLProfile::Compatibility);
            gl.set_context_version(2, 1);
            gl.set_double_buffer(true);
            gl.set_depth_size(24);
            let (fullscreen, size) = *DISPLAY.lock().unwrap_or_else(|e| e.into_inner());
            let (w, h) = size.unwrap_or((width, height));
            let mut builder = video.window("Redguard native experiment", w, h);
            builder.opengl();
            if fullscreen {
                builder.fullscreen_desktop();
            }
            let window = builder.build().map_err(|e| e.to_string())?;
            let events = sdl.event_pump()?;
            self.window = Some(window);
            self.events = Some(events);
            self.sdl = Some(sdl);
        } else if *DISPLAY.lock().unwrap_or_else(|e| e.into_inner()) == (false, None) {
            self.window
                .as_mut()
                .unwrap()
                .set_size(width, height)
                .map_err(|e| e.to_string())?;
        }
        self.focused = true;
        let window = self.window.as_ref().unwrap();
        #[cfg(unix)]
        return Ok(window.raw().cast());
        #[cfg(windows)]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            match window.window_handle().map_err(|e| e.to_string())?.as_raw() {
                RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as *mut c_void),
                _ => Err("SDL did not provide a Windows window handle".into()),
            }
        }
    }

    fn pump(&mut self) -> bool {
        while let Some(event) = self.events.as_mut().and_then(|events| events.poll_event()) {
            match event {
                Event::Quit { .. }
                | Event::Window {
                    win_event: WindowEvent::Close,
                    ..
                } => self.closed = true,
                Event::Window {
                    win_event: WindowEvent::FocusGained,
                    ..
                } => {
                    self.focused = true;
                    self.known = false;
                }
                Event::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } => {
                    for code in 1..512 {
                        if self.held[code] {
                            self.scan(code as u16, true);
                        }
                    }
                    self.focused = false;
                    self.buttons = 0;
                    self.dx = 0;
                    self.dy = 0;
                    self.known = false;
                    if let Some(sdl) = &self.sdl {
                        sdl.mouse().capture(false);
                    }
                }
                Event::KeyDown {
                    scancode: Some(key),
                    ..
                }
                | Event::KeyUp {
                    scancode: Some(key),
                    ..
                } => {
                    let code = dos_scan(key);
                    if self.focused && code != 0 {
                        self.scan(code, matches!(event, Event::KeyUp { .. }));
                    }
                }
                Event::MouseMotion { x, y, .. } => {
                    if let Some(window) = &self.window {
                        let (width, height) = window.size();
                        if self.focused && width != 0 && height != 0 {
                            let nx =
                                (i64::from(x) * i64::from(self.width) / i64::from(width)) as i32;
                            let ny =
                                (i64::from(y) * i64::from(self.height) / i64::from(height)) as i32;
                            if self.known {
                                self.dx = self.dx.wrapping_add(nx.wrapping_sub(self.x));
                                self.dy = self.dy.wrapping_add(ny.wrapping_sub(self.y));
                            }
                            self.x = nx;
                            self.y = ny;
                            self.known = true;
                        }
                    }
                }
                Event::MouseButtonDown { mouse_btn, .. }
                | Event::MouseButtonUp { mouse_btn, .. } => {
                    let mask = match mouse_btn {
                        MouseButton::Left => 1,
                        MouseButton::Right => 2,
                        _ => 0,
                    };
                    if matches!(event, Event::MouseButtonUp { .. }) {
                        self.buttons &= !mask;
                    } else if self.focused {
                        self.buttons |= mask;
                    }
                    if let Some(sdl) = &self.sdl {
                        sdl.mouse().capture(self.buttons != 0);
                    }
                }
                _ => {}
            }
        }
        !self.closed
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_input_window(width: u32, height: u32) -> *mut c_void {
    INPUT.with(|input| {
        input
            .borrow_mut()
            .window(width, height)
            .unwrap_or_else(|error| {
                eprintln!("Native SDL window failed: {error}");
                std::ptr::null_mut()
            })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_input_pump() -> c_int {
    INPUT.with(|input| input.borrow_mut().pump()) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_input_take_scan() -> c_int {
    INPUT.with(|input| input.borrow_mut().queue.pop_front().map_or(-1, c_int::from))
}

/// # Safety
/// `registers` points to seven writable u32 values. All input/window calls
/// belong to the game thread that initialized SDL.
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
                if input.window(640, 480).is_err() {
                    return 0;
                }
                input.buttons = 0;
                input.dx = 0;
                input.dy = 0;
                input.known = false;
                regs[0] = 65535;
                regs[1] = 2;
            }
            1 | 2 => {
                if let Some(sdl) = &input.sdl {
                    sdl.mouse().show_cursor(regs[0] & 65535 == 1);
                }
            }
            3 => {
                regs[1] = input.buttons;
                regs[2] = u32::from(input.x.wrapping_mul(2) as u16);
                regs[3] = u32::from(input.y as u16);
            }
            4 => {
                if input.window.is_none() {
                    return 0;
                }
                input.x = i32::from(regs[2] as u16) / 2;
                input.y = i32::from(regs[3] as u16);
                let (width, height) = input.window.as_ref().unwrap().size();
                if input.focused
                    && let Some(sdl) = &input.sdl
                {
                    sdl.mouse().warp_mouse_in_window(
                        input.window.as_ref().unwrap(),
                        (i64::from(input.x) * i64::from(width) / i64::from(input.width)) as i32,
                        (i64::from(input.y) * i64::from(height) / i64::from(input.height)) as i32,
                    );
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
    fn window_size_accepts_only_sane_dimensions() {
        assert_eq!(parse_size("1600x1200"), Some((1600, 1200)));
        assert_eq!(parse_size("1280X720"), Some((1280, 720)));
        for bad in ["", "1600", "0x0", "100x100", "axb", "9000x9000", "1600x"] {
            assert_eq!(parse_size(bad), None, "{bad}");
        }
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
            Scancode::LCtrl,
            Scancode::RCtrl,
            Scancode::Up,
            Scancode::Kp8,
            Scancode::Space,
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
