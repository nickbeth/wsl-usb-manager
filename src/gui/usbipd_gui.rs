use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    ops::Deref,
    rc::Rc,
};

use native_windows_gui as nwg;
use nwg::{NativeUi, PartialUi};

use super::auto_attach_tab::AutoAttachTab;
use super::connected_tab::ConnectedTab;
use super::persisted_tab::PersistedTab;
use crate::{
    auto_attach::AutoAttacher,
    gui::helpers,
    win_utils::{self, DeviceNotification},
};
use crate::{
    gui::RESOURCES,
    usbipd::{UsbDevice, list_devices},
};

pub(super) trait GuiTab {
    /// Initializes the tab. The root window handle is provided.
    fn init(&self, window: &Rc<nwg::Window>);

    /// Refreshes the data displayed in the tab.
    fn refresh(&self);
}

#[derive(Default)]
pub struct UsbipdGui {
    device_notification: Cell<DeviceNotification>,
    menu_tray_event_handler: Cell<Option<nwg::EventHandler>>,
    start_minimized: bool,

    // Window
    window: Rc<nwg::Window>,
    window_layout: nwg::FlexboxLayout,
    refresh_notice: nwg::Notice,

    // Tabs
    tabs_container: nwg::TabsContainer,
    connected_tab: nwg::Tab,
    connected_tab_content: ConnectedTab,
    persisted_tab: nwg::Tab,
    persisted_tab_content: PersistedTab,
    auto_attach_tab: nwg::Tab,
    auto_attach_tab_content: AutoAttachTab,

    // Tray icon
    tray: nwg::TrayNotification,

    // File menu
    menu_file: nwg::Menu,
    menu_file_refresh: nwg::MenuItem,
    menu_file_sep1: nwg::MenuSeparator,
    menu_file_exit: nwg::MenuItem,
}

impl UsbipdGui {
    pub fn new(auto_attacher: &Rc<RefCell<AutoAttacher>>, start_minimized: bool) -> Self {
        Self {
            connected_tab_content: ConnectedTab::new(auto_attacher),
            auto_attach_tab_content: AutoAttachTab::new(auto_attacher),
            start_minimized,
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

        // Window is initialized as invisible (because of nwg limitations)
        // Show it if we're not starting minimized
        if !self.start_minimized {
            self.window.set_visible(true);
        }
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
            // Only show bound devices
            if !device.is_bound() {
                continue;
            }

            // Get device name or fallback to "Unknown Device (bus_id)"
            let device_name = device
                .description
                .as_deref()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| {
                    let bus_id = device.bus_id.as_deref().unwrap_or("-");
                    Cow::Owned(format!("Unknown Device ({})", bus_id))
                });
            let name = device_name.as_ref();

            // Truncate long device names
            const MAX_LENGTH: usize = 30;
            let description = helpers::ellipsize_middle(name, MAX_LENGTH);

            let menu_item = self
                .new_menu_item(menu_tray.handle, &description, false, device.is_attached())
                .unwrap();
            menu_items.push((menu_item, device));
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
        let devices = list_devices();
        self.connected_tab_content.refresh_with_devices(&devices);
        self.persisted_tab_content.refresh_with_devices(&devices);
        self.auto_attach_tab_content.refresh();
    }

    fn exit() {
        nwg::stop_thread_dispatch();
    }
}

pub struct UsbipdGuiUi {
    inner: Rc<UsbipdGui>,
    default_handler: nwg::EventHandler,
}

