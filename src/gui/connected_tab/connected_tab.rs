use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use native_windows_gui as nwg;
use nwg::PartialUi;
use nwg::stretch::{
    geometry::{Rect, Size},
    style::{Dimension as D, FlexDirection},
};
use windows_sys::Win32::UI::Controls::LVSCW_AUTOSIZE;
use windows_sys::Win32::UI::Controls::LVSCW_AUTOSIZE_USEHEADER;

use super::device_info::DeviceInfo;
use crate::gui::{
    RESOURCES, connected_tab::auto_attach::AutoAttachWindowUi, main_window::GuiTab,
    nwg_ext::{ListViewExt, MenuItemEx},
};
use crate::i18n::{Localizable, Translator, keys};
use crate::usbipd::{self, UsbDevice, UsbipState};
use crate::{auto_attacher::AutoAttacher, gui::connected_tab::auto_attach::AutoAttachWindow};

const PADDING_LEFT: Rect<D> = Rect {
    start: D::Points(8.0),
    end: D::Points(0.0),
    top: D::Points(0.0),
    bottom: D::Points(0.0),
};

const DETAILS_PANEL_WIDTH: f32 = 285.0;
const DETAILS_PANEL_PADDING: u32 = 4;

#[derive(Default)]
pub struct ConnectedTab {
    auto_attacher: Rc<RefCell<AutoAttacher>>,
    auto_attach_window: Cell<Option<Box<AutoAttachWindowUi>>>,

    window: RefCell<Rc<nwg::Window>>,

    /// Active translator snapshot. Injected by the main window on init and on
    /// every language switch.
    translator: RefCell<Rc<Translator>>,

    /// A notice to refresh the connected tab
    refresh_notice: nwg::Notice,

    /// A notice sender to notify the auto attach tab to refresh
    pub auto_attach_notice: Cell<Option<nwg::NoticeSender>>,

    connected_devices: RefCell<Vec<usbipd::UsbDevice>>,

    connected_tab_layout: nwg::FlexboxLayout,
    list_view: nwg::ListView,

    // Device info
    details_frame: nwg::Frame,
    details_layout: nwg::FlexboxLayout,
    // Multi-line RichLabels send a WM_CLOSE message when the ESC key is pressed
    device_info_frame: nwg::Frame,
    device_info: DeviceInfo,

    // Buttons
    buttons_frame: nwg::Frame,
    buttons_layout: nwg::FlexboxLayout,
    attach_detach_button: nwg::Button,
    bind_unbind_button: nwg::Button,
    auto_attach_button: nwg::Button,

    // Device context menu
    menu: nwg::Menu,
    menu_attach: nwg::MenuItem,
    menu_detach: nwg::MenuItem,
    menu_sep: nwg::MenuSeparator,
    menu_bind: nwg::MenuItem,
    menu_bind_force: nwg::MenuItem,
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
        let t = self.translator.borrow().clone();
        let list = &self.list_view;
        list.clear();
        list.insert_column(t.get(keys::LIST_CONNECTED_BUS_ID));
        list.insert_column(t.get(keys::LIST_CONNECTED_STATE));
        list.insert_column(t.get(keys::LIST_CONNECTED_DEVICE));
        list.set_headers_enabled(true);

        // Insert a dummy row, with max-width content in columns where AUTOSIZE is used
        list.insert_items_row(
            None,
            &[
                "-",
                &format!("{}  ", UsbipState::None.to_localized_string(&t)),
                t.get(keys::PLACEHOLDER_UNKNOWN_DEVICE),
            ],
        );

        list.set_column_width(0, LVSCW_AUTOSIZE_USEHEADER as isize);
        list.set_column_width(1, LVSCW_AUTOSIZE as isize);
        list.set_column_width(2, LVSCW_AUTOSIZE_USEHEADER as isize);

