//! The launcher window: the original Redguard launcher, recreated, with its artwork read from the
//! player's `REDGUARD.EXE`. Without that file it shows plain buttons instead.
use crate::artwork::{Artwork, BUTTON_POSITIONS, Image};
use crate::{locate, picker, settings};
use egui::{Align2, Color32, Pos2, Rect, Sense, TextureHandle, TextureOptions, Vec2};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mixer::{Chunk, LoaderRWops};
use sdl2::mouse::MouseButton;
use settings::Settings;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const ISSUES: &str = "https://github.com/michidk/maiko/issues";
const BETHESDA: &str = "https://bethesda.net/";

/// The original's buttons, in order. Uninstall has nothing to do here and is not shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Play,
    Options,
    Support,
    Website,
    Uninstall,
    Quit,
}
const ACTIONS: [Action; 6] = [
    Action::Play,
    Action::Options,
    Action::Support,
    Action::Website,
    Action::Uninstall,
    Action::Quit,
];
const LABELS: [&str; 6] = [
    "Play Redguard",
    "Game Options",
    "Technical Support",
    "www.bethesda.net",
    "",
    "Cancel",
];

fn shown(action: Action) -> bool {
    action != Action::Uninstall
}

#[derive(Debug, Clone, PartialEq)]
enum Dialog {
    None,
    /// The game folder was not found; the text is the path typed so far.
    ChooseGame {
        typed: String,
        error: Option<String>,
    },
    Options {
        draft: Settings,
        size_error: bool,
    },
    Support,
    Error(String),
}

struct Textures {
    background: TextureHandle,
    /// Up, focus, down for each button.
    buttons: Vec<[TextureHandle; 3]>,
}

struct App {
    program: PathBuf,
    game: Option<PathBuf>,
    settings: Settings,
    art: Option<Artwork>,
    textures: Option<Textures>,
    sounds: HashMap<String, Chunk>,
    dialog: Dialog,
    /// The button under the mouse or chosen with the arrow keys.
    focus: Option<usize>,
    rattle: usize,
    quit: bool,
}

fn texture(context: &egui::Context, name: &str, image: &Image) -> TextureHandle {
    let pixels = egui::ColorImage::from_rgba_unmultiplied(
        [image.width as usize, image.height as usize],
        &image.rgba,
    );
    context.load_texture(name, pixels, TextureOptions::NEAREST)
}

