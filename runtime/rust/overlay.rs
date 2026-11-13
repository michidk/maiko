//! In-game overlay drawn over the finished Glide frame, toggled with the backtick key.
//! While it is open the game receives no keyboard, mouse or controller input.
use crate::rg_log;
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::mouse::MouseButton;
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::Arc;
use std::time::Instant;

pub use egui;

/// Draws the overlay's content. The host supplies it because only the host
/// links the translated game state.
pub type Panel = fn(&mut egui::Ui);

struct Overlay {
    open: bool,
    context: egui::Context,
    painter: Option<(egui_glow::Painter, AttribStack)>,
    failed: bool,
    events: Vec<egui::Event>,
    panel: Option<Panel>,
    started: Instant,
}

thread_local! {
    static OVERLAY: RefCell<Overlay> = RefCell::new(Overlay {
        open: false,
        context: egui::Context::default(),
        painter: None,
        failed: false,
        events: Vec::new(),
        panel: None,
        started: Instant::now(),
    });
}

/// glPushAttrib/glPopAttrib and the client variants are legacy entry points glow does not load.
struct AttribStack {
    push: extern "system" fn(u32),
    pop: extern "system" fn(),
    push_client: extern "system" fn(u32),
    pop_client: extern "system" fn(),
}

pub fn set_panel(panel: Panel) {
    OVERLAY.with(|overlay| overlay.borrow_mut().panel = Some(panel));
}

pub fn is_open() -> bool {
    OVERLAY.with(|overlay| overlay.borrow().open)
}

/// What the input pump should do with an SDL event.
#[derive(Debug, PartialEq)]
pub(crate) enum Routed {
    Game,
    Consumed,
    Toggled(bool),
}

/// Takes the toggle key and, while open, every keyboard and mouse event.
/// Mouse positions are window pixels, which are also the overlay's points.
pub(crate) fn route(event: &Event) -> Routed {
    OVERLAY.with(|overlay| {
        let mut overlay = overlay.borrow_mut();
        match event {
            Event::KeyDown {
                scancode: Some(Scancode::Grave),
                repeat: false,
                ..
            } => {
                overlay.open = !overlay.open;
                overlay.events.clear();
                return Routed::Toggled(overlay.open);
            }
            Event::KeyDown {
                scancode: Some(Scancode::Grave),
                ..
            }
            | Event::KeyUp {
                scancode: Some(Scancode::Grave),
                ..
            } => return Routed::Consumed,
            _ => {}
        }
        if !overlay.open {
            return Routed::Game;
        }
        let pointer = |button: &MouseButton| match button {
            MouseButton::Left => Some(egui::PointerButton::Primary),
            MouseButton::Right => Some(egui::PointerButton::Secondary),
            MouseButton::Middle => Some(egui::PointerButton::Middle),
            _ => None,
        };
        let translated = match *event {
            Event::MouseMotion { x, y, .. } => {
                Some(egui::Event::PointerMoved(egui::pos2(x as f32, y as f32)))
            }
            Event::MouseButtonDown {
                mouse_btn, x, y, ..
            }
            | Event::MouseButtonUp {
                mouse_btn, x, y, ..
            } => pointer(&mouse_btn).map(|button| egui::Event::PointerButton {
                pos: egui::pos2(x as f32, y as f32),
                button,
                pressed: matches!(event, Event::MouseButtonDown { .. }),
                modifiers: egui::Modifiers::NONE,
            }),
            Event::MouseWheel {
                precise_x,
                precise_y,
                ..
            } => Some(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(precise_x, precise_y),
                modifiers: egui::Modifiers::NONE,
            }),
            Event::KeyDown { .. } | Event::KeyUp { .. } | Event::TextInput { .. } => None,
            _ => return Routed::Game,
        };
        overlay.events.extend(translated);
        Routed::Consumed
    })
}

