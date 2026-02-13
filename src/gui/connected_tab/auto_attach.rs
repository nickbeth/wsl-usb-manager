use std::{
    cell::{Cell, RefCell},
    ops::Deref,
    rc::Rc,
};

use native_windows_gui::{self as nwg, NativeUi, RadioButtonState};
use nwg::stretch::{
    geometry::{Rect, Size},
    style::{Dimension as D, Dimension::Points as Pt, FlexDirection},
};

use crate::{auto_attach::AutoAttacher, gui::RESOURCES, usbipd::UsbDevice};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum AutoAttachMode {
    #[default]
    Device,
    Port,
    Both,
}

#[derive(Default)]
pub struct AutoAttachWindow {
    device: UsbDevice,
    attach_mode: Cell<Option<AutoAttachMode>>,
    auto_attacher: Rc<RefCell<AutoAttacher>>,

    parent_window: Rc<nwg::Window>,
    window: nwg::Window,
    main_layout: nwg::FlexboxLayout,

    label: nwg::Label,
    device_button: nwg::RadioButton,
    port_button: nwg::RadioButton,
    both_button: nwg::RadioButton,

    // Buttons
    buttons_frame: nwg::Frame,
    buttons_layout: nwg::FlexboxLayout,
    cancel_button: nwg::Button,
    ok_button: nwg::Button,
}

impl AutoAttachWindow {
    pub fn new(
        window: &Rc<nwg::Window>,
        device: UsbDevice,
        auto_attacher: &Rc<RefCell<AutoAttacher>>,
    ) -> Self {
        Self {
            device,
            auto_attacher: auto_attacher.clone(),
            parent_window: window.clone(),
            ..Default::default()
        }
    }

    fn select_device(&self) {
        self.attach_mode.set(Some(AutoAttachMode::Device));
        self.update_checked();
    }

    fn select_port(&self) {
        self.attach_mode.set(Some(AutoAttachMode::Port));
        self.update_checked();
    }

    fn select_both(&self) {
        self.attach_mode.set(Some(AutoAttachMode::Both));
        self.update_checked();
    }

    fn update_checked(&self) {
        macro_rules! radio_state {
            ($checked:expr) => {
                if $checked {
                    RadioButtonState::Checked
                } else {
                    RadioButtonState::Unchecked
                }
            };
        }

        let mode = self.attach_mode.get();
        self.ok_button.set_enabled(mode.is_some());

        self.device_button
            .set_check_state(radio_state!(mode == Some(AutoAttachMode::Device)));
        self.port_button
            .set_check_state(radio_state!(mode == Some(AutoAttachMode::Port)));
        self.both_button
            .set_check_state(radio_state!(mode == Some(AutoAttachMode::Both)));
    }

    fn auto_attach(&self) {
        match self.auto_attacher.borrow_mut().add_device(&self.device) {
            Ok(()) => {
                self.close();
            }
            Err(e) => {
                nwg::modal_error_message(
                    self.window.handle,
                    "WSL USB Manager: Auto Attach Error",
                    &e,
                );
            }
        }
    }

    fn close(&self) {
        self.window.close();
    }

    fn enable_parent(&self, enable: bool) {
        self.parent_window.set_enabled(enable);
    }
}

pub struct AutoAttachWindowUi {
    inner: Rc<AutoAttachWindow>,
    default_handler: nwg::EventHandler,
}

impl NativeUi<AutoAttachWindowUi> for AutoAttachWindow {
    fn build_ui(mut data: Self) -> Result<AutoAttachWindowUi, nwg::NwgError> {
        let description = data
            .device
            .description
            .as_deref()
            .unwrap_or("Unknown device");
        let bus_id = data.device.bus_id.as_deref().unwrap_or("Unknown port");

        // Compute centered position relative to parent window (DPI-aware)
        let child_size = (360, 200);
        let (px, py) = data.parent_window.position();
        let (pw, ph) = data.parent_window.size();
        let position = (
            px + (pw as i32 - child_size.0) / 2,
            py + (ph as i32 - child_size.1) / 2,
        );

        // Window
        nwg::Window::builder()
            .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE | nwg::WindowFlags::POPUP)
            .parent(Some(data.parent_window.handle))
            .size(child_size)
            .position(position)
            .title("Configure Auto Attach")
            .icon(Some(&RESOURCES.app_icon))
            .build(&mut data.window)?;

        // Controls
        nwg::Label::builder()
            .flags(nwg::LabelFlags::VISIBLE)
            .parent(&data.window)
            .text("Select an auto attach mode:")
            .build(&mut data.label)?;

        nwg::RadioButton::builder()
            .flags(nwg::RadioButtonFlags::VISIBLE | nwg::RadioButtonFlags::GROUP)
            .parent(&data.window)
            .text(&format!("Device: {}", description))
            .build(&mut data.device_button)?;

        nwg::RadioButton::builder()
            .flags(nwg::RadioButtonFlags::VISIBLE)
            .parent(&data.window)
            .text(&format!("Port: {}", bus_id))
            .build(&mut data.port_button)?;

        nwg::RadioButton::builder()
            .flags(nwg::RadioButtonFlags::VISIBLE)
            .parent(&data.window)
            .text(&format!("Both: {} on {}", description, bus_id))
            .build(&mut data.both_button)?;

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
            .enabled(false) // Initially disabled until an option is selected
            .build(&mut data.ok_button)?;

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
            .child(&data.device_button)
            .child_size(Size {
                width: D::Auto,
                height: Pt(25.0),
            })
            .child(&data.port_button)
            .child_size(Size {
                width: D::Auto,
                height: Pt(25.0),
            })
            .child(&data.both_button)
            .child_size(Size {
                width: D::Auto,
                height: Pt(25.0),
            })
            .child(&data.buttons_frame)
            .child_size(Size {
                width: D::Auto,
                height: Pt(25.0),
            })
            .child_margin(Rect {
                start: Pt(0.0),
                end: Pt(0.0),
                top: Pt(15.0),
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

        // Disable the parent window to make this a modal dialog
        data.enable_parent(false);

        // Wrap in Rc and bind events
        let inner = Rc::new(data);
        let evt_ui = Rc::downgrade(&inner);

        let window_handle = inner.window.handle;
        let default_handler =
            nwg::full_bind_event_handler(&window_handle, move |evt, _evt_data, handle| {
                if let Some(ui) = evt_ui.upgrade() {
                    match evt {
                        nwg::Event::OnWindowClose => {
                            if handle == ui.window.handle {
                                AutoAttachWindow::enable_parent(&ui, true);
                            }
                        }
                        nwg::Event::OnButtonClick => {
                            if handle == ui.device_button.handle {
                                AutoAttachWindow::select_device(&ui);
                            } else if handle == ui.port_button.handle {
                                AutoAttachWindow::select_port(&ui);
                            } else if handle == ui.both_button.handle {
                                AutoAttachWindow::select_both(&ui);
                            } else if handle == ui.cancel_button.handle {
                                AutoAttachWindow::close(&ui);
                            } else if handle == ui.ok_button.handle {
                                AutoAttachWindow::auto_attach(&ui);
                            }
                        }
                        _ => {}
                    }
                }
            });

        let ui = AutoAttachWindowUi {
            inner,
            default_handler,
        };

        Ok(ui)
    }
}

impl Drop for AutoAttachWindowUi {
    fn drop(&mut self) {
        nwg::unbind_event_handler(&self.default_handler);
    }
}

impl Deref for AutoAttachWindowUi {
    type Target = AutoAttachWindow;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