/// Open a web page or folder with the system's default program.
fn open(target: &std::ffi::OsStr) -> std::io::Result<()> {
    let mut command = if cfg!(windows) {
        std::process::Command::new("explorer")
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else {
        std::process::Command::new("xdg-open")
    };
    command.arg(target).spawn().map(drop)
}

impl App {
    fn new(program: PathBuf) -> Self {
        let game = locate::find(
            &locate::preference_files(),
            std::env::var_os(locate::ENV_VAR).map(PathBuf::from),
            &program,
            &locate::Machine::current(),
        );
        let mut app = Self {
            program,
            game: None,
            settings: Settings::load(),
            art: None,
            textures: None,
            sounds: HashMap::new(),
            dialog: Dialog::None,
            // The original dialog opens with Play Redguard focused, so Enter starts the game.
            focus: Some(0),
            rattle: 0,
            quit: false,
        };
        match game {
            Some(game) => app.use_game(game),
            None => {
                app.dialog = Dialog::ChooseGame {
                    typed: String::new(),
                    error: None,
                }
            }
        }
        app
    }

    fn use_game(&mut self, game: PathBuf) {
        self.art = Artwork::load(&game);
        self.textures = None;
        self.sounds.clear();
        if let Some(art) = &self.art {
            for (name, wave) in &art.sounds {
                if let Ok(chunk) =
                    sdl2::rwops::RWops::from_bytes(wave).and_then(|data| data.load_wav())
                {
                    self.sounds.insert(name.clone(), chunk);
                }
            }
        }
        self.game = Some(game);
    }

    /// Accept a chosen folder (or one inside or around the game) and remember it.
    fn choose(&mut self, picked: &Path) -> Result<(), String> {
        let game = locate::resolve(picked).ok_or_else(|| {
            "The Redguard files (RGFX.EXE, FXART and MAPS) are not in or around that folder."
                .to_owned()
        })?;
        // A remembered folder only saves asking next time; failing to store it is not an error.
        let _ = locate::remember(&game);
        self.use_game(game);
        Ok(())
    }

    fn play_sound(&self, name: &str) {
        if let Some(chunk) = self.sounds.get(name) {
            let _ = sdl2::mixer::Channel::all().play(chunk, 0);
        }
    }

    fn set_focus(&mut self, focus: Option<usize>) {
        if focus.is_some() && focus != self.focus {
            // The original rattles the sword in turn as the focus moves.
            self.rattle = self.rattle % 3 + 1;
            self.play_sound(&format!("RATTLE{}", self.rattle));
        }
        self.focus = focus;
    }

    fn move_focus(&mut self, step: isize) {
        let visible: Vec<usize> = (0..ACTIONS.len()).filter(|&i| shown(ACTIONS[i])).collect();
        let at = self
            .focus
            .and_then(|focus| visible.iter().position(|&i| i == focus))
            .map_or(if step > 0 { visible.len() - 1 } else { 0 }, |at| at);
        let next = (at as isize + step).rem_euclid(visible.len() as isize) as usize;
        self.set_focus(Some(visible[next]));
    }

    fn activate(&mut self, action: Action) {
        match action {
            Action::Play => self.play(),
            Action::Options => {
                self.dialog = Dialog::Options {
                    draft: self.settings.clone(),
                    size_error: false,
                }
            }
            Action::Support => self.dialog = Dialog::Support,
            Action::Website => {
                let _ = open(BETHESDA.as_ref());
            }
            Action::Uninstall => {}
            Action::Quit => self.quit = true,
        }
    }

    fn user_directory(&self) -> Option<PathBuf> {
        settings::user_directory(&self.program, self.game.as_ref()?).ok()
    }

    fn play(&mut self) {
        let Some(game) = self.game.clone() else {
            return;
        };
        self.play_sound("SCRAPE2");
        let executable = self.program.join(settings::GAME);
        let result = self
            .user_directory()
            .ok_or_else(|| std::io::Error::other("no folder for saves"))
            .and_then(|user| {
                std::process::Command::new(&executable)
                    .args(self.settings.arguments(&game, &user))
                    .current_dir(&self.program)
                    .spawn()
            });
        match result {
            Ok(_) => self.quit = true,
            Err(error) => {
                self.dialog = Dialog::Error(format!(
                    "Redguard could not be started from {}:\n{error}",
                    executable.display()
                ))
            }
        }
    }

    fn ui(&mut self, context: &egui::Context) {
        if self.textures.is_none()
            && let Some(art) = &self.art
        {
            self.textures = Some(Textures {
                background: texture(context, "background", &art.background),
                buttons: art
                    .buttons
                    .iter()
                    .enumerate()
                    .map(|(i, b)| {
                        [
                            texture(context, &format!("b{i}u"), &b.up),
                            texture(context, &format!("b{i}f"), &b.focus),
                            texture(context, &format!("b{i}d"), &b.down),
                        ]
                    })
                    .collect(),
            });
        }
        let modal = self.dialog != Dialog::None;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(40, 28, 18)))
            .show(context, |ui| {
                let mut clicked = None;
                let mut hovered = None;
                if let Some(textures) = &self.textures {
                    let full = Rect::from_min_size(Pos2::ZERO, Vec2::new(WIDTH as f32, HEIGHT as f32));
                    ui.painter().image(
                        textures.background.id(),
                        full,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    for (i, states) in textures.buttons.iter().enumerate() {
                        if !shown(ACTIONS[i]) {
                            continue;
                        }
                        let (x, y) = BUTTON_POSITIONS[i];
                        let size = states[0].size_vec2();
                        let rect = Rect::from_min_size(Pos2::new(x as f32, y as f32), size);
                        let response = ui.interact(rect, ui.id().with(i), if modal { Sense::hover() } else { Sense::click() });
                        if !modal && response.hovered() {
                            hovered = Some(i);
                        }
                        if response.clicked() {
                            clicked = Some(i);
                        }
                        let state = if !modal && response.is_pointer_button_down_on() {
                            2
                        } else if !modal && self.focus == Some(i) {
                            1
                        } else {
                            0
                        };
                        ui.painter().image(
                            states[state].id(),
                            rect,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                } else if self.game.is_some() {
                    ui.add_space(120.0);
                    ui.vertical_centered(|ui| {
                        ui.heading(egui::RichText::new("Redguard").size(40.0).color(Color32::from_rgb(200, 40, 30)));
                        ui.label("REDGUARD.EXE was not found in your game folder, so the original launcher artwork is not shown.");
                        ui.add_space(20.0);
                        for (i, action) in ACTIONS.iter().enumerate() {
                            if !shown(*action) || *action == Action::Website {
                                continue;
                            }
                            let button = ui.add_enabled(!modal, egui::Button::new(LABELS[i]).min_size(Vec2::new(220.0, 30.0)));
                            if button.hovered() {
                                hovered = Some(i);
                            }
                            if button.clicked() {
                                clicked = Some(i);
                            }
                        }
                    });
                }
                if hovered.is_some() {
                    self.set_focus(hovered);
                }
                if let Some(i) = clicked {
                    self.activate(ACTIONS[i]);
                }
            });
        self.dialogs(context);
    }

    fn dialogs(&mut self, context: &egui::Context) {
        let window = |title: &str| {
            egui::Window::new(title.to_owned())
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        };
        let mut close = false;
        let mut choose = None;
        match &mut self.dialog {
            Dialog::None => {}
            Dialog::ChooseGame { typed, error } => {
                window("Find Redguard").show(context, |ui| {
                    ui.set_max_width(440.0);
                    ui.label("Maiko could not find your Redguard installation. Choose the Redguard folder: the one that contains RGFX.EXE, or the GOG folder around it.");
                    ui.add_space(8.0);
                    if ui.button("Browse…").clicked()
                        && let Some(folder) = picker::choose_folder() {
                            choose = Some(folder);
                        }
                    ui.horizontal(|ui| {
                        ui.label("Folder:");
                        let entered = ui.text_edit_singleline(typed).lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter));
                        if (ui.button("Use").clicked() || entered) && !typed.trim().is_empty() {
                            choose = Some(PathBuf::from(typed.trim()));
                        }
                    });
                    if let Some(error) = error {
                        ui.colored_label(Color32::from_rgb(220, 80, 60), error.as_str());
                    }
                    ui.add_space(8.0);
                    if ui.button("Quit").clicked() {
                        self.quit = true;
                    }
                });
            }
            Dialog::Options { draft, size_error } => {
                let mut save = false;
                window("Redguard Options").show(context, |ui| {
                    ui.set_max_width(420.0);
                    ui.group(|ui| {
                        ui.label("Game folder");
                        ui.monospace(self.game.as_deref().map_or("(none)".into(), |game| game.display().to_string()));
                        if ui.button("Change…").clicked()
                            && let Some(folder) = picker::choose_folder() {
                                choose = Some(folder);
                            }
                    });
                    ui.group(|ui| {
                        ui.label("Display");
                        ui.checkbox(&mut draft.fullscreen, "Fullscreen");
                        ui.add_enabled_ui(!draft.fullscreen, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Window size");
                                ui.add(egui::TextEdit::singleline(&mut draft.window_size).hint_text("1280x960").desired_width(100.0));
                            });
                        });
                        if *size_error {
                            ui.colored_label(Color32::from_rgb(220, 80, 60), "Enter the size as WIDTHxHEIGHT, at least 320x240, or leave it empty.");
                        }
                    });
                    ui.group(|ui| {
                        ui.label("Movies");
                        ui.checkbox(&mut draft.skip_intro, "Skip the intro when starting a new game");
                        ui.checkbox(&mut draft.skip_outro, "Skip the outro");
                    });
                    ui.group(|ui| {
                        ui.label("CD music");
                        ui.monospace(draft.cd_image.as_deref().map_or("From the game folder".into(), |image| image.display().to_string()));
                        ui.horizontal(|ui| {
                            if ui.button("Disc image folder…").clicked()
                                && let Some(folder) = picker::choose_folder() {
                                    draft.cd_image = Some(folder);
                                }
                            if draft.cd_image.is_some() && ui.button("Use the game folder").clicked() {
                                draft.cd_image = None;
                            }
                        });
                    });
                    ui.weak("Subtitles, sound and controls are set in the game's own Options menu.");
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            save = true;
                        }
                        if ui.button("Default").clicked() {
                            *draft = Settings::default();
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });
                if save {
                    if draft.window_size_valid() {
                        self.settings = draft.clone();
                        let _ = self.settings.save();
                        close = true;
                    } else {
                        *size_error = true;
                    }
                }
            }
            Dialog::Support => {
                let logs = std::env::var_os("MAIKO_LOG_DIR")
                    .map(PathBuf::from)
                    .map_or_else(settings::log_directory, Ok)
                    .ok();
                window("Technical Support").show(context, |ui| {
                    ui.set_max_width(420.0);
                    ui.label("Every start of the game writes a log. If something goes wrong, attach the newest one to an issue.");
                    if let Some(logs) = &logs {
                        ui.monospace(logs.display().to_string());
                    }
                    ui.horizontal(|ui| {
                        if let Some(logs) = &logs
                            && ui.add_enabled(logs.is_dir(), egui::Button::new("Open log folder")).clicked() {
                                let _ = open(logs.as_os_str());
                            }
                        if ui.button("Report an issue").clicked() {
                            let _ = open(ISSUES.as_ref());
                        }
                        if ui.button("OK").clicked() {
                            close = true;
                        }
                    });
                });
            }
            Dialog::Error(text) => {
                window("Redguard").show(context, |ui| {
                    ui.set_max_width(420.0);
                    ui.label(text.as_str());
                    if ui.button("OK").clicked() {
                        close = true;
                    }
                });
            }
        }
        if let Some(folder) = choose {
            match self.choose(&folder) {
                Ok(()) => {
                    if matches!(self.dialog, Dialog::ChooseGame { .. }) {
                        close = true;
                    }
                }
                Err(message) => match &mut self.dialog {
                    Dialog::ChooseGame { error, .. } => *error = Some(message),
                    _ => self.dialog = Dialog::Error(message),
                },
            }
        }
        if close {
            self.dialog = Dialog::None;
        }
    }

    /// Keyboard use as in the original dialog: arrows or Tab move between the buttons, Enter
    /// presses the focused one, Escape cancels.
    fn key(&mut self, key: Keycode, keymod: Mod) {
        if self.dialog != Dialog::None {
            if key == Keycode::Escape && !matches!(self.dialog, Dialog::ChooseGame { .. }) {
                self.dialog = Dialog::None;
            }
            return;
        }
        match key {
            Keycode::Down | Keycode::Right => self.move_focus(1),
            Keycode::Up | Keycode::Left => self.move_focus(-1),
            Keycode::Tab => {
                self.move_focus(if keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) {
                    -1
                } else {
                    1
                })
            }
            Keycode::Return | Keycode::KpEnter | Keycode::Space => {
                if let Some(focus) = self.focus {
                    self.activate(ACTIONS[focus]);
                }
            }
            Keycode::Escape => self.quit = true,
            _ => {}
        }
    }
}

