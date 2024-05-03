#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg(target_os = "windows")]

mod auto_attach;
mod gui;
mod usbipd;
mod win_utils;

use std::{cell::RefCell, rc::Rc};

use auto_attach::AutoAttacher;

fn main() {
    // Ensure that only one instance of the application is running
    if !win_utils::acquire_single_instance_lock() {
        gui::show_multiple_instance_warning();
        return;
    }

    if !usbipd::check_installed() {
        gui::show_usbipd_not_found_error();
        return;
    }

    if usbipd::version().major < 4 {
        gui::show_usbipd_untested_version_warning();
        return;
    }

    let auto_attacher = Rc::new(RefCell::new(AutoAttacher::new()));

    let start = gui::start(&auto_attacher);

    if let Err(err) = start {
        gui::show_start_failure(&err.to_string());
    }
}
