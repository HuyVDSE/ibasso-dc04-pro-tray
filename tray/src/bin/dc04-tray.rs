#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
#[path = "../win32/mod.rs"]
mod win32;
#[cfg(windows)]
#[path = "../worker.rs"]
mod worker;

#[cfg(windows)]
fn main() {
    if let Err(error) = win32::run() {
        win32::show_error(&error.to_string());
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("dc04-tray requires Windows.");
    std::process::exit(1);
}
