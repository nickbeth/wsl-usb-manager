use native_windows_gui as nwg;

use windows_sys::Win32::Foundation::{HANDLE, HWND};
use windows_sys::Win32::Graphics::Gdi::DeleteObject;
use windows_sys::Win32::UI::Shell::{
    SHGSI_ICON, SHGSI_SMALLICON, SHGetStockIconInfo, SHSTOCKICONID, SHSTOCKICONINFO,
};
use windows_sys::Win32::UI::Controls::{LVCF_TEXT, LVM_SETCOLUMNW, LVCOLUMNW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CopyImage, DestroyIcon, DrawMenuBar, GetIconInfoExW, GetMenuItemCount, GetSubMenu, HMENU,
    ICONINFOEXW, IMAGE_BITMAP, IsIconic, LR_CREATEDIBSECTION, MENUITEMINFOW, MF_BYCOMMAND,
    MF_BYPOSITION, MIIM_BITMAP, MIIM_STRING, SendMessageW, SetForegroundWindow,
    SetMenuItemInfoW,
};

/// Extends [`nwg::Window`] with additional functionality.
pub trait WindowEx {
    /// Returns `true` if the window is minimized (iconic).
    fn is_minimized(&self) -> bool;

    /// Brings the window to the foreground and activates it.
    fn set_foreground(&self);
}

impl WindowEx for nwg::Window {
    fn is_minimized(&self) -> bool {
        let hwnd = self.handle.hwnd().unwrap() as HWND;
        unsafe { IsIconic(hwnd) != 0 }
    }

    fn set_foreground(&self) {
        let hwnd = self.handle.hwnd().unwrap() as HWND;
        unsafe {
            SetForegroundWindow(hwnd);
        }
    }
}

/// Extends [`nwg::Bitmap`] with additional functionality.
pub trait BitmapEx {
    fn from_system_icon(icon: SHSTOCKICONID) -> nwg::Bitmap;
}

impl BitmapEx for nwg::Bitmap {
    /// Creates a bitmap from a [`SHSTOCKICONID`] system icon ID.
    fn from_system_icon(icon: SHSTOCKICONID) -> nwg::Bitmap {
        // Retrieve the icon
        let mut stock_icon_info = SHSTOCKICONINFO {
            cbSize: std::mem::size_of::<SHSTOCKICONINFO>() as u32,
            hIcon: std::ptr::null_mut(),
            iSysImageIndex: 0,
            iIcon: 0,
            szPath: [0; 260],
        };
        unsafe {
            SHGetStockIconInfo(
                icon,
                SHGSI_ICON | SHGSI_SMALLICON,
                &mut stock_icon_info as *mut _,
            );
        }

        // Retrieve the bitmap for the icon
        let mut icon_info = ICONINFOEXW {
            cbSize: std::mem::size_of::<ICONINFOEXW>() as u32,
            fIcon: 0,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: std::ptr::null_mut(),
            hbmColor: std::ptr::null_mut(),
            wResID: 0,
            szModName: [0; 260],
            szResName: [0; 260],
        };
        unsafe {
            GetIconInfoExW(stock_icon_info.hIcon, &mut icon_info as *mut _);
        }

        // Create a copy of the bitmap with transparent background from the icon bitmap
        let hbitmap = unsafe {
            CopyImage(
                icon_info.hbmColor as HANDLE,
                IMAGE_BITMAP,
                0,
                0,
                LR_CREATEDIBSECTION,
            )
        };

        // Delete the unused icon and bitmaps
        unsafe {
            DeleteObject(icon_info.hbmMask);
            DeleteObject(icon_info.hbmColor);
            DestroyIcon(stock_icon_info.hIcon);
        };

        if hbitmap.is_null() {
            panic!("Failed to create bitmap from system icon");
        } else {
            #[allow(unused)]
            struct Bitmap {
                handle: HANDLE,
                owned: bool,
            }

            let bitmap = Bitmap {
                handle: hbitmap as HANDLE,
                owned: true,
            };

            // Ugly hack to set the private `owned` field inside nwg::Bitmap to true
            unsafe { std::mem::transmute::<Bitmap, nwg::Bitmap>(bitmap) }
        }
    }
}

/// Extends [`nwg::MenuItem`] with additional functionality.
pub trait MenuItemEx {
    fn set_bitmap(&self, bitmap: Option<&nwg::Bitmap>);

    /// Changes the item's text label. NWG's `MenuItem` has no `set_text`, so
    /// this drives `SetMenuItemInfoW` with `MIIM_STRING` directly.
    fn set_text(&self, text: &str);
}

