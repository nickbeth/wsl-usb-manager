use std::{collections::HashMap, hash::Hash, io::Read, os::windows::io::AsRawHandle};

use serde::{Deserialize, Serialize};

use crate::{
    usbipd::{self, UsbDevice},
    win_utils,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Profile {
    Device {
        hw_id: String,
        description: Option<String>,
    },
    Port {
        bus_id: String,
    },
}

#[derive(Default)]
struct ProfileData {
    process: Option<std::process::Child>,
    last_error: Option<String>,
}

pub struct ProfileInfo {
    pub profile: Profile,
    pub active: bool,
    pub last_error: Option<String>,
}

impl ProfileData {
    pub fn update_process_status(&mut self) {
        // Check if a process is active for this profile
        let Some(process) = self.process.as_mut() else {
            return;
        };
        println!("Checking process status for profile...");
        // Poll the process to see if it exited (likely because of an error)
        let Ok(Some(exit_status)) = process.try_wait() else {
            println!("Process is still running.");
            return;
        };
        println!("Process exited with status: {:?}", exit_status);

        // Should not happen, ignore
        if exit_status.success() {
            return;
        }

        println!("Process for profile exited with an error, capturing stderr...");
        // Check that we can read stderr
        let mut stderr = process
            .stderr
            .take()
            .expect("Failed to take stderr of the process");

        // Peek the pipe to see if there is data available, since read() blocks indefinitely if the
        // process was killed before it could send EOF
        match win_utils::peek_pipe(stderr.as_raw_handle()) {
            None | Some(0) => {
                self.last_error = Some("Process exited with no error message.".to_string());
            }

            Some(bytes_available) => {
                let mut buf = vec![0u8; bytes_available as usize];
                let _ = stderr.read(&mut buf);

                let error_str = String::from_utf8_lossy(&buf).to_string();

                self.last_error = Some(error_str);
            }
        }

        self.process = None;
    }
}

impl Drop for ProfileData {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
    }
}

#[derive(Default)]
pub struct AutoAttacher {
    profiles: HashMap<Profile, ProfileData>,
}

impl AutoAttacher {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_device(&mut self, device: &UsbDevice) -> Result<(), String> {
        let new_profile = Profile::Device {
            hw_id: device
                .vid_pid()
                .as_ref()
                .ok_or("The device does not have a hardware ID.")?
                .clone(),
            description: device.description.clone(),
        };

        if self.profiles.contains_key(&new_profile) {
            return Err("The device is already in the auto attach list.".to_string());
        }

        self.activate_profile(new_profile)
    }

    pub fn add_port(&mut self, device: &UsbDevice) -> Result<(), String> {
        let new_profile = Profile::Port {
            bus_id: device
                .bus_id
                .as_ref()
                .ok_or("The device does not have a bus ID.".to_string())?
                .to_owned(),
        };

        if self.profiles.contains_key(&new_profile) {
            return Err("The port is already in the auto attach list.".to_string());
        }

        self.activate_profile(new_profile)
    }

    pub fn activate_profile(&mut self, profile: Profile) -> Result<(), String> {
        let process = match &profile {
            Profile::Device { hw_id, .. } => usbipd::auto_attach_device(hw_id),
            Profile::Port { bus_id } => {
                // Binds the device as a (wanted) side effect
                usbipd::policy_add(bus_id)?;
                usbipd::auto_attach_port(bus_id).inspect_err(|_| {
                    // Cleanup the added policy if auto-attach fails
                    let _ = usbipd::policy_remove(bus_id);
                })
            }
        }?;

        let insert_data = ProfileData {
            process: Some(process),
            last_error: None,
        };

        // If there was a process for this profile already, it will be killed automatically
        // when the old ProfileData is dropped, see Drop implementation of ProfileData
        self.profiles.insert(profile, insert_data);
        Ok(())
    }

    pub fn remove(&mut self, profile: &Profile) -> Result<(), String> {
        if let Profile::Port { bus_id } = profile {
            usbipd::policy_remove(bus_id)?;
        }

        self.profiles.remove(profile);

        Ok(())
    }

    pub fn profiles(&mut self) -> Vec<ProfileInfo> {
        self.profiles
            .iter_mut()
            .map(|(profile, data)| {
                data.update_process_status();

                ProfileInfo {
                    profile: profile.clone(),
                    active: data.process.is_some(),
                    last_error: data.last_error.clone(),
                }
            })
            .collect()
    }
}
