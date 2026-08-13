use std::{
    cell::{LazyCell, RefCell},
    rc::Rc,
};

use native_windows_gui as nwg;
use windows_sys::Win32::UI::Shell::SIID_SHIELD;

use crate::{
    auto_attacher::AutoAttacher,
    gui::{nwg_ext::BitmapEx, tray::Tray},
    i18n::{Translator, keys},
};

#[derive(Default)]
pub struct GuiResources {
    pub embed: nwg::EmbedResource,
    pub app_icon: nwg::Icon,
    pub shield_bitmap: nwg::Bitmap,
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

    // Load the UAC shield icon from the system icons
    resources.shield_bitmap = nwg::Bitmap::from_system_icon(SIID_SHIELD);

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

/// Loads the active translator for the startup error dialogs.
///
/// These `show_*` functions run before / outside the [`MainWindow`], so there
/// is no shared translator to consult. Each call loads a fresh snapshot from
/// the persisted language preference (silent fallback to English). The cost is
/// a few milliseconds and only happens on startup-failure paths.
fn startup_translator() -> Translator {
    let lang = crate::i18n::settings::load_language().unwrap_or_else(|| "en".to_owned());
    Translator::load(&lang).unwrap_or_default()
}

/// Shows a warning message telling the user that another instance is already running.
///
/// This function is called when the app fails to obtain the instance lock because one is already held.
pub fn show_multiple_instance_warning() {
    let t = startup_translator();
    nwg::message(&nwg::MessageParams {
        title: t.get(keys::DIALOG_TITLE_MULTIPLE_INSTANCES),
        content: t.get(keys::DIALOG_CONTENT_MULTIPLE_INSTANCES),
        buttons: nwg::MessageButtons::Ok,
        icons: nwg::MessageIcons::Warning,
    });
}

/// Shows an error message telling the user that USBIPD was not found.
///
/// This function is called when the app fails to find the USBIPD executable during startup.
pub fn show_usbipd_not_found_error() {
    let t = startup_translator();
    nwg::message(&nwg::MessageParams {
        title: t.get(keys::DIALOG_TITLE_USBIPD_NOT_FOUND),
        content: t.get(keys::DIALOG_CONTENT_USBIPD_NOT_FOUND),
        buttons: nwg::MessageButtons::Ok,
        icons: nwg::MessageIcons::Error,
    });
}

/// Shows an error message telling the user that an unsupported version of USBIPD was found.
pub fn show_usbipd_unsupported_version_error() {
    let t = startup_translator();
    nwg::message(&nwg::MessageParams {
        title: t.get(keys::DIALOG_TITLE_USBIPD_UNSUPPORTED),
        content: t.get(keys::DIALOG_CONTENT_USBIPD_UNSUPPORTED),
        buttons: nwg::MessageButtons::Ok,
        icons: nwg::MessageIcons::Error,
    });
}

/// Shows an error message telling the user that the app failed to start.
/// The passed message should contain details about the error that occurred.
///
/// This function is called when the app fails to start the GUI.
pub fn show_start_failure(error: &str) {
    let t = startup_translator();
    let content = t.getf(keys::DIALOG_CONTENT_START_FAILURE, &[("error", error)]);

    nwg::message(&nwg::MessageParams {
        title: t.get(keys::DIALOG_TITLE_START_FAILURE),
        content: &content,
        buttons: nwg::MessageButtons::Ok,
        icons: nwg::MessageIcons::Error,
    });
}
