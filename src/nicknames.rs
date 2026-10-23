//! This module provides persistent, user-defined nicknames for USB devices.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::usbipd::UsbDevice;

#[derive(Default)]
pub struct Nicknames {
    storage_path: Option<PathBuf>,
    nicknames: BTreeMap<String, String>,
}

impl Nicknames {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_storage(storage_path: &Path) -> Self {
        let mut nicknames = Nicknames {
            storage_path: Some(storage_path.to_owned()),
            ..Default::default()
        };

        let Ok(mut file) = File::open(storage_path) else {
            return nicknames;
        };

        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).is_err() {
            return nicknames;
        }

        if let Ok(persisted) = serde_json::from_slice(&buf) {
            nicknames.nicknames = persisted;
        }

        nicknames
    }

    /// Returns the key used to identify a device across reconnections.
    ///
    /// Only devices that report a serial number can be uniquely identified,
    /// devices without one have no key and cannot be given a nickname.
    pub fn key(device: &UsbDevice) -> Option<String> {
        Some(format!("{}/{}", device.vid_pid()?, device.serial()?))
    }

    /// Returns whether the device can be given a nickname.
    pub fn supports(device: &UsbDevice) -> bool {
        Self::key(device).is_some()
    }

    /// Returns the nickname of the device, if one was set.
    pub fn get(&self, device: &UsbDevice) -> Option<&str> {
        let key = Self::key(device)?;
        self.nicknames.get(&key).map(String::as_str)
    }

    /// Sets the nickname of the device. An empty nickname removes it.
    pub fn set(&mut self, device: &UsbDevice, nickname: &str) -> Result<(), String> {
        let key = Self::key(device)
            .ok_or("The device does not have a serial number and cannot be given a nickname.")?;

        let nickname = nickname.trim();
        if nickname.is_empty() {
            self.nicknames.remove(&key);
        } else {
            self.nicknames.insert(key, nickname.to_owned());
        }

        self.persist()
    }

    fn persist(&self) -> Result<(), String> {
        let Some(storage_path) = &self.storage_path else {
            // Skip if no storage path was provided
            return Ok(());
        };

        let serialized =
            serde_json::to_string_pretty(&self.nicknames).map_err(|e| e.to_string())?;

        let mut file =
            File::create(storage_path).map_err(|e| format!("Failed to save nicknames: {}", e))?;

        file.write_all(serialized.as_bytes())
            .map_err(|e| format!("Failed to save nicknames: {}", e))
    }
}
