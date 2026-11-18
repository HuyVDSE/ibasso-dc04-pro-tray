mod popup;
mod startup;
mod tray;

use crate::worker::Worker;
use popup::Popup;
use std::{cell::RefCell, mem::size_of, rc::Rc};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::HDC,
        System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
        UI::{Controls::*, WindowsAndMessaging::*},
    },
    core::{GUID, PCWSTR, w},
};

const SNAPSHOT: u32 = WM_APP + 1;
const TRAY: u32 = WM_APP + 2;
const TRAY_RETRY: usize = 1;
const HID_INTERFACE: GUID = GUID::from_u128(0x4d1e55b2_f16f_11cf_88cb_001111000030);
const CLASS: PCWSTR = w!("IbassoDc04ProTrayPopup");
const TITLE: PCWSTR = w!("iBasso DC04 Pro");

thread_local! {
    static APP: RefCell<Option<Rc<App>>> = const { RefCell::new(None) };
}

struct App {
    worker: Worker,
    popup: Popup,
    icon: tray::Tray,
    taskbar_created: u32,
    notification: HDEVNOTIFY,
}

struct Instance(HANDLE);

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn app() -> Option<Rc<App>> {
    APP.with(|slot| slot.borrow().clone())
}

pub fn run() -> windows::core::Result<()> {
    // All HWNDs and callbacks except the worker's PostMessage live on this thread.
    unsafe {
        let singleton = CreateMutexW(None, false, w!("Local\\IbassoDc04ProTray"))?;
        let already_running = GetLastError() == ERROR_ALREADY_EXISTS;
        let _instance = Instance(singleton);
        if already_running {
            return Ok(());
        }
        InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES,
        })
        .ok()?;

        let module = HINSTANCE(GetModuleHandleW(None)?.0);
        if RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: module,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: CLASS,
            ..Default::default()
        }) == 0
        {
            return Err(windows::core::Error::from_thread());
        }
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            CLASS,
            TITLE,
            WS_POPUP | WS_CLIPCHILDREN,
            0,
            0,
            popup::WIDTH,
            popup::HEIGHT,
            None,
            None,
            Some(module),
            None,
        )?;
        let popup = Popup::new(hwnd, module)?;
        let icon = tray::Tray::new(hwnd, module)?;
        let taskbar_created = RegisterWindowMessageW(w!("TaskbarCreated"));
        if taskbar_created == 0 {
            return Err(windows::core::Error::from_thread());
        }
        let filter = DEV_BROADCAST_DEVICEINTERFACE_W {
            dbcc_size: size_of::<DEV_BROADCAST_DEVICEINTERFACE_W>() as u32,
            dbcc_devicetype: DBT_DEVTYP_DEVICEINTERFACE.0,
            dbcc_classguid: HID_INTERFACE,
            ..Default::default()
        };
        let notification = RegisterDeviceNotificationW(
            HANDLE(hwnd.0),
            std::ptr::from_ref(&filter).cast(),
            DEVICE_NOTIFY_WINDOW_HANDLE,
        )?;

        let address = hwnd.0 as usize;
        let worker = Worker::start(move || {
            let _ = PostMessageW(Some(HWND(address as _)), SNAPSHOT, WPARAM(0), LPARAM(0));
        })
        .map_err(|error| windows::core::Error::new(E_FAIL, error.to_string()))?;
        let app = Rc::new(App {
            worker,
            popup,
            icon,
            taskbar_created,
            notification,
        });
        APP.with(|slot| slot.replace(Some(app.clone())));
        app.render();
        app.ensure_tray(hwnd);

        let result = loop {
            let mut message = MSG::default();
            match GetMessageW(&mut message, None, 0, 0).0 {
                -1 => break Err(windows::core::Error::from_thread()),
                0 => break Ok(()),
                _ if app.popup.handle_message(&app, &message) => {}
                _ => {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        };
        APP.with(|slot| slot.replace(None));
        let _ = UnregisterDeviceNotification(app.notification);
        if IsWindow(Some(hwnd)).as_bool() {
            let _ = DestroyWindow(hwnd);
        }
        result
    }
}

