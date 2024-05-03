use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use serde::{Deserialize, Serialize};

use crate::usbipd::UsbDevice;

#[derive(Serialize, Deserialize, Clone, Eq)]
pub struct AutoAttachProfile {
    /// Unique identifier of the profile (persisted_guid)
    pub id: String,
    pub description: Option<String>,
}

impl PartialEq for AutoAttachProfile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for AutoAttachProfile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Default)]
pub struct AutoAttacher {
    profiles: HashSet<AutoAttachProfile>,
    process_map: HashMap<String, std::process::Child>,
}

impl AutoAttacher {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_device(&mut self, device: &UsbDevice) -> Result<(), String> {
        let id = device
            .persisted_guid
            .clone()
            .ok_or("The device does not have a persisted GUID, are you sure it's bound?")?;

        // Auto attaching spawns a process that might fail immediately and exit silently
        // We cannot detect this failure as that would require waiting for the process to exit
        // As a workaround, attach the device manually first to catch any errors
        if !device.is_attached() {
            device.attach()?;
            device.wait(|d| d.is_some_and(|d| d.is_attached()))?;
        }

        if !self.profiles.insert(AutoAttachProfile {
            id: id.clone(),
            description: device.description.clone(),
        }) {
            return Err("The device is already in the auto attach list.".to_string());
        }

        let process = device.auto_attach()?;
        self.process_map.insert(id, process);

        Ok(())
    }

    pub fn remove(&mut self, profile: &AutoAttachProfile) -> Result<(), String> {
        self.profiles.remove(profile);

        if let Some(mut process) = self.process_map.remove(&profile.id) {
            let _ = process.kill();
        }

        Ok(())
    }

    pub fn profiles(&self) -> Vec<AutoAttachProfile> {
        self.profiles.iter().cloned().collect()
    }
}

impl Drop for AutoAttacher {
    fn drop(&mut self) {
        for (_, mut process) in self.process_map.drain() {
            let _ = process.kill();
        }
    }
}
