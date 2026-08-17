use std::cell::RefCell;
use std::rc::Rc;

use native_windows_gui as nwg;
use nwg::PartialUi;
use nwg::stretch::{
    geometry::Size,
    style::{Dimension as D, Dimension::Points as Pt, FlexDirection},
};

use crate::i18n::{Localizable, Translator, keys};
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

    /// Active translator. Injected by the parent tab via `set_translator` and
    /// kept in sync on language switch. `Rc` so the snapshot is cheap to share.
    translator: RefCell<Rc<Translator>>,

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
    /// Updates the shared translator snapshot. Called on init and on every
    /// language switch.
    pub fn set_translator(&self, t: Rc<Translator>) {
        *self.translator.borrow_mut() = t;
    }

    pub fn update(&self, device: Option<&UsbDevice>) {
        let t = self.translator.borrow().clone();
        if let Some(device) = device {
            self.bus_id_content
                .set_text(device.bus_id.as_deref().unwrap_or("-"));
            self.vid_pid_content
                .set_text(device.vid_pid().as_deref().unwrap_or("-"));
            self.serial_content
                .set_text(device.serial().as_deref().unwrap_or("-"));
            self.state_content
                .set_text(&device.state().to_localized_string(&t));
            // Collect into an owned `String` so the translator borrow (with its
            // own lifetime) does not collide with the device borrow.
            let description = device
                .description
                .clone()
                .unwrap_or_else(|| t.get(keys::PLACEHOLDER_NO_DESCRIPTION).to_owned());
            self.description_content.set_text(&description);
        } else {
            self.bus_id_content.set_text("-");
            self.vid_pid_content.set_text("-");
            self.serial_content.set_text("-");
            self.state_content
                .set_text(&UsbipState::None.to_localized_string(&t));
            self.description_content
                .set_text(t.get(keys::PLACEHOLDER_NO_DEVICE_SELECTED));
        }
    }
}

impl Localizable for DeviceInfo {
    fn re_localize(&self, t: &Translator) {
        // Static labels only; dynamic values are repainted by `update`, which
        // the parent tab invokes immediately after this through
        // `update_device_details`.
        self.device_info.set_text(t.get(keys::INFO_DEVICE_TITLE));
        self.bus_id
            .set_text(t.get(keys::INFO_DEVICE_BUS_ID_LABEL));
        self.vid_pid
            .set_text(t.get(keys::INFO_DEVICE_VID_PID_LABEL));
        self.serial
            .set_text(t.get(keys::INFO_DEVICE_SERIAL_LABEL));
        self.state.set_text(t.get(keys::INFO_DEVICE_STATE_LABEL));
        self.description
            .set_text(t.get(keys::INFO_DEVICE_DESCRIPTION_LABEL));
    }
}

impl PartialUi for DeviceInfo {
    fn build_partial<W: Into<nwg::ControlHandle>>(
        data: &mut Self,
        parent: Option<W>,
    ) -> Result<(), nwg::NwgError> {
        let parent = parent.map(|p| p.into());
        let parent_ref = parent.as_ref();

        // Initial English text; the real language is applied by `re_localize`
        // once the parent injects the active translator.
        let t = Translator::default();

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
            .text(t.get(keys::INFO_DEVICE_TITLE))
            .font(Some(&data.font_bold_big))
            .parent(parent_ref.unwrap())
            .build(&mut data.device_info)?;

        nwg::Frame::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.separator)?;

        nwg::Label::builder()
            .text(t.get(keys::INFO_DEVICE_BUS_ID_LABEL))
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.bus_id)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.bus_id_content)?;

        nwg::Label::builder()
            .text(t.get(keys::INFO_DEVICE_VID_PID_LABEL))
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.vid_pid)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.vid_pid_content)?;

        nwg::Label::builder()
            .text(t.get(keys::INFO_DEVICE_SERIAL_LABEL))
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.serial)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.serial_content)?;

        nwg::Label::builder()
            .text(t.get(keys::INFO_DEVICE_STATE_LABEL))
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.state)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.state_content)?;

        nwg::Label::builder()
            .text(t.get(keys::INFO_DEVICE_DESCRIPTION_LABEL))
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
