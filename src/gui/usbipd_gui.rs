use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use native_windows_derive::NwgUi;
use native_windows_gui as nwg;

use super::auto_attach_tab::AutoAttachTab;
use super::connected_tab::ConnectedTab;
use super::persisted_tab::PersistedTab;
use crate::usbipd::{list_devices, UsbDevice};
use crate::{
    auto_attach::AutoAttacher,
    win_utils::{self, DeviceNotification},
};

pub(super) trait GuiTab {
    /// Initializes the tab. The root window handle is provided.
    fn init(&self, window: &nwg::Window);

    /// Refreshes the data displayed in the tab.
    fn refresh(&self);
}

#[derive(Default, NwgUi)]
pub struct UsbipdGui {
    device_notification: Cell<DeviceNotification>,
    menu_tray_event_handler: Cell<Option<nwg::EventHandler>>,

    #[nwg_resource]
    embed: nwg::EmbedResource,

    #[nwg_resource(source_embed: Some(&data.embed), source_embed_str: Some("MAINICON"))]
    app_icon: nwg::Icon,

    // Window
    #[nwg_control(size: (780, 430), center: true, title: "WSL USB Manager", icon: Some(&data.app_icon))]
    #[nwg_events(
        OnInit: [UsbipdGui::init],
        OnMinMaxInfo: [UsbipdGui::min_max_info(EVT_DATA)],
        OnWindowClose: [UsbipdGui::hide(SELF, EVT_DATA)]
    )]
    window: nwg::Window,

    #[nwg_layout(parent: window, auto_spacing: Some(2))]
    window_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: window)]
    #[nwg_events(OnNotice: [UsbipdGui::refresh])]
    refresh_notice: nwg::Notice,

    // Tabs
    #[nwg_control(parent: window)]
    #[nwg_layout_item(layout: window_layout)]
    tabs_container: nwg::TabsContainer,

    // Connected devices tab
    #[nwg_control(parent: tabs_container, text: "Connected")]
    connected_tab: nwg::Tab,

    #[nwg_partial(parent: connected_tab)]
    connected_tab_content: ConnectedTab,

    // Persisted devices tab
    #[nwg_control(parent: tabs_container, text: "Persisted")]
    persisted_tab: nwg::Tab,

    #[nwg_partial(parent: persisted_tab)]
    persisted_tab_content: PersistedTab,

    #[nwg_control(parent: tabs_container, text: "Auto Attach")]
    auto_attach_tab: nwg::Tab,

    #[nwg_partial(parent: auto_attach_tab)]
    auto_attach_tab_content: AutoAttachTab,

    // Tray icon
    #[nwg_control(icon: Some(&data.app_icon), tip: Some("WSL USB Manager"))]
    #[nwg_events(OnContextMenu: [UsbipdGui::show_menu_tray], MousePressLeftUp: [UsbipdGui::show])]
    tray: nwg::TrayNotification,

    // File menu
    #[nwg_control(parent: window, text: "File", popup: false)]
    menu_file: nwg::Menu,

    #[nwg_control(parent: menu_file, text: "Refresh")]
    #[nwg_events(OnMenuItemSelected: [UsbipdGui::refresh])]
    menu_file_refresh: nwg::MenuItem,

    #[nwg_control(parent: menu_file)]
    menu_file_sep1: nwg::MenuSeparator,

    #[nwg_control(parent: menu_file, text: "Exit")]
    #[nwg_events(OnMenuItemSelected: [UsbipdGui::exit()])]
    menu_file_exit: nwg::MenuItem,
}

impl UsbipdGui {
    pub fn new(auto_attacher: &Rc<RefCell<AutoAttacher>>) -> Self {
        Self {
            connected_tab_content: ConnectedTab::new(auto_attacher),
            auto_attach_tab_content: AutoAttachTab::new(auto_attacher),
            ..Default::default()
        }
    }

    fn init(&self) {
        self.connected_tab_content.init(&self.window);
        self.persisted_tab_content.init(&self.window);
        self.auto_attach_tab_content.init(&self.window);

        // Give the connected tab a way to notify the auto attach tab that it needs to refresh
        self.connected_tab_content
            .auto_attach_notice
            .set(Some(self.auto_attach_tab_content.refresh_notice.sender()));

        let sender = self.refresh_notice.sender();
        self.device_notification.set(
            win_utils::register_usb_device_notifications(move || {
                sender.notice();
            })
            .expect("Failed to register USB device notifications"),
        );
    }

    fn min_max_info(data: &nwg::EventData) {
        if let nwg::EventData::OnMinMaxInfo(info) = data {
            info.set_min_size(600, 410);
        }
    }