        // Clear the dummy row
        list.clear();
    }

    /// Clears the device list and reloads it with the provided connected devices.
    fn refresh_list(&self, devices: &[usbipd::UsbDevice]) {
        let t = self.translator.borrow().clone();
        *self.connected_devices.borrow_mut() = devices
            .iter()
            .filter(|d| d.is_connected())
            .cloned()
            .collect();

        self.list_view.clear();
        for device in self.connected_devices.borrow().iter() {
            // Description fallback may come from the translator (distinct
            // lifetime from the device borrow), so resolve it to an owned
            // `String` before borrowing for the row.
            let description = device
                .description
                .clone()
                .unwrap_or_else(|| t.get(keys::PLACEHOLDER_UNKNOWN_DEVICE).to_owned());
            self.list_view.insert_items_row(
                None,
                &[
                    device.bus_id.as_deref().unwrap_or("-"),
                    &device.state().to_localized_string(&t),
                    &description,
                ],
            );
        }
    }

    /// Updates the device details panel with the currently selected device.
    fn update_device_details(&self) {
        let t = self.translator.borrow().clone();
        let devices = self.connected_devices.borrow();
        let device = self.list_view.selected_item().and_then(|i| devices.get(i));

        self.device_info.update(device);

        // Update buttons
        if let Some(device) = device {
            if device.is_bound() {
                self.bind_unbind_button.set_text(t.get(keys::BUTTON_UNBIND));
                self.auto_attach_button.set_enabled(true);

                // Attaching a bound device doesn't require admin privileges, hide the UAC shield icon
                self.attach_detach_button.set_bitmap(None);
            } else {
                self.bind_unbind_button.set_text(t.get(keys::BUTTON_BIND));

                // Attaching an unbound device requires admin privileges, show the UAC shield icon
                self.attach_detach_button
                    .set_bitmap(Some(&RESOURCES.shield_bitmap));
            }

            if device.is_attached() {
                self.attach_detach_button.set_text(t.get(keys::BUTTON_DETACH));
            } else {
                self.attach_detach_button.set_text(t.get(keys::BUTTON_ATTACH));
            }

            self.auto_attach_button.set_enabled(true);
            self.bind_unbind_button.set_enabled(true);
            self.attach_detach_button.set_enabled(true);
        } else {
            self.attach_detach_button
                .set_text(t.get(keys::BUTTON_ATTACH));
            self.bind_unbind_button.set_text(t.get(keys::BUTTON_BIND));
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

        let is_attached = device.is_attached();
        self.menu_attach.set_enabled(!is_attached);
        self.menu_detach.set_enabled(is_attached);

        let is_bound = device.is_bound();
        self.menu_bind.set_enabled(!is_bound);
        self.menu_bind_force.set_enabled(!is_bound);
        self.menu_unbind.set_enabled(is_bound);

        // Show the UAC shield icon on menu items that require admin privileges
        let bitmap = if !is_bound {
            // Attaching an unbound device requires admin privileges, show the UAC shield icon
            Some(&RESOURCES.shield_bitmap)
        } else {
            // Attaching a bound device doesn't require admin privileges, hide the UAC shield icon
            None
        };
        self.menu_attach.set_bitmap(bitmap);

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
        let Some(selected_index) = self.list_view.selected_item() else {
            return;
        };

        let devices = self.connected_devices.borrow();
        let device = match devices.get(selected_index) {
            Some(device) => device,
            None => return,
        };

        let translator = self.translator.borrow().clone();
        let Ok(mut ui) = AutoAttachWindow::build_ui(
            AutoAttachWindow::new(device.clone(), &self.auto_attacher, translator),
            &self.window.borrow(),
        ) else {
            return;
        };

        // Prepare closure access to parent data
        let parent_window = Rc::downgrade(&self.window.borrow());
        let child_ui = Rc::downgrade(&ui.inner);
        let connected_tab_notice = self.refresh_notice.sender();
        let auto_attach_notice = self.auto_attach_notice.get();

        let parent_handler =
            nwg::full_bind_event_handler(&ui.window.handle, move |evt, _evt_data, handle| {
                let Some(ui) = child_ui.upgrade() else {
                    return;
                };

                let Some(parent_window) = parent_window.upgrade() else {
                    return;
                };

                if evt == nwg::Event::OnWindowClose && handle == ui.window.handle {
                    parent_window.set_enabled(true);
                    connected_tab_notice.notice();
                    if let Some(n) = auto_attach_notice {
                        n.notice();
                    }
                }
            });

        ui.default_handlers.push(parent_handler);

        // Disable the parent window
        self.window.borrow().set_enabled(false);

        // Store the auto attach window UI so that it doesn't get dropped immediately
        // Drops any old auto attach window in the process, if present
        self.auto_attach_window.set(Some(Box::new(ui)));
    }

    /// Runs a `command` function on the currently selected device.
    /// No-op if no device is selected.
    ///
    /// If the command completes successfully, the view is reloaded.
    ///
    /// If an error occurs, an error dialog is shown.
    fn run_command(&self, command: impl Fn(&UsbDevice) -> Result<(), String>) {
        let window = self.window.borrow().handle;
        let t = self.translator.borrow().clone();

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
            nwg::modal_error_message(window, t.get(keys::DIALOG_TITLE_COMMAND_ERROR), &err);
        }

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
        self.update_device_details();
    }
}

