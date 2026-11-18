//! The launcher window: the original Redguard launcher, recreated, with its artwork read from the
//! player's `REDGUARD.EXE`. Without that file it shows plain buttons instead.
use crate::artwork::{Artwork, BUTTON_POSITIONS, Image};
use crate::font::Font;
use crate::{locate, picker, report, settings};
use egui::{Align2, Color32, Pos2, Rect, Sense, TextureHandle, TextureOptions, Vec2};
use settings::Settings;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const PROJECT: &str = "https://github.com/michidk/maiko";
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
    Options(Settings),
    Support,
    /// A report was written (or failed); `true` when the game had stopped unexpectedly.
    Report(bool, Result<PathBuf, String>),
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
    /// The original's WAV sounds and the device they play on; without one the launcher is silent.
    sounds: HashMap<String, Arc<[u8]>>,
    audio: Option<rodio::OutputStream>,
    /// The game's gold menu font for dialog titles, and the titles drawn with it.
    title_font: Option<Font>,
    titles: HashMap<&'static str, TextureHandle>,
    dialog: Dialog,
    /// The button under the mouse or chosen with the arrow keys.
    focus: Option<usize>,
    rattle: usize,
    quit: bool,
    /// The running game; the launcher stays hidden until it ends.
    game_process: Option<std::process::Child>,
}

fn texture(context: &egui::Context, name: &str, image: &Image) -> TextureHandle {
    let pixels = egui::ColorImage::from_rgba_unmultiplied(
        [image.width as usize, image.height as usize],
        &image.rgba,
    );
    context.load_texture(name, pixels, TextureOptions::NEAREST)
}

/// Where the game writes its logs (`MAIKO_LOG_DIR` overrides it for the game too).
fn log_folder() -> Option<PathBuf> {
    std::env::var_os("MAIKO_LOG_DIR")
        .map(PathBuf::from)
        .map_or_else(settings::log_directory, Ok)
        .ok()
}

