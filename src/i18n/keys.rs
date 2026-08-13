//! Translation key constants.
//!
//! Every constant must match a key in `locales/en.json` and `locales/zh.json`
//! **byte-for-byte**. Keep this file grouped identically to the JSON files so
//! that review is mechanical.
//!
//! Naming convention: `key.to_uppercase().replace('.', "_")` with existing
//! snake_case segments preserved (e.g. `menu.device.bind_force` -> `MENU_DEVICE_BIND_FORCE`).

// --- menu (menu bar + context) ---
pub const MENU_FILE_LABEL: &str = "menu.file.label";
pub const MENU_FILE_REFRESH: &str = "menu.file.refresh";
pub const MENU_FILE_EXIT: &str = "menu.file.exit";

pub const MENU_LANGUAGE_LABEL: &str = "menu.language.label";
pub const MENU_LANGUAGE_MORE: &str = "menu.language.more";

pub const MENU_DEVICE_LABEL: &str = "menu.device.label";
pub const MENU_DEVICE_ATTACH: &str = "menu.device.attach";
pub const MENU_DEVICE_DETACH: &str = "menu.device.detach";
pub const MENU_DEVICE_BIND: &str = "menu.device.bind";
pub const MENU_DEVICE_BIND_FORCE: &str = "menu.device.bind_force";
pub const MENU_DEVICE_UNBIND: &str = "menu.device.unbind";
pub const MENU_DEVICE_DELETE: &str = "menu.device.delete";
pub const MENU_DEVICE_RESTART: &str = "menu.device.restart";

// --- tab titles ---
pub const TAB_CONNECTED_LABEL: &str = "tab.connected.label";
pub const TAB_PERSISTED_LABEL: &str = "tab.persisted.label";
pub const TAB_AUTO_ATTACH_LABEL: &str = "tab.auto_attach.label";

// --- list headers ---
pub const LIST_CONNECTED_BUS_ID: &str = "list.connected.bus_id";
pub const LIST_CONNECTED_STATE: &str = "list.connected.state";
pub const LIST_CONNECTED_DEVICE: &str = "list.connected.device";
pub const LIST_PERSISTED_DEVICE: &str = "list.persisted.device";
pub const LIST_AUTO_ATTACH_STATUS: &str = "list.auto_attach.status";
pub const LIST_AUTO_ATTACH_DESCRIPTION: &str = "list.auto_attach.description";

// --- info panels ---
pub const INFO_DEVICE_TITLE: &str = "info.device.title";
pub const INFO_DEVICE_BUS_ID_LABEL: &str = "info.device.bus_id_label";
pub const INFO_DEVICE_VID_PID_LABEL: &str = "info.device.vid_pid_label";
pub const INFO_DEVICE_SERIAL_LABEL: &str = "info.device.serial_label";
pub const INFO_DEVICE_STATE_LABEL: &str = "info.device.state_label";
pub const INFO_DEVICE_DESCRIPTION_LABEL: &str = "info.device.description_label";

pub const INFO_PERSISTED_TITLE: &str = "info.persisted.title";
pub const INFO_PERSISTED_VID_PID_LABEL: &str = "info.persisted.vid_pid_label";
pub const INFO_PERSISTED_SERIAL_LABEL: &str = "info.persisted.serial_label";
pub const INFO_PERSISTED_ID_LABEL: &str = "info.persisted.id_label";
pub const INFO_PERSISTED_DESCRIPTION_LABEL: &str = "info.persisted.description_label";

pub const INFO_AUTO_ATTACH_TITLE: &str = "info.auto_attach.title";
pub const INFO_AUTO_ATTACH_BUS_ID_LABEL: &str = "info.auto_attach.bus_id_label";
pub const INFO_AUTO_ATTACH_HARDWARE_ID_LABEL: &str = "info.auto_attach.hardware_id_label";
pub const INFO_AUTO_ATTACH_DESCRIPTION_LABEL: &str = "info.auto_attach.description_label";
pub const INFO_AUTO_ATTACH_STATUS_LABEL: &str = "info.auto_attach.status_label";
pub const INFO_AUTO_ATTACH_LAST_ERROR_LABEL: &str = "info.auto_attach.last_error_label";

