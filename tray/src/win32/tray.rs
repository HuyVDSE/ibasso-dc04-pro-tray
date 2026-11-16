use super::{App, TRAY, point, show_error, startup, wide};
use crate::worker::{Snapshot, Status};
use std::{cell::Cell, mem::size_of};
use windows::{
    Win32::{
        Foundation::*,
        UI::{
            Controls::{LIM_SMALL, LoadIconMetric},
            Shell::*,
            WindowsAndMessaging::*,
        },
    },
    core::{PCWSTR, w},
};

const NIN_KEYSELECT: u32 = NIN_SELECT | NINF_KEY;
const ICON_ID: u32 = 1;
const RESCAN: usize = 1;
const EXIT: usize = 2;
const STARTUP: usize = 3;
const REMOVE_OLD_STARTUP: usize = 4;
const HID_ERROR: usize = 5;

pub struct Tray {
    hwnd: HWND,
    icon: HICON,
    added: Cell<bool>,
    tooltip: Cell<Status>,
}

impl Tray {
    pub fn new(hwnd: HWND, module: HINSTANCE) -> windows::core::Result<Self> {
        // MAKEINTRESOURCEW(1): the icon group in assets/tray.rc.
        let resource = PCWSTR(std::ptr::without_provenance(1));
        Ok(Self {
            hwnd,
            icon: unsafe { LoadIconMetric(Some(module), resource, LIM_SMALL)? },
            added: Cell::new(false),
            tooltip: Cell::new(Status::Reading),
        })
    }