impl MenuItemEx for nwg::MenuItem {
    /// Sets a bitmap to be displayed on a menu item. Pass `None` to remove the bitmap.
    fn set_bitmap(&self, bitmap: Option<&nwg::Bitmap>) {
        let (hmenu, item_id) = self.handle.hmenu_item().unwrap();
        let hbitmap = match bitmap {
            Some(b) => b.handle as HANDLE,
            None => std::ptr::null_mut(),
        };

        let menu_item_info = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_BITMAP,
            fType: 0,
            fState: 0,
            wID: 0,
            hSubMenu: std::ptr::null_mut(),
            hbmpChecked: std::ptr::null_mut(),
            hbmpUnchecked: std::ptr::null_mut(),
            dwItemData: 0,
            dwTypeData: std::ptr::null_mut(),
            cch: 0,
            hbmpItem: hbitmap,
        };

        unsafe {
            SetMenuItemInfoW(
                hmenu as HMENU,
                item_id,
                MF_BYCOMMAND as i32,
                &menu_item_info as *const _,
            );
        }
    }

    /// Changes the item's text label.
    fn set_text(&self, text: &str) {
        let (hmenu, item_id) = self.handle.hmenu_item().unwrap();

        let mut wide: Vec<u16> = text.encode_utf16().collect();
        wide.push(0);

        let mut info = MENUITEMINFOW::default();
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
        info.fMask = MIIM_STRING;
        info.dwTypeData = wide.as_mut_ptr();
        info.cch = wide.len() as u32;

        unsafe {
            SetMenuItemInfoW(hmenu as HMENU, item_id, MF_BYCOMMAND as i32, &info);
        }
    }
}

/// Extends [`nwg::ListView`] with functionality not exposed by NWG.
pub trait ListViewExt {
    /// Sets the text of a ListView column header. `col` is 0-based.
    ///
    /// NWG has no API to change a column header after creation, so this drives
    /// the Win32 `LVM_SETCOLUMNW` message directly (the project already uses
    /// `LVSCW_AUTOSIZE_*` from the same `Win32_UI_Controls` family).
    fn set_header_text(&self, col: usize, text: &str);
}

impl ListViewExt for nwg::ListView {
    fn set_header_text(&self, col: usize, text: &str) {
        let hwnd = match self.handle.hwnd() {
            Some(h) => h as HWND,
            None => return,
        };

        // NUL-terminated UTF-16 buffer, kept alive across the call.
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        wide.push(0);

        // Default-initialised (windows-sys derives `Default`); only the fields
        // we care about are overridden.
        let mut column = LVCOLUMNW::default();
        column.mask = LVCF_TEXT;
        column.pszText = wide.as_mut_ptr();
        column.cchTextMax = wide.len() as i32;

        unsafe {
            SendMessageW(
                hwnd,
                LVM_SETCOLUMNW,
                col,
                &column as *const LVCOLUMNW as isize,
            );
        }
    }
}

/// Extends [`nwg::Menu`] (top-level menu bar entries) with functionality not
/// exposed by NWG.
pub trait MenuExt {
    /// Changes the label of a top-level menu bar entry.
    ///
    /// NWG's `Menu` has no `set_text`. The control handle stores the parent
    /// menu-bar `HMENU` and the entry's popup submenu `HMENU`, so we locate the
    /// entry by matching its submenu inside the menu bar, then apply the new
    /// text via `SetMenuItemInfoW` (by position) and redraw the bar.
    fn set_text(&self, window: &nwg::Window, text: &str);
}

impl MenuExt for nwg::Menu {
    fn set_text(&self, window: &nwg::Window, text: &str) {
        let hwnd = match window.handle.hwnd() {
            Some(h) => h as HWND,
            None => return,
        };
        // `Menu` handle = (parent menu-bar HMENU, popup submenu HMENU).
        let (menubar, popup) = match self.handle.hmenu() {
            Some(h) => (h.0 as HMENU, h.1 as HMENU),
            None => return,
        };

        let mut wide: Vec<u16> = text.encode_utf16().collect();
        wide.push(0);

        let mut info = MENUITEMINFOW::default();
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
        info.fMask = MIIM_STRING;
        info.dwTypeData = wide.as_mut_ptr();
        info.cch = wide.len() as u32;

        let count = unsafe { GetMenuItemCount(menubar) };
        for i in 0..count {
            // Safety: `i` is in range `[0, count)`.
            if unsafe { GetSubMenu(menubar, i) } == popup {
                unsafe {
                    SetMenuItemInfoW(menubar, i as u32, MF_BYPOSITION as i32, &info);
                    DrawMenuBar(hwnd);
                }
                break;
            }
        }
    }
}
