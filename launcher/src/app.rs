//! The launcher window: the original Redguard launcher, recreated, with its artwork read from the
//! player's `REDGUARD.EXE`. Without that file it shows plain buttons instead.
use crate::artwork::{Artwork, BUTTON_POSITIONS, Image};
use crate::font::Font;
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
    Options(Settings),
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
    /// The game's gold menu font for dialog titles, and the titles drawn with it.
    title_font: Option<Font>,
    titles: HashMap<&'static str, TextureHandle>,
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
            title_font: None,
            titles: HashMap::new(),
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
        self.title_font = Font::load(&game, "FONTSEL.FNT");
        self.titles.clear();
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
            Action::Options => self.dialog = Dialog::Options(self.settings.clone()),
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
        let mut close = false;
        let mut choose = None;
        match &mut self.dialog {
            Dialog::None => {}
            Dialog::ChooseGame { typed, error } => {
                dialog(context, &title, |ui| {
                    ui.label("Maiko could not find your Redguard installation. Choose the folder that contains RGFX.EXE, or the GOG folder around it.");
                    ui.add_space(12.0);
                    if primary(ui, "Browse…").clicked()
                        && let Some(folder) = picker::choose_folder()
                    {
                        choose = Some(folder);
                    }
                    ui.add_space(12.0);
                    ui.label(muted("Or enter its path:"));
                    ui.horizontal(|ui| {
                        let field = ui.add(
                            egui::TextEdit::singleline(typed)
                                .hint_text("/path/to/Redguard")
                                .desired_width(ui.available_width() - 70.0),
                        );
                        let entered = field.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter));
                        if (secondary(ui, "Use").clicked() || entered) && !typed.trim().is_empty() {
                            choose = Some(PathBuf::from(typed.trim()));
                        }
                    });
                    if let Some(error) = error {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(error.as_str()).color(ERROR));
                    }
                    footer(ui, |ui| {
                        if secondary(ui, "Quit").clicked() {
                            self.quit = true;
                        }
                    });
                });
            }
            Dialog::Options(draft) => {
                let mut save = false;
                dialog(context, &title, |ui| {
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
                    let game = self
                        .game
                        .as_deref()
                        .map_or("Not chosen".into(), |game| game.display().to_string());
                    row(ui, "Redguard folder", &game, |ui| {
                        if secondary(ui, "Change…").clicked()
                            && let Some(folder) = picker::choose_folder()
                        {
                            choose = Some(folder);
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
                            save = true;
                        }
                        if secondary(ui, "Cancel").clicked() {
                            close = true;
                        }
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            if ui.link("Restore defaults").clicked() {
                                *draft = Settings::default();
                            }
                        });
                    });
                });
                if save {
                    self.settings = draft.clone();
                    let _ = self.settings.save();
                    close = true;
                }
            }
            Dialog::Support => {
                let logs = std::env::var_os("MAIKO_LOG_DIR")
                    .map(PathBuf::from)
                    .map_or_else(settings::log_directory, Ok)
                    .ok();
                dialog(context, &title, |ui| {
                    ui.label("Every start of the game writes a log. If something goes wrong, open an issue and attach the newest log.");
                    if let Some(logs) = &logs {
                        ui.add_space(8.0);
                        path(ui, &logs.display().to_string());
                    }
                    footer(ui, |ui| {
                        if secondary(ui, "Close").clicked() {
                            close = true;
                        }
                        if primary(ui, "Report an issue").clicked() {
                            let _ = open(ISSUES.as_ref());
                        }
                        if let Some(logs) = &logs
                            && ui
                                .add_enabled_ui(logs.is_dir(), |ui| {
                                    secondary(ui, "Open log folder")
                                })
                                .inner
                                .clicked()
                        {
                            let _ = open(logs.as_os_str());
                        }
                    });
                });
            }
            Dialog::Error(text) => {
                dialog(context, &title, |ui| {
                    ui.label(text.as_str());
                    footer(ui, |ui| {
                        if primary(ui, "OK").clicked() {
                            close = true;
                        }
                    });
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
            egui::Frame::window(&context.style())
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
    let mut style = (*context.style()).clone();
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
    context.set_style(style);
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
    theme(&context);
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
