#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg(target_os = "windows")]

mod args;
mod auto_attacher;
mod gui;
mod usbipd;
mod win_utils;

use std::{cell::RefCell, path::PathBuf, process::ExitCode, rc::Rc};

use args::Args;
use auto_attacher::AutoAttacher;

fn main() -> ExitCode {
    // Ensure that all child processes are terminated when the main application exits
    if let Err(e) = win_utils::setup_job_object_grouping() {
        eprintln!("Warning: Could not group processes: {}", e);
    }

    // Parse arguments
    let args = match Args::parse() {
        Ok(args) => args,
        Err(code) => return code,
    };

    // Ensure that only one instance of the application is running
    if !win_utils::acquire_single_instance_lock() {
        gui::show_multiple_instance_warning();
        return ExitCode::FAILURE;
    }

    // Check installed and minimum supported version
    match usbipd::version() {
        None => {
            gui::show_usbipd_not_found_error();
            return ExitCode::FAILURE;
        }

        Some(version) => {
            if !(version.major >= 4 && version.minor >= 2) {
                gui::show_usbipd_unsupported_version_error();
                return ExitCode::FAILURE;
            }
        }
    }

    let storage_path =
        match std::env::var("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("WSL USB Manager")) {
            Ok(path) => std::fs::create_dir_all(&path).map(|_| path).ok(),
            Err(_) => None,
        };

    let auto_attacher = if let Some(mut path) = storage_path {
        path.push("profiles.json");
        AutoAttacher::with_storage(&path)
    } else {
        AutoAttacher::new()
    };
    let auto_attacher_rc = Rc::new(RefCell::new(auto_attacher));

    let start = gui::start(&auto_attacher_rc, args.minimized);

    if let Err(err) = start {
        gui::show_start_failure(&err.to_string());
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
