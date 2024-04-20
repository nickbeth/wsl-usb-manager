//! Various Windows utilities.

use std::ptr::null_mut;

use windows_sys::Win32::{
    Devices::{
        DeviceAndDriverInstallation::{
            CM_Register_Notification, CM_Unregister_Notification, CM_NOTIFY_ACTION,
            CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL, CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL,
            CM_NOTIFY_EVENT_DATA, CM_NOTIFY_FILTER, CM_NOTIFY_FILTER_0, CM_NOTIFY_FILTER_0_2,
            CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE, CR_SUCCESS, HCMNOTIFICATION,
        },
        Usb::GUID_DEVINTERFACE_USB_DEVICE,
    },
    Foundation::{GetLastError, ERROR_ALREADY_EXISTS, ERROR_SUCCESS},
    System::{
        Diagnostics::Debug::{FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM},
        Threading::CreateMutexW,
    },
};

/// Acquires a single instance lock for the application. Returns `true` if the lock was acquired.
pub fn acquire_single_instance_lock() -> bool {
    // Convert to null-terminated UTF-16 string
    let mutex_name: Vec<u16> = "WSL_USB_MANAGER_SINGLE_INSTANCE_LOCK\0"
        .encode_utf16()
        .collect();

    let mutex_handle = unsafe { CreateMutexW(null_mut(), 1, mutex_name.as_ptr()) };
    if mutex_handle == 0 {
        return false;
    }

    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return false;
    }

    true
}

/// Retrieves the last error message from the system.
pub fn get_last_error_string() -> String {
    let mut buffer = [0u16; 256];

    let error_code = unsafe { GetLastError() };
    let msg_slice = unsafe {
        let len = FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM,
            null_mut(),
            error_code,
            0x0409_u32, // en-US language ID
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            null_mut(),
        );
        &buffer[..len as usize]
    };

    String::from_utf16_lossy(msg_slice).trim_end().to_owned()
}

/// Registers a closure to be called when a USB device is connected or disconnected.
pub fn register_usb_device_notifications(
    callback: impl Fn() + 'static,
) -> Result<DeviceNotification, u32> {
    extern "system" fn callback_impl(
        _hnotify: HCMNOTIFICATION,
        context: *const std::ffi::c_void,
        action: CM_NOTIFY_ACTION,
        _eventdata: *const CM_NOTIFY_EVENT_DATA,
        _eventdatasize: u32,
    ) -> u32 {
        match action {
            // We only care about device arrival and removal events
            CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL | CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL => {
                let user_callback = unsafe { &*(context as *const Box<dyn Fn()>) };
                user_callback();
            }
            _ => {}
        }

        ERROR_SUCCESS
    }

    let mut notif = DeviceNotification {
        handle: 0,
        closure: Box::new(Box::new(callback)),
    };

    // A filter that matches all device instances of the USB device interface class
    let filter = CM_NOTIFY_FILTER {
        cbSize: std::mem::size_of::<CM_NOTIFY_FILTER>() as u32,
        Flags: 0,
        FilterType: CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
        Reserved: 0,
        u: CM_NOTIFY_FILTER_0 {
            DeviceInterface: CM_NOTIFY_FILTER_0_2 {
                ClassGuid: GUID_DEVINTERFACE_USB_DEVICE,
            },
        },
    };

    // A pointer to the closure that can be cast to void
    let closure_ptr = notif.closure.as_ref() as *const _;

    let error = unsafe {
        CM_Register_Notification(
            &filter as *const _,
            closure_ptr as *const _,
            Some(callback_impl),
            &mut notif.handle as *mut _,
        )
    };

    if error != CR_SUCCESS {
        Err(error)
    } else {
        Ok(notif)
    }
}

/// A device notification registration handle.
///
/// The notification is automatically unregistered when the handle is dropped.
pub struct DeviceNotification {
    pub handle: HCMNOTIFICATION,
    closure: Box<Box<dyn Fn()>>,
}

impl Default for DeviceNotification {
    fn default() -> Self {
        Self {
            handle: 0,
            closure: Box::new(Box::new(|| {})),
        }
    }
}

impl Drop for DeviceNotification {
    fn drop(&mut self) {
        if self.handle != 0 {
            unsafe { CM_Unregister_Notification(self.handle) };
        }
    }
}