fn egui_key(key: Keycode) -> Option<egui::Key> {
    Some(match key {
        Keycode::Backspace => egui::Key::Backspace,
        Keycode::Delete => egui::Key::Delete,
        Keycode::Return | Keycode::KpEnter => egui::Key::Enter,
        Keycode::Tab => egui::Key::Tab,
        Keycode::Escape => egui::Key::Escape,
        Keycode::Left => egui::Key::ArrowLeft,
        Keycode::Right => egui::Key::ArrowRight,
        Keycode::Up => egui::Key::ArrowUp,
        Keycode::Down => egui::Key::ArrowDown,
        Keycode::Home => egui::Key::Home,
        Keycode::End => egui::Key::End,
        Keycode::A => egui::Key::A,
        Keycode::C => egui::Key::C,
        Keycode::V => egui::Key::V,
        Keycode::X => egui::Key::X,
        _ => return None,
    })
}

/// The grey, square look of the original's Windows 98 dialogs.
fn classic_style(context: &egui::Context) {
    let grey = Color32::from_rgb(192, 192, 192);
    let mut visuals = egui::Visuals::light();
    visuals.window_fill = grey;
    visuals.panel_fill = grey;
    visuals.window_corner_radius = egui::CornerRadius::ZERO;
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.window_stroke = egui::Stroke::new(2.0, Color32::from_rgb(64, 64, 64));
    visuals.override_text_color = Some(Color32::BLACK);
    visuals.extreme_bg_color = Color32::WHITE;
    visuals.selection.bg_fill = Color32::from_rgb(0, 0, 128);
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::ZERO;
    }
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(212, 208, 200);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(64, 64, 64));
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(224, 222, 216);
    context.set_visuals(visuals);
}

