mod persisted_info;

use std::cell::{Cell, RefCell};

use native_windows_derive::NwgPartial;
use native_windows_gui as nwg;
use nwg::stretch::{
    geometry::{Rect, Size},
    style::{Dimension as D, FlexDirection},
};
use windows_sys::Win32::UI::{Controls::LVSCW_AUTOSIZE_USEHEADER, Shell::SIID_SHIELD};

use self::persisted_info::PersistedInfo;
use crate::gui::{
    nwg_ext::{BitmapEx, MenuItemEx},
    usbipd_gui::GuiTab,
};
use crate::usbipd::{self, UsbDevice};

const PADDING_LEFT: Rect<D> = Rect {
    start: D::Points(8.0),
    end: D::Points(0.0),
    top: D::Points(0.0),
    bottom: D::Points(0.0),
};

const DETAILS_PANEL_WIDTH: f32 = 285.0;
const DETAILS_PANEL_PADDING: u32 = 4;

#[derive(Default, NwgPartial)]
pub struct PersistedTab {
    window: Cell<nwg::ControlHandle>,
    shield_bitmap: Cell<nwg::Bitmap>,

    persisted_devices: RefCell<Vec<usbipd::UsbDevice>>,

    #[nwg_layout(flex_direction: FlexDirection::Row)]
    persisted_tab_layout: nwg::FlexboxLayout,

    #[nwg_control(list_style: nwg::ListViewStyle::Detailed, focus: true,
        flags: "VISIBLE|SINGLE_SELECTION|TAB_STOP",
        ex_flags: nwg::ListViewExFlags::FULL_ROW_SELECT,
    )]
    #[nwg_events(OnListViewRightClick: [PersistedTab::show_menu],
        OnListViewItemChanged: [PersistedTab::update_persisted_details]
    )]
    #[nwg_layout_item(layout: persisted_tab_layout, flex_grow: 1.0)]
    list_view: nwg::ListView,

    // Persisted info
    #[nwg_control]
    #[nwg_layout_item(layout: persisted_tab_layout, margin: PADDING_LEFT,
        size: Size { width: D::Points(DETAILS_PANEL_WIDTH), height: D::Auto },
    )]
    details_frame: nwg::Frame,

    #[nwg_layout(parent: details_frame, flex_direction: FlexDirection::Column,
        auto_spacing: Some(DETAILS_PANEL_PADDING))]
    details_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: details_frame, flags: "VISIBLE")]
    #[nwg_layout_item(layout: details_layout, flex_grow: 1.0)]
    // Multi-line RichLabels send a WM_CLOSE message when the ESC key is pressed
    #[nwg_events(OnWindowClose: [PersistedTab::inhibit_close(EVT_DATA)])]
    persisted_info_frame: nwg::Frame,

    #[nwg_partial(parent: persisted_info_frame)]
    persisted_info: PersistedInfo,

    // Buttons
    #[nwg_control(parent: details_frame, flags: "VISIBLE")]
    #[nwg_layout_item(layout: details_layout, size: Size { width: D::Auto, height: D::Points(25.0) })]
    buttons_frame: nwg::Frame,

    #[nwg_layout(parent: buttons_frame, flex_direction: FlexDirection::RowReverse, auto_spacing: None)]
    buttons_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: buttons_frame, text: "Delete")]
    #[nwg_layout_item(layout: buttons_layout, flex_grow: 0.33)]
    #[nwg_events(OnButtonClick: [PersistedTab::delete])]
    delete_button: nwg::Button,

    // Device context menu
    #[nwg_control(text: "Device", popup: true)]
    menu: nwg::Menu,

    #[nwg_control(parent: menu, text: "Delete")]
    #[nwg_events(OnMenuItemSelected: [PersistedTab::delete])]
    menu_delete: nwg::MenuItem,
}

impl PersistedTab {
    fn init_list(&self) {
        let dv = &self.list_view;
        dv.clear();
        dv.insert_column("Device");
        dv.set_headers_enabled(true);

        // Auto-size before adding items to ensure we don't overflow the list view
        dv.set_column_width(0, LVSCW_AUTOSIZE_USEHEADER as isize);
    }

    /// Clears the device list and reloads it with the currently persisted devices.
    fn refresh_list(&self) {
        self.update_devices();

        self.list_view.clear();
        for device in self.persisted_devices.borrow().iter() {
            self.list_view.insert_items_row(
                None,
                &[device.description.as_deref().unwrap_or("Unknown device")],
            );
        }
    }

    /// Refreshes the device list using the provided devices.
    fn refresh_list_with_devices(&self, devices: &[usbipd::UsbDevice]) {
        *self.persisted_devices.borrow_mut() = devices
            .iter()
            .filter(|d| !d.is_connected())
            .cloned()
            .collect();

        self.list_view.clear();
        for device in self.persisted_devices.borrow().iter() {
            self.list_view.insert_items_row(
                None,
                &[device.description.as_deref().unwrap_or("Unknown device")],
            );
        }
    }

    /// Updates the details panel with the currently selected device.
    fn update_persisted_details(&self) {
        let devices = self.persisted_devices.borrow();
        let device = self.list_view.selected_item().and_then(|i| devices.get(i));

        if device.is_some() {
            self.delete_button.set_enabled(true);
        } else {
            self.delete_button.set_enabled(false);
        }

        self.persisted_info.update(device);
    }

    fn show_menu(&self) {
        if self.list_view.selected_item().is_none() {
            return;
        }

        let (x, y) = nwg::GlobalCursor::position();
        // Disable menu animations because they cause incorrect rendering of the bitmaps
        self.menu
            .popup_with_flags(x, y, nwg::PopupMenuFlags::ANIMATE_NONE);
    }

    fn delete(&self) {
        self.run_command(|device| {
            device.unbind()?;
            device.wait(|d| d.is_none())
        });
    }

    /// Runs a `command` function on the currently selected device.
    /// No-op if no device is selected.
    ///
    /// If the command completes successfully, the view is reloaded.
    ///
    /// If an error occurs, an error dialog is shown.
    fn run_command(&self, command: fn(&UsbDevice) -> Result<(), String>) {
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
            let devices = self.persisted_devices.borrow();
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
        *self.persisted_devices.borrow_mut() = usbipd::list_devices()
            .into_iter()
            .filter(|d| !d.is_connected())
            .collect();
    }

    /// Inhibits the window close event.
    fn inhibit_close(data: &nwg::EventData) {
        if let nwg::EventData::OnWindowClose(close_data) = data {
            close_data.close(false);
        }
    }
}

impl GuiTab for PersistedTab {
    fn init(&self, window: &nwg::Window) {
        self.window.replace(window.handle);

        let shield_bitmap = nwg::Bitmap::from_system_icon(SIID_SHIELD);
        self.delete_button.set_bitmap(Some(&shield_bitmap));
        self.menu_delete.set_bitmap(Some(&shield_bitmap));

        self.shield_bitmap.set(shield_bitmap);

        self.init_list();
        self.refresh();
    }

    fn refresh(&self) {
        self.refresh_list();
        self.update_persisted_details();
    }

    fn refresh_with_devices(&self, devices: &[usbipd::UsbDevice]) {
        self.refresh_list_with_devices(devices);
        self.update_persisted_details();
    }
}