impl App {
    fn new(program: PathBuf) -> Self {
        let game = locate::find(
            locate::preference_file().as_deref(),
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
            audio: rodio::OutputStreamBuilder::open_default_stream()
                .map(|mut stream| {
                    stream.log_on_drop(false);
                    stream
                })
                .ok(),
            title_font: None,
            titles: HashMap::new(),
            dialog: Dialog::None,
            // The original dialog opens with Play Redguard focused, so Enter starts the game.
            focus: Some(0),
            rattle: 0,
            quit: false,
            game_process: None,
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
        self.title_font = Font::load(&game, "FONTSEL.FNT");
        self.titles.clear();
        self.textures = None;
        self.sounds.clear();
        if let Some(art) = &self.art {
            for (name, wave) in &art.sounds {
                self.sounds.insert(name.clone(), Arc::from(wave.as_slice()));
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
        if let (Some(wave), Some(audio)) = (self.sounds.get(name), &self.audio)
            && let Ok(sound) = rodio::Decoder::new(std::io::Cursor::new(wave.clone()))
        {
            audio.mixer().add(sound);
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
            Action::Options => self.dialog = Dialog::Options(self.settings.clone()),
            Action::Support => self.dialog = Dialog::Support,
            Action::Website => {
                let _ = open::that_detached(BETHESDA);
            }
            Action::Uninstall => {}
            Action::Quit => self.quit = true,
        }
    }

    fn play(&mut self) {
        let Some(game) = self.game.clone() else {
            return;
        };
        self.play_sound("SCRAPE2");
        let executable = self.program.join(settings::GAME);
        let mut command = std::process::Command::new(&executable);
        command
            .args(self.settings.arguments(&game))
            .current_dir(&self.program);
        if let Some(backend) = self.settings.renderer_backend() {
            command.env("WGPU_BACKEND", backend);
        }
        // The game is a console program; without the setting it runs without a window for its
        // log (the log file is written either way).
        #[cfg(windows)]
        if !self.settings.console {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        match command.spawn() {
            Ok(child) => self.game_process = Some(child),
            Err(error) => {
                self.dialog = Dialog::Error(format!(
                    "Redguard could not be started from {}:\n{error}",
                    executable.display()
                ))
            }
        }
    }

    /// Once the game ends: close the launcher after a normal quit, or write a crash report
    /// into the log folder's `crash-reports` folder and show it after a crash, a freeze the player ended, or a failed start.
    fn watch_game(&mut self) {
        let Some(child) = &mut self.game_process else {
            return;
        };
        let status = match child.try_wait() {
            Ok(None) => return,
            Ok(Some(status)) => status,
            Err(error) => {
                self.game_process = None;
                self.dialog = Dialog::Error(format!("Lost track of the game: {error}"));
                return;
            }
        };
        let pid = child.id();
        self.game_process = None;
        if status.success() {
            self.quit = true;
            return;
        }
        let exit = report::Exit {
            pid,
            status: status.to_string(),
        };
        self.dialog = Dialog::Report(true, self.report(Some(&exit)));
    }

    fn report(&self, exit: Option<&report::Exit>) -> Result<PathBuf, String> {
        let sources = report::Sources {
            logs: log_folder(),
            game: self.game.as_deref(),
            // The game keeps its INI files and saves in its own folder.
            user: self.game.clone(),
            settings: &self.settings,
        };
        report::create(&self.program, &sources, exit).map_err(|error| error.to_string())
    }

    fn ui(&mut self, root: &mut egui::Ui) {
        let context = &root.ctx().clone();
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
            .frame(egui::Frame::NONE)
            .show(root, |ui| {
                let mut clicked = None;
                let mut hovered = None;
                // The original's 640x480 screen, centred; the rest of the window stays black.
                let full = Rect::from_center_size(ui.max_rect().center(), Vec2::new(WIDTH as f32, HEIGHT as f32));
                ui.painter().rect_filled(full, 0.0, Color32::from_rgb(40, 28, 18));
                if let Some(textures) = &self.textures {
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
                        let rect = Rect::from_min_size(full.min + Vec2::new(x as f32, y as f32), size);
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
                    ui.scope_builder(egui::UiBuilder::new().max_rect(full), |ui| {
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
                    });
                }
                // Maiko's own mark in the corner the original art leaves to the map.
                let text = format!("Maiko {} · github.com/michidk/maiko", env!("CARGO_PKG_VERSION"));
                let galley = ui.painter().layout_no_wrap(text, egui::FontId::proportional(12.0), PARCHMENT);
                let rect = Rect::from_min_size(full.min + Vec2::new(10.0, HEIGHT as f32 - 28.0), galley.size() + Vec2::new(14.0, 8.0));
                let link = ui.interact(rect, ui.id().with("project"), if modal { Sense::hover() } else { Sense::click() });
                let hover = !modal && link.hovered();
                ui.painter().rect(rect, 4.0, Color32::from_black_alpha(if hover { 200 } else { 150 }), egui::Stroke::new(1.0, if hover { GOLD } else { GOLD_DIM }), egui::StrokeKind::Inside);
                ui.painter().galley(rect.min + Vec2::new(7.0, 4.0), galley, PARCHMENT);
                if hover {
                    context.set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if link.clicked() {
                    let _ = open::that_detached(PROJECT);
                }
                if modal {
                    ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_black_alpha(150));
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
        let name = match self.dialog {
            Dialog::None => return,
            Dialog::ChooseGame { .. } => "Find Redguard",
            Dialog::Options(_) => "Game Options",
            Dialog::Support => "Technical Support",
            Dialog::Report(true, _) => "Redguard stopped",
            Dialog::Report(false, _) => "Report saved",
            Dialog::Error(_) => "Something went wrong",
        };
        if let Some(font) = &self.title_font
            && !self.titles.contains_key(name)
        {
            self.titles
                .insert(name, texture(context, name, &font.render(name)));
        }
        let title = Title {
            text: name,
            texture: self.titles.get(name).cloned(),
        };
        let outcome = match &mut self.dialog {
            Dialog::None => Outcome::default(),
            Dialog::ChooseGame { typed, error } => {
                choose_game_dialog(context, &title, typed, error)
            }
            Dialog::Options(draft) => {
                let outcome = options_dialog(context, &title, draft, self.game.as_deref());
                if outcome.save {
                    self.settings = draft.clone();
                    let _ = self.settings.save();
                }
                outcome
            }
            Dialog::Support => support_dialog(context, &title),
            Dialog::Report(crashed, result) => report_dialog(context, &title, *crashed, result),
            Dialog::Error(text) => error_dialog(context, &title, text),
        };
        self.quit |= outcome.quit;
        let mut close = outcome.close || outcome.save;
        if let Some(folder) = outcome.choose {
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
        if outcome.report {
            self.dialog = Dialog::Report(false, self.report(None));
        }
    }

    /// Keyboard use as in the original dialog: arrows or Tab move between the buttons, Enter
    /// presses the focused one, Escape cancels.
    fn key(&mut self, key: egui::Key, shift: bool) {
        use egui::Key;
        if self.dialog != Dialog::None {
            if key == Key::Escape && !matches!(self.dialog, Dialog::ChooseGame { .. }) {
                self.dialog = Dialog::None;
            }
            return;
        }
        match key {
            Key::ArrowDown | Key::ArrowRight => self.move_focus(1),
            Key::ArrowUp | Key::ArrowLeft => self.move_focus(-1),
            Key::Tab => self.move_focus(if shift { -1 } else { 1 }),
            Key::Enter | Key::Space => {
                if let Some(focus) = self.focus {
                    self.activate(ACTIONS[focus]);
                }
            }
            Key::Escape => self.quit = true,
            _ => {}
        }
    }
}

// The dialogs' palette, taken from the launcher art: dark leather, gold trim, parchment text.
const INK: Color32 = Color32::from_rgb(26, 18, 12);
const LEATHER: Color32 = Color32::from_rgb(56, 40, 26);
const GOLD: Color32 = Color32::from_rgb(207, 168, 96);
const GOLD_DIM: Color32 = Color32::from_rgb(122, 96, 56);
const PARCHMENT: Color32 = Color32::from_rgb(238, 224, 192);
const MUTED: Color32 = Color32::from_rgb(170, 150, 118);
const RED: Color32 = Color32::from_rgb(148, 30, 20);
const ERROR: Color32 = Color32::from_rgb(232, 112, 92);
const DIALOG_WIDTH: f32 = 470.0;

/// What a dialog asked for this frame.
#[derive(Default)]
struct Outcome {
    close: bool,
    save: bool,
    quit: bool,
    report: bool,
    choose: Option<PathBuf>,
}

fn choose_game_dialog(
    context: &egui::Context,
    title: &Title,
    typed: &mut String,
    error: &Option<String>,
) -> Outcome {
    let mut outcome = Outcome::default();
    dialog(context, title, |ui| {
        ui.label("Maiko could not find your Redguard installation. Choose the folder that contains RGFX.EXE, or the GOG folder around it.");
        ui.add_space(12.0);
        if primary(ui, "Browse…").clicked()
            && let Some(folder) = picker::choose_folder()
        {
            outcome.choose = Some(folder);
        }
        ui.add_space(12.0);
        ui.label(muted("Or enter its path:"));
        ui.horizontal(|ui| {
            let field = ui.add(
                egui::TextEdit::singleline(typed)
                    .hint_text("/path/to/Redguard")
                    .desired_width(ui.available_width() - 70.0),
            );
            let entered =
                field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if (secondary(ui, "Use").clicked() || entered) && !typed.trim().is_empty() {
                outcome.choose = Some(PathBuf::from(typed.trim()));
            }
        });
        if let Some(error) = error {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(error.as_str()).color(ERROR));
        }
        footer(ui, |ui| {
            if secondary(ui, "Quit").clicked() {
                outcome.quit = true;
            }
        });
    });
    outcome
}

fn options_dialog(
    context: &egui::Context,
    title: &Title,
    draft: &mut Settings,
    game: Option<&Path>,
) -> Outcome {
    let mut outcome = Outcome::default();
    dialog(context, title, |ui| {
        section(ui, "Display");
        row(
            ui,
            "Fullscreen",
            "Borderless, at your desktop's resolution",
            |ui| {
                toggle(ui, &mut draft.fullscreen);
            },
        );
        row(ui, "Window size", "Used when not in fullscreen", |ui| {
            ui.add_enabled_ui(!draft.fullscreen, |ui| {
                choice(ui, "window size", &mut draft.window_size, &WINDOW_SIZES)
            });
        });
        row(
            ui,
            "Textures",
            "Original is the game's own soft, filtered look",
            |ui| {
                choice(
                    ui,
                    "texture filter",
                    &mut draft.texture_filter,
                    &TEXTURE_FILTERS,
                );
            },
        );
        row(
            ui,
            "Renderer",
            "Try another if the game shows nothing or runs badly",
            |ui| {
                choice(ui, "renderer", &mut draft.renderer, RENDERERS);
            },
        );
        if cfg!(windows) {
            row(
                ui,
                "Console window",
                "Shows the game's log while it runs",
                |ui| {
                    toggle(ui, &mut draft.console);
                },
            );
        }
        section(ui, "Movies");
        row(
            ui,
            "Skip the intro",
            "New Game goes straight to the ship",
            |ui| {
                toggle(ui, &mut draft.skip_intro);
            },
        );
        row(
            ui,
            "Skip the outro",
            "Quit closes the book without its animation",
            |ui| {
                toggle(ui, &mut draft.skip_outro);
            },
        );
        section(ui, "Game files");
        let game = game.map_or("Not chosen".into(), |game| game.display().to_string());
        row(ui, "Redguard folder", &game, |ui| {
            if secondary(ui, "Change…").clicked()
                && let Some(folder) = picker::choose_folder()
            {
                outcome.choose = Some(folder);
            }
        });
        let music = draft
            .cd_image
            .as_deref()
            .map_or("From the game folder's disc image".into(), |image| {
                image.display().to_string()
            });
        row(ui, "CD music", &music, |ui| {
            if draft.cd_image.is_some() && secondary(ui, "Reset").clicked() {
                draft.cd_image = None;
            }
            if secondary(ui, "Choose…").clicked()
                && let Some(folder) = picker::choose_folder()
            {
                draft.cd_image = Some(folder);
            }
        });
        ui.add_space(6.0);
        ui.label(muted(
            "Subtitles, sound and controls are in the game's own Options menu.",
        ));
        footer(ui, |ui| {
            if primary(ui, "Save").clicked() {
                outcome.save = true;
            }
            if secondary(ui, "Cancel").clicked() {
                outcome.close = true;
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if ui.link("Restore defaults").clicked() {
                    *draft = Settings::default();
                }
            });
        });
    });
    outcome
}

fn support_dialog(context: &egui::Context, title: &Title) -> Outcome {
    let mut outcome = Outcome::default();
    let logs = log_folder();
    dialog(context, title, |ui| {
        ui.label("Every start of the game writes a log. If something goes wrong, create a report and attach it to an issue. When the game crashes, the launcher creates one by itself.");
        if let Some(logs) = &logs {
            ui.add_space(8.0);
            path(ui, &logs.display().to_string());
        }
        footer(ui, |ui| {
            if secondary(ui, "Close").clicked() {
                outcome.close = true;
            }
            if secondary(ui, "Maiko on GitHub").clicked() {
                let _ = open::that_detached(PROJECT);
            }
            if primary(ui, "Create report").clicked() {
                outcome.report = true;
            }
            if let Some(logs) = &logs
                && ui
                    .add_enabled_ui(logs.is_dir(), |ui| secondary(ui, "Open log folder"))
                    .inner
                    .clicked()
            {
                let _ = open::that_detached(logs);
            }
        });
    });
    outcome
}

fn report_dialog(
    context: &egui::Context,
    title: &Title,
    crashed: bool,
    result: &Result<PathBuf, String>,
) -> Outcome {
    let mut outcome = Outcome::default();
    dialog(context, title, |ui| {
        if crashed {
            ui.label("Redguard stopped unexpectedly. Maiko saved a report with the game's log, its last screenshot, your system details, your settings and newest save (no game files).");
        } else {
            ui.label("Maiko saved a report with the newest log and screenshot, your system details, your settings and newest save (no game files).");
        }
        ui.add_space(8.0);
        match result {
            Ok(path) => {
                self::path(ui, &path.display().to_string());
                ui.add_space(6.0);
                ui.label(muted(
                    "Attach it to an issue so the problem can be looked into.",
                ));
            }
            Err(error) => {
                ui.label(
                    egui::RichText::new(format!("The report could not be written: {error}"))
                        .color(ERROR),
                );
            }
        }
        footer(ui, |ui| {
            if secondary(ui, "Close").clicked() {
                outcome.close = true;
            }
            if primary(ui, "Report an issue").clicked() {
                let _ = open::that_detached(ISSUES);
            }
            if let Ok(path) = result
                && let Some(folder) = path.parent()
                && secondary(ui, "Show report").clicked()
            {
                let _ = open::that_detached(folder);
            }
        });
    });
    outcome
}

fn error_dialog(context: &egui::Context, title: &Title, text: &str) -> Outcome {
    let mut outcome = Outcome::default();
    dialog(context, title, |ui| {
        ui.label(text);
        footer(ui, |ui| {
            if primary(ui, "OK").clicked() {
                outcome.close = true;
            }
        });
    });
    outcome
}

fn muted(text: &str) -> egui::RichText {
    egui::RichText::new(text).color(MUTED).size(12.5)
}

/// A dialog's title: drawn in the game's menu font when the installation has it.
struct Title {
    text: &'static str,
    texture: Option<TextureHandle>,
}

/// A centred dialog with a gold title and rule; the main screen is dimmed behind it.
fn dialog(context: &egui::Context, title: &Title, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Window::new(title.text)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size([DIALOG_WIDTH, 0.0])
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(
            egui::Frame::window(&context.global_style())
                .fill(INK)
                .stroke(egui::Stroke::new(1.0, GOLD_DIM))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(20, 16)),
        )
        .show(context, |ui| {
            match &title.texture {
                Some(texture) => {
                    ui.add(egui::Image::new(texture).fit_to_original_size(1.0));
                }
                None => {
                    ui.label(egui::RichText::new(title.text).size(22.0).color(GOLD));
                }
            }
            ui.add_space(2.0);
            let line = ui.available_rect_before_wrap();
            ui.painter()
                .hline(line.x_range(), line.top(), egui::Stroke::new(1.0, GOLD_DIM));
            ui.add_space(8.0);
            contents(ui);
        });
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(title.to_uppercase())
            .size(11.5)
            .color(GOLD)
            .strong(),
    );
}

/// One setting: its name and a short explanation on the left, its control on the right.
fn row(ui: &mut egui::Ui, title: &str, detail: &str, control: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_height(33.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            control(ui);
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(egui::RichText::new(title).color(PARCHMENT));
                ui.add(egui::Label::new(muted(detail)).truncate());
            });
        });
    });
}