    fn hide(&self, data: &nwg::EventData) {
        if let nwg::EventData::OnWindowClose(close_data) = data {
            close_data.close(false);
        }
        self.window.set_visible(false);
    }

    fn show(&self) {
        self.window.set_visible(true);
    }

    fn show_menu_tray(self: &Rc<UsbipdGui>) {
        // This prevents a memory leak in which the event handler closure is
        // kept alive after the menu is destroyed. An attempt was made to unbind
        // from the OnMenuExit event, but it seems to prevent the menu event
        // handlers from running at all.
        if let Some(handler) = self.menu_tray_event_handler.take() {
            nwg::unbind_event_handler(&handler);
        }

        let mut menu_tray = nwg::Menu::default();
        nwg::Menu::builder()
            .popup(true)
            .parent(self.window.handle)
            .build(&mut menu_tray)
            .unwrap();

        let devices = list_devices()
            .into_iter()
            .filter(|d| d.is_connected())
            .collect::<Vec<_>>();

        let mut menu_items: Vec<(nwg::MenuItem, UsbDevice)> = Vec::with_capacity(devices.len());
        for device in devices {
            let device_name = device.description.as_deref();
            let vid_pid = device.vid_pid();
            let description = device_name.map(|s| s.to_string()).unwrap_or(
                vid_pid
                    .clone()
                    .unwrap_or_else(|| "Unknown Device".to_string()),
            );

            if device.is_bound() {
                let menu_item = self
                    .new_menu_item(menu_tray.handle, &description, false, device.is_attached())
                    .unwrap();

                menu_items.push((menu_item, device));
            }
        }

        if menu_items.is_empty() {
            self.new_menu_item(menu_tray.handle, "No bound devices", true, false)
                .unwrap();
        };

        self.new_menu_separator(menu_tray.handle).unwrap();
        let open_item = self
            .new_menu_item(menu_tray.handle, "Open", false, false)
            .unwrap();
        self.new_menu_separator(menu_tray.handle).unwrap();
        let exit_item = self
            .new_menu_item(menu_tray.handle, "Exit", false, false)
            .unwrap();

        let rc_self_weak = Rc::downgrade(self);
        let handler =
            nwg::full_bind_event_handler(&self.window.handle, move |evt, _evt_data, handle| {
                // Ignore events that are not menu item selections
                if evt != nwg::Event::OnMenuItemSelected {
                    return;
                }

                // Retrieve the GUI instance
                let Some(rc_self) = rc_self_weak.upgrade() else {
                    return;
                };

                // Handle the menu item selection
                if handle == open_item.handle {
                    // The open menu item was selected
                    UsbipdGui::show(rc_self.as_ref());
                } else if handle == exit_item.handle {
                    // The exit menu item was selected
                    UsbipdGui::exit();
                } else {
                    // A device menu item was selected
                    let Some(device) = menu_items
                        .iter()
                        .find(|(item, _)| item.handle == handle)
                        .map(|(_, d)| d)
                    else {
                        return;
                    };

                    if device.is_attached() {
                        // Silently ignore errors here as the device may have been unplugged
                        device.detach().ok();
                    } else {
                        // TODO: this currently blocks the UI
                        device.attach().unwrap_or_else(|err| {
                            nwg::modal_error_message(
                                rc_self.window.handle,
                                "WSL USB Manager: Command Error",
                                &err,
                            );
                        });
                    }
                }
            });
        self.menu_tray_event_handler.set(Some(handler));

        let (x, y) = nwg::GlobalCursor::position();
        menu_tray.popup(x, y);
    }

    fn new_menu_item(
        &self,
        parent: nwg::ControlHandle,
        text: &str,
        disabled: bool,
        check: bool,
    ) -> Result<nwg::MenuItem, nwg::NwgError> {
        let mut menu_item = nwg::MenuItem::default();
        nwg::MenuItem::builder()
            .text(text)
            .disabled(disabled)
            .parent(parent)
            .check(check)
            .build(&mut menu_item)
            .map(|_| menu_item)
    }

    fn new_menu_separator(
        &self,
        parent: nwg::ControlHandle,
    ) -> Result<nwg::MenuSeparator, nwg::NwgError> {
        let mut sep = nwg::MenuSeparator::default();
        nwg::MenuSeparator::builder()
            .parent(parent)
            .build(&mut sep)
            .map(|_| sep)
    }

    fn refresh(&self) {
        self.connected_tab_content.refresh();
        self.persisted_tab_content.refresh();
        self.auto_attach_tab_content.refresh();
    }

    fn exit() {
        nwg::stop_thread_dispatch();
    }
}
