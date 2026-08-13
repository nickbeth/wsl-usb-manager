use std::cell::RefCell;
use std::rc::Rc;

use native_windows_gui as nwg;
use nwg::PartialUi;
use nwg::stretch::{
    geometry::Size,
    style::{Dimension as D, Dimension::Points as Pt, FlexDirection},
};

use crate::auto_attacher::{Profile, ProfileInfo};
use crate::i18n::{Localizable, Translator, keys};

/// The auto attach profile info tab.
/// It displays detailed information about an auto attach profile.
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
pub struct AutoAttachInfo {
    font_bold: nwg::Font,
    font_bold_big: nwg::Font,

    /// Active translator. Injected by the parent tab via `set_translator` and
    /// kept in sync on language switch.
    translator: RefCell<Rc<Translator>>,

    info_layout: nwg::FlexboxLayout,

    auto_attach_info: nwg::Label,
    separator: nwg::Frame,
    bus_id: nwg::Label,
    bus_id_content: nwg::RichLabel,
    description: nwg::Label,
    description_content: nwg::RichLabel,
    status: nwg::Label,
    status_content: nwg::RichLabel,
    last_error: nwg::Label,
    last_error_content: nwg::RichLabel,
}

impl AutoAttachInfo {
    /// Updates the shared translator snapshot. Called on init and on every
    /// language switch.
    pub fn set_translator(&self, t: Rc<Translator>) {
        *self.translator.borrow_mut() = t;
    }

    pub fn update(&self, info: Option<&ProfileInfo>) {
        let t = self.translator.borrow().clone();
        if info.is_none() {
            self.bus_id_content.set_text("-");
            self.description_content
                .set_text(t.get(keys::PLACEHOLDER_NO_PROFILE_SELECTED));
            self.last_error_content.set_text("");
            return;
        }

        let info = info.unwrap();

        match &info.profile {
            Profile::Device { hw_id, description } => {
                self.bus_id
                    .set_text(t.get(keys::INFO_AUTO_ATTACH_HARDWARE_ID_LABEL));
                self.bus_id_content.set_text(hw_id);
                let description = description
                    .clone()
                    .unwrap_or_else(|| t.get(keys::PLACEHOLDER_NO_DESCRIPTION).to_owned());
                self.description_content.set_text(&description);
            }
            Profile::Port { bus_id } => {
                self.bus_id
                    .set_text(t.get(keys::INFO_AUTO_ATTACH_BUS_ID_LABEL));
                self.bus_id_content.set_text(bus_id);
                self.description_content
                    .set_text(t.get(keys::PLACEHOLDER_ANY_DEVICE_PORT));
            }
        }

        self.status_content
            .set_text(if info.active {
                t.get(keys::STATUS_ACTIVE)
            } else {
                t.get(keys::STATUS_INACTIVE)
            });
        let last_error = info
            .last_error
            .clone()
            .unwrap_or_else(|| t.get(keys::PLACEHOLDER_NO_ERROR).to_owned());
        self.last_error_content.set_text(&last_error);
    }
}

impl Localizable for AutoAttachInfo {
    fn re_localize(&self, t: &Translator) {
        // Static labels only. The `bus_id` label is dynamic (Hardware ID vs
        // Bus ID depending on profile kind), so we set the default here and
        // let `update` overwrite it with the correct value immediately after.
        self.auto_attach_info
            .set_text(t.get(keys::INFO_AUTO_ATTACH_TITLE));
        self.bus_id
            .set_text(t.get(keys::INFO_AUTO_ATTACH_BUS_ID_LABEL));
        self.description
            .set_text(t.get(keys::INFO_AUTO_ATTACH_DESCRIPTION_LABEL));
        self.status
            .set_text(t.get(keys::INFO_AUTO_ATTACH_STATUS_LABEL));
        self.last_error
            .set_text(t.get(keys::INFO_AUTO_ATTACH_LAST_ERROR_LABEL));
    }
}

impl PartialUi for AutoAttachInfo {
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

        // Controls
        nwg::Label::builder()
            .text(t.get(keys::INFO_AUTO_ATTACH_TITLE))
            .font(Some(&data.font_bold_big))
            .parent(parent_ref.unwrap())
            .build(&mut data.auto_attach_info)?;

        nwg::Frame::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.separator)?;

        nwg::Label::builder()
            .text(t.get(keys::INFO_AUTO_ATTACH_BUS_ID_LABEL))
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.bus_id)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.bus_id_content)?;

        nwg::Label::builder()
            .text(t.get(keys::INFO_AUTO_ATTACH_DESCRIPTION_LABEL))
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.description)?;

        nwg::RichLabel::builder()
            .flags(nwg::RichLabelFlags::VISIBLE | nwg::RichLabelFlags::MULTI_LINE)
            .parent(parent_ref.unwrap())
            .build(&mut data.description_content)?;

        nwg::Label::builder()
            .text(t.get(keys::INFO_AUTO_ATTACH_STATUS_LABEL))
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.status)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.status_content)?;

        nwg::Label::builder()
            .text(t.get(keys::INFO_AUTO_ATTACH_LAST_ERROR_LABEL))
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.last_error)?;

        nwg::RichLabel::builder()
            .flags(nwg::RichLabelFlags::VISIBLE | nwg::RichLabelFlags::MULTI_LINE)
            .parent(parent_ref.unwrap())
            .build(&mut data.last_error_content)?;

        // Layout
        nwg::FlexboxLayout::builder()
            .parent(parent_ref.unwrap())
            .flex_direction(FlexDirection::Column)
            .auto_spacing(None)
            // "Auto Attach Info" header
            .child(&data.auto_attach_info)
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
            // Description label
            .child(&data.description)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // Description content (multi-line, min-height)
            .child(&data.description_content)
            .child_size(Size {
                width: D::Auto,
                height: D::Auto,
            })
            .child_min_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // Status label
            .child(&data.status)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // Status content
            .child(&data.status_content)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // Last error label
            .child(&data.last_error)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            // Last error content (multi-line, min-height)
            .child(&data.last_error_content)
            .child_size(Size {
                width: D::Auto,
                height: D::Auto,
            })
            .child_min_size(Size {
                width: D::Auto,
                height: Pt(20.0),
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
        // No events on AutoAttachInfo
    }

    fn handles(&self) -> Vec<&nwg::ControlHandle> {
        Vec::new()
    }
}
