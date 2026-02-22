#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg(target_os = "windows")]

mod args;
mod auto_attach;
mod gui;
mod settings;
mod usbipd;
mod win_utils;

use std::{cell::RefCell, process::ExitCode, rc::Rc};

use args::Args;
use auto_attach::AutoAttacher;

fn main() -> ExitCode {
    // Parse arguments
    let args = match Args::parse() {
        Ok(args) => args,
        Err(code) => return code,
    };

    let _settings_location = settings::ensure_settings_dir();

    // Ensure that only one instance of the application is running
    if !win_utils::acquire_single_instance_lock() {
        gui::show_multiple_instance_warning();
        return ExitCode::FAILURE;
    }

    if !usbipd::check_installed() {
        gui::show_usbipd_not_found_error();
        return ExitCode::FAILURE;
    }

    if usbipd::version().major < 4 {
        gui::show_usbipd_untested_version_warning();
    }

    let auto_attacher = Rc::new(RefCell::new(AutoAttacher::new()));

    let start = gui::start(&auto_attacher, args.minimized);

    if let Err(err) = start {
        gui::show_start_failure(&err.to_string());
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
