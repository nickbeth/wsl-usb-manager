use std::{
    cell::{Cell, RefCell},
    ops::Deref,
    rc::Rc,
};

use native_windows_gui::{self as nwg, RadioButtonState};
use nwg::stretch::{
    geometry::{Rect, Size},
    style::{Dimension as D, Dimension::Points as Pt, FlexDirection},
};

use crate::{
    auto_attacher::AutoAttacher,
    gui::{RESOURCES, helpers},
    usbipd::UsbDevice,
};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum AutoAttachMode {
    #[default]
    Device,
    Port,
}

#[derive(Default)]
pub struct AutoAttachWindow {
    device: UsbDevice,
    attach_mode: Cell<Option<AutoAttachMode>>,
    auto_attacher: Rc<RefCell<AutoAttacher>>,

    pub window: nwg::Window,
    main_layout: nwg::FlexboxLayout,

    label: nwg::Label,
    device_button: nwg::RadioButton,
    port_button: nwg::RadioButton,

    // Buttons
    buttons_frame: nwg::Frame,
    buttons_layout: nwg::FlexboxLayout,
    cancel_button: nwg::Button,
    ok_button: nwg::Button,
}

impl AutoAttachWindow {
    pub fn new(device: UsbDevice, auto_attacher: &Rc<RefCell<AutoAttacher>>) -> Self {
        Self {
            device,
            auto_attacher: auto_attacher.clone(),
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

        if !self.device.is_bound() || mode == Some(AutoAttachMode::Port) {
            self.ok_button.set_bitmap(Some(&RESOURCES.shield_bitmap));
        } else {
            self.ok_button.set_bitmap(None);
        }
    }

    fn auto_attach(&self) {
        let Some(attach_mode) = self.attach_mode.get() else {
            return;
        };

        // Show wait cursor while the command is running
        let window = self.window.handle;
        let wait_cursor = nwg::Cursor::from_system(nwg::OemCursor::Wait);
        let cursor_event =
            nwg::full_bind_event_handler(&window, move |event, _event_data, _handle| match event {
                nwg::Event::OnMousePress(_) | nwg::Event::OnMouseMove => {
                    nwg::GlobalCursor::set(&wait_cursor)
                }
                _ => {}
            });

        let attach_result = (|| {
            match attach_mode {
                AutoAttachMode::Device => {
                    // If the device isn't bound, bind it before adding the profile
                    if !self.device.is_bound() {
                        self.device.bind(false)?;
                        self.device.wait(|d| d.is_some_and(|d| !d.is_bound()))?;
                    }
                    self.auto_attacher.borrow_mut().add_device(&self.device)
                }
                AutoAttachMode::Port => self.auto_attacher.borrow_mut().add_port(&self.device),
            }
        })();

        nwg::unbind_event_handler(&cursor_event);
        match attach_result {
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
}

pub struct AutoAttachWindowUi {
    pub inner: Rc<AutoAttachWindow>,
    pub default_handlers: Vec<nwg::EventHandler>,
}

impl AutoAttachWindow {
    pub fn build_ui(
        mut data: Self,
        parent_window: &nwg::Window,
    ) -> Result<AutoAttachWindowUi, nwg::NwgError> {
        let description = data
            .device
            .description
            .as_deref()
            .unwrap_or("Unknown device");
        let bus_id = data.device.bus_id.as_deref().unwrap_or("Unknown port");

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

        // Wrap in Rc and bind events
        let inner = Rc::new(data);
        let evt_ui = Rc::downgrade(&inner);

        let window_handle = inner.window.handle;
        let default_handler =
            nwg::full_bind_event_handler(&window_handle, move |evt, _evt_data, handle| {
                if let Some(ui) = evt_ui.upgrade()
                    && evt == nwg::Event::OnButtonClick
                {
                    if handle == ui.device_button.handle {
                        AutoAttachWindow::select_device(&ui);
                    } else if handle == ui.port_button.handle {
                        AutoAttachWindow::select_port(&ui);
                    } else if handle == ui.cancel_button.handle {
                        AutoAttachWindow::close(&ui);
                    } else if handle == ui.ok_button.handle {
                        AutoAttachWindow::auto_attach(&ui);
                    }
                }
            });

        let ui = AutoAttachWindowUi {
            inner,
            default_handlers: vec![default_handler],
        };

        Ok(ui)
    }
}

impl Drop for AutoAttachWindowUi {
    fn drop(&mut self) {
        for handler in self.default_handlers.iter() {
            nwg::unbind_event_handler(handler);
        }
    }
}

impl Deref for AutoAttachWindowUi {
    type Target = AutoAttachWindow;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