impl Localizable for ConnectedTab {
    fn re_localize(&self, t: &Translator) {
        // ListView headers (no NWG API; direct LVM_SETCOLUMN)
        self.list_view
            .set_header_text(0, t.get(keys::LIST_CONNECTED_BUS_ID));
        self.list_view
            .set_header_text(1, t.get(keys::LIST_CONNECTED_STATE));
        self.list_view
            .set_header_text(2, t.get(keys::LIST_CONNECTED_DEVICE));

        // Detail panel static labels
        self.device_info.re_localize(t);

        // Context menu items
        self.menu_attach.set_text(t.get(keys::MENU_DEVICE_ATTACH));
        self.menu_detach.set_text(t.get(keys::MENU_DEVICE_DETACH));
        self.menu_bind.set_text(t.get(keys::MENU_DEVICE_BIND));
        self.menu_bind_force
            .set_text(t.get(keys::MENU_DEVICE_BIND_FORCE));
        self.menu_unbind.set_text(t.get(keys::MENU_DEVICE_UNBIND));

        // Repaint dynamic rows so state text follows the new language. This
        // clears the current selection; acceptable for an infrequent action
        // like a language switch (user can re-select).
        let devices = self.connected_devices.borrow().clone();
        self.refresh_list(&devices);
        self.update_device_details();
    }
}

impl GuiTab for ConnectedTab {
    fn init(&self, window: &Rc<nwg::Window>) {
        *self.window.borrow_mut() = Rc::clone(window);

        // Set the UAC shield icon for menu items and buttons that always require admin privileges
        self.menu_bind.set_bitmap(Some(&RESOURCES.shield_bitmap));
        self.menu_bind_force
            .set_bitmap(Some(&RESOURCES.shield_bitmap));
        self.menu_unbind.set_bitmap(Some(&RESOURCES.shield_bitmap));
        self.bind_unbind_button
            .set_bitmap(Some(&RESOURCES.shield_bitmap));

        self.init_list();
        self.refresh();
    }

    fn refresh(&self) {
        let devices = usbipd::list_devices();
        self.refresh_with_devices(&devices);
    }

    fn set_translator(&self, t: Rc<Translator>) {
        *self.translator.borrow_mut() = t.clone();
        self.device_info.set_translator(t);
    }
}

impl PartialUi for ConnectedTab {
    fn build_partial<W: Into<nwg::ControlHandle>>(
        data: &mut Self,
        parent: Option<W>,
    ) -> Result<(), nwg::NwgError> {
        let parent = parent.map(|p| p.into());
        let parent_ref = parent.as_ref();

        // Initial English text; the active language is applied by `re_localize`
        // once the main window injects the translator.
        let t = Translator::default();

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

        nwg::Notice::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.refresh_notice)?;

