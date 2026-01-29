mod device_info;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use native_windows_derive::NwgPartial;
use native_windows_gui as nwg;
use nwg::stretch::{
    geometry::{Rect, Size},
    style::{Dimension as D, FlexDirection},
};
use windows_sys::Win32::UI::Controls::LVSCW_AUTOSIZE;
use windows_sys::Win32::UI::Controls::LVSCW_AUTOSIZE_USEHEADER;
use windows_sys::Win32::UI::Shell::SIID_SHIELD;

use self::device_info::DeviceInfo;
use crate::auto_attach::AutoAttacher;
use crate::gui::{
    nwg_ext::{BitmapEx, MenuItemEx},
    usbipd_gui::GuiTab,
};
use crate::usbipd::{self, UsbDevice, UsbipState};

const PADDING_LEFT: Rect<D> = Rect {
    start: D::Points(8.0),
    end: D::Points(0.0),
    top: D::Points(0.0),
    bottom: D::Points(0.0),
};

const DETAILS_PANEL_WIDTH: f32 = 285.0;
const DETAILS_PANEL_PADDING: u32 = 4;

#[derive(Default, NwgPartial)]
pub struct ConnectedTab {
    auto_attacher: Rc<RefCell<AutoAttacher>>,

    window: Cell<nwg::ControlHandle>,
    shield_bitmap: Cell<nwg::Bitmap>,

    /// A notice sender to notify the auto attach tab to refresh
    pub auto_attach_notice: Cell<Option<nwg::NoticeSender>>,

    connected_devices: RefCell<Vec<usbipd::UsbDevice>>,

    #[nwg_layout(flex_direction: FlexDirection::Row)]
    connected_tab_layout: nwg::FlexboxLayout,

    #[nwg_control(list_style: nwg::ListViewStyle::Detailed, focus: true,
        flags: "VISIBLE|SINGLE_SELECTION|TAB_STOP",
        ex_flags: nwg::ListViewExFlags::FULL_ROW_SELECT,
    )]
    #[nwg_events(OnListViewRightClick: [ConnectedTab::show_menu],
        OnListViewItemChanged: [ConnectedTab::update_device_details]
    )]
    #[nwg_layout_item(layout: connected_tab_layout, flex_grow: 1.0)]
    list_view: nwg::ListView,

    // Device info
    #[nwg_control]
    #[nwg_layout_item(layout: connected_tab_layout, margin: PADDING_LEFT,
        size: Size { width: D::Points(DETAILS_PANEL_WIDTH), height: D::Auto },
    )]
    details_frame: nwg::Frame,

    #[nwg_layout(parent: details_frame, flex_direction: FlexDirection::Column,
        auto_spacing: Some(DETAILS_PANEL_PADDING))]
    details_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: details_frame, flags: "VISIBLE")]
    #[nwg_layout_item(layout: details_layout, flex_grow: 1.0)]
    // Multi-line RichLabels send a WM_CLOSE message when the ESC key is pressed
    #[nwg_events(OnWindowClose: [ConnectedTab::inhibit_close(EVT_DATA)])]
    device_info_frame: nwg::Frame,

    #[nwg_partial(parent: device_info_frame)]
    device_info: DeviceInfo,

    // Buttons
    #[nwg_control(parent: details_frame, flags: "VISIBLE")]
    #[nwg_layout_item(layout: details_layout, size: Size { width: D::Auto, height: D::Points(25.0) })]
    buttons_frame: nwg::Frame,

    #[nwg_layout(parent: buttons_frame, flex_direction: FlexDirection::RowReverse, auto_spacing: None)]
    buttons_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: buttons_frame, text: "Attach")]
    #[nwg_layout_item(layout: buttons_layout, flex_grow: 0.33)]
    #[nwg_events(OnButtonClick: [ConnectedTab::attach_detach_device])]
    attach_detach_button: nwg::Button,

    #[nwg_control(parent: buttons_frame, text: "Bind")]
    #[nwg_layout_item(layout: buttons_layout, flex_grow: 0.33)]
    #[nwg_events(OnButtonClick: [ConnectedTab::bind_unbind_device])]
    bind_unbind_button: nwg::Button,

    #[nwg_control(parent: buttons_frame, text: "Auto Attach")]
    #[nwg_layout_item(layout: buttons_layout, flex_grow: 0.33)]
    #[nwg_events(OnButtonClick: [ConnectedTab::auto_attach_device])]
    auto_attach_button: nwg::Button,

    // Device context menu
    #[nwg_control(text: "Device", popup: true)]
    menu: nwg::Menu,

    #[nwg_control(parent: menu, text: "Attach")]
    #[nwg_events(OnMenuItemSelected: [ConnectedTab::attach_device])]
    menu_attach: nwg::MenuItem,

    #[nwg_control(parent: menu, text: "Detach")]
    #[nwg_events(OnMenuItemSelected: [ConnectedTab::detach_device])]
    menu_detach: nwg::MenuItem,

    #[nwg_control(parent: menu)]
    menu_sep: nwg::MenuSeparator,

    #[nwg_control(parent: menu, text: "Bind")]
    #[nwg_events(OnMenuItemSelected: [ConnectedTab::bind_device])]
    menu_bind: nwg::MenuItem,

    #[nwg_control(parent: menu, text: "Bind (force)")]
    #[nwg_events(OnMenuItemSelected: [ConnectedTab::bind_device_force])]
    menu_bind_force: nwg::MenuItem,

    #[nwg_control(parent: menu, text: "Unbind")]
    #[nwg_events(OnMenuItemSelected: [ConnectedTab::unbind_device])]
    menu_unbind: nwg::MenuItem,
}

