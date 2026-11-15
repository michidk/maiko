#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() {
    std::process::exit(maiko_launcher::app::maiko_launcher_main());
}
