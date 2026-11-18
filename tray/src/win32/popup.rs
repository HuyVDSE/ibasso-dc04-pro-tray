use super::{App, app, point, wide};
use crate::worker::{GainSnapshot, Snapshot, Status};
use ibasso_dc04_pro::dc04::{GainLevel, MAX_TARGET, MIN_TARGET};
use std::cell::Cell;
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{
            Controls::*,
            HiDpi::GetDpiForWindow,
            Input::KeyboardAndMouse::*,
            Shell::{DefSubclassProc, SetWindowSubclass},
            WindowsAndMessaging::*,
        },
    },
    core::{PCWSTR, w},
};

const TBM_GETPOS: u32 = WM_USER;
const SS_RIGHT: WINDOW_STYLE = WINDOW_STYLE(0x0002);
const SS_ENDELLIPSIS: WINDOW_STYLE = WINDOW_STYLE(0x4000);
pub const WIDTH: i32 = 320;
pub const HEIGHT: i32 = 224;

const BACKGROUND: COLORREF = rgb(36, 38, 42);
const BORDER: COLORREF = rgb(70, 73, 81);
const BADGE: COLORREF = rgb(55, 62, 74);
const TEXT: COLORREF = rgb(249, 250, 251);
const SECONDARY: COLORREF = rgb(194, 205, 221);
const BLUE: COLORREF = rgb(119, 184, 255);
const MUTED_BLUE: COLORREF = rgb(103, 127, 155);
const TRACK: COLORREF = rgb(59, 59, 59);
const TRACK_BORDER: COLORREF = rgb(133, 133, 133);
const BUTTON: COLORREF = rgb(48, 53, 61);
const BUTTON_PRESSED: COLORREF = rgb(64, 80, 98);
const DISABLED_TEXT: COLORREF = rgb(145, 153, 165);
const GAIN_LEVELS: [GainLevel; 3] = [GainLevel::Low, GainLevel::Mid, GainLevel::High];

pub struct Popup {
    hwnd: HWND,
    title: HWND,
    status: HWND,
    volume: HWND,
    value: HWND,
    slider: HWND,
    gain_label: HWND,
    gain_current: HWND,
    gain_status: HWND,
    gain_buttons: [HWND; 3],
    background: HBRUSH,
    fonts: Cell<[HFONT; 3]>,
    enabled_epoch: Cell<Option<u64>>,
    gain_epoch: Cell<Option<u64>>,
    gain_displayed: Cell<(&'static str, Option<GainLevel>)>,
    drag: Cell<Option<u64>>,
    key: Cell<Option<(usize, u64)>>,
    wheel_remainder: Cell<i32>,
    displayed: Cell<(&'static str, Option<u8>)>,
}

impl Popup {
    pub fn new(hwnd: HWND, module: HINSTANCE) -> windows::core::Result<Self> {
        let child = |class, text, style| unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                text,
                WS_CHILD | WS_VISIBLE | style,
                0,
                0,
                1,
                1,
                Some(hwnd),
                None,
                Some(module),
                None,
            )
        };
        let gain_button = |level| {
            let label = wide(level_name(level));
            child(
                w!("BUTTON"),
                PCWSTR(label.as_ptr()),
                WS_TABSTOP | WS_DISABLED | WINDOW_STYLE(BS_OWNERDRAW as u32),
            )
        };
        let this = Self {
            hwnd,
            title: child(w!("STATIC"), w!("iBasso DC04 Pro"), WINDOW_STYLE(0))?,
            status: child(w!("STATIC"), w!(""), SS_ENDELLIPSIS)?,
            value: child(w!("STATIC"), w!("— / 100"), SS_RIGHT)?,
            // The static immediately before the trackbar supplies its accessible name.
            volume: child(w!("STATIC"), w!("DAC Volume"), WINDOW_STYLE(0))?,
            slider: child(
                TRACKBAR_CLASSW,
                w!("DAC Volume"),
                WS_TABSTOP | WINDOW_STYLE(TBS_NOTICKS),
            )?,
            gain_label: child(w!("STATIC"), w!("Gain Level"), WINDOW_STYLE(0))?,
            gain_current: child(w!("STATIC"), w!("Current gain: Unknown"), SS_RIGHT)?,
            gain_status: child(w!("STATIC"), w!(""), SS_ENDELLIPSIS)?,
            gain_buttons: [
                gain_button(GAIN_LEVELS[0])?,
                gain_button(GAIN_LEVELS[1])?,
                gain_button(GAIN_LEVELS[2])?,
            ],
            background: unsafe { CreateSolidBrush(BACKGROUND) },
            fonts: Cell::new([HFONT::default(); 3]),
            enabled_epoch: Cell::new(None),
            gain_epoch: Cell::new(None),
            gain_displayed: Cell::new(("", None)),
            drag: Cell::new(None),
            key: Cell::new(None),
            wheel_remainder: Cell::new(0),
            displayed: Cell::new(("", None)),
        };
        unsafe {
            SetWindowSubclass(this.slider, Some(slider_proc), 1, 0).ok()?;
            SendMessageW(
                this.slider,
                TBM_SETRANGEMIN,
                Some(WPARAM(0)),
                Some(LPARAM(MIN_TARGET.into())),
            );
            SendMessageW(
                this.slider,
                TBM_SETRANGEMAX,
                Some(WPARAM(1)),
                Some(LPARAM(MAX_TARGET.into())),
            );
            let _ = EnableWindow(this.slider, false);
        }
        this.layout(unsafe { GetDpiForWindow(hwnd) }.max(96));
        Ok(this)
    }

