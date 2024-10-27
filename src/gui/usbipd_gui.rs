use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use native_windows_derive::NwgUi;
use native_windows_gui as nwg;

use super::auto_attach_tab::AutoAttachTab;
use super::connected_tab::ConnectedTab;
use super::persisted_tab::PersistedTab;
use crate::usbipd::UsbDevice;
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
    menu_tray_event_handler: RefCell<Option<nwg::EventHandler>>,

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
    #[nwg_events(OnContextMenu: [UsbipdGui::show_menu_tray(RC_SELF)], MousePressLeftUp: [UsbipdGui::show(RC_SELF)])]
    tray: nwg::TrayNotification,

    // File menu
    #[nwg_control(parent: window, text: "File", popup: false)]
    menu_file: nwg::Menu,

    #[nwg_control(parent: menu_file, text: "Refresh")]
    #[nwg_events(OnMenuItemSelected: [UsbipdGui::refresh(RC_SELF)])]
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

    fn show(self: &Rc<UsbipdGui>) {
        self.window.set_visible(true);
    }

    fn show_menu_tray(self: &Rc<UsbipdGui>) {
        if let Some(handler) = self.menu_tray_event_handler.borrow().as_ref() {
            nwg::unbind_event_handler(handler);
        }

        let mut menu_tray = nwg::Menu::default();
        nwg::Menu::builder()
            .popup(true)
            .parent(self.window.handle)
            .build(&mut menu_tray)
            .unwrap();

        let devices = self
            .connected_tab_content
            .connected_devices
            .borrow()
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        let mut menu_items: Vec<(nwg::MenuItem, Rc<UsbDevice>)> = Vec::with_capacity(devices.len());
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
                    .new_menu_item(menu_tray.handle, &description, device.is_attached())
                    .unwrap();

                menu_items.push((menu_item, Rc::new(device.clone())));
            }
        }

        self.new_menu_separator(menu_tray.handle).unwrap();
        let open_item = self.new_menu_item(menu_tray.handle, "Open", false).unwrap();
        self.new_menu_separator(menu_tray.handle).unwrap();
        let exit_item = self.new_menu_item(menu_tray.handle, "Exit", false).unwrap();

        let rc_self_weak = Rc::downgrade(&self);
        *self.menu_tray_event_handler.borrow_mut() = Some(nwg::full_bind_event_handler(
            &self.window.handle,
            move |evt, _evt_data, handle| {
                if let Some(rc_self) = rc_self_weak.upgrade() {
                    match evt {
                        nwg::Event::OnMenuItemSelected => {
                            if handle == open_item.handle {
                                UsbipdGui::show(&rc_self);
                            } else if handle == exit_item.handle {
                                UsbipdGui::exit();
                            } else {
                                for (menu_item, device) in menu_items.iter() {
                                    if handle == menu_item.handle {
                                        if device.is_attached() {
                                            device.detach().ok();
                                        } else {
                                            device.attach().ok();
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
            },
        ));

        let (x, y) = nwg::GlobalCursor::position();
        menu_tray.popup(x, y);
    }

    fn new_menu_item(
        self: &Rc<UsbipdGui>,
        parent: nwg::ControlHandle,
        text: &str,
        check: bool,
    ) -> Result<nwg::MenuItem, nwg::NwgError> {
        let mut menu_item = nwg::MenuItem::default();
        nwg::MenuItem::builder()
            .text(text)
            .parent(parent)
            .check(check)
            .build(&mut menu_item)
            .map(|_| menu_item)
    }

    fn new_menu_separator(
        self: &Rc<UsbipdGui>,
        parent: nwg::ControlHandle,
    ) -> Result<nwg::MenuSeparator, nwg::NwgError> {
        let mut sep = nwg::MenuSeparator::default();
        nwg::MenuSeparator::builder()
            .parent(parent)
            .build(&mut sep)
            .map(|_| sep)
    }

    fn refresh(self: &Rc<UsbipdGui>) {
        self.connected_tab_content.refresh();
        self.persisted_tab_content.refresh();
        self.auto_attach_tab_content.refresh();
    }

    fn exit() {
        nwg::stop_thread_dispatch();
    }
}
