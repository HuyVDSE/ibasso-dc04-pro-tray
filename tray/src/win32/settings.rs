use super::{app, wide};
use crate::worker::{GainSnapshot, Status};
use ibasso_dc04_pro::dc04::GainLevel;
use std::cell::Cell;
use windows::{
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
    },
    core::{PCWSTR, w},
};

const DIALOG_ID: usize = 101;
const LOW: u16 = 1001;
const MID: u16 = 1002;
const HIGH: u16 = 1003;
const STATUS: i32 = 1004;
const DETAILS: u16 = 1005;

pub struct Settings {
    hwnd: HWND,
    enabled_epoch: Cell<Option<u64>>,
}

impl Settings {
    pub fn new(module: HINSTANCE) -> windows::core::Result<Self> {
        let resource = PCWSTR(std::ptr::without_provenance(DIALOG_ID));
        let hwnd = unsafe {
            CreateDialogParamW(Some(module), resource, None, Some(dialog_proc), LPARAM(0))?
        };
        Ok(Self {
            hwnd,
            enabled_epoch: Cell::new(None),
        })
    }

    pub fn visible(&self) -> bool {
        unsafe { IsWindowVisible(self.hwnd).as_bool() }
    }

    pub fn show(&self) -> windows::core::Result<()> {
        unsafe {
            if IsIconic(self.hwnd).as_bool() {
                let _ = ShowWindow(self.hwnd, SW_RESTORE);
            }
            // Raise an existing dialog even when it is already visible behind another window.
            SetWindowPos(
                self.hwnd,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            )?;
            let _ = SetForegroundWindow(self.hwnd);
        }
        Ok(())
    }

    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn handle_message(&self, message: &MSG) -> bool {
        self.visible() && unsafe { IsDialogMessageW(self.hwnd, message).as_bool() }
    }

    pub fn render(&self, snapshot: GainSnapshot) {
        let enabled = snapshot.can_set_gain();
        self.enabled_epoch.set(enabled.then_some(snapshot.epoch));
        let status = match (snapshot.requested, snapshot.last_sent, snapshot.status) {
            (Some(level), _, _) => format!("Sending {}…", level_name(level)),
            (_, Some(level), Status::Read(_)) => format!("Last sent: {}", level_name(level)),
            (_, _, status) if snapshot.error.is_some() => match status {
                Status::Offline => "Gain write failed · DAC disconnected".into(),
                Status::Read(_) => "Gain write failed · Ready to retry".into(),
                _ => "Gain write failed · Waiting for DAC".into(),
            },
            (_, _, status) => match status {
                Status::Read(_) => "Connected via USB",
                Status::Offline => "DAC disconnected",
                Status::Multiple => "Connect only one DC04 Pro",
                _ => "Waiting for DAC…",
            }
            .into(),
        };
        let status = wide(&status);
        unsafe {
            for id in [LOW, MID, HIGH] {
                if let Ok(button) = GetDlgItem(Some(self.hwnd), i32::from(id)) {
                    let _ = EnableWindow(button, enabled);
                }
            }
            let _ = SetDlgItemTextW(self.hwnd, STATUS, PCWSTR(status.as_ptr()));
            if let Ok(button) = GetDlgItem(Some(self.hwnd), i32::from(DETAILS)) {
                let _ = ShowWindow(
                    button,
                    if snapshot.error.is_some() {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
            }
        }
    }
}

impl Drop for Settings {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

fn level_name(level: GainLevel) -> &'static str {
    match level {
        GainLevel::Low => "Low",
        GainLevel::Mid => "Mid",
        GainLevel::High => "High",
    }
}

unsafe extern "system" fn dialog_proc(hwnd: HWND, msg: u32, wp: WPARAM, _: LPARAM) -> isize {
    if msg == WM_INITDIALOG {
        return 1;
    }
    let Some(app) = app() else {
        return 0;
    };
    match msg {
        WM_CLOSE => app.hide_settings(),
        WM_COMMAND if (wp.0 >> 16) as u16 == BN_CLICKED as u16 => {
            let level = match wp.0 as u16 {
                LOW => GainLevel::Low,
                MID => GainLevel::Mid,
                HIGH => GainLevel::High,
                DETAILS => {
                    if let Some(error) = app.worker.gain_snapshot().error {
                        let text = wide(&error);
                        unsafe {
                            MessageBoxW(
                                Some(hwnd),
                                PCWSTR(text.as_ptr()),
                                w!("Gain Error Details"),
                                MB_OK | MB_ICONWARNING,
                            );
                        }
                    }
                    return 1;
                }
                id if id == IDCANCEL.0 as u16 => {
                    app.hide_settings();
                    return 1;
                }
                _ => return 0,
            };
            if app.settings.visible()
                && let Some(epoch) = app.settings.enabled_epoch.get()
            {
                app.worker.submit_gain(epoch, level);
                app.settings.render(app.worker.gain_snapshot());
            }
        }
        _ => return 0,
    }
    1
}