impl App {
    fn ensure_tray(&self, hwnd: HWND) {
        unsafe {
            if self.icon.add() {
                let _ = KillTimer(Some(hwnd), TRAY_RETRY);
            } else {
                // Explorer can be unavailable or still starting when Run launches us.
                SetTimer(Some(hwnd), TRAY_RETRY, 1000, None);
            }
        }
    }

    fn render(&self) {
        let snapshot = self.worker.snapshot();
        self.popup.render(snapshot);
        self.popup.render_gain(self.worker.gain_snapshot());
        self.icon.update(snapshot);
    }

    fn hide(&self) {
        self.popup.hide();
        self.worker.visible(false);
    }

    fn show(&self, anchor: RECT) {
        self.popup.show(anchor);
        self.worker.visible(self.popup.visible());
        self.render();
    }

    fn rescan(&self) {
        self.worker.rescan();
        self.render();
    }
}

pub fn show_error(error: &str) {
    let text = wide(error);
    unsafe {
        MessageBoxW(None, PCWSTR(text.as_ptr()), TITLE, MB_OK | MB_ICONERROR);
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn point(lparam: isize) -> POINT {
    POINT {
        x: lparam as i16 as i32,
        y: (lparam >> 16) as i16 as i32,
    }
}

// WM_DEVICECHANGE owns this variable-size buffer for the duration of the callback.
fn is_dc04_notification(wparam: WPARAM, lparam: LPARAM) -> bool {
    if !matches!(
        wparam.0 as u32,
        DBT_DEVICEARRIVAL | DBT_DEVICEREMOVECOMPLETE
    ) || lparam.0 == 0
    {
        return false;
    }
    unsafe {
        let header = &*(lparam.0 as *const DEV_BROADCAST_HDR);
        let offset = std::mem::offset_of!(DEV_BROADCAST_DEVICEINTERFACE_W, dbcc_name);
        let size = header.dbch_size as usize;
        if header.dbch_devicetype != DBT_DEVTYP_DEVICEINTERFACE || size <= offset {
            return false;
        }
        let path = std::slice::from_raw_parts(
            (lparam.0 as *const u8).add(offset).cast::<u16>(),
            (size - offset) / 2,
        );
        let path = String::from_utf16_lossy(path).to_ascii_lowercase();
        path.contains("#vid_262a&pid_187f")
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if let Some(app) = app() {
        match msg {
            SNAPSHOT => app.render(),
            TRAY => app.icon.event(&app, wp, lp),
            WM_DEVICECHANGE if is_dc04_notification(wp, lp) => app.rescan(),
            WM_POWERBROADCAST if wp.0 == PBT_APMRESUMEAUTOMATIC as usize => {
                app.rescan();
                return LRESULT(1);
            }
            WM_ACTIVATE if wp.0 as u16 == WA_INACTIVE as u16 => app.hide(),
            WM_COMMAND => app.popup.command(&app, wp, lp),
            WM_MOUSEWHEEL => app.popup.wheel(&app, wp, lp),
            WM_HSCROLL if lp.0 == app.popup.slider().0 as isize => app.popup.drag_update(&app),
            WM_PAINT => app.popup.paint(),
            WM_ERASEBKGND => return LRESULT(1),
            WM_CTLCOLORSTATIC => return app.popup.label_color(HDC(wp.0 as _), HWND(lp.0 as _)),
            WM_DRAWITEM if unsafe { app.popup.draw_button(lp) } => return LRESULT(1),
            WM_NOTIFY => {
                if let Some(result) = unsafe { app.popup.custom_draw(lp) } {
                    return result;
                }
                return unsafe { DefWindowProcW(hwnd, msg, wp, lp) };
            }
            WM_DPICHANGED => app.popup.dpi_changed(wp.0 as u16 as u32, lp),
            WM_CLOSE => unsafe {
                let _ = DestroyWindow(hwnd);
            },
            WM_DESTROY => {
                app.icon.remove();
                unsafe {
                    PostQuitMessage(0);
                }
            }
            WM_TIMER if wp.0 == TRAY_RETRY => app.ensure_tray(hwnd),
            msg if msg == app.taskbar_created => {
                app.hide();
                app.icon.remove();
                app.ensure_tray(hwnd);
            }
            _ => return unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}
