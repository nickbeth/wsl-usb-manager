use native_windows_gui as nwg;
use nwg::PartialUi;
use nwg::stretch::{
    geometry::Size,
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
#[derive(Default)]
pub struct DeviceInfo {
    font_bold: nwg::Font,
    font_bold_big: nwg::Font,

    device_info_layout: nwg::FlexboxLayout,

    device_info: nwg::Label,
    separator: nwg::Frame,
    bus_id: nwg::Label,
    bus_id_content: nwg::RichLabel,
    vid_pid: nwg::Label,
    vid_pid_content: nwg::RichLabel,
    serial: nwg::Label,
    serial_content: nwg::RichLabel,
    state: nwg::Label,
    state_content: nwg::RichLabel,
    description: nwg::Label,
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

impl PartialUi for DeviceInfo {
    fn build_partial<W: Into<nwg::ControlHandle>>(
        data: &mut Self,
        parent: Option<W>,
    ) -> Result<(), nwg::NwgError> {
        let parent = parent.map(|p| p.into());
        let parent_ref = parent.as_ref();

        // Resources
        nwg::Font::builder()
            .family("Segoe UI Semibold")
            .size(16)
            .weight(400)
            .build(&mut data.font_bold)?;

        nwg::Font::builder()
            .family("Segoe UI Semibold")
            .size(20)
            .weight(400)
            .build(&mut data.font_bold_big)?;

        // Controls (all parented to parent_ref)
        nwg::Label::builder()
            .text("Device Info")
            .font(Some(&data.font_bold_big))
            .parent(parent_ref.unwrap())
            .build(&mut data.device_info)?;

        nwg::Frame::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.separator)?;

        nwg::Label::builder()
            .text("Bus ID:")
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.bus_id)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.bus_id_content)?;

        nwg::Label::builder()
            .text("VID:PID:")
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.vid_pid)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.vid_pid_content)?;

        nwg::Label::builder()
            .text("Serial number:")
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.serial)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.serial_content)?;

        nwg::Label::builder()
            .text("State:")
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.state)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.state_content)?;

        nwg::Label::builder()
            .text("Description:")
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.description)?;

        nwg::RichLabel::builder()
            .flags(nwg::RichLabelFlags::VISIBLE | nwg::RichLabelFlags::MULTI_LINE)
            .parent(parent_ref.unwrap())
            .build(&mut data.description_content)?;

        // Layout
        nwg::FlexboxLayout::builder()
            .parent(parent_ref.unwrap())
            .flex_direction(FlexDirection::Column)
            .auto_spacing(None)
            // "Device Info" header
            .child(&data.device_info)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // Separator
            .child(&data.separator)
            .child_size(Size {
                width: D::Auto,
                height: Pt(1.0),
            })
            .child_margin(nwg::stretch::geometry::Rect {
                start: Pt(0.0),
                end: Pt(0.0),
                top: Pt(5.0),
                bottom: Pt(0.0),
            })
            // Bus ID label
            .child(&data.bus_id)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            .child_margin(nwg::stretch::geometry::Rect {
                start: Pt(0.0),
                end: Pt(0.0),
                top: Pt(6.0),
                bottom: Pt(0.0),
            })
            // Bus ID content
            .child(&data.bus_id_content)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // VID:PID label
            .child(&data.vid_pid)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // VID:PID content
            .child(&data.vid_pid_content)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // Serial label
            .child(&data.serial)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // Serial content
            .child(&data.serial_content)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // State label
            .child(&data.state)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // State content
            .child(&data.state_content)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // Description label
            .child(&data.description)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // Description content (multi-line, flex_grow)
            .child(&data.description_content)
            .child_size(Size {
                width: D::Auto,
                height: D::Auto,
            })
            .child_flex_grow(1.0)
            .build(&data.device_info_layout)?;

        Ok(())
    }

    fn process_event(
        &self,
        _evt: nwg::Event,
        _evt_data: &nwg::EventData,
        _handle: nwg::ControlHandle,
    ) {
        // No events on DeviceInfo
    }

    fn handles(&self) -> Vec<&nwg::ControlHandle> {
        Vec::new()
    }
}