        nwg::Frame::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.details_frame)?;

        nwg::Frame::builder()
            .parent(&data.details_frame)
            .flags(nwg::FrameFlags::VISIBLE)
            .build(&mut data.device_info_frame)?;

        nwg::Frame::builder()
            .parent(&data.details_frame)
            .flags(nwg::FrameFlags::VISIBLE)
            .build(&mut data.buttons_frame)?;

        nwg::Button::builder()
            .parent(&data.buttons_frame)
            .text(t.get(keys::BUTTON_ATTACH))
            .build(&mut data.attach_detach_button)?;

        nwg::Button::builder()
            .parent(&data.buttons_frame)
            .text(t.get(keys::BUTTON_BIND))
            .build(&mut data.bind_unbind_button)?;

        nwg::Button::builder()
            .parent(&data.buttons_frame)
            .text(t.get(keys::BUTTON_AUTO_ATTACH))
            .build(&mut data.auto_attach_button)?;

        nwg::Menu::builder()
            .text(t.get(keys::MENU_DEVICE_LABEL))
            .popup(true)
            .parent(parent_ref.unwrap())
            .build(&mut data.menu)?;

        nwg::MenuItem::builder()
            .parent(&data.menu)
            .text(t.get(keys::MENU_DEVICE_ATTACH))
            .build(&mut data.menu_attach)?;

        nwg::MenuItem::builder()
            .parent(&data.menu)
            .text(t.get(keys::MENU_DEVICE_DETACH))
            .build(&mut data.menu_detach)?;

        nwg::MenuSeparator::builder()
            .parent(&data.menu)
            .build(&mut data.menu_sep)?;

        nwg::MenuItem::builder()
            .parent(&data.menu)
            .text(t.get(keys::MENU_DEVICE_BIND))
            .build(&mut data.menu_bind)?;

        nwg::MenuItem::builder()
            .parent(&data.menu)
            .text(t.get(keys::MENU_DEVICE_BIND_FORCE))
            .build(&mut data.menu_bind_force)?;

        nwg::MenuItem::builder()
            .parent(&data.menu)
            .text(t.get(keys::MENU_DEVICE_UNBIND))
            .build(&mut data.menu_unbind)?;

        // Build nested partial
        DeviceInfo::build_partial(&mut data.device_info, Some(&data.device_info_frame))?;

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
            .build(&data.connected_tab_layout)?;

        nwg::FlexboxLayout::builder()
            .parent(&data.details_frame)
            .flex_direction(FlexDirection::Column)
            .auto_spacing(Some(DETAILS_PANEL_PADDING))
            // Device info frame
            .child(&data.device_info_frame)
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
            .child(&data.attach_detach_button)
            .child_flex_grow(0.33)
            .child(&data.bind_unbind_button)
            .child_flex_grow(0.33)
            .child(&data.auto_attach_button)
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
                    ConnectedTab::show_menu(self);
                }
            }
            nwg::Event::OnListViewItemChanged => {
                if handle == self.list_view.handle {
                    ConnectedTab::update_device_details(self);
                }
            }
            nwg::Event::OnWindowClose => {
                if handle == self.device_info_frame.handle {
                    ConnectedTab::inhibit_close(evt_data);
                }
            }
            nwg::Event::OnNotice => {
                if handle == self.refresh_notice.handle {
                    ConnectedTab::refresh(self);
                }
            }
            nwg::Event::OnButtonClick => {
                if handle == self.attach_detach_button.handle {
                    ConnectedTab::attach_detach_device(self);
                }
                if handle == self.bind_unbind_button.handle {
                    ConnectedTab::bind_unbind_device(self);
                }
                if handle == self.auto_attach_button.handle {
                    ConnectedTab::auto_attach_device(self);
                }
            }
            nwg::Event::OnMenuItemSelected => {
                if handle == self.menu_attach.handle {
                    ConnectedTab::attach_device(self);
                }
                if handle == self.menu_detach.handle {
                    ConnectedTab::detach_device(self);
                }
                if handle == self.menu_bind.handle {
                    ConnectedTab::bind_device(self);
                }
                if handle == self.menu_bind_force.handle {
                    ConnectedTab::bind_device_force(self);
                }
                if handle == self.menu_unbind.handle {
                    ConnectedTab::unbind_device(self);
                }
            }
            _ => {}
        }

        // Forward to nested partial
        self.device_info.process_event(evt, evt_data, handle);
    }

    fn handles(&self) -> Vec<&nwg::ControlHandle> {
        Vec::new()
    }
}
