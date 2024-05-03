use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use native_windows_derive::NwgUi;
use native_windows_gui as nwg;

use super::auto_attach_tab::AutoAttachTab;
use super::connected_tab::ConnectedTab;
use super::persisted_tab::PersistedTab;
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
    #[nwg_events(OnContextMenu: [UsbipdGui::show_tray_menu], MousePressLeftUp: [UsbipdGui::show])]
    tray: nwg::TrayNotification,

    // Tray menu
    #[nwg_control(parent: window, popup: true)]
    menu_tray: nwg::Menu,

    #[nwg_control(parent: menu_tray, text: "Open")]
    #[nwg_events(OnMenuItemSelected: [UsbipdGui::show])]
    menu_tray_open: nwg::MenuItem,

    #[nwg_control(parent: menu_tray)]
    menu_tray_sep: nwg::MenuSeparator,

    #[nwg_control(parent: menu_tray, text: "Exit")]
    #[nwg_events(OnMenuItemSelected: [UsbipdGui::exit])]
    menu_tray_exit: nwg::MenuItem,

    // File menu
    #[nwg_control(parent: window, text: "File", popup: false)]
    menu_file: nwg::Menu,

    #[nwg_control(parent: menu_file, text: "Refresh")]
    #[nwg_events(OnMenuItemSelected: [UsbipdGui::refresh])]
    menu_file_refresh: nwg::MenuItem,

    #[nwg_control(parent: menu_file)]
    menu_file_sep1: nwg::MenuSeparator,

    #[nwg_control(parent: menu_file, text: "Exit")]
    #[nwg_events(OnMenuItemSelected: [UsbipdGui::exit])]
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

    fn show_tray_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.menu_tray.popup(x, y);
    }

    fn refresh(&self) {
        self.connected_tab_content.refresh();
        self.persisted_tab_content.refresh();
        self.auto_attach_tab_content.refresh();
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}
