use native_windows_gui as nwg;
use nwg::PartialUi;
use nwg::stretch::{
    geometry::Size,
    style::{Dimension as D, Dimension::Points as Pt, FlexDirection},
};

use crate::usbipd::UsbDevice;

/// The persisted device info tab.
/// It displays detailed information about a persisted device.
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
pub struct PersistedInfo {
    font_bold: nwg::Font,
    font_bold_big: nwg::Font,

    info_layout: nwg::FlexboxLayout,

    persisted_info: nwg::Label,
    separator: nwg::Frame,
    vid_pid: nwg::Label,
    vid_pid_content: nwg::RichLabel,
    serial: nwg::Label,
    serial_content: nwg::RichLabel,
    persisted: nwg::Label,
    persisted_content: nwg::RichLabel,
    description: nwg::Label,
    description_content: nwg::RichLabel,
}

impl PersistedInfo {
    pub fn update(&self, device: Option<&UsbDevice>) {
        if let Some(device) = device {
            self.vid_pid_content
                .set_text(device.vid_pid().as_deref().unwrap_or("-"));
            self.serial_content
                .set_text(device.serial().as_deref().unwrap_or("-"));
            self.persisted_content
                .set_text(device.persisted_guid.as_deref().unwrap_or("-"));
            self.description_content.set_text(
                device
                    .description
                    .as_deref()
                    .unwrap_or("No description available"),
            );
        } else {
            self.vid_pid_content.set_text("-");
            self.serial_content.set_text("-");
            self.persisted_content.set_text("-");
            self.description_content.set_text("No device selected");
        }
    }
}

impl PartialUi for PersistedInfo {
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

        // Controls
        nwg::Label::builder()
            .text("Persisted Info")
            .font(Some(&data.font_bold_big))
            .parent(parent_ref.unwrap())
            .build(&mut data.persisted_info)?;

        nwg::Frame::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.separator)?;

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
            .text("Persisted ID:")
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.persisted)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.persisted_content)?;

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
            // "Persisted Info" header
            .child(&data.persisted_info)
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
            // VID:PID label
            .child(&data.vid_pid)
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
            // Persisted ID label
            .child(&data.persisted)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // Persisted ID content
            .child(&data.persisted_content)
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
            .build(&data.info_layout)?;

        Ok(())
    }

    fn process_event(
        &self,
        _evt: nwg::Event,
        _evt_data: &nwg::EventData,
        _handle: nwg::ControlHandle,
    ) {
        // No events on PersistedInfo
    }

    fn handles(&self) -> Vec<&nwg::ControlHandle> {
        Vec::new()
    }
}