impl ConnectedTab {
    pub fn new(auto_attacher: &Rc<RefCell<AutoAttacher>>) -> Self {
        Self {
            auto_attacher: auto_attacher.clone(),
            ..Default::default()
        }
    }

    fn init_list(&self) {
        let list = &self.list_view;
        list.clear();
        list.insert_column("Bus ID");
        list.insert_column("State");
        list.insert_column("Device");
        list.set_headers_enabled(true);

        // Insert a dummy row, with max-width content in columns where AUTOSIZE is used
        list.insert_items_row(None, &["-", &format!("{}  ", UsbipState::None), "Device"]);

        list.set_column_width(0, LVSCW_AUTOSIZE_USEHEADER as isize);
        list.set_column_width(1, LVSCW_AUTOSIZE as isize);
        list.set_column_width(2, LVSCW_AUTOSIZE_USEHEADER as isize);

        // Clear the dummy row
        list.clear();
    }

    /// Clears the device list and reloads it with the currently connected devices.
    fn refresh_list(&self) {
        self.update_devices();

        self.list_view.clear();
        for device in self.connected_devices.borrow().iter() {
            self.list_view.insert_items_row(
                None,
                &[
                    device.bus_id.as_deref().unwrap_or("-"),
                    &device.state().to_string(),
                    device.description.as_deref().unwrap_or("Unknown device"),
                ],
            );
        }
    }

    /// Refreshes the device list using the provided devices.
    fn refresh_list_with_devices(&self, devices: &[usbipd::UsbDevice]) {
        *self.connected_devices.borrow_mut() = devices
            .iter()
            .filter(|d| d.is_connected())
            .cloned()
            .collect();

        self.list_view.clear();
        for device in self.connected_devices.borrow().iter() {
            self.list_view.insert_items_row(
                None,
                &[
                    device.bus_id.as_deref().unwrap_or("-"),
                    &device.state().to_string(),
                    device.description.as_deref().unwrap_or("Unknown device"),
                ],
            );
        }
    }

    /// Updates the device details panel with the currently selected device.
    fn update_device_details(&self) {
        let devices = self.connected_devices.borrow();
        let device = self.list_view.selected_item().and_then(|i| devices.get(i));

        self.device_info.update(device);

        // Update buttons
        if let Some(device) = device {
            if device.is_bound() {
                self.bind_unbind_button.set_text("Unbind");
                self.auto_attach_button.set_enabled(true);

                // Attaching a bound device doesn't require admin privileges, hide the UAC shield icon
                self.attach_detach_button.set_bitmap(None);
            } else {
                self.bind_unbind_button.set_text("Bind");
                self.auto_attach_button.set_enabled(false);

                // Attaching an unbound device requires admin privileges, show the UAC shield icon
                let shield_bitmap = self.shield_bitmap.take();
                self.attach_detach_button.set_bitmap(Some(&shield_bitmap));
                self.shield_bitmap.set(shield_bitmap);
            }

            if device.is_attached() {
                self.attach_detach_button.set_text("Detach");
            } else {
                self.attach_detach_button.set_text("Attach");
            }

            self.bind_unbind_button.set_enabled(true);
            self.attach_detach_button.set_enabled(true);
        } else {
            self.attach_detach_button.set_text("Attach");
            self.bind_unbind_button.set_text("Bind");
            self.attach_detach_button.set_bitmap(None);

            self.auto_attach_button.set_enabled(false);
            self.bind_unbind_button.set_enabled(false);
            self.attach_detach_button.set_enabled(false);
        }
    }

    fn show_menu(&self) {
        let selected_index = match self.list_view.selected_item() {
            Some(index) => index,
            None => return,
        };
        let devices = self.connected_devices.borrow();
        let device = devices.get(selected_index).unwrap();

        if device.is_attached() {
            self.menu_detach.set_enabled(true);
            self.menu_attach.set_enabled(false);
        } else {
            self.menu_detach.set_enabled(false);
            self.menu_attach.set_enabled(true);
        }

        if device.is_bound() {
            self.menu_bind.set_enabled(false);
            self.menu_bind_force.set_enabled(false);
            self.menu_unbind.set_enabled(true);

            // Attaching a bound device doesn't require admin privileges, hide the UAC shield icon
            self.menu_attach.set_bitmap(None);
        } else {
            self.menu_bind.set_enabled(true);
            self.menu_bind_force.set_enabled(true);
            self.menu_unbind.set_enabled(false);

            // Attaching an unbound device requires admin privileges, show the UAC shield icon
            let shield_bitmap = self.shield_bitmap.take();
            self.menu_attach.set_bitmap(Some(&shield_bitmap));
            self.shield_bitmap.set(shield_bitmap);
        }

        let (x, y) = nwg::GlobalCursor::position();
        // Disable menu animations because they cause incorrect rendering of the bitmaps
        self.menu
            .popup_with_flags(x, y, nwg::PopupMenuFlags::ANIMATE_NONE);
    }

