//! Various Windows utilities.

use std::fmt::Debug;
use std::mem::{size_of, zeroed};
use std::os::windows::io::RawHandle;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Pipes::PeekNamedPipe;
use windows_sys::Win32::System::Threading::{
    CreateEventW, INFINITE, SetEvent, WaitForMultipleObjects,
};
use windows_sys::Win32::{
    Devices::{
        DeviceAndDriverInstallation::{
            CM_NOTIFY_ACTION, CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL,
            CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL, CM_NOTIFY_EVENT_DATA, CM_NOTIFY_FILTER,
            CM_NOTIFY_FILTER_0, CM_NOTIFY_FILTER_0_0, CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
            CM_Register_Notification, CM_Unregister_Notification, CR_SUCCESS, HCMNOTIFICATION,
        },
        Usb::GUID_DEVINTERFACE_USB_DEVICE,
    },
    Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, GetLastError},
    System::{
        Diagnostics::Debug::{FORMAT_MESSAGE_FROM_SYSTEM, FormatMessageW},
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        },
        Threading::CreateMutexW,
        Threading::GetCurrentProcess,
    },
};

/// Acquires a single instance lock for the application. Returns `true` if the lock was acquired.
pub fn acquire_single_instance_lock() -> bool {
    // Convert to null-terminated UTF-16 string
    let mutex_name: Vec<u16> = "WSL_USB_MANAGER_SINGLE_INSTANCE_LOCK\0"
        .encode_utf16()
        .collect();

    let mutex_handle = unsafe { CreateMutexW(null_mut(), 1, mutex_name.as_ptr()) };
    if mutex_handle.is_null() {
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
    // The callback function that will be called by the system, which will then call the user's closure
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
        handle: std::ptr::null_mut(),
        closure: Box::new(Box::new(callback)),
    };

    // A filter that matches all device instances of the USB device interface class
    let filter = CM_NOTIFY_FILTER {
        cbSize: std::mem::size_of::<CM_NOTIFY_FILTER>() as u32,
        Flags: 0,
        FilterType: CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
        Reserved: 0,
        u: CM_NOTIFY_FILTER_0 {
            DeviceInterface: CM_NOTIFY_FILTER_0_0 {
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
            handle: std::ptr::null_mut(),
            closure: Box::new(Box::new(|| {})),
        }
    }
}

impl Drop for DeviceNotification {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { CM_Unregister_Notification(self.handle) };
        }
    }
}

pub fn setup_job_object_grouping() -> Result<(), String> {
    unsafe {
        // Create the Job Object
        let job_handle = CreateJobObjectW(null(), null());
        if job_handle.is_null() {
            return Err(format!(
                "Failed to create job object: {}",
                get_last_error_string()
            ));
        }

        // Configure the job to kill children when parent handle closes
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        let res = SetInformationJobObject(
            job_handle,
            JobObjectExtendedLimitInformation,
            &info as *const _ as _,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );

        if res == 0 {
            CloseHandle(job_handle);
            return Err(format!(
                "Failed to set job information: {}",
                get_last_error_string()
            ));
        }

        // Assign the CURRENT process to this job
        // All subsequent spawned children (like usbipd) will inherit this job
        let process_handle = GetCurrentProcess();
        if AssignProcessToJobObject(job_handle, process_handle) == 0 {
            CloseHandle(job_handle);
            return Err(format!(
                "Failed to assign process to job: {}",
                get_last_error_string()
            ));
        }

        // We intentionally leak the job_handle or store it for the app's lifetime
        // If we close it now, the JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE might trigger
        Ok(())
    }
}

/// Peeks the number of bytes available in a pipe without blocking or consuming the data.
/// Returns `None` if the handle is invalid or an error occurs.
pub fn peek_pipe(handle: RawHandle) -> Option<u32> {
    let handle = handle as windows_sys::Win32::Foundation::HANDLE;
    let mut bytes_available: u32 = 0;

    match unsafe {
        PeekNamedPipe(
            handle,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut bytes_available,
            std::ptr::null_mut(),
        )
    } {
        0 => None,
        _ => Some(bytes_available),
    }
}

#[repr(transparent)]
#[derive(Clone)]
pub struct Event(HANDLE);
unsafe impl Send for Event {}

impl From<Event> for HANDLE {
    fn from(event: Event) -> Self {
        event.0
    }
}

impl Event {
    /// Creates a new manual-reset, unsignaled anonymous event.
    pub fn new() -> Self {
        let h = unsafe {
            CreateEventW(
                null_mut(), // Attributes
                0,          // bManualReset: TRUE
                0,          // bInitialState: FALSE
                null_mut(), // Name
            )
        };
        if h.is_null() {
            panic!("Failed to create Win32 Event");
        }
        Event(h)
    }

    /// Transitions the event to a signaled state, waking up the waiting thread.
    pub fn set(&self) {
        unsafe {
            SetEvent(self.0);
        }
    }

    pub fn as_raw_handle(&self) -> HANDLE {
        self.0
    }
}

impl Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Event").field(&self.0).finish()
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[repr(transparent)]
pub struct SendHandle(pub RawHandle);
unsafe impl Send for SendHandle {}

impl From<HANDLE> for SendHandle {
    fn from(handle: HANDLE) -> Self {
        SendHandle(handle as RawHandle)
    }
}

impl From<Event> for SendHandle {
    fn from(event: Event) -> Self {
        SendHandle(event.as_raw_handle())
    }
}

impl Debug for SendHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SendHandle").field(&self.0).finish()
    }
}

/// Waits for any of the given handles to be signaled.
/// Returns the index of the handle that was signaled, or `None` if an error occurs.
pub fn wait_for_handles(handles: &[SendHandle]) -> Option<usize> {
    let wait_result = unsafe {
        WaitForMultipleObjects(
            handles.len() as u32,
            handles.as_ptr() as *const HANDLE,
            0, // wait for any
            INFINITE,
        )
    } as usize;

    const WAIT_OBJ_0: usize = WAIT_OBJECT_0 as usize;

    if wait_result < WAIT_OBJ_0 + handles.len() {
        Some(wait_result - WAIT_OBJ_0)
    } else {
        None
    }
}
