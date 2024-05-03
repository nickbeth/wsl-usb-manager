mod auto_attach_info;

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
use windows_sys::Win32::UI::Controls::LVSCW_AUTOSIZE_USEHEADER;

use self::auto_attach_info::AutoAttachInfo;
use crate::auto_attach::{self, AutoAttachProfile, AutoAttacher};
use crate::gui::usbipd_gui::GuiTab;

const PADDING_LEFT: Rect<D> = Rect {
    start: D::Points(8.0),
    end: D::Points(0.0),
    top: D::Points(0.0),
    bottom: D::Points(0.0),
};

const DETAILS_PANEL_WIDTH: f32 = 285.0;
const DETAILS_PANEL_PADDING: u32 = 4;

#[derive(Default, NwgPartial)]
pub struct AutoAttachTab {
    auto_attacher: Rc<RefCell<AutoAttacher>>,

    window: Cell<nwg::ControlHandle>,

    auto_attach_profiles: RefCell<Vec<auto_attach::AutoAttachProfile>>,

    #[nwg_control]
    #[nwg_events(OnNotice: [AutoAttachTab::refresh])]
    pub refresh_notice: nwg::Notice,

    #[nwg_layout(flex_direction: FlexDirection::Row)]
    tab_layout: nwg::FlexboxLayout,

    #[nwg_control(list_style: nwg::ListViewStyle::Detailed, focus: true,
        flags: "VISIBLE|SINGLE_SELECTION|TAB_STOP",
        ex_flags: nwg::ListViewExFlags::FULL_ROW_SELECT,
    )]
    #[nwg_events(OnListViewRightClick: [AutoAttachTab::show_menu],
        OnListViewItemChanged: [AutoAttachTab::update_auto_attach_details]
    )]
    #[nwg_layout_item(layout: tab_layout, flex_grow: 1.0)]
    list_view: nwg::ListView,

    // Profile info
    #[nwg_control]
    #[nwg_layout_item(layout: tab_layout, margin: PADDING_LEFT,
        size: Size { width: D::Points(DETAILS_PANEL_WIDTH), height: D::Auto },
    )]
    details_frame: nwg::Frame,

    #[nwg_layout(parent: details_frame, flex_direction: FlexDirection::Column,
        auto_spacing: Some(DETAILS_PANEL_PADDING))]
    details_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: details_frame, flags: "VISIBLE")]
    #[nwg_layout_item(layout: details_layout, flex_grow: 1.0)]
    // Multi-line RichLabels send a WM_CLOSE message when the ESC key is pressed
    #[nwg_events(OnWindowClose: [AutoAttachTab::inhibit_close(EVT_DATA)])]
    details_info_frame: nwg::Frame,

    #[nwg_partial(parent: details_info_frame)]
    auto_attach_info: AutoAttachInfo,

    // Buttons
    #[nwg_control(parent: details_frame, flags: "VISIBLE")]
    #[nwg_layout_item(layout: details_layout, size: Size { width: D::Auto, height: D::Points(25.0) })]
    buttons_frame: nwg::Frame,

    #[nwg_layout(parent: buttons_frame, flex_direction: FlexDirection::RowReverse, auto_spacing: None)]
    buttons_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: buttons_frame, text: "Delete")]
    #[nwg_layout_item(layout: buttons_layout, flex_grow: 0.33)]
    #[nwg_events(OnButtonClick: [AutoAttachTab::delete])]
    button_delete: nwg::Button,

    // Device context menu
    #[nwg_control(text: "Device", popup: true)]
    menu: nwg::Menu,

    #[nwg_control(parent: menu, text: "Delete")]
    #[nwg_events(OnMenuItemSelected: [AutoAttachTab::delete])]
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

        self.window.set(window);
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
    fn init(&self, window: &nwg::Window) {
        self.window.replace(window.handle);

        self.init_list();
        self.refresh();
    }

    fn refresh(&self) {
        self.refresh_list();
        self.update_auto_attach_details();
    }
}