/// The buttons along the bottom edge, from the right.
fn footer(ui: &mut egui::Ui, buttons: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(12.0);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), buttons);
}

fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).color(PARCHMENT).strong())
            .fill(RED)
            .stroke(egui::Stroke::new(1.0, GOLD_DIM))
            .min_size(Vec2::new(96.0, 30.0)),
    )
}

fn secondary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(text).min_size(Vec2::new(0.0, 30.0)))
}

fn path(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(14, 10, 6))
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    egui::RichText::new(text)
                        .monospace()
                        .size(12.0)
                        .color(MUTED),
                )
                .truncate(),
            );
        });
}

/// An on/off switch, drawn instead of a checkbox.
fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = Vec2::new(40.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let position = ui.ctx().animate_bool_responsive(response.id, *on);
    let radius = rect.height() / 2.0;
    let track = if *on { RED } else { LEATHER };
    let stroke = if response.hovered() { GOLD } else { GOLD_DIM };
    ui.painter().rect(
        rect,
        radius,
        track,
        egui::Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
    let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), position);
    ui.painter().circle_filled(
        Pos2::new(x, rect.center().y),
        radius - 4.0,
        if *on { PARCHMENT } else { MUTED },
    );
    response
}

/// Window sizes in the game's 4:3, from the original 640x480 up.
const WINDOW_SIZES: [(&str, &str); 6] = [
    ("", "Original (640 × 480)"),
    ("960x720", "960 × 720"),
    ("1280x960", "1280 × 960"),
    ("1600x1200", "1600 × 1200"),
    ("1920x1440", "1920 × 1440"),
    ("2560x1920", "2560 × 1920"),
];

