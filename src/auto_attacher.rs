use std::{
    collections::HashMap,
    fs::File,
    hash::Hash,
    io::{Read, Write},
    os::windows::io::AsRawHandle,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use native_windows_gui as nwg;
use serde::{Deserialize, Serialize};

use crate::{
    usbipd::{self, UsbDevice},
    win_utils,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Profile {
    Device {
        hw_id: String,
        description: Option<String>,
    },
    Port {
        bus_id: String,
    },
}

// Devices can change description while being bound/unbound, don't include it in the hash
impl Hash for Profile {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Profile::Device { hw_id, .. } => hw_id.hash(state),
            Profile::Port { bus_id } => bus_id.hash(state),
        }
    }
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
        // Poll the process to see if it exited (likely because of an error)
        let Ok(Some(exit_status)) = process.try_wait() else {
            return;
        };
        // Process exited with no error (WSL was shut down), clear the process without error
        if exit_status.success() {
            self.process = None;
            self.last_error = Some("Process exited without error.".to_string());
            return;
        }

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

                let error_str =
                    usbipd::get_error_message(String::from_utf8_lossy(&buf).to_string());

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

struct ProcessWatcher {
    #[allow(dead_code)]
    thread: JoinHandle<()>,
    wake_event: win_utils::Event,
}

#[derive(Default)]
struct SharedState {
    profiles: HashMap<Profile, ProfileData>,
    ui_refresh_notice: Option<nwg::NoticeSender>,
}

#[derive(Default)]
pub struct AutoAttacher {
    shared: Arc<Mutex<SharedState>>,
    process_watcher: Option<ProcessWatcher>,
    storage_path: Option<PathBuf>,
}

impl AutoAttacher {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_ui_refresh_notice(&mut self, notice: Option<nwg::NoticeSender>) {
        self.shared.lock().unwrap().ui_refresh_notice = notice;

        // Start the watcher when the notice is set, no point starting it earlier
        self.start_watcher();
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

        if self
            .shared
            .lock()
            .unwrap()
            .profiles
            .contains_key(&new_profile)
        {
            return Err("The device is already in the auto attach list.".to_string());
        }

        self.activate_profile(new_profile)
            .inspect(|_| self.persist_profiles())
    }

    pub fn add_port(&mut self, device: &UsbDevice) -> Result<(), String> {
        let bus_id = device
            .bus_id
            .as_ref()
            .ok_or("The device does not have a bus ID.".to_string())?;

        let new_profile = Profile::Port {
            bus_id: bus_id.clone(),
        };

        if self
            .shared
            .lock()
            .unwrap()
            .profiles
            .contains_key(&new_profile)
        {
            return Err("The port is already in the auto attach list.".to_string());
        }

        // Binds the device as a (wanted) side effect
        usbipd::policy_add(bus_id)?;

        // Cleanup the added policy if auto-attach fails
        self.activate_profile(new_profile)
            .inspect(|_| self.persist_profiles())
            .inspect_err(|_| {
                let _ = usbipd::policy_remove(bus_id);
            })
    }

    pub fn activate_profile(&mut self, profile: Profile) -> Result<(), String> {
        let process = match &profile {
            Profile::Device { hw_id, .. } => usbipd::auto_attach_device(hw_id),
            Profile::Port { bus_id } => usbipd::auto_attach_port(bus_id),
        }?;

        let insert_data = ProfileData {
            process: Some(process),
            last_error: None,
        };

        // If there was a process for this profile already, it will be killed automatically
        // when the old ProfileData is dropped, see Drop implementation of ProfileData
        self.shared
            .lock()
            .unwrap()
            .profiles
            .insert(profile, insert_data);

        if let Some(watcher) = &self.process_watcher {
            watcher.wake_event.set();
        }
        Ok(())
    }

    pub fn remove(&mut self, profile: &Profile) -> Result<(), String> {
        if let Profile::Port { bus_id } = profile {
            usbipd::policy_remove(bus_id)?;
        }

        self.shared.lock().unwrap().profiles.remove(profile);

        self.persist_profiles();
        Ok(())
    }

    pub fn profiles(&mut self) -> Vec<ProfileInfo> {
        self.shared
            .lock()
            .unwrap()
            .profiles
            .iter_mut()
            .map(|(profile, data)| ProfileInfo {
                profile: profile.clone(),
                active: data.process.is_some(),
                last_error: data.last_error.clone(),
            })
            .collect()
    }

    fn start_watcher(&mut self) {
        let wake_event = win_utils::Event::new();
        let wake_raw = wake_event.as_raw_handle() as usize;
        let shared = self.shared.clone();

        let watcher_thread = std::thread::spawn(move || {
            loop {
                let mut process_handles: Vec<win_utils::SendHandle> = {
                    shared
                        .lock()
                        .unwrap()
                        .profiles
                        .values()
                        .filter_map(|data| {
                            data.process
                                .as_ref()
                                .map(|p| win_utils::SendHandle(p.as_raw_handle()))
                        })
                        .collect()
                };

                // Push the wake event handle as part of the handles to wait on, for manual wakeup
                process_handles.push(win_utils::SendHandle(wake_raw as _));

                let Some(wakeup_index) = win_utils::wait_for_handles(&process_handles) else {
                    continue;
                };

                // Return to waiting if the stop event was signaled
                if wakeup_index == process_handles.len() - 1 {
                    continue;
                }

                // Some process status changed, update the status
                let mut state = shared.lock().unwrap();
                state.profiles.values_mut().for_each(|data| {
                    data.update_process_status();
                });
                // Notify the UI
                state.ui_refresh_notice.inspect(|n| n.notice());
            }
        });

        self.process_watcher = Some(ProcessWatcher {
            thread: watcher_thread,
            wake_event,
        });
    }

    pub fn with_storage(storage_path: &Path) -> Self {
        let mut attacher = AutoAttacher {
            storage_path: Some(storage_path.to_owned()),
            ..Default::default()
        };

        let Ok(mut file) = File::open(storage_path) else {
            return attacher;
        };

        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).is_err() {
            return attacher;
        }

        let persisted_profiles: Vec<Profile> = match serde_json::from_slice(&buf) {
            Ok(profiles) => profiles,
            Err(_) => return attacher,
        };

        for profile in persisted_profiles {
            let _ = attacher.activate_profile(profile);
        }

        attacher
    }

    pub fn persist_profiles(&self) {
        let Some(storage_path) = &self.storage_path else {
            // Skip if no storage path was provided
            return;
        };

        let shared = self.shared.lock().unwrap();
        let profiles = shared.profiles.keys().collect::<Vec<_>>();

        let serialized = match serde_json::to_string_pretty(&profiles) {
            Ok(json) => json,
            Err(_) => return,
        };

        let Ok(mut file) = File::create(storage_path) else {
            return;
        };

        let _ = file.write_all(serialized.as_bytes());
    }
}
