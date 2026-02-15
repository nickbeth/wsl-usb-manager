use native_windows_gui as nwg;
use nwg::PartialUi;
use nwg::stretch::{
    geometry::Size,
    style::{Dimension as D, Dimension::Points as Pt, FlexDirection},
};

use crate::auto_attach::AutoAttachProfile;

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

    info_layout: nwg::FlexboxLayout,

    auto_attach_info: nwg::Label,
    separator: nwg::Frame,
    persisted_id: nwg::Label,
    persisted_id_content: nwg::RichLabel,
    description: nwg::Label,
    description_content: nwg::RichLabel,
}

impl AutoAttachInfo {
    pub fn update(&self, profile: Option<&AutoAttachProfile>) {
        if let Some(profile) = profile {
            self.persisted_id_content.set_text(&profile.id);
            self.description_content.set_text(
                profile
                    .description
                    .as_deref()
                    .unwrap_or("No description available"),
            );
        } else {
            self.persisted_id_content.set_text("-");
            self.description_content.set_text("No profile selected");
        }
    }
}

impl PartialUi for AutoAttachInfo {
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
            .text("Auto Attach Info")
            .font(Some(&data.font_bold_big))
            .parent(parent_ref.unwrap())
            .build(&mut data.auto_attach_info)?;

        nwg::Frame::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.separator)?;

        nwg::Label::builder()
            .text("Persisted ID:")
            .font(Some(&data.font_bold))
            .v_align(nwg::VTextAlign::Bottom)
            .parent(parent_ref.unwrap())
            .build(&mut data.persisted_id)?;

        nwg::RichLabel::builder()
            .parent(parent_ref.unwrap())
            .build(&mut data.persisted_id_content)?;

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
            // Persisted ID label
            .child(&data.persisted_id)
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
            // Persisted ID content
            .child(&data.persisted_id_content)
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
        // No events on AutoAttachInfo
    }

    fn handles(&self) -> Vec<&nwg::ControlHandle> {
        Vec::new()
    }
}