const TEXTURE_FILTERS: [(&str, &str); 3] = [
    ("", "Original"),
    ("smooth", "Always smooth"),
    ("sharp", "Sharp pixels"),
];

/// The graphics backends the renderer can use on this system; Automatic lets it pick.
const RENDERERS: &[(&str, &str)] = if cfg!(windows) {
    &[
        ("", "Automatic"),
        ("dx12", "Direct3D 12"),
        ("vulkan", "Vulkan"),
        ("gl", "OpenGL"),
    ]
} else if cfg!(target_os = "macos") {
    &[("", "Automatic"), ("metal", "Metal")]
} else {
    &[("", "Automatic"), ("vulkan", "Vulkan"), ("gl", "OpenGL")]
};

/// A drop-down of `(setting value, label)` choices; an unknown stored value is shown as itself.
fn choice(ui: &mut egui::Ui, id: &str, value: &mut String, choices: &[(&str, &str)]) {
    let current = choices
        .iter()
        .find(|(choice, _)| choice == value)
        .map_or_else(
            || value.replace('x', " × "),
            |(_, label)| (*label).to_owned(),
        );
    egui::ComboBox::from_id_salt(id)
        .selected_text(current)
        .width(170.0)
        .show_ui(ui, |ui| {
            for (choice, label) in choices {
                ui.selectable_value(value, (*choice).to_owned(), *label);
            }
        });
}