    pub fn slider(&self) -> HWND {
        self.slider
    }

    pub fn visible(&self) -> bool {
        unsafe { IsWindowVisible(self.hwnd).as_bool() }
    }

    pub fn show(&self, anchor: RECT) {
        let monitor = unsafe { MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST) };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        unsafe {
            if !GetMonitorInfoW(monitor, &mut info).as_bool() {
                return;
            }
            // Move to the icon's monitor first so the window uses that monitor's DPI.
            let _ = SetWindowPos(
                self.hwnd,
                None,
                anchor.left,
                anchor.top,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
            let dpi = GetDpiForWindow(self.hwnd).max(96);
            let width = px(WIDTH, dpi);
            let height = px(HEIGHT, dpi);
            let gap = px(6, dpi);
            let work = info.rcWork;
            let center_x = (anchor.left + anchor.right) / 2;
            let center_y = (anchor.top + anchor.bottom) / 2;
            let (x, y) = if center_x <= work.left {
                (anchor.right + gap, center_y - height / 2)
            } else if center_x >= work.right {
                (anchor.left - gap - width, center_y - height / 2)
            } else if anchor.top - gap - height >= work.top {
                (center_x - width / 2, anchor.top - gap - height)
            } else {
                (center_x - width / 2, anchor.bottom + gap)
            };
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                x.clamp(work.left, (work.right - width).max(work.left)),
                y.clamp(work.top, (work.bottom - height).max(work.top)),
                width,
                height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = SetForegroundWindow(self.hwnd);
            if let Ok(first) = GetNextDlgTabItem(self.hwnd, None, false) {
                let _ = SetFocus(Some(first));
            }
        }
    }

    pub fn handle_message(&self, app: &App, message: &MSG) -> bool {
        if message.message != WM_KEYDOWN
            || !self.visible()
            || (message.hwnd != self.hwnd && unsafe { !IsChild(self.hwnd, message.hwnd).as_bool() })
        {
            return false;
        }
        match VIRTUAL_KEY(message.wParam.0 as u16) {
            VK_ESCAPE => app.hide(),
            VK_TAB => unsafe {
                let current = (message.hwnd != self.hwnd).then_some(message.hwnd);
                let reverse = GetKeyState(VK_SHIFT.0.into()) < 0;
                if let Ok(next) = GetNextDlgTabItem(self.hwnd, current, reverse) {
                    let _ = SetFocus(Some(next));
                }
            },
            _ => return false,
        }
        true
    }

    pub fn hide(&self) {
        if self.visible() {
            self.cancel_input();
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
        }
    }

    fn cancel_input(&self) {
        self.drag.set(None);
        self.key.set(None);
        self.wheel_remainder.set(0);
        unsafe {
            if GetCapture() == self.slider {
                let _ = ReleaseCapture();
            }
        }
    }