    fn bind_device(&self) {
        self.run_command(|device| {
            device.bind(false)?;
            device.wait(|d| d.is_some_and(|d| d.is_bound()))
        });
    }

    fn bind_device_force(&self) {
        self.run_command(|device| {
            device.bind(true)?;
            device.wait(|d| d.is_some_and(|d| d.is_bound() && d.is_forced))
        });
    }

    fn unbind_device(&self) {
        self.run_command(|device| {
            device.unbind()?;
            device.wait(|d| d.is_some_and(|d| !d.is_bound()))
        });
    }

    fn attach_device(&self) {
        self.run_command(|device| {
            device.attach()?;
            device.wait(|d| d.is_some_and(|d| d.is_attached()))
        });
    }

    fn detach_device(&self) {
        self.run_command(|device| {
            device.detach()?;
            device.wait(|d| d.is_some_and(|d| d.is_attached()))
        });
    }

    fn attach_detach_device(&self) {
        self.run_command(|device| {
            if !device.is_attached() {
                device.attach()?;
                device.wait(|d| d.is_some_and(|d| d.is_attached()))
            } else {
                device.detach()?;
                device.wait(|d| d.is_some_and(|d| !d.is_attached()))
            }
        });
    }

    fn bind_unbind_device(&self) {
        self.run_command(|device| {
            if !device.is_bound() {
                device.bind(false)?;
                device.wait(|d| d.is_some_and(|d| d.is_bound()))
            } else {
                device.unbind()?;
                device.wait(|d| d.is_some_and(|d| !d.is_bound()))
            }
        });
    }

    fn auto_attach_device(&self) {
        self.run_command(|device| {
            self.auto_attacher.borrow_mut().add_device(device)?;

            let auto_attach_notice = self.auto_attach_notice.get().unwrap();
            auto_attach_notice.notice();
            self.auto_attach_notice.set(Some(auto_attach_notice));

            Ok(())
        });
    }

    /// Runs a `command` function on the currently selected device.
    /// No-op if no device is selected.
    ///
    /// If the command completes successfully, the view is reloaded.
    ///
    /// If an error occurs, an error dialog is shown.
    fn run_command(&self, command: impl Fn(&UsbDevice) -> Result<(), String>) {
        let window = self.window.get();

        let wait_cursor = nwg::Cursor::from_system(nwg::OemCursor::Wait);
        let cursor_event =
            nwg::full_bind_event_handler(&window, move |event, _event_data, _handle| match event {
                nwg::Event::OnMousePress(_) | nwg::Event::OnMouseMove => {
                    nwg::GlobalCursor::set(&wait_cursor)
                }
                _ => {}
            });

        let result = {
            let selected_index = match self.list_view.selected_item() {
                Some(index) => index,
                None => return,
            };
            // Borrow devices in a scoped block so that the ref is released as soon as possible
            let devices = self.connected_devices.borrow();
            let device = match devices.get(selected_index) {
                Some(device) => device,
                None => return,
            };

            command(device)
        };

        if let Err(err) = result {
            nwg::modal_error_message(window, "WSL USB Manager: Command Error", &err);
        }

        self.window.set(window);
        self.refresh();
        nwg::unbind_event_handler(&cursor_event);
    }

    fn update_devices(&self) {
        *self.connected_devices.borrow_mut() = usbipd::list_devices()
            .into_iter()
            .filter(|d| d.is_connected())
            .collect();
    }

    /// Inhibits the window close event.
    fn inhibit_close(data: &nwg::EventData) {
        if let nwg::EventData::OnWindowClose(close_data) = data {
            close_data.close(false);
        }
    }
}

impl GuiTab for ConnectedTab {
    fn init(&self, window: &nwg::Window) {
        self.window.replace(window.handle);

        let shield_bitmap = nwg::Bitmap::from_system_icon(SIID_SHIELD);

        // Set the UAC shield icon for menu items and buttons that always require admin privileges
        self.menu_bind.set_bitmap(Some(&shield_bitmap));
        self.menu_bind_force.set_bitmap(Some(&shield_bitmap));
        self.menu_unbind.set_bitmap(Some(&shield_bitmap));
        self.bind_unbind_button.set_bitmap(Some(&shield_bitmap));

        self.shield_bitmap.set(shield_bitmap);

        self.init_list();
        self.refresh();
    }

    fn refresh(&self) {
        self.refresh_list();
        self.update_device_details();
    }

    fn refresh_with_devices(&self, devices: &[usbipd::UsbDevice]) {
        self.refresh_list_with_devices(devices);
        self.update_device_details();
    }
}