fn theme(context: &egui::Context) {
    let mut style = (*context.global_style()).clone();
    use egui::{FontId, TextStyle};
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(22.0)),
        (TextStyle::Body, FontId::proportional(14.5)),
        (TextStyle::Button, FontId::proportional(14.0)),
        (TextStyle::Small, FontId::proportional(12.0)),
        (TextStyle::Monospace, FontId::monospace(12.5)),
    ]
    .into();
    style.spacing.button_padding = Vec2::new(12.0, 5.0);
    style.spacing.item_spacing = Vec2::new(8.0, 4.0);
    let visuals = &mut style.visuals;
    *visuals = egui::Visuals::dark();
    visuals.weak_text_color = Some(MUTED);
    visuals.window_fill = INK;
    visuals.panel_fill = INK;
    visuals.extreme_bg_color = Color32::from_rgb(14, 10, 6);
    visuals.hyperlink_color = GOLD;
    visuals.selection.bg_fill = RED;
    visuals.selection.stroke = egui::Stroke::new(1.0, GOLD);
    visuals.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(160),
    };
    let widgets = &mut visuals.widgets;
    for (widget, fill, stroke) in [
        (&mut widgets.inactive, LEATHER, GOLD_DIM),
        (&mut widgets.hovered, Color32::from_rgb(78, 56, 36), GOLD),
        (&mut widgets.active, Color32::from_rgb(98, 70, 44), GOLD),
        (&mut widgets.open, Color32::from_rgb(78, 56, 36), GOLD),
    ] {
        widget.weak_bg_fill = fill;
        widget.bg_fill = fill;
        widget.bg_stroke = egui::Stroke::new(1.0, stroke);
        widget.fg_stroke = egui::Stroke::new(1.0, PARCHMENT);
        widget.corner_radius = egui::CornerRadius::same(4);
    }
    // Labels take their colour from the non-interactive widget style.
    widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, PARCHMENT);
    context.set_global_style(style);
}

