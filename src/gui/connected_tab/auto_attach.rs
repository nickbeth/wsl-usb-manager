use native_windows_derive::NwgUi;
use native_windows_gui as nwg;
use nwg::stretch::{
    geometry::{Rect, Size},
    style::{Dimension as D, FlexDirection},
};

use crate::usbipd::UsbDevice;

#[derive(Default, NwgUi)]
pub struct AutoAttachWindow {
    #[nwg_control(size: (500, 160), center: true, title: "Configure Auto Attach", flags: "WINDOW|VISIBLE")]
    #[nwg_events(OnWindowClose: [AutoAttachWindow::close])]
    pub window: nwg::Window,

    #[nwg_layout(parent: window, flex_direction: FlexDirection::Column, auto_spacing: Some(10),
        padding: Rect { start: D::Points(10.0), end: D::Points(10.0), top: D::Points(10.0), bottom: D::Points(10.0) })]
    main_layout: nwg::FlexboxLayout,

    // Row 1
    #[nwg_control(parent: window)]
    #[nwg_layout_item(layout: main_layout, size: Size { width: D::Auto, height: D::Points(30.0) })]
    row1: nwg::Frame,

    #[nwg_layout(parent: row1, flex_direction: FlexDirection::Row, auto_spacing: Some(10))]
    layout1: nwg::FlexboxLayout,

    #[nwg_control(parent: row1, text: "Device")]
    #[nwg_layout_item(layout: layout1, size: Size { width: D::Points(80.0), height: D::Auto })]
    #[nwg_events(OnButtonClick: [AutoAttachWindow::close])]
    device_button: nwg::Button,

    #[nwg_control(parent: row1, text: "")]
    #[nwg_layout_item(layout: layout1, flex_grow: 1.0)]
    device_label: nwg::Label,

    // Row 2
    #[nwg_control(parent: window)]
    #[nwg_layout_item(layout: main_layout, size: Size { width: D::Auto, height: D::Points(30.0) })]
    row2: nwg::Frame,

    #[nwg_layout(parent: row2, flex_direction: FlexDirection::Row, auto_spacing: Some(10))]
    layout2: nwg::FlexboxLayout,

    #[nwg_control(parent: row2, text: "Port")]
    #[nwg_layout_item(layout: layout2, size: Size { width: D::Points(80.0), height: D::Auto })]
    #[nwg_events(OnButtonClick: [AutoAttachWindow::close])]
    port_button: nwg::Button,

    #[nwg_control(parent: row2, text: "")]
    #[nwg_layout_item(layout: layout2, flex_grow: 1.0)]
    port_label: nwg::Label,

    // Row 3
    #[nwg_control(parent: window)]
    #[nwg_layout_item(layout: main_layout, size: Size { width: D::Auto, height: D::Points(30.0) })]
    row3: nwg::Frame,

    #[nwg_layout(parent: row3, flex_direction: FlexDirection::Row, auto_spacing: Some(10))]
    layout3: nwg::FlexboxLayout,

    #[nwg_control(parent: row3, text: "Both")]
    #[nwg_layout_item(layout: layout3, size: Size { width: D::Points(80.0), height: D::Auto })]
    #[nwg_events(OnButtonClick: [AutoAttachWindow::close])]
    both_button: nwg::Button,

    #[nwg_control(parent: row3, text: "")]
    #[nwg_layout_item(layout: layout3, flex_grow: 1.0)]
    both_label: nwg::Label,
}

impl AutoAttachWindow {
    pub fn update(&self, device: &UsbDevice) {
        let description = device.description.as_deref().unwrap_or("Unknown device");
        let bus_id = device.bus_id.as_deref().unwrap_or("-");

        self.device_label.set_text(description);
        self.port_label.set_text(bus_id);
        self.both_label.set_text(&format!("{} on {}", description, bus_id));
    }

    fn close(&self) {
        self.window.set_visible(false);
    }
}