    pub fn dpi_changed(&self, dpi: u32, lp: LPARAM) {
        // Windows passes the suggested outer window rectangle for this DPI.
        let rect = unsafe { &*(lp.0 as *const RECT) };
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                None,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
        }
        self.layout(dpi);
    }

    fn layout(&self, dpi: u32) {
        unsafe {
            let fonts = [
                label_font(14, FW_BOLD, dpi),
                label_font(12, FW_NORMAL, dpi),
                label_font(14, FW_NORMAL, dpi),
            ];
            let old_fonts = self.fonts.replace(fonts);
            for (child, x, y, width, height, font) in [
                (self.title, 61, 17, 242, 19, Some(fonts[0])),
                (self.status, 61, 36, 242, 17, Some(fonts[1])),
                (self.volume, 17, 66, 205, 21, Some(fonts[2])),
                (self.value, 225, 66, 78, 21, Some(fonts[0])),
                (self.slider, 10, 90, 300, 24, None),
                (self.gain_label, 17, 130, 112, 19, Some(fonts[2])),
                (self.gain_current, 129, 132, 174, 17, Some(fonts[1])),
                (self.gain_buttons[0], 17, 155, 92, 28, None),
                (self.gain_buttons[1], 114, 155, 92, 28, None),
                (self.gain_buttons[2], 211, 155, 92, 28, None),
                (self.gain_status, 17, 194, 286, 17, Some(fonts[1])),
            ] {
                if let Some(font) = font {
                    SendMessageW(child, WM_SETFONT, Some(WPARAM(font.0 as usize)), None);
                }
                let _ = MoveWindow(
                    child,
                    px(x, dpi),
                    px(y, dpi),
                    px(width, dpi),
                    px(height, dpi),
                    true,
                );
            }
            for font in old_fonts {
                let _ = DeleteObject(HGDIOBJ(font.0));
            }
            let corner = px(24, dpi);
            let region = CreateRoundRectRgn(0, 0, px(WIDTH, dpi), px(HEIGHT, dpi), corner, corner);
            if !region.is_invalid() && SetWindowRgn(self.hwnd, Some(region), true) == 0 {
                let _ = DeleteObject(HGDIOBJ(region.0));
            }
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn label_color(&self, dc: HDC, label: HWND) -> LRESULT {
        unsafe {
            SetTextColor(
                dc,
                if [self.status, self.gain_current, self.gain_status].contains(&label) {
                    SECONDARY
                } else {
                    TEXT
                },
            );
            SetBkColor(dc, BACKGROUND);
        }
        LRESULT(self.background.0 as isize)
    }

    pub fn paint(&self) {
        unsafe {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(self.hwnd, &mut paint);
            let dpi = GetDpiForWindow(self.hwnd).max(96);
            let p = |dip| px(dip, dpi);
            let width = p(WIDTH);
            let height = p(HEIGHT);
            let line = p(1);
            let canvas = Shapes::new(dc);
            canvas.round(0, 0, width, height, p(24), BORDER);
            canvas.round(line, line, width - line, height - line, p(22), BACKGROUND);
            canvas.round(p(17), p(17), p(51), p(51), p(16), BADGE);

            // Draw the headband with an inset cutout, then the two earcups.
            canvas.round(p(28), p(27), p(40), p(40), p(12), BLUE);
            canvas.round(p(29), p(28), p(39), p(39), p(10), BADGE);
            canvas.round(p(27), p(34), p(41), p(42), 0, BADGE);
            canvas.round(p(27), p(34), p(32), p(41), p(4), BLUE);
            canvas.round(p(36), p(34), p(41), p(41), p(4), BLUE);
            canvas.round(p(28), p(35), p(31), p(40), p(2), BADGE);
            canvas.round(p(37), p(35), p(40), p(40), p(2), BADGE);
            canvas.round(p(17), p(118), p(303), p(118) + line, 0, BORDER);
            drop(canvas);
            let _ = EndPaint(self.hwnd, &paint);
        }
    }

    pub unsafe fn draw_button(&self, lp: LPARAM) -> bool {
        let Some(draw) = (unsafe { (lp.0 as *const DRAWITEMSTRUCT).as_ref() }) else {
            return false;
        };
        let Some(index) = self
            .gain_buttons
            .iter()
            .position(|&hwnd| hwnd == draw.hwndItem)
        else {
            return false;
        };
        let mut rect = draw.rcItem;
        let dpi = unsafe { GetDpiForWindow(self.hwnd) }.max(96);
        let line = px(1, dpi);
        let state = draw.itemState.0;
        let focused = state & ODS_FOCUS.0 != 0;
        let pressed = state & ODS_SELECTED.0 != 0;
        let disabled = state & ODS_DISABLED.0 != 0;
        let canvas = Shapes::new(draw.hDC);
        canvas.round(rect.left, rect.top, rect.right, rect.bottom, 0, BACKGROUND);
        canvas.round(
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            px(8, dpi),
            if focused { BLUE } else { BORDER },
        );
        canvas.round(
            rect.left + line,
            rect.top + line,
            rect.right - line,
            rect.bottom - line,
            px(6, dpi),
            if pressed { BUTTON_PRESSED } else { BUTTON },
        );
        let mut label = wide(level_name(GAIN_LEVELS[index]));
        unsafe {
            let font = SelectObject(draw.hDC, HGDIOBJ(self.fonts.get()[2].0));
            let color = SetTextColor(draw.hDC, if disabled { DISABLED_TEXT } else { TEXT });
            let mode = SetBkMode(draw.hDC, TRANSPARENT);
            DrawTextW(
                draw.hDC,
                &mut label,
                &mut rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
            SetBkMode(draw.hDC, BACKGROUND_MODE(mode as u32));
            SetTextColor(draw.hDC, color);
            SelectObject(draw.hDC, font);
        }
        true
    }

    pub fn render_gain(&self, snapshot: GainSnapshot) {
        let epoch = snapshot.can_set_gain().then_some(snapshot.epoch);
        if self.gain_epoch.replace(epoch).is_some() != epoch.is_some() {
            for button in self.gain_buttons {
                unsafe {
                    let _ = EnableWindow(button, epoch.is_some());
                }
            }
        }
        let status = match (snapshot.requested, snapshot.last_sent, snapshot.status) {
            (Some(level), _, _) => ("Sending ", Some(level)),
            (_, Some(level), Status::Read(_)) => ("Last sent: ", Some(level)),
            (_, _, Status::Offline) if snapshot.error.is_some() => {
                ("Gain write failed · DAC disconnected", None)
            }
            (_, _, Status::Read(_)) if snapshot.error.is_some() => {
                ("Gain write failed · Ready to retry", None)
            }
            (_, _, _) if snapshot.error.is_some() => ("Gain write failed · Waiting for DAC", None),
            _ => ("Last sent: —", None),
        };
        if self.gain_displayed.replace(status) != status {
            let (prefix, level) = status;
            let label = match level {
                Some(level) => {
                    let suffix = if snapshot.requested.is_some() {
                        "…"
                    } else {
                        ""
                    };
                    wide(&format!("{prefix}{}{suffix}", level_name(level)))
                }
                None => wide(prefix),
            };
            unsafe {
                let _ = SetWindowTextW(self.gain_status, PCWSTR(label.as_ptr()));
            }
        }
    }

    pub fn command(&self, app: &App, wp: WPARAM, lp: LPARAM) {
        if (wp.0 >> 16) as u16 == BN_CLICKED as u16
            && self.visible()
            && let Some(epoch) = self.gain_epoch.get()
            && let Some(index) = self
                .gain_buttons
                .iter()
                .position(|&button| button.0 as isize == lp.0)
        {
            app.worker.submit_gain(epoch, GAIN_LEVELS[index]);
            self.render_gain(app.worker.gain_snapshot());
        }
    }

    pub unsafe fn custom_draw(&self, lp: LPARAM) -> Option<LRESULT> {
        // Trackbar notifications borrow an NMHDR (or NMCUSTOMDRAW for NM_CUSTOMDRAW).
        let header = unsafe { (lp.0 as *const NMHDR).as_ref()? };
        if header.hwndFrom != self.slider || header.code != NM_CUSTOMDRAW {
            return None;
        }
        let draw = unsafe { &*(lp.0 as *const NMCUSTOMDRAW) };
        if draw.dwDrawStage != CDDS_PREPAINT {
            return None;
        }
        let slider_rect = |message| {
            let mut rect = RECT::default();
            unsafe {
                SendMessageW(
                    self.slider,
                    message,
                    None,
                    Some(LPARAM(std::ptr::from_mut(&mut rect) as isize)),
                );
            }
            rect
        };
        let channel = slider_rect(TBM_GETCHANNELRECT);
        let thumb = slider_rect(TBM_GETTHUMBRECT);
        let mut client = RECT::default();
        unsafe {
            let _ = GetClientRect(self.slider, &mut client);
        }
        let dpi = unsafe { GetDpiForWindow(self.hwnd) }.max(96);
        let p = |dip| px(dip, dpi);
        let x = (thumb.left + thumb.right) / 2;
        let y = p(11);
        // The thumb stays centered on the native hit target at every volume.
        let left = channel.left;
        let right = channel.right;
        let line = p(1);
        let canvas = Shapes::new(draw.hdc);
        canvas.round(0, 0, client.right, client.bottom, 0, BACKGROUND);
        canvas.round(left, y - p(4), right, y + p(4), p(8), TRACK_BORDER);
        canvas.round(
            left + line,
            y - p(4) + line,
            right - line,
            y + p(4) - line,
            p(6),
            TRACK,
        );
        if self.displayed.get().1.is_some() {
            let color = if self.enabled_epoch.get().is_some() {
                BLUE
            } else {
                MUTED_BLUE
            };
            canvas.round(
                left + line,
                y - p(4) + line,
                x,
                y + p(4) - line,
                p(6),
                color,
            );
            canvas.round(x - p(7), y - p(7), x + p(8), y + p(8), p(15), color);
        }
        Some(LRESULT(CDRF_SKIPDEFAULT as isize))
    }

    pub fn render(&self, snapshot: Snapshot) {
        let enabled = snapshot.adjustable().then_some(snapshot.epoch);
        if self.enabled_epoch.replace(enabled) != enabled {
            self.cancel_input();
            unsafe {
                let _ = EnableWindow(self.slider, enabled.is_some());
                let _ = InvalidateRect(Some(self.slider), None, false);
                if enabled.is_some() && GetForegroundWindow() == self.hwnd {
                    let _ = SetFocus(Some(self.slider));
                }
            }
        }
        let (status, confirmed) = match snapshot.status {
            Status::Reading => ("Reading volume…", None),
            Status::Offline => ("DAC disconnected", None),
            Status::Multiple => ("Connect only one DC04 Pro", None),
            Status::Unavailable => ("Unable to read DAC · Retrying", None),
            Status::Sync => ("Sync needed · Reading DAC again…", None),
            Status::Read(reading) => {
                let status = if !reading.centered() {
                    "Center balance unconfirmed"
                } else if !reading.adjustable() {
                    "Volume limit unverified"
                } else if snapshot.requested.is_some() {
                    "Adjusting volume…"
                } else {
                    "Connected via USB"
                };
                (status, Some(reading.step()))
            }
        };
        let step = if self.drag.get().is_some() {
            Some(self.position())
        } else {
            snapshot.requested.or(confirmed)
        };
        if let Some(step) = step {
            self.set_position(step.clamp(MIN_TARGET, MAX_TARGET));
        }
        let (old_status, old_step) = self.displayed.replace((status, step));
        unsafe {
            if old_status != status {
                let status = wide(status);
                let _ = SetWindowTextW(self.status, PCWSTR(status.as_ptr()));
            }
            if old_step != step {
                let value =
                    wide(&step.map_or_else(|| "— / 100".into(), |step| format!("{step} / 100")));
                let _ = SetWindowTextW(self.value, PCWSTR(value.as_ptr()));
                let _ = InvalidateRect(Some(self.slider), None, false);
            }
        }
    }

    fn position(&self) -> u8 {
        unsafe { SendMessageW(self.slider, TBM_GETPOS, None, None).0 as u8 }
    }

    fn set_position(&self, step: u8) {
        if self.position() != step {
            unsafe {
                SendMessageW(
                    self.slider,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(step.into())),
                );
            }
        }
    }

    fn submit(&self, app: &App, epoch: u64, step: u8) {
        if self.visible() && app.worker.submit(epoch, step) {
            self.set_position(step);
            self.render(app.worker.snapshot());
        } else {
            self.render(app.worker.snapshot());
        }
    }

    fn adjust(&self, app: &App, epoch: u64, delta: i32) {
        let position = i32::from(self.position());
        let step = (position + delta).clamp(MIN_TARGET.into(), MAX_TARGET.into());
        if self.drag.get().is_none() && step != position {
            self.submit(app, epoch, step as u8);
        }
    }

    pub fn drag_update(&self, app: &App) {
        if let Some(epoch) = self.drag.get() {
            self.submit(app, epoch, self.position());
        }
    }

    pub fn wheel(&self, app: &App, wp: WPARAM, lp: LPARAM) {
        let mut rect = RECT::default();
        if let Some(epoch) = self.enabled_epoch.get()
            && unsafe {
                GetWindowRect(self.slider, &mut rect).is_ok()
                    && PtInRect(&rect, point(lp.0)).as_bool()
            }
        {
            let amount = self.wheel_remainder.get() + ((wp.0 >> 16) as i16 as i32);
            self.wheel_remainder.set(amount % WHEEL_DELTA as i32);
            self.adjust(app, epoch, amount / WHEEL_DELTA as i32);
        }
    }
}

impl Drop for Popup {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.background.0));
            for font in self.fonts.get() {
                let _ = DeleteObject(HGDIOBJ(font.0));
            }
        }
    }
}

