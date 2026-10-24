use std::{cell::RefCell, ops::Deref, rc::Rc};

use native_windows_gui as nwg;
use nwg::stretch::{
    geometry::{Rect, Size},
    style::{Dimension as D, Dimension::Points as Pt, FlexDirection},
};

use crate::{
    gui::{RESOURCES, helpers},
    nicknames::Nicknames,
    usbipd::UsbDevice,
};

/// The maximum length of a nickname, in characters.
const MAX_NICKNAME_LENGTH: usize = 64;

#[derive(Default)]
pub struct NicknameWindow {
    device: UsbDevice,
    nicknames: Rc<RefCell<Nicknames>>,

    pub window: nwg::Window,
    main_layout: nwg::FlexboxLayout,

    label: nwg::Label,
    nickname_input: nwg::TextInput,
    hint_label: nwg::Label,

    // Buttons
    buttons_frame: nwg::Frame,
    buttons_layout: nwg::FlexboxLayout,
    cancel_button: nwg::Button,
    ok_button: nwg::Button,
}

impl NicknameWindow {
    pub fn new(device: UsbDevice, nicknames: &Rc<RefCell<Nicknames>>) -> Self {
        Self {
            device,
            nicknames: nicknames.clone(),
            ..Default::default()
        }
    }

    fn save(&self) {
        let nickname = self.nickname_input.text();
        let result = self.nicknames.borrow_mut().set(&self.device, &nickname);

        match result {
            Ok(()) => self.close(),
            Err(e) => {
                nwg::modal_error_message(self.window.handle, "WSL USB Manager: Nickname Error", &e);
            }
        }
    }

    fn close(&self) {
        self.window.close();
    }
}

pub struct NicknameWindowUi {
    pub inner: Rc<NicknameWindow>,
    pub default_handlers: Vec<nwg::EventHandler>,
}

impl NicknameWindow {
    pub fn build_ui(
        mut data: Self,
        parent_window: &nwg::Window,
    ) -> Result<NicknameWindowUi, nwg::NwgError> {
        let description = data
            .device
            .description
            .as_deref()
            .unwrap_or("Unknown device");
        let current_nickname = data
            .nicknames
            .borrow()
            .get(&data.device)
            .unwrap_or_default()
            .to_owned();

        // Truncate long descriptions
        const MAX_LENGTH: usize = 50;
        let description = helpers::ellipsize_middle(description, MAX_LENGTH);

        // Compute centered position relative to parent window (DPI-aware)
        let child_size = (385, 176);
        let (px, py) = parent_window.position();
        let (pw, ph) = parent_window.size();
        let position = (
            px + (pw as i32 - child_size.0) / 2,
            py + (ph as i32 - child_size.1) / 2,
        );

        // Window
        nwg::Window::builder()
            .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE | nwg::WindowFlags::POPUP)
            .parent(Some(parent_window.handle))
            .size(child_size)
            .position(position)
            .title("Set Nickname")
            .icon(Some(&RESOURCES.app_icon))
            .build(&mut data.window)?;

        // Controls
        nwg::Label::builder()
            .flags(nwg::LabelFlags::VISIBLE)
            .parent(&data.window)
            .text(&format!("Nickname for {}:", description))
            .build(&mut data.label)?;

        nwg::TextInput::builder()
            .parent(&data.window)
            .text(&current_nickname)
            .limit(MAX_NICKNAME_LENGTH)
            .focus(true)
            .build(&mut data.nickname_input)?;

        nwg::Label::builder()
            .flags(nwg::LabelFlags::VISIBLE)
            .parent(&data.window)
            .text("Leave empty to remove the nickname.")
            .build(&mut data.hint_label)?;

        nwg::Frame::builder()
            .flags(nwg::FrameFlags::VISIBLE)
            .parent(&data.window)
            .build(&mut data.buttons_frame)?;

        nwg::Button::builder()
            .parent(&data.buttons_frame)
            .text("Cancel")
            .build(&mut data.cancel_button)?;

        nwg::Button::builder()
            .parent(&data.buttons_frame)
            .text("OK")
            .build(&mut data.ok_button)?;

        // Select the current nickname so that typing replaces it
        data.nickname_input
            .set_selection(0..current_nickname.chars().count() as u32);

        // Layouts
        nwg::FlexboxLayout::builder()
            .parent(&data.window)
            .flex_direction(FlexDirection::Column)
            .padding(Rect {
                start: Pt(15.0),
                end: Pt(15.0),
                top: Pt(10.0),
                bottom: Pt(0.0),
            })
            .child(&data.label)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            .child_margin(Rect {
                start: Pt(0.0),
                end: Pt(0.0),
                top: Pt(0.0),
                bottom: Pt(4.0),
            })
            .child(&data.nickname_input)
            .child_size(Size {
                width: D::Auto,
                height: Pt(25.0),
            })
            .child(&data.hint_label)
            .child_size(Size {
                width: D::Auto,
                height: Pt(20.0),
            })
            .child_margin(Rect {
                start: Pt(0.0),
                end: Pt(0.0),
                top: Pt(4.0),
                bottom: Pt(0.0),
            })
            .child(&data.buttons_frame)
            .child_size(Size {
                width: D::Auto,
                height: Pt(25.0),
            })
            .child_margin(Rect {
                start: Pt(0.0),
                end: Pt(0.0),
                top: Pt(11.0),
                bottom: Pt(0.0),
            })
            .build(&data.main_layout)?;

        nwg::FlexboxLayout::builder()
            .parent(&data.buttons_frame)
            .flex_direction(FlexDirection::RowReverse)
            .auto_spacing(None)
            .child(&data.cancel_button)
            .child_size(Size {
                width: Pt(80.0),
                height: D::Auto,
            })
            .child(&data.ok_button)
            .child_size(Size {
                width: Pt(80.0),
                height: D::Auto,
            })
            .child_margin(Rect {
                start: Pt(0.0),
                end: Pt(5.0),
                top: Pt(0.0),
                bottom: Pt(0.0),
            })
            .build(&data.buttons_layout)?;

        // Wrap in Rc and bind events
        let inner = Rc::new(data);
        let evt_ui = Rc::downgrade(&inner);

        let window_handle = inner.window.handle;
        let default_handler =
            nwg::full_bind_event_handler(&window_handle, move |evt, evt_data, handle| {
                let Some(ui) = evt_ui.upgrade() else {
                    return;
                };

                match evt {
                    nwg::Event::OnButtonClick => {
                        if handle == ui.cancel_button.handle {
                            NicknameWindow::close(&ui);
                        } else if handle == ui.ok_button.handle {
                            NicknameWindow::save(&ui);
                        }
                    }
                    nwg::Event::OnKeyPress => {
                        if handle == ui.nickname_input.handle {
                            match evt_data.on_key() {
                                nwg::keys::RETURN => NicknameWindow::save(&ui),
                                nwg::keys::ESCAPE => NicknameWindow::close(&ui),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            });

        let ui = NicknameWindowUi {
            inner,
            default_handlers: vec![default_handler],
        };

        Ok(ui)
    }
}

impl Drop for NicknameWindowUi {
    fn drop(&mut self) {
        for handler in self.default_handlers.iter() {
            nwg::unbind_event_handler(handler);
        }
    }
}

impl Deref for NicknameWindowUi {
    type Target = NicknameWindow;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
