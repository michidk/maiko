//! In-game overlay drawn over the finished Glide frame, toggled with the backtick key.
//! While it is open the game receives no keyboard, mouse or controller input.
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::mouse::MouseButton;
use std::cell::RefCell;
use std::time::Instant;

pub use egui;

/// Draws the overlay's content. The host supplies it because only the host
/// links the translated game state.
pub type Panel = fn(&mut egui::Ui);

struct Overlay {
    open: bool,
    context: egui::Context,
    painter: Option<egui_wgpu::Renderer>,
    events: Vec<egui::Event>,
    panel: Option<Panel>,
    started: Instant,
}

thread_local! {
    static OVERLAY: RefCell<Overlay> = RefCell::new(Overlay {
        open: false,
        context: egui::Context::default(),
        painter: None,
        events: Vec::new(),
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
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            }),
            Event::KeyDown { .. } | Event::KeyUp { .. } | Event::TextInput { .. } => None,
            _ => return Routed::Game,
        };
        overlay.events.extend(translated);
        Routed::Consumed
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
