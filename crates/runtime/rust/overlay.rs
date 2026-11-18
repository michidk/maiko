//! In-game overlay drawn over the finished Glide frame, toggled with the backtick key.
//! While it is open the game receives no keyboard, mouse or controller input.
use std::cell::RefCell;
use std::time::Instant;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

pub use egui;

/// Draws the overlay's content. The host supplies it because only the host
/// links the translated game state.
pub type Panel = fn(&mut egui::Ui);

struct Overlay {
    open: bool,
    context: egui::Context,
    painter: Option<egui_wgpu::Renderer>,
    events: Vec<egui::Event>,
    /// The last pointer position; winit's button events carry none.
    pointer: egui::Pos2,
    panel: Option<Panel>,
    started: Instant,
}

thread_local! {
    static OVERLAY: RefCell<Overlay> = RefCell::new(Overlay {
        open: false,
        context: egui::Context::default(),
        painter: None,
        events: Vec::new(),
        pointer: egui::Pos2::ZERO,
        panel: None,
        started: Instant::now(),
    });
}

pub fn set_panel(panel: Panel) {
    OVERLAY.with(|overlay| overlay.borrow_mut().panel = Some(panel));
}

pub fn is_open() -> bool {
    OVERLAY.with(|overlay| overlay.borrow().open)
}

/// What the input pump should do with a window event.
#[derive(Debug, PartialEq)]
pub(crate) enum Routed {
    Game,
    Consumed,
    Toggled(bool),
}

/// Takes the toggle key and, while open, every keyboard and mouse event. Positions arrive in
/// window pixels and become the overlay's points through `scale`.
pub(crate) fn route(event: &WindowEvent, scale: f64) -> Routed {
    if let WindowEvent::KeyboardInput { event: key, .. } = event {
        return match key.physical_key {
            PhysicalKey::Code(code) => {
                route_key(code, key.state == ElementState::Pressed, key.repeat)
            }
            _ if is_open() => Routed::Consumed,
            _ => Routed::Game,
        };
    }
    OVERLAY.with(|overlay| {
        let mut overlay = overlay.borrow_mut();
        if !overlay.open {
            return Routed::Game;
        }
        let at = |x: f64, y: f64| egui::pos2((x / scale) as f32, (y / scale) as f32);
        let translated = match event {
            WindowEvent::CursorMoved { position, .. } => {
                overlay.pointer = at(position.x, position.y);
                Some(egui::Event::PointerMoved(overlay.pointer))
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let button = match button {
                    MouseButton::Left => Some(egui::PointerButton::Primary),
                    MouseButton::Right => Some(egui::PointerButton::Secondary),
                    MouseButton::Middle => Some(egui::PointerButton::Middle),
                    _ => None,
                };
                button.map(|button| egui::Event::PointerButton {
                    pos: overlay.pointer,
                    button,
                    pressed: *state == ElementState::Pressed,
                    modifiers: egui::Modifiers::NONE,
                })
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (unit, delta) = match *delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        (egui::MouseWheelUnit::Line, egui::vec2(x, y))
                    }
                    MouseScrollDelta::PixelDelta(p) => (
                        egui::MouseWheelUnit::Point,
                        egui::vec2((p.x / scale) as f32, (p.y / scale) as f32),
                    ),
                };
                Some(egui::Event::MouseWheel {
                    unit,
                    delta,
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                })
            }
            WindowEvent::Ime(_) => None,
            _ => return Routed::Game,
        };
        overlay.events.extend(translated);
        Routed::Consumed
    })
}

/// The backtick toggles the overlay; while it is open it keeps every key from the game.
fn route_key(code: KeyCode, pressed: bool, repeat: bool) -> Routed {
    OVERLAY.with(|overlay| {
        let mut overlay = overlay.borrow_mut();
        if code == KeyCode::Backquote {
            if pressed && !repeat {
                overlay.open = !overlay.open;
                overlay.events.clear();
                return Routed::Toggled(overlay.open);
            }
            return Routed::Consumed;
        }
        if overlay.open {
            Routed::Consumed
        } else {
            Routed::Game
        }
    })
}

/// The frame being presented, which the overlay draws on top of.
pub struct Target<'a> {
    pub device: &'a egui_wgpu::wgpu::Device,
    pub queue: &'a egui_wgpu::wgpu::Queue,
    pub encoder: &'a mut egui_wgpu::wgpu::CommandEncoder,
    pub view: &'a egui_wgpu::wgpu::TextureView,
    pub format: egui_wgpu::wgpu::TextureFormat,
}

/// Paints the open overlay over the presented frame, at window size.
pub(crate) fn draw(target: Target<'_>, points: (u32, u32), pixels: (u32, u32)) {
    OVERLAY.with(|overlay| {
        let mut overlay = overlay.borrow_mut();
        let overlay = &mut *overlay;
        if !overlay.open || points.0 == 0 || points.1 == 0 {
            return;
        }
        let renderer = overlay.painter.get_or_insert_with(|| {
            egui_wgpu::Renderer::new(
                target.device,
                target.format,
                egui_wgpu::RendererOptions::default(),
            )
        });
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
        let output = overlay.context.run_ui(input, |ui| {
            egui::Window::new("Maiko")
                .default_pos([16.0, 16.0])
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    if let Some(panel) = panel {
                        panel(ui);
                    }
                    ui.weak("` closes this overlay");
                });
        });
        let primitives = overlay
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [pixels.0, pixels.1],
            pixels_per_point: output.pixels_per_point,
        };
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(target.device, target.queue, *id, delta);
            }
        }
        let extra = renderer.update_buffers(
            target.device,
            target.queue,
            target.encoder,
            &primitives,
            &screen,
        );
        if !extra.is_empty() {
            target.queue.submit(extra);
        }
        {
            use egui_wgpu::wgpu;
            let mut pass = target
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("overlay"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            renderer.render(&mut pass, &primitives, &screen);
        }
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backtick_toggles_and_open_overlay_takes_input() {
        let device_id = winit::event::DeviceId::dummy();
        let motion = WindowEvent::CursorMoved {
            device_id,
            position: winit::dpi::PhysicalPosition::new(10.0, 14.0),
        };
        assert_eq!(route_key(KeyCode::KeyW, true, false), Routed::Game);
        assert_eq!(route(&motion, 2.0), Routed::Game);
        assert_eq!(
            route_key(KeyCode::Backquote, true, false),
            Routed::Toggled(true)
        );
        assert_eq!(
            route_key(KeyCode::Backquote, false, false),
            Routed::Consumed
        );
        assert!(is_open());
        assert_eq!(route_key(KeyCode::KeyW, true, false), Routed::Consumed);
        assert_eq!(route(&motion, 2.0), Routed::Consumed);
        OVERLAY.with(|o| {
            let o = o.borrow();
            assert_eq!(o.events.len(), 1);
            assert_eq!(o.pointer, egui::pos2(5.0, 7.0), "pixels become points");
        });
        assert_eq!(
            route_key(KeyCode::Backquote, true, false),
            Routed::Toggled(false)
        );
        assert!(OVERLAY.with(|o| o.borrow().events.is_empty()));
        assert_eq!(route_key(KeyCode::KeyW, true, false), Routed::Game);
    }
}
