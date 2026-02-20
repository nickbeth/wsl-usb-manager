use std::{
    cell::{LazyCell, RefCell},
    rc::Rc,
};

use native_windows_gui as nwg;

use crate::{auto_attach::AutoAttacher, gui::tray::Tray};

#[derive(Default)]
pub struct GuiResources {
    pub embed: nwg::EmbedResource,
    pub app_icon: nwg::Icon,
}

// This is fine since these resources are only accessed from the same thread
#[allow(clippy::declare_interior_mutable_const)]
pub const RESOURCES: LazyCell<GuiResources> = LazyCell::new(|| {
    let mut resources = GuiResources::default();

    // Load the embedded resources from the executable
    nwg::EmbedResource::builder()
        .build(&mut resources.embed)
        .expect("Failed to load embedded resources");

    // Load the app icon from the embedded resources
    nwg::Icon::builder()
        .source_embed(Some(&resources.embed))
        .source_embed_str(Some("MAINICON"))
        .build(&mut resources.app_icon)
        .expect("Failed to load app icon from embedded resources");

    resources
});

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

    let _tray = Tray::build_ui(Tray::new(auto_attacher), start_minimized)?;

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

/// Shows an error message telling the user that an unsupported version of USBIPD was found.
pub fn show_usbipd_unsupported_version_error() {
    nwg::message(&nwg::MessageParams {
        title: "WSL USB Manager: Unsupported USBIPD Version",
        content: "An unsupported version of USBIPD was found, please install USBIPD version 4.2.0 or newer.",
        buttons: nwg::MessageButtons::Ok,
        icons: nwg::MessageIcons::Error,
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