/// The window's drawing surface and the egui renderer that paints into it.
struct Gpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: egui_wgpu::Renderer,
}

impl Gpu {
    fn new(window: Arc<Window>) -> Result<Self, String> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|e| format!("no graphics adapter: {e}"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("launcher"),
            required_limits:
                wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .map_err(|e| e.to_string())?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("the window cannot be drawn to")?;
        // egui blends in gamma space; a non-sRGB view keeps its colours as designed.
        config.format = config.format.remove_srgb_suffix();
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &config);
        let renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            egui_wgpu::RendererOptions::default(),
        );
        Ok(Self {
            surface,
            device,
            queue,
            config,
            renderer,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            (self.config.width, self.config.height) = (width, height);
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn paint(
        &mut self,
        primitives: &[egui::ClippedPrimitive],
        textures: &egui::TexturesDelta,
        pixels_per_point: f32,
    ) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format),
            ..Default::default()
        });
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point,
        };
        for (id, deltas) in &textures.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&self.device, &self.queue, *id, delta);
            }
        }
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut commands = self.renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            primitives,
            &screen,
        );
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("launcher"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.renderer.render(&mut pass, primitives, &screen);
        }
        commands.push(encoder.finish());
        self.queue.submit(commands);
        self.queue.present(frame);
        for id in &textures.free {
            self.renderer.free_texture(id);
        }
    }
}

/// The launcher window around [`App`]: winit events in, egui frames out. While the game runs
/// the window is hidden and the loop only checks on the game twice a second.
struct Shell {
    app: App,
    context: egui::Context,
    window: Option<Arc<Window>>,
    state: Option<egui_winit::State>,
    gpu: Option<Gpu>,
    hidden: bool,
    repaint_at: Option<Instant>,
    error: Option<String>,
}