/// Paints the open overlay into the current OpenGL context. The caller must
/// have flushed the game's pending triangles; the game's GL state is restored.
pub(crate) fn draw(video: &sdl2::VideoSubsystem, points: (u32, u32), pixels: (u32, u32)) {
    OVERLAY.with(|overlay| {
        let mut overlay = overlay.borrow_mut();
        let overlay = &mut *overlay;
        if !overlay.open || overlay.failed {
            return;
        }
        if overlay.painter.is_none() {
            match unsafe { painter(video) } {
                Ok(painter) => overlay.painter = Some(painter),
                Err(error) => {
                    rg_log!("Native overlay unavailable: {error}");
                    overlay.failed = true;
                    return;
                }
            }
        }
        if points.0 == 0 || points.1 == 0 {
            return;
        }
        overlay
            .context
            .set_pixels_per_point(pixels.0 as f32 / points.0 as f32);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(points.0 as f32, points.1 as f32),
            )),
            time: Some(overlay.started.elapsed().as_secs_f64()),
            events: std::mem::take(&mut overlay.events),
            ..Default::default()
        };
        let panel = overlay.panel;
        let output = overlay.context.run(input, |context| {
            egui::Window::new("Maiko")
                .default_pos([16.0, 16.0])
                .resizable(false)
                .show(context, |ui| {
                    if let Some(panel) = panel {
                        panel(ui);
                    }
                    ui.weak("` closes this overlay");
                });
        });
        let primitives = overlay
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        let (painter, attrib) = overlay.painter.as_mut().unwrap();
        // GL_ALL_ATTRIB_BITS and GL_CLIENT_ALL_ATTRIB_BITS.
        (attrib.push)(0x000f_ffff);
        (attrib.push_client)(0xffff_ffff);
        painter.paint_and_update_textures(
            [pixels.0, pixels.1],
            output.pixels_per_point,
            &primitives,
            &output.textures_delta,
        );
        // OpenGLide draws with fixed-function client arrays and no program or buffers.
        unsafe {
            use glow::HasContext;
            let gl = painter.gl();
            gl.use_program(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        }
        (attrib.pop_client)();
        (attrib.pop)();
    });
}

unsafe fn painter(
    video: &sdl2::VideoSubsystem,
) -> Result<(egui_glow::Painter, AttribStack), String> {
    let load = |name: &str| video.gl_get_proc_address(name).cast::<std::ffi::c_void>();
    let legacy = |name: &str| {
        let function = load(name);
        (!function.is_null())
            .then_some(function)
            .ok_or_else(|| format!("{name} is not available"))
    };
    // SAFETY: these are the OpenGL 1.1 entry points of the current context.
    let attrib = unsafe {
        AttribStack {
            push: std::mem::transmute::<*const c_void, extern "system" fn(u32)>(legacy(
                "glPushAttrib",
            )?),
            pop: std::mem::transmute::<*const c_void, extern "system" fn()>(legacy("glPopAttrib")?),
            push_client: std::mem::transmute::<*const c_void, extern "system" fn(u32)>(legacy(
                "glPushClientAttrib",
            )?),
            pop_client: std::mem::transmute::<*const c_void, extern "system" fn()>(legacy(
                "glPopClientAttrib",
            )?),
        }
    };
    let gl = unsafe { glow::Context::from_loader_function(load) };
    let painter =
        egui_glow::Painter::new(Arc::new(gl), "", None, false).map_err(|e| e.to_string())?;
    Ok((painter, attrib))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(down: bool, scancode: Scancode) -> Event {
        let (keycode, keymod) = (None, sdl2::keyboard::Mod::NOMOD);
        if down {
            Event::KeyDown {
                timestamp: 0,
                window_id: 0,
                keycode,
                scancode: Some(scancode),
                keymod,
                repeat: false,
            }
        } else {
            Event::KeyUp {
                timestamp: 0,
                window_id: 0,
                keycode,
                scancode: Some(scancode),
                keymod,
                repeat: false,
            }
        }
    }

    #[test]
    fn backtick_toggles_and_open_overlay_takes_input() {
        let motion = Event::MouseMotion {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mousestate: sdl2::mouse::MouseState::from_sdl_state(0),
            x: 5,
            y: 7,
            xrel: 0,
            yrel: 0,
        };
        assert_eq!(route(&key(true, Scancode::W)), Routed::Game);
        assert_eq!(route(&motion), Routed::Game);
        assert_eq!(route(&key(true, Scancode::Grave)), Routed::Toggled(true));
        assert_eq!(route(&key(false, Scancode::Grave)), Routed::Consumed);
        assert!(is_open());
        assert_eq!(route(&key(true, Scancode::W)), Routed::Consumed);
        assert_eq!(route(&motion), Routed::Consumed);
        assert_eq!(OVERLAY.with(|o| o.borrow().events.len()), 1);
        assert_eq!(route(&key(true, Scancode::Grave)), Routed::Toggled(false));
        assert!(OVERLAY.with(|o| o.borrow().events.is_empty()));
        assert_eq!(route(&key(true, Scancode::W)), Routed::Game);
    }
}