impl NativeUi<UsbipdGuiUi> for UsbipdGui {
    fn build_ui(mut data: Self) -> Result<UsbipdGuiUi, nwg::NwgError> {
        // Controls (parent-first order)
        nwg::Window::builder()
            .flags(nwg::WindowFlags::MAIN_WINDOW)
            .size((780, 430))
            .center(true)
            .title("WSL USB Manager")
            .icon(Some(&RESOURCES.app_icon))
            .build(Rc::get_mut(&mut data.window).unwrap())?;

        nwg::Notice::builder()
            .parent(&*data.window)
            .build(&mut data.refresh_notice)?;

        nwg::TabsContainer::builder()
            .parent(&*data.window)
            .build(&mut data.tabs_container)?;

        nwg::Tab::builder()
            .parent(&data.tabs_container)
            .text("Connected")
            .build(&mut data.connected_tab)?;

        nwg::Tab::builder()
            .parent(&data.tabs_container)
            .text("Persisted")
            .build(&mut data.persisted_tab)?;

        nwg::Tab::builder()
            .parent(&data.tabs_container)
            .text("Auto Attach")
            .build(&mut data.auto_attach_tab)?;

        nwg::TrayNotification::builder()
            .parent(&*data.window)
            .icon(Some(&RESOURCES.app_icon))
            .tip(Some("WSL USB Manager"))
            .build(&mut data.tray)?;

        nwg::Menu::builder()
            .parent(&*data.window)
            .text("File")
            .popup(false)
            .build(&mut data.menu_file)?;

        nwg::MenuItem::builder()
            .parent(&data.menu_file)
            .text("Refresh")
            .build(&mut data.menu_file_refresh)?;

        nwg::MenuSeparator::builder()
            .parent(&data.menu_file)
            .build(&mut data.menu_file_sep1)?;

        nwg::MenuItem::builder()
            .parent(&data.menu_file)
            .text("Exit")
            .build(&mut data.menu_file_exit)?;

        // Build partials
        ConnectedTab::build_partial(&mut data.connected_tab_content, Some(&data.connected_tab))?;
        PersistedTab::build_partial(&mut data.persisted_tab_content, Some(&data.persisted_tab))?;
        AutoAttachTab::build_partial(
            &mut data.auto_attach_tab_content,
            Some(&data.auto_attach_tab),
        )?;

        // Wrap in Rc
        let inner = Rc::new(data);
        // Bind events
        let evt_ui = Rc::downgrade(&inner);

        let window_handle = inner.window.handle;
        let default_handler =
            nwg::full_bind_event_handler(&window_handle, move |evt, evt_data, handle| {
                if let Some(ui) = evt_ui.upgrade() {
                    match evt {
                        nwg::Event::OnInit => {
                            if handle == ui.window.handle {
                                UsbipdGui::init(&ui);
                            }
                        }
                        nwg::Event::OnMinMaxInfo => {
                            if handle == ui.window.handle {
                                UsbipdGui::min_max_info(&evt_data);
                            }
                        }
                        nwg::Event::OnWindowClose => {
                            if handle == ui.window.handle {
                                UsbipdGui::hide(&ui, &evt_data);
                            }
                        }
                        nwg::Event::OnNotice => {
                            if handle == ui.refresh_notice.handle {
                                UsbipdGui::refresh(&ui);
                            }
                        }
                        nwg::Event::OnContextMenu => {
                            if handle == ui.tray.handle {
                                UsbipdGui::show_menu_tray(&ui);
                            }
                        }
                        nwg::Event::OnMousePress(nwg::MousePressEvent::MousePressLeftUp) => {
                            if handle == ui.tray.handle {
                                UsbipdGui::show(&ui);
                            }
                        }
                        nwg::Event::OnMenuItemSelected => {
                            if handle == ui.menu_file_refresh.handle {
                                UsbipdGui::refresh(&ui);
                            }
                            if handle == ui.menu_file_exit.handle {
                                UsbipdGui::exit();
                            }
                        }
                        _ => {}
                    }

                    // Forward events to partials
                    ui.connected_tab_content
                        .process_event(evt, &evt_data, handle);
                    ui.persisted_tab_content
                        .process_event(evt, &evt_data, handle);
                    ui.auto_attach_tab_content
                        .process_event(evt, &evt_data, handle);
                }
            });

        let ui = UsbipdGuiUi {
            inner,
            default_handler,
        };

        // Build layouts
        nwg::FlexboxLayout::builder()
            .parent(&*ui.window)
            .auto_spacing(Some(2))
            .child(&ui.tabs_container)
            .build(&ui.window_layout)?;

        Ok(ui)
    }
}

impl Drop for UsbipdGuiUi {
    fn drop(&mut self) {
        nwg::unbind_event_handler(&self.default_handler);
    }
}

impl Deref for UsbipdGuiUi {
    type Target = UsbipdGui;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