fn modifiers(keymod: Mod) -> egui::Modifiers {
    let ctrl = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
    let command = keymod.intersects(Mod::LGUIMOD | Mod::RGUIMOD);
    egui::Modifiers {
        alt: keymod.intersects(Mod::LALTMOD | Mod::RALTMOD),
        ctrl,
        shift: keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
        mac_cmd: cfg!(target_os = "macos") && command,
        command: if cfg!(target_os = "macos") {
            command
        } else {
            ctrl
        },
    }
}

fn run() -> Result<(), String> {
    let program = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("the launcher has no folder")?
        .to_path_buf();
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    // Sounds are a nicety: without audio the launcher still works.
    let _audio = sdl.audio().ok().and_then(|audio| {
        sdl2::mixer::open_audio(22050, sdl2::mixer::AUDIO_S16LSB, 2, 1024).ok()?;
        sdl2::mixer::allocate_channels(4);
        Some(audio)
    });
    let window = video
        .window("Maiko", WIDTH, HEIGHT)
        .opengl()
        .allow_highdpi()
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
    let _gl_context = window.gl_create_context()?;
    let _ = video.gl_set_swap_interval(1);
    let gl = unsafe {
        glow::Context::from_loader_function(|name| video.gl_get_proc_address(name).cast())
    };
    let mut painter =
        egui_glow::Painter::new(Arc::new(gl), "", None, false).map_err(|e| e.to_string())?;
    let context = egui::Context::default();
    classic_style(&context);
    let mut app = App::new(program);
    let mut events = sdl.event_pump()?;
    let started = Instant::now();
    video.text_input().start();
    while !app.quit {
        let mut input = Vec::new();
        // Wait for input instead of spinning; egui asks for a repaint when it animates.
        let first = events.wait_event_timeout(100);
        for event in first.into_iter().chain(events.poll_iter()) {
            match event {
                Event::Quit { .. } => app.quit = true,
                Event::MouseMotion { x, y, .. } => {
                    input.push(egui::Event::PointerMoved(Pos2::new(x as f32, y as f32)))
                }
                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    x,
                    y,
                    ..
                }
                | Event::MouseButtonUp {
                    mouse_btn: MouseButton::Left,
                    x,
                    y,
                    ..
                } => input.push(egui::Event::PointerButton {
                    pos: Pos2::new(x as f32, y as f32),
                    button: egui::PointerButton::Primary,
                    pressed: matches!(event, Event::MouseButtonDown { .. }),
                    modifiers: egui::Modifiers::NONE,
                }),
                Event::Window {
                    win_event: WindowEvent::Leave,
                    ..
                } => input.push(egui::Event::PointerGone),
                Event::TextInput { text, .. } => input.push(egui::Event::Text(text)),
                Event::KeyDown {
                    keycode: Some(key),
                    keymod,
                    repeat,
                    ..
                } => {
                    app.key(key, keymod);
                    if let Some(key) = egui_key(key) {
                        input.push(egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: true,
                            repeat,
                            modifiers: modifiers(keymod),
                        });
                    }
                }
                Event::KeyUp {
                    keycode: Some(key),
                    keymod,
                    ..
                } => {
                    if let Some(key) = egui_key(key) {
                        input.push(egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: false,
                            repeat: false,
                            modifiers: modifiers(keymod),
                        });
                    }
                }
                _ => {}
            }
        }
        let (points, pixels) = (window.size(), window.drawable_size());
        context.set_pixels_per_point(pixels.0 as f32 / points.0.max(1) as f32);
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(points.0 as f32, points.1 as f32),
            )),
            time: Some(started.elapsed().as_secs_f64()),
            events: input,
            ..Default::default()
        };
        let output = context.run(raw, |context| app.ui(context));
        let primitives = context.tessellate(output.shapes, output.pixels_per_point);
        painter.clear([pixels.0, pixels.1], [0.0, 0.0, 0.0, 1.0]);
        painter.paint_and_update_textures(
            [pixels.0, pixels.1],
            output.pixels_per_point,
            &primitives,
            &output.textures_delta,
        );
        window.gl_swap_window();
    }
    painter.destroy();
    Ok(())
}

/// The launcher's entry point; the packaged executables call it from a small C `main` (see
/// `tools/build_link_inputs.py`) and `cargo run` from `main.rs`.
#[unsafe(no_mangle)]
pub extern "C" fn maiko_launcher_main() -> std::ffi::c_int {
    match run() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("Maiko launcher: {error}");
            let _ = sdl2::messagebox::show_simple_message_box(
                sdl2::messagebox::MessageBoxFlag::ERROR,
                "Maiko",
                &format!("The launcher could not start: {error}"),
                None,
            );
            1
        }
    }
}