const fn rgb(r: u32, g: u32, b: u32) -> COLORREF {
    COLORREF(r | (g << 8) | (b << 16))
}

fn level_name(level: GainLevel) -> &'static str {
    match level {
        GainLevel::Low => "Low",
        GainLevel::Mid => "Mid",
        GainLevel::High => "High",
    }
}

fn label_font(size: i32, weight: FONT_WEIGHT, dpi: u32) -> HFONT {
    unsafe {
        CreateFontW(
            -px(size, dpi),
            0,
            0,
            0,
            weight.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            0,
            w!("Segoe UI"),
        )
    }
}

struct Shapes {
    dc: HDC,
    brush: HGDIOBJ,
    pen: HGDIOBJ,
    color: COLORREF,
}

impl Shapes {
    fn new(dc: HDC) -> Self {
        unsafe {
            Self {
                dc,
                brush: SelectObject(dc, GetStockObject(DC_BRUSH)),
                pen: SelectObject(dc, GetStockObject(NULL_PEN)),
                color: GetDCBrushColor(dc),
            }
        }
    }

    fn round(&self, left: i32, top: i32, right: i32, bottom: i32, diameter: i32, color: COLORREF) {
        if right > left && bottom > top {
            unsafe {
                SetDCBrushColor(self.dc, color);
                if diameter == 0 {
                    let _ = Rectangle(self.dc, left, top, right, bottom);
                } else {
                    let _ = RoundRect(self.dc, left, top, right, bottom, diameter, diameter);
                }
            }
        }
    }
}