impl Shell {
    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let mut attributes = Window::default_attributes()
            .with_title("Maiko")
            .with_inner_size(winit::dpi::LogicalSize::new(WIDTH, HEIGHT))
            .with_min_inner_size(winit::dpi::LogicalSize::new(WIDTH / 2, HEIGHT / 2));
        if let Some(monitor) = event_loop.primary_monitor() {
            let (screen, scale) = (monitor.size(), monitor.scale_factor());
            let x = (f64::from(screen.width) - f64::from(WIDTH) * scale) / 2.0;
            let y = (f64::from(screen.height) - f64::from(HEIGHT) * scale) / 2.0;
            attributes =
                attributes.with_position(winit::dpi::PhysicalPosition::new(x.max(0.0), y.max(0.0)));
        }
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|e| e.to_string())?,
        );
        self.gpu = Some(Gpu::new(window.clone())?);
        self.state = Some(egui_winit::State::new(
            self.context.clone(),
            egui::ViewportId::ROOT,
            event_loop,
            Some(window.scale_factor() as f32),
            None,
            None,
        ));
        self.window = Some(window);
        Ok(())
    }

    fn frame(&mut self) {
        let (Some(window), Some(state), Some(gpu)) = (&self.window, &mut self.state, &mut self.gpu)
        else {
            return;
        };
        // The 640x480 screen is zoomed to fit the window; App::ui centres it with black bars.
        let size = window.inner_size();
        let native = window.scale_factor() as f32;
        let zoom = (size.width as f32 / (WIDTH as f32 * native))
            .min(size.height as f32 / (HEIGHT as f32 * native));
        if zoom > 0.0 && (self.context.zoom_factor() - zoom).abs() > 0.001 {
            self.context.set_zoom_factor(zoom);
        }
        let input = state.take_egui_input(window);
        for event in &input.events {
            if let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                self.app.key(*key, modifiers.shift);
            }
        }
        let output = self.context.run_ui(input, |ui| self.app.ui(ui));
        state.handle_platform_output(window, output.platform_output);
        let primitives = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        gpu.paint(&primitives, &output.textures_delta, output.pixels_per_point);
        let delay = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(Duration::MAX, |viewport| viewport.repaint_delay);
        self.repaint_at = Instant::now().checked_add(delay);
    }
}

impl ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.open(event_loop)
        {
            self.error = Some(error);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let (Some(window), Some(state)) = (&self.window, &mut self.state) else {
            return;
        };
        let response = state.on_window_event(window, &event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => self.frame(),
            _ if response.repaint => window.request_redraw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.app.watch_game();
        if self.app.quit {
            event_loop.exit();
            return;
        }
        let Some(window) = &self.window else { return };
        if self.app.game_process.is_some() != self.hidden {
            self.hidden = !self.hidden;
            window.set_visible(!self.hidden);
            if !self.hidden {
                window.focus_window();
                window.request_redraw();
            }
        }
        if self.hidden {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(500),
            ));
            return;
        }
        match self.repaint_at {
            Some(at) if at <= Instant::now() => {
                self.repaint_at = None;
                window.request_redraw();
            }
            Some(at) => event_loop.set_control_flow(ControlFlow::WaitUntil(at)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }
}

fn run() -> Result<(), String> {
    let program = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("the launcher has no folder")?
        .to_path_buf();
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    let context = egui::Context::default();
    theme(&context);
    let mut shell = Shell {
        app: App::new(program),
        context,
        window: None,
        state: None,
        gpu: None,
        hidden: false,
        repaint_at: None,
        error: None,
    };
    event_loop.run_app(&mut shell).map_err(|e| e.to_string())?;
    shell.error.map_or(Ok(()), Err)
}

/// A modal error box with the system's own dialog.
fn show_error(text: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Maiko")
        .set_description(text)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// The launcher's entry point; the packaged executables call it from a small C `main` (see
/// `tools/build_link_inputs.py`) and `cargo run` from `main.rs`.
#[unsafe(no_mangle)]
pub extern "C" fn maiko_launcher_main() -> std::ffi::c_int {
    match run() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("Maiko launcher: {error}");
            show_error(&format!("The launcher could not start: {error}"));
            1
        }
    }
}