// --- buttons ---
pub const BUTTON_ATTACH: &str = "button.attach";
pub const BUTTON_DETACH: &str = "button.detach";
pub const BUTTON_BIND: &str = "button.bind";
pub const BUTTON_UNBIND: &str = "button.unbind";
pub const BUTTON_AUTO_ATTACH: &str = "button.auto_attach";
pub const BUTTON_DELETE: &str = "button.delete";
pub const BUTTON_RESTART: &str = "button.restart";
pub const BUTTON_OK: &str = "button.ok";
pub const BUTTON_CANCEL: &str = "button.cancel";

// --- UsbipState strings ---
pub const STATE_NOT_SHARED: &str = "state.not_shared";
pub const STATE_PERSISTED: &str = "state.persisted";
pub const STATE_SHARED: &str = "state.shared";
pub const STATE_ATTACHED: &str = "state.attached";
pub const STATE_FORCED_SUFFIX: &str = "state.forced_suffix";

// --- placeholders ---
pub const PLACEHOLDER_UNKNOWN_DEVICE: &str = "placeholder.unknown_device";
pub const PLACEHOLDER_NO_DESCRIPTION: &str = "placeholder.no_description";
pub const PLACEHOLDER_NO_DEVICE_SELECTED: &str = "placeholder.no_device_selected";
pub const PLACEHOLDER_NO_PROFILE_SELECTED: &str = "placeholder.no_profile_selected";
pub const PLACEHOLDER_UNKNOWN_PORT: &str = "placeholder.unknown_port";
pub const PLACEHOLDER_NO_ERROR: &str = "placeholder.no_error";
pub const PLACEHOLDER_ANY_DEVICE_PORT: &str = "placeholder.any_device_port";
pub const PLACEHOLDER_NO_BOUND_DEVICES: &str = "placeholder.no_bound_devices";
pub const PLACEHOLDER_ANY_DEVICE_ON_PORT: &str = "placeholder.any_device_on_port";

// --- status indicators ---
pub const STATUS_ACTIVE: &str = "status.active";
pub const STATUS_INACTIVE: &str = "status.inactive";

// --- tray ---
pub const TRAY_OPEN: &str = "tray.open";
pub const TRAY_EXIT: &str = "tray.exit";
pub const TRAY_UNKNOWN_DEVICE_WITH_BUS: &str = "tray.unknown_device_with_bus";

// --- dialogs ---
pub const DIALOG_TITLE_MAIN: &str = "dialog.title.main";
pub const DIALOG_TITLE_COMMAND_ERROR: &str = "dialog.title.command_error";
pub const DIALOG_TITLE_MULTIPLE_INSTANCES: &str = "dialog.title.multiple_instances";
pub const DIALOG_TITLE_USBIPD_NOT_FOUND: &str = "dialog.title.usbipd_not_found";
pub const DIALOG_TITLE_USBIPD_UNSUPPORTED: &str = "dialog.title.usbipd_unsupported";
pub const DIALOG_TITLE_START_FAILURE: &str = "dialog.title.start_failure";
pub const DIALOG_TITLE_AUTO_ATTACH_ERROR: &str = "dialog.title.auto_attach_error";
pub const DIALOG_TITLE_CONFIGURE_AUTO_ATTACH: &str = "dialog.title.configure_auto_attach";
pub const DIALOG_TITLE_MORE_LANGUAGES: &str = "dialog.title.more_languages";

pub const DIALOG_CONTENT_MULTIPLE_INSTANCES: &str = "dialog.content.multiple_instances";
pub const DIALOG_CONTENT_USBIPD_NOT_FOUND: &str = "dialog.content.usbipd_not_found";
pub const DIALOG_CONTENT_USBIPD_UNSUPPORTED: &str = "dialog.content.usbipd_unsupported";
pub const DIALOG_CONTENT_START_FAILURE: &str = "dialog.content.start_failure";
pub const DIALOG_CONTENT_AUTO_ATTACH_MODE_PROMPT: &str = "dialog.content.auto_attach_mode_prompt";
pub const DIALOG_CONTENT_DEVICE_OPTION: &str = "dialog.content.device_option";
pub const DIALOG_CONTENT_PORT_OPTION: &str = "dialog.content.port_option";
