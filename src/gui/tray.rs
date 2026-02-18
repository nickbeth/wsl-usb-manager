use std::{
    borrow::Cow,
    cell::{Cell, OnceCell, RefCell},
    ops::Deref,
    rc::Rc,
};

use native_windows_gui::{self as nwg, NwgError};
use nwg::NativeUi;

use crate::{
    auto_attach::AutoAttacher,
    gui::{
        RESOURCES, helpers,
        main_window::{MainWindow, MainWindowUi},
    },
    usbipd::{self, UsbDevice},
};

#[derive(Default)]
pub struct Tray {
    auto_attacher: Rc<RefCell<AutoAttacher>>,

    window: nwg::MessageWindow,
    menu_tray_event_handler: Cell<Option<nwg::EventHandler>>,
    tray: nwg::TrayNotification,

    // Main window storage
    main_window: OnceCell<MainWindowUi>,
}

impl Tray {
    pub fn new(auto_attacher: &Rc<RefCell<AutoAttacher>>) -> Self {
        Self {
            auto_attacher: auto_attacher.clone(),
            ..Default::default()
        }
    }

    fn open(&self) {
        let main_window = self.main_window.get_or_init(|| {
            MainWindow::build_ui(MainWindow::new(&self.auto_attacher))
                .expect("Failed to create main window")
        });

        main_window.open();
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }

    fn show_menu(self: &Rc<Self>) {
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

        let devices = usbipd::list_devices()
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
                    rc_self.open();
                } else if handle == exit_item.handle {
                    // The exit menu item was selected
                    rc_self.exit();
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
}

pub struct TrayUi {
    inner: Rc<Tray>,
    default_handler: nwg::EventHandler,
}

impl Tray {
    pub fn build_ui(mut data: Tray, start_minimized: bool) -> Result<TrayUi, NwgError> {
        nwg::MessageWindow::builder()
            .build(&mut data.window)
            .expect("Failed to create message window");

        nwg::TrayNotification::builder()
            .parent(&data.window)
            .icon(Some(&RESOURCES.app_icon))
            .tip(Some("WSL USB Manager"))
            .build(&mut data.tray)?;

        // Wrap in Rc
        let inner = Rc::new(data);

        // Show main window
        if !start_minimized {
            inner.open();
        }

        // Bind events
        let evt_ui = Rc::downgrade(&inner);

        let window_handle = inner.window.handle;
        let default_handler =
            nwg::full_bind_event_handler(&window_handle, move |evt, _evt_data, handle| {
                if let Some(ui) = evt_ui.upgrade() {
                    match evt {
                        nwg::Event::OnContextMenu => {
                            if handle == ui.tray.handle {
                                Tray::show_menu(&ui);
                            }
                        }
                        nwg::Event::OnMousePress(nwg::MousePressEvent::MousePressLeftUp) => {
                            if handle == ui.tray.handle {
                                Tray::open(&ui);
                            }
                        }
                        _ => {}
                    }
                }
            });

        let ui = TrayUi {
            inner,
            default_handler,
        };

        Ok(ui)
    }
}

impl Drop for TrayUi {
    fn drop(&mut self) {
        nwg::unbind_event_handler(&self.default_handler);
    }
}

impl Deref for TrayUi {
    type Target = Tray;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
