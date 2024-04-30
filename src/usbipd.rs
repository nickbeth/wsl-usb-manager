//! This module provides objects and functions for interacting with the `usbipd`
//! executable and the USB devices it manages.

use std::fmt::Display;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::time::{Duration, Instant};

use serde::Deserialize;
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SHELLEXECUTEINFOW_0};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

use crate::win_utils::get_last_error_string;

/// The `usbipd` executable name.
const USBIPD_EXE: &str = "usbipd";

/// An enum representing the state of a USB device in `usbipd`.
pub enum UsbipState {
    None,
    Persisted,
    Shared(bool),
    Attached(bool),
}

impl Display for UsbipState {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            UsbipState::None => write!(fmt, "Not shared")?,
            UsbipState::Persisted => write!(fmt, "Persisted")?,
            UsbipState::Shared(_) => write!(fmt, "Shared")?,
            UsbipState::Attached(_) => write!(fmt, "Attached")?,
        }

        match self {
            UsbipState::None | UsbipState::Persisted => Ok(()),
            UsbipState::Shared(forced) | UsbipState::Attached(forced) => {
                if *forced {
                    write!(fmt, " (forced)")
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// A struct representing a USB device as returned by `usbipd`.
#[derive(Debug, Deserialize)]
pub struct UsbDevice {
    #[serde(rename = "BusId")]
    pub bus_id: Option<String>,
    #[serde(rename = "ClientIPAddress")]
    pub client_ip_address: Option<String>,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "InstanceId")]
    pub instance_id: Option<String>,
    #[serde(rename = "IsForced")]
    pub is_forced: bool,
    #[serde(rename = "PersistedGuid")]
    pub persisted_guid: Option<String>,
    #[serde(rename = "StubInstanceGuid")]
    pub stub_instance_id: Option<String>,
}

impl UsbDevice {
    /// Returns whether the device is connected to the system.
    pub fn is_connected(&self) -> bool {
        self.bus_id.is_some()
    }

    /// Returns whether the device is shared by usbipd.
    pub fn is_bound(&self) -> bool {
        self.is_connected() && self.persisted_guid.is_some()
    }

    /// Returns whether the device is attached to a usbip client.
    pub fn is_attached(&self) -> bool {
        self.is_connected() && self.client_ip_address.is_some()
    }

    /// Returns the VID:PID of the device if available.
    pub fn vid_pid(&self) -> Option<String> {
        // USB\VID_XXXX&PID_XXXX\XXXX
        let instance_id = self.instance_id.as_deref()?;
        // VID_XXXX&PID_XXXX
        let vid_pid = instance_id.split('\\').nth(1)?;
        // VVVV:PPPP
        let vid_pid = vid_pid.replace("VID_", "").replace("&PID_", ":");

        Some(vid_pid)
    }

    /// Returns the serial number of the device if available.
    pub fn serial(&self) -> Option<String> {
        // USB\VID_XXXX&PID_XXXX\XXXX
        let instance_id = self.instance_id.as_deref()?;
        // XXXX
        let serial = instance_id.split('\\').nth(2)?;

        // Windows generates instance IDs for devices that do not provide a
        // serial number. Instance IDs are not persistent across disconnections,
        // therefore they do cannot be used to uniquely identify devices
        // They can be recognized by the presence of ampersands
        if serial.contains('&') {
            None
        } else {
            Some(serial.to_owned())
        }
    }

    /// Returns the state of the USB device as a `UsbipState` enum.
    pub fn state(&self) -> UsbipState {
        if self.bus_id.is_none() {
            UsbipState::Persisted
        } else if self.is_attached() {
            UsbipState::Attached(self.is_forced)
        } else if self.is_bound() {
            UsbipState::Shared(self.is_forced)
        } else {
            UsbipState::None
        }
    }

    /// Binds the device. Asks for admin privileges if necessary.
    pub fn bind(&self, force: bool) -> Result<(), String> {
        let bus_id = self
            .bus_id
            .as_deref()
            .ok_or("The device does not have a bus ID.".to_owned())?;

        let args = if force {
            ["bind", "--force", "--busid", bus_id].to_vec()
        } else {
            ["bind", "--busid", bus_id].to_vec()
        };

        usbipd(&args).or_else(|err| {
            if err.contains("administrator") {
                usbipd_admin(&args)
            } else {
                Err(err)
            }
        })
    }

    /// Unbinds the device. Asks for admin privileges if necessary.
    pub fn unbind(&self) -> Result<(), String> {
        let guid = self
            .persisted_guid
            .as_deref()
            .ok_or("The device is already unbound.".to_owned())?;

        let args = ["unbind", "--guid", guid].to_vec();

        usbipd(&args).or_else(|err| {
            if err.contains("administrator") {
                usbipd_admin(&args)
            } else {
                Err(err)
            }
        })
    }

    /// Attaches the device. Binds the device if necessary.
    pub fn attach(&self) -> Result<(), String> {
        let bus_id = self
            .bus_id
            .as_deref()
            .ok_or("The device does not have a bus ID.".to_owned())?;

        if !self.is_bound() {
            self.bind(false)?;
        }

        let args = if version().major < 4 {
            ["wsl", "attach", "--busid", bus_id].to_vec()
        } else {
            ["attach", "--wsl", "--busid", bus_id].to_vec()
        };

        usbipd(&args)
    }

    /// Detaches the device.
    pub fn detach(&self) -> Result<(), String> {
        let bus_id = self
            .bus_id
            .as_deref()
            .ok_or("The device does not have a bus ID.".to_owned())?;

        let args = if version().major < 4 {
            ["wsl", "detach", "--busid", bus_id].to_vec()
        } else {
            ["detach", "--busid", bus_id].to_vec()
        };

        usbipd(&args)
    }

    /// Waits until `wait_cond` is satisfied for the device.
    ///
    /// `wait_cond` receives an optional reference to the updated device.
    /// A value of `None` might mean that either the device was disconnected or
    /// that it was temporarily removed as part of a `usbipd` operation.
    /// Users of this function should take this into account when implementing `wait_cond`.
    /// `wait_cond` should return `true` when the device reaches the desired state
    /// and waiting should stop.
    ///
    /// The maximum wait time is 5 seconds, which takes into account the worst-case
    /// scenario of Windows remounting the USB device after a `usbipd` operation.
    /// If the wait times out, the device is assumed to be lost.
    pub fn wait(&self, wait_cond: fn(Option<&UsbDevice>) -> bool) -> Result<(), String> {
        let start = Instant::now();

        // Wait for the device to be in the desired state with a timeout
        while start.elapsed() < Duration::from_secs(5) {
            let devices = list_devices();
            let device = devices.iter().find(|d| d.instance_id == self.instance_id);
            // Pass Option as we might want to check for the device being removed
            if wait_cond(device) {
                return Ok(());
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // Assume the device was disconnected if the maximum wait time was reached
        Err("The device was lost while waiting for the operation to complete.".to_owned())
    }
}

/// Retrieves the list of USB devices from `usbipd`.
pub fn list_devices() -> Vec<UsbDevice> {
    let state_str = {
        let cmd = Command::new(USBIPD_EXE)
            .arg("state")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .unwrap();

        String::from_utf8(cmd.stdout).unwrap()
    };

    #[derive(Deserialize)]
    struct StateResult {
        #[serde(rename = "Devices")]
        devices: Vec<UsbDevice>,
    }

    let state_res: StateResult = serde_json::from_str(&state_str).unwrap();
    state_res.devices
}

/// Executes `usbipd` with the given arguments.
fn usbipd<'a, I>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = &'a &'a str>,
{
    match Command::new(USBIPD_EXE)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8(output.stderr).unwrap())
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

/// Executes `usbipd` as administrator with the given arguments.
fn usbipd_admin<'a, I>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = &'a &'a str>,
{
    // Build a space-separated string of arguments
    let mut args_str: String = String::new();
    for arg in args {
        args_str.push_str(&format!("{arg} "));
    }
    // Remove the trailing comma
    args_str.pop();
    // Insert a null terminator
    args_str.push('\0');

    // Prepare u16 strings
    let verb = "runas\0".encode_utf16().collect::<Vec<_>>();
    let file = (USBIPD_EXE.to_owned() + "\0")
        .encode_utf16()
        .collect::<Vec<_>>();
    let params = args_str.encode_utf16().collect::<Vec<_>>();

    let mut shell_exec_info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: 0,
        hwnd: 0,
        lpVerb: verb.as_ptr(),
        lpFile: file.as_ptr(),
        lpParameters: params.as_ptr(),
        lpDirectory: std::ptr::null(),
        nShow: SW_HIDE,
        hInstApp: 0,
        lpIDList: std::ptr::null_mut(),
        lpClass: std::ptr::null(),
        hkeyClass: 0,
        dwHotKey: 0,
        Anonymous: SHELLEXECUTEINFOW_0 { hMonitor: 0 },
        hProcess: 0,
    };

    if unsafe { ShellExecuteExW(&mut shell_exec_info as *mut _) } == 0 {
        Err(get_last_error_string())
    } else {
        Ok(())
    }
}

/// A `ubpidp` version struct with major, minor, and patch fields.
#[allow(unused)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// Returns the version of `usbipd`, split into major, minor, and patch fields.
pub fn version() -> Version {
    let cmd = Command::new(USBIPD_EXE)
        .arg("--version")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .unwrap();
    let version_string = String::from_utf8(cmd.stdout).unwrap();

    let version_split: Vec<&str> = version_string.split('+').collect();
    let version_parts: Vec<&str> = version_split.first().unwrap().split('.').collect();

    let parse = |i| -> u32 {
        version_parts
            .get(i)
            .and_then(|part: &&str| part.parse().ok())
            .unwrap_or(0)
    };

    Version {
        major: parse(0),
        minor: parse(1),
        patch: parse(2),
    }
}
