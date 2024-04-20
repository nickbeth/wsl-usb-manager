#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg(target_os = "windows")]

mod gui;
mod usbipd;
mod win_utils;

fn main() {
    // Ensure that only one instance of the application is running
    if !win_utils::acquire_single_instance_lock() {
        gui::show_multiple_instance_message();
        return;
    }

    let start = gui::start();

    if let Err(err) = start {
        gui::show_start_failure_message(&err.to_string());
    }
}
