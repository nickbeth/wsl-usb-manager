use std::{cell::RefCell, rc::Rc};

use native_windows_gui as nwg;
use nwg::NativeUi;

use super::usbipd_gui::UsbipdGui;
use crate::auto_attach::AutoAttacher;

/// Starts the GUI and runs the event loop.
///
/// This function will not return until the app is closed.
pub fn start(
    auto_attacher: &Rc<RefCell<AutoAttacher>>,
    start_minimized: bool,
) -> Result<(), nwg::NwgError> {
    nwg::init()?;

    let mut font = nwg::Font::default();
    nwg::Font::builder()
        .family("Segoe UI Variable Text")
        .size(16)
        .weight(400)
        .build(&mut font)?;

    nwg::Font::set_global_default(Some(font));

    let _gui = UsbipdGui::build_ui(UsbipdGui::new(auto_attacher, start_minimized))?;

    // Run the event loop
    nwg::dispatch_thread_events();
    Ok(())
}

/// Shows a warning message telling the user that another instance is already running.
///
/// This function is called when the app fails to obtain the instance lock because one is already held.
pub fn show_multiple_instance_warning() {
    nwg::message(&nwg::MessageParams {
        title: "WSL USB Manager: Multiple Instances Detected",
        content: concat!(
            "Another instance of the app is already running.\n",
            "Please check the system tray."
        ),
        buttons: nwg::MessageButtons::Ok,
        icons: nwg::MessageIcons::Warning,
    });
}

/// Shows an error message telling the user that USBIPD was not found.
///
/// This function is called when the app fails to find the USBIPD executable during startup.
pub fn show_usbipd_not_found_error() {
    nwg::message(&nwg::MessageParams {
        title: "WSL USB Manager: USBIPD Not Found",
        content: "USBIPD was not found, please make sure that it is installed and available in the system PATH.",
        buttons: nwg::MessageButtons::Ok,
        icons: nwg::MessageIcons::Error,
    });
}

/// Shows a warning message telling the user that an untested version of USBIPD was found.
///
/// This function is called when the app finds a version of USBIPD lower than 4.
pub fn show_usbipd_untested_version_warning() {
    nwg::message(&nwg::MessageParams {
        title: "WSL USB Manager: Untested USBIPD Version",
        content: concat!(
            "An untested version of USBIPD was found, this app may not work correctly. ",
            "Please install USBIPD version 4 or newer."
        ),
        buttons: nwg::MessageButtons::Ok,
        icons: nwg::MessageIcons::Warning,
    });
}

/// Shows an error message telling the user that the app failed to start.
/// The passed message should contain details about the error that occurred.
///
/// This function is called when the app fails to start the GUI.
pub fn show_start_failure(error: &str) {
    let content = format!(
        concat!(
            "An error occurred while starting the app, ",
            "try opening the app again or reboot the system if the issue persists.\n\n",
            "Error:\n",
            "{}"
        ),
        error
    );

    nwg::message(&nwg::MessageParams {
        title: "WSL USB Manager: Start Failure",
        content: &content,
        buttons: nwg::MessageButtons::Ok,
        icons: nwg::MessageIcons::Error,
    });
}
