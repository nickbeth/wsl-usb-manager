mod persisted_info;

use std::cell::{Cell, RefCell};

use native_windows_gui as nwg;
use nwg::PartialUi;
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

#[derive(Default)]
pub struct PersistedTab {
    window: Cell<nwg::ControlHandle>,
    shield_bitmap: Cell<nwg::Bitmap>,

    persisted_devices: RefCell<Vec<usbipd::UsbDevice>>,

    persisted_tab_layout: nwg::FlexboxLayout,
    list_view: nwg::ListView,

    // Persisted info
    details_frame: nwg::Frame,
    details_layout: nwg::FlexboxLayout,
    // Multi-line RichLabels send a WM_CLOSE message when the ESC key is pressed
    persisted_info_frame: nwg::Frame,
    persisted_info: PersistedInfo,

    // Buttons
    buttons_frame: nwg::Frame,
    buttons_layout: nwg::FlexboxLayout,
    delete_button: nwg::Button,

    // Device context menu
    menu: nwg::Menu,
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

    /// Clears the device list and reloads it with the provided persisted devices.
    fn refresh_list(&self, devices: &[usbipd::UsbDevice]) {
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

    /// Inhibits the window close event.
    fn inhibit_close(data: &nwg::EventData) {
        if let nwg::EventData::OnWindowClose(close_data) = data {
            close_data.close(false);
        }
    }

    /// Refreshes the tab with the provided device list.
    /// This is used to share the device list among multiple tabs to avoid redundant process spawning.
    pub fn refresh_with_devices(&self, devices: &[usbipd::UsbDevice]) {
        self.refresh_list(devices);
        self.update_persisted_details();
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
        let devices = usbipd::list_devices();
        self.refresh_with_devices(&devices);
    }
}

impl PartialUi for PersistedTab {
    fn build_partial<W: Into<nwg::ControlHandle>>(
        data: &mut Self,
        parent: Option<W>,
    ) -> Result<(), nwg::NwgError> {
        let parent = parent.map(|p| p.into());
        let parent_ref = parent.as_ref();

        // Controls
        nwg::ListView::builder()
            .list_style(nwg::ListViewStyle::Detailed)
            .focus(true)
            .flags(
                nwg::ListViewFlags::VISIBLE
                    | nwg::ListViewFlags::SINGLE_SELECTION
                    | nwg::ListViewFlags::TAB_STOP,
            )
            .ex_flags(nwg::ListViewExFlags::FULL_ROW_SELECT)
            .parent(parent_ref.unwrap())
            .build(&mut data.list_view)?;

        nwg::Frame::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.details_frame)?;

        nwg::Frame::builder()
            .parent(&data.details_frame)
            .flags(nwg::FrameFlags::VISIBLE)
            .build(&mut data.persisted_info_frame)?;

        nwg::Frame::builder()
            .parent(&data.details_frame)
            .flags(nwg::FrameFlags::VISIBLE)
            .build(&mut data.buttons_frame)?;

        nwg::Button::builder()
            .parent(&data.buttons_frame)
            .text("Delete")
            .build(&mut data.delete_button)?;

        nwg::Menu::builder()
            .text("Device")
            .popup(true)
            .parent(parent_ref.unwrap())
            .build(&mut data.menu)?;

        nwg::MenuItem::builder()
            .parent(&data.menu)
            .text("Delete")
            .build(&mut data.menu_delete)?;

        // Build nested partial
        PersistedInfo::build_partial(&mut data.persisted_info, Some(&data.persisted_info_frame))?;

        // Build layouts
        nwg::FlexboxLayout::builder()
            .parent(parent_ref.unwrap())
            .flex_direction(FlexDirection::Row)
            // List view
            .child(&data.list_view)
            .child_flex_grow(1.0)
            // Details frame
            .child(&data.details_frame)
            .child_margin(PADDING_LEFT)
            .child_size(Size {
                width: D::Points(DETAILS_PANEL_WIDTH),
                height: D::Auto,
            })
            .build(&data.persisted_tab_layout)?;

        nwg::FlexboxLayout::builder()
            .parent(&data.details_frame)
            .flex_direction(FlexDirection::Column)
            .auto_spacing(Some(DETAILS_PANEL_PADDING))
            // Persisted info frame
            .child(&data.persisted_info_frame)
            .child_flex_grow(1.0)
            // Buttons frame
            .child(&data.buttons_frame)
            .child_size(Size {
                width: D::Auto,
                height: D::Points(25.0),
            })
            .build(&data.details_layout)?;

        nwg::FlexboxLayout::builder()
            .parent(&data.buttons_frame)
            .flex_direction(FlexDirection::RowReverse)
            .auto_spacing(None)
            .child(&data.delete_button)
            .child_flex_grow(0.33)
            .build(&data.buttons_layout)?;

        Ok(())
    }

    fn process_event(
        &self,
        evt: nwg::Event,
        evt_data: &nwg::EventData,
        handle: nwg::ControlHandle,
    ) {
        match evt {
            nwg::Event::OnListViewRightClick => {
                if handle == self.list_view.handle {
                    PersistedTab::show_menu(self);
                }
            }
            nwg::Event::OnListViewItemChanged => {
                if handle == self.list_view.handle {
                    PersistedTab::update_persisted_details(self);
                }
            }
            nwg::Event::OnWindowClose => {
                if handle == self.persisted_info_frame.handle {
                    PersistedTab::inhibit_close(evt_data);
                }
            }
            nwg::Event::OnButtonClick => {
                if handle == self.delete_button.handle {
                    PersistedTab::delete(self);
                }
            }
            nwg::Event::OnMenuItemSelected => {
                if handle == self.menu_delete.handle {
                    PersistedTab::delete(self);
                }
            }
            _ => {}
        }

        // Forward to nested partial
        self.persisted_info.process_event(evt, evt_data, handle);
    }

    fn handles(&self) -> Vec<&nwg::ControlHandle> {
        Vec::new()
    }
}