impl Drop for Shapes {
    fn drop(&mut self) {
        unsafe {
            SetDCBrushColor(self.dc, self.color);
            SelectObject(self.dc, self.pen);
            SelectObject(self.dc, self.brush);
        }
    }
}

fn px(dip: i32, dpi: u32) -> i32 {
    (dip * dpi as i32 + 48) / 96
}

unsafe extern "system" fn slider_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if let Some(app) = app() {
        let popup = &app.popup;
        match msg {
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => popup.drag.set(popup.enabled_epoch.get()),
            WM_LBUTTONUP => {
                let epoch = popup.drag.take();
                let result = unsafe { DefSubclassProc(hwnd, msg, wp, lp) };
                if let Some(epoch) = epoch {
                    popup.submit(&app, epoch, popup.position());
                }
                return result;
            }
            WM_CAPTURECHANGED | WM_CANCELMODE => popup.drag.set(None),
            WM_MOUSEWHEEL => {
                popup.wheel(&app, wp, lp);
                return LRESULT(0);
            }
            WM_KEYDOWN => {
                let key = VIRTUAL_KEY(wp.0 as u16);
                let delta = match key {
                    VK_LEFT | VK_DOWN => -1,
                    VK_RIGHT | VK_UP => 1,
                    _ => 0,
                };
                if delta != 0 {
                    if let Some(epoch) = popup.enabled_epoch.get() {
                        if lp.0 & (1 << 30) == 0 {
                            popup.key.set(Some((wp.0, epoch)));
                        }
                        if popup.key.get() == Some((wp.0, epoch)) {
                            popup.adjust(&app, epoch, delta);
                        }
                    }
                    return LRESULT(0);
                }
                if matches!(key, VK_HOME | VK_END | VK_PRIOR | VK_NEXT) {
                    return LRESULT(0);
                }
            }
            _ => {}
        }
    }
    unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
}
