use std::{
    cell::{Cell, RefCell},
    ops::Deref,
    rc::Rc,
};

use native_windows_gui as nwg;
use nwg::{NativeUi, PartialUi};

use super::auto_attach_tab::AutoAttachTab;
use super::connected_tab::ConnectedTab;
use super::nwg_ext::WindowEx;
use super::persisted_tab::PersistedTab;
use crate::{
    auto_attach::AutoAttacher,
    win_utils::{self, DeviceNotification},
};
use crate::{gui::RESOURCES, usbipd::list_devices};

pub(super) trait GuiTab {
    /// Initializes the tab. The root window handle is provided.
    fn init(&self, window: &Rc<nwg::Window>);

    /// Refreshes the data displayed in the tab.
    fn refresh(&self);
}

#[derive(Default)]
pub struct MainWindow {
    device_notification: Cell<DeviceNotification>,

    // Window
    pub window: Rc<nwg::Window>,
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

    // File menu
    menu_file: nwg::Menu,
    menu_file_refresh: nwg::MenuItem,
    menu_file_sep1: nwg::MenuSeparator,
    menu_file_exit: nwg::MenuItem,
}

impl MainWindow {
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

    pub fn open(&self) {
        self.window.set_visible(true);
        if self.window.is_minimized() {
            self.window.restore();
        }
        self.window.set_foreground();
    }

    pub fn close(&self) {
        self.window.set_visible(false);
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

pub struct MainWindowUi {
    inner: Rc<MainWindow>,
    default_handler: nwg::EventHandler,
}

impl NativeUi<MainWindowUi> for MainWindow {
    fn build_ui(mut data: Self) -> Result<MainWindowUi, nwg::NwgError> {
        // Controls (parent-first order)
        nwg::Window::builder()
            .flags(nwg::WindowFlags::MAIN_WINDOW | nwg::WindowFlags::VISIBLE)
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
                        nwg::Event::OnWindowClose => {
                            if handle == ui.window.handle {
                                if let nwg::EventData::OnWindowClose(close_data) = &evt_data {
                                    close_data.close(false);
                                }

                                MainWindow::close(&ui);
                            }
                        }

                        nwg::Event::OnInit => {
                            if handle == ui.window.handle {
                                MainWindow::init(&ui);
                            }
                        }
                        nwg::Event::OnMinMaxInfo => {
                            if handle == ui.window.handle {
                                MainWindow::min_max_info(&evt_data);
                            }
                        }
                        nwg::Event::OnNotice => {
                            if handle == ui.refresh_notice.handle {
                                MainWindow::refresh(&ui);
                            }
                        }
                        nwg::Event::OnMenuItemSelected => {
                            if handle == ui.menu_file_refresh.handle {
                                MainWindow::refresh(&ui);
                            }
                            if handle == ui.menu_file_exit.handle {
                                MainWindow::exit();
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

        let ui = MainWindowUi {
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

impl Drop for MainWindowUi {
    fn drop(&mut self) {
        nwg::unbind_event_handler(&self.default_handler);
    }
}

impl Deref for MainWindowUi {
    type Target = MainWindow;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
