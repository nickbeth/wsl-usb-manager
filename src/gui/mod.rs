// Triggered by gui::RESOURCES because of Nwg limitations, because no control handle is Sync
// This is fine to allow since these resources are only accessed from the same thread
#![allow(clippy::borrow_interior_mutable_const)]

mod auto_attach_tab;
mod connected_tab;
#[allow(clippy::module_inception)]
mod gui;
mod nwg_ext;
mod persisted_tab;
mod usbipd_gui;

pub use gui::*;
