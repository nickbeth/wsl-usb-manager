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
use windows_sys::Win32::UI::Controls::LVSCW_AUTOSIZE_USEHEADER;

use super::auto_attach_info::AutoAttachInfo;
use crate::auto_attach::{self, AutoAttachProfile, AutoAttacher};
use crate::gui::main_window::GuiTab;

const PADDING_LEFT: Rect<D> = Rect {
    start: D::Points(8.0),
    end: D::Points(0.0),
    top: D::Points(0.0),
    bottom: D::Points(0.0),
};

const DETAILS_PANEL_WIDTH: f32 = 285.0;
const DETAILS_PANEL_PADDING: u32 = 4;

#[derive(Default)]
pub struct AutoAttachTab {
    auto_attacher: Rc<RefCell<AutoAttacher>>,

    window: Cell<nwg::ControlHandle>,

    auto_attach_profiles: RefCell<Vec<auto_attach::AutoAttachProfile>>,

    pub refresh_notice: nwg::Notice,

    tab_layout: nwg::FlexboxLayout,
    list_view: nwg::ListView,

    // Profile info
    details_frame: nwg::Frame,
    details_layout: nwg::FlexboxLayout,
    // Multi-line RichLabels send a WM_CLOSE message when the ESC key is pressed
    details_info_frame: nwg::Frame,
    auto_attach_info: AutoAttachInfo,

    // Buttons
    buttons_frame: nwg::Frame,
    buttons_layout: nwg::FlexboxLayout,
    button_delete: nwg::Button,

    // Device context menu
    menu: nwg::Menu,
    menu_delete: nwg::MenuItem,
}

impl AutoAttachTab {
    pub fn new(auto_attacher: &Rc<RefCell<AutoAttacher>>) -> Self {
        Self {
            auto_attacher: auto_attacher.clone(),
            ..Default::default()
        }
    }

    fn init_list(&self) {
        let dv = &self.list_view;
        dv.clear();
        dv.insert_column("Device");
        dv.set_headers_enabled(true);

        dv.set_column_width(0, LVSCW_AUTOSIZE_USEHEADER as isize);
    }

    /// Clears the auto attach profile list and reloads it.
    fn refresh_list(&self) {
        self.update_profiles();

        self.list_view.clear();
        for profile in self.auto_attach_profiles.borrow().iter() {
            self.list_view.insert_items_row(
                None,
                &[profile.description.as_deref().unwrap_or("Unknown device")],
            );
        }
    }

    /// Updates the auto attach details panel info.
    fn update_auto_attach_details(&self) {
        let profiles = self.auto_attach_profiles.borrow();
        let profile = self.list_view.selected_item().and_then(|i| profiles.get(i));

        self.auto_attach_info.update(profile);

        // Update buttons
        self.button_delete.set_enabled(profile.is_some());
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
        self.run_command(|profile| self.auto_attacher.borrow_mut().remove(profile));
    }

    /// Runs a `command` function on the currently selected profile.
    /// No-op if no profile is selected.
    ///
    /// If the command completes successfully, the view is reloaded.
    ///
    /// If an error occurs, an error dialog is shown.
    fn run_command(&self, command: impl Fn(&AutoAttachProfile) -> Result<(), String>) {
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
            let profiles = self.auto_attach_profiles.borrow();
            let profile = match profiles.get(selected_index) {
                Some(p) => p,
                None => return,
            };

            command(profile)
        };

        if let Err(err) = result {
            nwg::modal_error_message(window, "WSL USB Manager: Command Error", &err);
        }

        self.refresh();
        nwg::unbind_event_handler(&cursor_event);
    }

    fn update_profiles(&self) {
        *self.auto_attach_profiles.borrow_mut() = self.auto_attacher.borrow().profiles();
    }

    /// Inhibits the window close event.
    fn inhibit_close(data: &nwg::EventData) {
        if let nwg::EventData::OnWindowClose(close_data) = data {
            close_data.close(false);
        }
    }
}

impl GuiTab for AutoAttachTab {
    fn init(&self, window: &Rc<nwg::Window>) {
        self.window.set(window.handle);
        self.init_list();
        self.refresh();
    }

    fn refresh(&self) {
        self.refresh_list();
        self.update_auto_attach_details();
    }
}

impl PartialUi for AutoAttachTab {
    fn build_partial<W: Into<nwg::ControlHandle>>(
        data: &mut Self,
        parent: Option<W>,
    ) -> Result<(), nwg::NwgError> {
        let parent = parent.map(|p| p.into());
        let parent_ref = parent.as_ref();

        // Controls
        nwg::Notice::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.refresh_notice)?;

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
            .build(&mut data.details_info_frame)?;

        nwg::Frame::builder()
            .parent(&data.details_frame)
            .flags(nwg::FrameFlags::VISIBLE)
            .build(&mut data.buttons_frame)?;

        nwg::Button::builder()
            .parent(&data.buttons_frame)
            .text("Delete")
            .build(&mut data.button_delete)?;

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
        AutoAttachInfo::build_partial(&mut data.auto_attach_info, Some(&data.details_info_frame))?;

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
            .build(&data.tab_layout)?;

        nwg::FlexboxLayout::builder()
            .parent(&data.details_frame)
            .flex_direction(FlexDirection::Column)
            .auto_spacing(Some(DETAILS_PANEL_PADDING))
            // Details info frame
            .child(&data.details_info_frame)
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
            .child(&data.button_delete)
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
            nwg::Event::OnNotice => {
                if handle == self.refresh_notice.handle {
                    GuiTab::refresh(self);
                }
            }
            nwg::Event::OnListViewRightClick => {
                if handle == self.list_view.handle {
                    AutoAttachTab::show_menu(self);
                }
            }
            nwg::Event::OnListViewItemChanged => {
                if handle == self.list_view.handle {
                    AutoAttachTab::update_auto_attach_details(self);
                }
            }
            nwg::Event::OnWindowClose => {
                if handle == self.details_info_frame.handle {
                    AutoAttachTab::inhibit_close(evt_data);
                }
            }
            nwg::Event::OnButtonClick => {
                if handle == self.button_delete.handle {
                    AutoAttachTab::delete(self);
                }
            }
            nwg::Event::OnMenuItemSelected => {
                if handle == self.menu_delete.handle {
                    AutoAttachTab::delete(self);
                }
            }
            _ => {}
        }

        // Forward to nested partial
        self.auto_attach_info.process_event(evt, evt_data, handle);
    }

    fn handles(&self) -> Vec<&nwg::ControlHandle> {
        Vec::new()
    }
}