    fn data(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: ICON_ID,
            ..Default::default()
        }
    }

    fn tip(&self, data: &mut NOTIFYICONDATAW) {
        let tip = match self.tooltip.get() {
            Status::Read(reading) => format!(
                "iBasso DC04 Pro · {}/100{}",
                reading.step(),
                if reading.adjustable() {
                    ""
                } else {
                    " · Volume locked"
                }
            ),
            status => format!(
                "iBasso DC04 Pro · {}",
                match status {
                    Status::Offline => "DAC disconnected",
                    Status::Multiple => "Connect only one DAC",
                    Status::Unavailable => "Unable to read DAC",
                    Status::Sync => "Sync needed",
                    _ => "Reading…",
                }
            ),
        };
        for (slot, value) in data.szTip.iter_mut().zip(tip.encode_utf16()) {
            *slot = value;
        }
    }

    pub fn add(&self) -> bool {
        if self.added.get() {
            return true;
        }
        let mut data = self.data();
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
        data.uCallbackMessage = TRAY;
        data.hIcon = self.icon;
        self.tip(&mut data);
        unsafe {
            self.added.set(Shell_NotifyIconW(NIM_ADD, &data).as_bool());
            if self.added.get() {
                data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
                if !Shell_NotifyIconW(NIM_SETVERSION, &data).as_bool() {
                    self.remove();
                }
            }
        }
        self.added.get()
    }

    pub fn remove(&self) {
        if self.added.replace(false) {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.data());
            }
        }
    }

    pub fn update(&self, snapshot: Snapshot) {
        let next = snapshot.status;
        if self.tooltip.replace(next) != next && self.added.get() {
            let mut data = self.data();
            data.uFlags = NIF_TIP | NIF_SHOWTIP;
            self.tip(&mut data);
            unsafe {
                let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
            }
        }
    }

    fn anchor(&self, fallback: POINT) -> RECT {
        let ident = NOTIFYICONIDENTIFIER {
            cbSize: size_of::<NOTIFYICONIDENTIFIER>() as u32,
            hWnd: self.hwnd,
            uID: ICON_ID,
            ..Default::default()
        };
        if let Ok(rect) = unsafe { Shell_NotifyIconGetRect(&ident) }
            && rect.right > rect.left
        {
            rect
        } else {
            let mut cursor = fallback;
            if fallback == (POINT { x: -1, y: -1 }) {
                unsafe {
                    let _ = GetCursorPos(&mut cursor);
                }
            }
            RECT {
                left: cursor.x,
                top: cursor.y,
                right: cursor.x + 1,
                bottom: cursor.y + 1,
            }
        }
    }

    pub fn event(&self, app: &App, wp: WPARAM, lp: LPARAM) {
        if (lp.0 >> 16) as u16 != ICON_ID as u16 {
            return;
        }
        let cursor = point(wp.0 as isize);
        match lp.0 as u16 as u32 {
            WM_LBUTTONUP | NIN_SELECT | NIN_KEYSELECT => {
                if !app.popup.visible() {
                    app.show(self.anchor(cursor));
                }
            }
            WM_CONTEXTMENU => {
                app.hide();
                self.menu(app, cursor);
            }
            _ => {}
        }
    }

    fn menu(&self, app: &App, cursor: POINT) {
        let anchor = self.anchor(cursor);
        let startup = startup::Startup::current_exe()
            .and_then(|setting| setting.state().map(|state| (setting, state)));
        let hid_error = app.worker.last_error();
        let version = wide(concat!("iBasso DC04 Pro Tray v", env!("CARGO_PKG_VERSION")));
        unsafe {
            let Ok(menu) = CreatePopupMenu() else {
                return;
            };
            let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, PCWSTR(version.as_ptr()));
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            let _ = AppendMenuW(menu, MF_STRING, RESCAN, w!("Rescan DAC"));
            if hid_error.is_some() {
                let _ = AppendMenuW(menu, MF_STRING, HID_ERROR, w!("DAC Error Details"));
            }
            let (flag, label) = match &startup {
                Ok((_, startup::State::Current)) => (MF_CHECKED, w!("Start with Windows")),
                Ok((_, startup::State::Other)) => {
                    (MF_UNCHECKED, w!("Start with Windows (update path)"))
                }
                Ok((_, startup::State::Off)) => (MF_UNCHECKED, w!("Start with Windows")),
                Err(_) => (
                    MF_UNCHECKED,
                    w!("Start with Windows (settings unavailable)"),
                ),
            };
            let _ = AppendMenuW(menu, MF_STRING | flag, STARTUP, label);
            if matches!(startup, Ok((_, startup::State::Other))) {
                let _ = AppendMenuW(
                    menu,
                    MF_STRING,
                    REMOVE_OLD_STARTUP,
                    w!("Remove old startup entry"),
                );
            }
            let _ = AppendMenuW(menu, MF_STRING, EXIT, w!("Exit"));
            let _ = SetForegroundWindow(self.hwnd);
            let selected = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                anchor.left,
                anchor.top,
                Some(0),
                self.hwnd,
                None,
            )
            .0 as usize;
            let _ = DestroyMenu(menu);
            let _ = PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            let _ = Shell_NotifyIconW(NIM_SETFOCUS, &self.data());
            match selected {
                RESCAN => app.rescan(),
                HID_ERROR => {
                    if let Some(error) = hid_error {
                        let body = wide(&error);
                        MessageBoxW(
                            Some(self.hwnd),
                            PCWSTR(body.as_ptr()),
                            w!("DAC Error Details"),
                            MB_OK | MB_ICONWARNING,
                        );
                    }
                }
                STARTUP | REMOVE_OLD_STARTUP => {
                    let result = startup.and_then(|(setting, state)| {
                        if selected == REMOVE_OLD_STARTUP || state == startup::State::Current {
                            setting.disable()
                        } else {
                            setting.enable()
                        }
                    });
                    if let Err(error) = result {
                        show_error(&format!(
                            "Could not update the Windows startup setting.\n{error}"
                        ));
                    }
                }
                EXIT => {
                    let _ = DestroyWindow(self.hwnd);
                }
                _ => {}
            }
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.remove();
        unsafe {
            let _ = DestroyIcon(self.icon);
        }
    }
}
