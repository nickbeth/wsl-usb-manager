use native_windows_derive::NwgPartial;
use native_windows_gui as nwg;

use nwg::stretch::{
    geometry::{Rect, Size},
    style::{Dimension as D, Dimension::Points as Pt, FlexDirection},
};

use crate::usbipd::{UsbDevice, UsbipState};

/// The connected device info tab.
/// It displays detailed information about a connected device.
///
/// Call the `update` method to update the information displayed.
///
/// # Remarks
///
/// The `ES_MULTILINE` flag used to make the `Description` label multi-line
/// sends a `WM_CLOSE` message when the `ESC` key is pressed while the control
/// has focus. It is suggested to inhibit the `OnWindowClose` event on the
/// parent window (e.g. the parent `nwg::Frame`) to prevent it from closing.
#[derive(Default, NwgPartial)]
pub struct DeviceInfo {
    #[nwg_resource(family: "Segoe UI Semibold", size: 16, weight: 400)]
    font_bold: nwg::Font,

    #[nwg_resource(family: "Segoe UI Semibold", size: 20, weight: 400)]
    font_bold_big: nwg::Font,

    #[nwg_layout(flex_direction: FlexDirection::Column, auto_spacing: None)]
    device_info_layout: nwg::FlexboxLayout,

    #[nwg_control(text: "Device Info", font: Some(&data.font_bold_big))]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    device_info: nwg::Label,

    #[nwg_control]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: Pt(1.0) },
        margin: Rect { start: Pt(0.0), end: Pt(0.0), top: Pt(5.0), bottom: Pt(0.0)}
    )]
    separator: nwg::Frame,

    #[nwg_control(text: "Bus ID:", font: Some(&data.font_bold), v_align: nwg::VTextAlign::Bottom)]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: Pt(20.0)},
        margin: Rect { start: Pt(0.0), end: Pt(0.0), top: Pt(6.0), bottom: Pt(0.0)}
    )]
    bus_id: nwg::Label,

    #[nwg_control]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    bus_id_content: nwg::RichLabel,

    #[nwg_control(text: "VID:PID:", font: Some(&data.font_bold), v_align: nwg::VTextAlign::Bottom)]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    vid_pid: nwg::Label,

    #[nwg_control]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    vid_pid_content: nwg::RichLabel,

    #[nwg_control(text: "Serial number:", font: Some(&data.font_bold), v_align: nwg::VTextAlign::Bottom)]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    serial: nwg::Label,

    #[nwg_control]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    serial_content: nwg::RichLabel,

    #[nwg_control(text: "State:", font: Some(&data.font_bold), v_align: nwg::VTextAlign::Bottom)]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    state: nwg::Label,

    #[nwg_control]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    state_content: nwg::RichLabel,

    #[nwg_control(text: "Description:", font: Some(&data.font_bold), v_align: nwg::VTextAlign::Bottom)]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    description: nwg::Label,

    #[nwg_control(flags: "VISIBLE|MULTI_LINE")]
    #[nwg_layout_item(layout: device_info_layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    description_content: nwg::RichLabel,
}

impl DeviceInfo {
    pub fn update(&self, device: Option<&UsbDevice>) {
        if let Some(device) = device {
            self.bus_id_content
                .set_text(device.bus_id.as_deref().unwrap_or("-"));
            self.vid_pid_content
                .set_text(device.vid_pid().as_deref().unwrap_or("-"));
            self.serial_content
                .set_text(device.serial().as_deref().unwrap_or("-"));
            self.state_content.set_text(&device.state().to_string());
            self.description_content.set_text(
                device
                    .description
                    .as_deref()
                    .unwrap_or("No description available"),
            );
        } else {
            self.bus_id_content.set_text("-");
            self.vid_pid_content.set_text("-");
            self.serial_content.set_text("-");
            self.state_content.set_text(&UsbipState::None.to_string());
            self.description_content.set_text("No device selected");
        }
    }
}
