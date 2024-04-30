mod connected_tab;
mod nwg_ext;
mod persisted_tab;
mod usbipd_gui;

use native_windows_gui as nwg;
use nwg::NativeUi;
use usbipd_gui::UsbipdGui;

/// Starts the GUI and runs the event loop.
///
/// This function will not return until the app is closed.
pub fn start() -> Result<(), nwg::NwgError> {
    nwg::init()?;

    let mut font = nwg::Font::default();
    nwg::Font::builder()
        .family("Segoe UI")
        .size(16)
        .weight(400)
        .build(&mut font)?;

    nwg::Font::set_global_default(Some(font));

    let _gui = UsbipdGui::build_ui(Default::default())?;

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
