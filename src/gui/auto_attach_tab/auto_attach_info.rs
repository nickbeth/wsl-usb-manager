use native_windows_derive::NwgPartial;
use native_windows_gui as nwg;

use nwg::stretch::{
    geometry::{Rect, Size},
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
#[derive(Default, NwgPartial)]
pub struct AutoAttachInfo {
    #[nwg_resource(family: "Segoe UI Semibold", size: 16, weight: 400)]
    font_bold: nwg::Font,

    #[nwg_resource(family: "Segoe UI Semibold", size: 20, weight: 400)]
    font_bold_big: nwg::Font,

    #[nwg_layout(flex_direction: FlexDirection::Column, auto_spacing: None)]
    info_layout: nwg::FlexboxLayout,

    #[nwg_control(text: "Auto Attach Info", font: Some(&data.font_bold_big))]
    #[nwg_layout_item(layout: info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    auto_attach_info: nwg::Label,

    #[nwg_control]
    #[nwg_layout_item(layout: info_layout, size: Size { width: D::Auto, height: Pt(1.0) },
        margin: Rect { start: Pt(0.0), end: Pt(0.0), top: Pt(5.0), bottom: Pt(0.0)}
    )]
    separator: nwg::Frame,

    #[nwg_control(text: "Persisted ID:", font: Some(&data.font_bold), v_align: nwg::VTextAlign::Bottom)]
    #[nwg_layout_item(layout: info_layout, size: Size { width: D::Auto, height: Pt(20.0)},
        margin: Rect { start: Pt(0.0), end: Pt(0.0), top: Pt(6.0), bottom: Pt(0.0)}
    )]
    persisted_id: nwg::Label,

    #[nwg_control]
    #[nwg_layout_item(layout: info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    persisted_id_content: nwg::RichLabel,

    #[nwg_control(text: "Description:", font: Some(&data.font_bold), v_align: nwg::VTextAlign::Bottom)]
    #[nwg_layout_item(layout: info_layout, size: Size { width: D::Auto, height: Pt(20.0) })]
    description: nwg::Label,

    #[nwg_control(flags: "VISIBLE|MULTI_LINE")]
    #[nwg_layout_item(layout: info_layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
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
