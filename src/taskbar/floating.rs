//! Compact topmost quota circle for Windows.
//!
//! This is deliberately independent from Explorer's taskbar window. It keeps
//! working while the taskbar is auto-hidden or unavailable, but shares the
//! taskbar model, Direct2D renderer and bridge actions.

use std::{
    cell::RefCell,
    collections::HashMap,
    ptr::null_mut,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicIsize, Ordering},
    },
};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM},
    Graphics::Gdi::{
        AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CombineRgn,
        CreateCompatibleDC, CreateDIBSection, CreateEllipticRgn, CreateRectRgn, CreateRoundRectRgn,
        DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW, HBITMAP, HDC, HGDIOBJ,
        MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, RGN_OR, ReleaseDC, SelectObject,
        SetWindowRgn,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{
            ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
        },
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClassNameW,
            GetCursorPos, GetMessageW, HWND_MESSAGE, HWND_TOPMOST, IDC_ARROW, IsWindow,
            LoadCursorW, MSG, PostMessageW, RegisterClassW, SW_SHOWNA, SWP_NOACTIVATE, SWP_NOMOVE,
            SWP_NOSIZE, SetTimer, SetWindowPos, ShowWindow, TranslateMessage, ULW_ALPHA,
            UpdateLayeredWindow, WM_APP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONUP,
            WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
            WS_POPUP, WindowFromPoint,
        },
    },
};

use super::{
    Action, Config,
    model::{self, Appearance, WidgetView},
    render::{
        FLOATING_CARD_HEIGHT, FLOATING_CARD_LEFT, FLOATING_CARD_RADIUS, FLOATING_CARD_TOP,
        FLOATING_CARD_WIDTH, FLOATING_HEIGHT, FLOATING_RING_DIAMETER, FLOATING_RING_LEFT,
        FLOATING_RING_TOP, FLOATING_WIDTH, Metrics, Renderer,
    },
};
use crate::{limits::ProviderLimits, settings::PopupPosition};

pub(super) enum Message {
    Config(Config),
    Limits(ProviderLimits),
    ProviderErrors(HashMap<crate::instances::ProviderId, String>),
}

const WM_WAKE: u32 = WM_APP + 21;
const WM_MOUSELEAVE: u32 = 0x02A3;
const TIMER_POLL: usize = 1;
const POLL_MS: u32 = 500;
const EDGE_MARGIN_DIP: f32 = 20.0;
const DRAG_THRESHOLD_DIP: f32 = 4.0;
const CIRCLE_GAP_DIP: f32 = 10.0;

static QUEUE: Mutex<Vec<Message>> = Mutex::new(Vec::new());
static CONTROLLER: AtomicIsize = AtomicIsize::new(0);
static STARTED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

pub(super) fn publish(message: Message) {
    let start = matches!(&message, Message::Config(config) if config.floating_circle_enabled);
    if let Ok(mut queue) = QUEUE.lock() {
        queue.retain(|queued| {
            !matches!(
                (queued, &message),
                (Message::Config(_), Message::Config(_))
                    | (Message::Limits(_), Message::Limits(_))
                    | (Message::ProviderErrors(_), Message::ProviderErrors(_))
            )
        });
        queue.push(message);
    }
    if start && !STARTED.swap(true, Ordering::SeqCst) {
        let spawned = std::thread::Builder::new()
            .name("floating-quota-circle".into())
            .spawn(run);
        if let Err(error) = spawned {
            STARTED.store(false, Ordering::SeqCst);
            trace(format!("Could not start the floating circle: {error}"));
        }
    }
    let controller = CONTROLLER.load(Ordering::SeqCst);
    if controller != 0 {
        unsafe { PostMessageW(controller as HWND, WM_WAKE, 0, 0) };
    }
}

fn trace(message: impl AsRef<str>) {
    #[cfg(debug_assertions)]
    eprintln!("floating circle: {}", message.as_ref());
    crate::logger::info(message);
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

const CONTROLLER_CLASS: &str = "AgentMinibarFloatingCircleController";
const CIRCLE_CLASS: &str = "AgentMinibarFloatingCircle";

/// Whether this screen point is over the shaped floating-circle window.
///
/// The popup's global outside-click detector must leave this click to the
/// circle itself. Otherwise mouse-down starts closing the quota card and the
/// circle's mouse-up can race that animation and reopen it.
pub(super) fn contains_screen_point(x: i32, y: i32) -> bool {
    let hwnd = unsafe { WindowFromPoint(POINT { x, y }) };
    if hwnd.is_null() {
        return false;
    }
    let mut class = [0u16; 64];
    let len =
        unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) }.max(0) as usize;
    CIRCLE_CLASS.encode_utf16().eq(class[..len].iter().copied())
}

fn run() {
    unsafe {
        let instance = GetModuleHandleW(null_mut());
        let controller_class = wide(CONTROLLER_CLASS);
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(controller_proc),
            hInstance: instance,
            lpszClassName: controller_class.as_ptr(),
            ..std::mem::zeroed()
        });
        let circle_class = wide(CIRCLE_CLASS);
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(circle_proc),
            hInstance: instance,
            lpszClassName: circle_class.as_ptr(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            ..std::mem::zeroed()
        });
        let controller = CreateWindowExW(
            0,
            controller_class.as_ptr(),
            null_mut(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            null_mut(),
            instance,
            null_mut(),
        );
        if controller.is_null() {
            trace("Could not create the floating circle controller");
            STARTED.store(false, Ordering::SeqCst);
            return;
        }
        HOST.with(|host| *host.borrow_mut() = Some(Host::new(controller)));
        CONTROLLER.store(controller as isize, Ordering::SeqCst);
        SetTimer(controller, TIMER_POLL, POLL_MS, None);
        with_host(Host::drain);

        let mut message = std::mem::zeroed::<MSG>();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn with_host<R>(f: impl FnOnce(&mut Host) -> R) -> Option<R> {
    HOST.with(|host| {
        let mut host = host.try_borrow_mut().ok()?;
        host.as_mut().map(f)
    })
}

unsafe extern "system" fn controller_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_WAKE => {
            with_host(Host::drain);
            0
        }
        WM_TIMER if wparam == TIMER_POLL => {
            with_host(Host::poll);
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

unsafe extern "system" fn circle_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_MOUSEMOVE => {
            let track = with_host(|host| host.mouse_move(hwnd)).unwrap_or(false);
            if track {
                let mut event = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe { TrackMouseEvent(&mut event) };
            }
            0
        }
        WM_MOUSELEAVE => {
            with_host(|host| host.mouse_leave(hwnd));
            0
        }
        WM_LBUTTONDOWN => {
            if with_host(|host| host.press(hwnd)).unwrap_or(false) {
                unsafe { SetCapture(hwnd) };
            }
            0
        }
        WM_LBUTTONUP => {
            unsafe { ReleaseCapture() };
            if let Some(Some(action)) = with_host(|host| host.release(hwnd)) {
                super::post_action(action);
            }
            0
        }
        WM_RBUTTONUP => {
            super::post_action(Action::AppSettings {
                show_preview: false,
            });
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    size: (i32, i32),
}

impl Surface {
    fn new(size: (i32, i32)) -> Option<Self> {
        unsafe {
            let dc = CreateCompatibleDC(null_mut());
            if dc.is_null() {
                return None;
            }
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = size.0;
            info.bmiHeader.biHeight = -size.1;
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            info.bmiHeader.biCompression = BI_RGB;
            let mut bits = null_mut();
            let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if bitmap.is_null() {
                DeleteDC(dc);
                return None;
            }
            let previous = SelectObject(dc, bitmap);
            Some(Self {
                dc,
                bitmap,
                previous,
                size,
            })
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

#[derive(Clone, Copy)]
struct Drag {
    cursor: (i32, i32),
    origin: (i32, i32),
    slot: usize,
    moved: bool,
}

struct Circle {
    window: HWND,
    view: WidgetView,
    surface: Option<Surface>,
    hover: bool,
    tracking: bool,
    pressed: bool,
    shown: bool,
}

impl Circle {
    fn new(view: WidgetView) -> Self {
        Self {
            window: null_mut(),
            view,
            surface: None,
            hover: false,
            tracking: false,
            pressed: false,
            shown: false,
        }
    }
}

struct Host {
    controller: HWND,
    config: Option<Config>,
    limits: ProviderLimits,
    circles: Vec<Circle>,
    appearance: Appearance,
    renderer: Option<Renderer>,
    position: Option<(i32, i32)>,
    scale: f32,
    width: i32,
    height: i32,
    drag: Option<Drag>,
    provider_errors: HashMap<crate::instances::ProviderId, String>,
}

impl Host {
    fn new(controller: HWND) -> Self {
        Self {
            controller,
            config: None,
            limits: ProviderLimits::default(),
            circles: Vec::new(),
            appearance: super::host::current_appearance(true),
            renderer: None,
            position: None,
            scale: 1.0,
            width: FLOATING_WIDTH as i32,
            height: FLOATING_HEIGHT as i32,
            drag: None,
            provider_errors: HashMap::new(),
        }
    }

    fn drain(&mut self) {
        let messages = QUEUE
            .lock()
            .map(|mut queue| std::mem::take(&mut *queue))
            .unwrap_or_default();
        if messages.is_empty() {
            return;
        }
        for message in messages {
            match message {
                Message::Config(config) => {
                    let moved = self
                        .config
                        .as_ref()
                        .map(|old| &old.floating_circle_position)
                        != Some(&config.floating_circle_position);
                    self.config = Some(config);
                    if moved && self.drag.is_none() {
                        self.position = None;
                    }
                }
                Message::Limits(limits) => self.limits = limits,
                Message::ProviderErrors(errors) => self.provider_errors = errors,
            }
        }
        self.reconcile();
    }

    fn poll(&mut self) {
        for circle in &mut self.circles {
            if !circle.window.is_null() && unsafe { IsWindow(circle.window) } == 0 {
                circle.window = null_mut();
                circle.surface = None;
                circle.shown = false;
            }
        }
        // The floating panel always uses the provider's own mark so its source
        // remains recognizable when several AI tools are configured.
        let appearance = super::host::current_appearance(true);
        let first_window = self
            .circles
            .first()
            .map_or(null_mut(), |circle| circle.window);
        let dpi = if first_window.is_null() {
            0
        } else {
            unsafe { GetDpiForWindow(first_window) }
        };
        let scale = if dpi == 0 {
            self.scale
        } else {
            dpi as f32 / 96.0
        };
        if appearance != self.appearance || (scale - self.scale).abs() > 0.01 {
            self.appearance = appearance;
            self.scale = scale;
            self.width = (FLOATING_WIDTH * scale).round() as i32;
            self.height = (FLOATING_HEIGHT * scale).round() as i32;
            if let Some(position) = self.position {
                self.position = Some(clamp_position(position, self.group_size(), None));
            }
            for circle in &mut self.circles {
                circle.surface = None;
            }
            self.apply_regions();
            self.reconcile();
        } else {
            for circle in &self.circles {
                if !circle.window.is_null() {
                    unsafe {
                        SetWindowPos(
                            circle.window,
                            HWND_TOPMOST,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }
            }
        }
    }

    fn reconcile(&mut self) {
        let Some(config) = self.config.as_ref() else {
            return;
        };
        if !config.floating_circle_enabled {
            self.hide();
            return;
        }
        if config.floating_circle_items.is_empty() {
            self.hide();
            return;
        }
        self.appearance = super::host::current_appearance(true);
        let views = config
            .floating_circle_items
            .iter()
            .map(|item| {
                let provider = item
                    .slot
                    .provider()
                    .expect("floating-circle config resolves every slot");
                let error = self.provider_errors.get(&provider).map(String::as_str);
                model::resolve_floating(
                    &item.widget,
                    provider,
                    &self.limits,
                    self.appearance,
                    error,
                )
            })
            .collect::<Vec<_>>();
        while self.circles.len() > views.len() {
            if let Some(mut circle) = self.circles.pop() {
                Self::destroy_circle(&mut circle);
            }
        }
        for (index, view) in views.into_iter().enumerate() {
            if let Some(circle) = self.circles.get_mut(index) {
                circle.view = view;
            } else {
                self.circles.push(Circle::new(view));
            }
        }
        if self.renderer.is_none() {
            match Renderer::new() {
                Ok(renderer) => self.renderer = Some(renderer),
                Err(error) => {
                    trace(format!("Floating circle renderer failed: {error}"));
                    self.hide();
                    return;
                }
            }
        }
        if self.position.is_none() {
            self.position = Some(self.configured_position());
        }
        if !self.ensure_windows() {
            return;
        }
        self.render_all();
    }

    fn group_size(&self) -> (i32, i32) {
        let count = self.circles.len().max(1) as i32;
        let gap = (CIRCLE_GAP_DIP * self.scale).round() as i32;
        (self.width, self.height * count + gap * (count - 1))
    }

    fn slot_position(&self, index: usize) -> (i32, i32) {
        let origin = self.position.unwrap_or_default();
        let gap = (CIRCLE_GAP_DIP * self.scale).round() as i32;
        (origin.0, origin.1 + index as i32 * (self.height + gap))
    }

    fn configured_position(&self) -> (i32, i32) {
        if let Some(saved) = self
            .config
            .as_ref()
            .and_then(|config| config.floating_circle_position.as_ref())
        {
            return clamp_position((saved.screen_x, saved.screen_y), self.group_size(), None);
        }
        let mut cursor = POINT { x: 0, y: 0 };
        unsafe { GetCursorPos(&mut cursor) };
        let work = work_area(cursor);
        let margin = (EDGE_MARGIN_DIP * self.scale).round() as i32;
        let size = self.group_size();
        (work.right - margin - size.0, work.top + margin)
    }

    fn ensure_windows(&mut self) -> bool {
        for index in 0..self.circles.len() {
            if !self.circles[index].window.is_null() {
                continue;
            }
            let position = self.slot_position(index);
            let window = unsafe {
                let class = wide(CIRCLE_CLASS);
                let title = wide("Agent Minibar Floating Circle");
                CreateWindowExW(
                    WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                    class.as_ptr(),
                    title.as_ptr(),
                    WS_POPUP,
                    position.0,
                    position.1,
                    self.width,
                    self.height,
                    null_mut(),
                    null_mut(),
                    GetModuleHandleW(null_mut()),
                    null_mut(),
                )
            };
            if window.is_null() {
                trace(format!(
                    "Could not create floating circle {}: {}",
                    index + 1,
                    std::io::Error::last_os_error()
                ));
                return false;
            }
            self.circles[index].window = window;
            let dpi = unsafe { GetDpiForWindow(window) };
            self.scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
            self.width = (FLOATING_WIDTH * self.scale).round() as i32;
            self.height = (FLOATING_HEIGHT * self.scale).round() as i32;
            self.position = Some(clamp_position(
                self.position.unwrap_or_default(),
                self.group_size(),
                None,
            ));
            self.apply_region(window);
        }
        self.move_windows();
        true
    }

    fn apply_regions(&self) {
        for circle in &self.circles {
            self.apply_region(circle.window);
        }
    }

    fn apply_region(&self, window: HWND) {
        if window.is_null() {
            return;
        }
        let px = |dip: f32| (dip * self.scale).round() as i32;
        unsafe {
            let circle = CreateEllipticRgn(
                px(FLOATING_RING_LEFT),
                px(FLOATING_RING_TOP),
                px(FLOATING_RING_LEFT + FLOATING_RING_DIAMETER) + 1,
                px(FLOATING_RING_TOP + FLOATING_RING_DIAMETER) + 1,
            );
            let card = CreateRoundRectRgn(
                px(FLOATING_CARD_LEFT),
                px(FLOATING_CARD_TOP),
                px(FLOATING_CARD_LEFT + FLOATING_CARD_WIDTH) + 1,
                px(FLOATING_CARD_TOP + FLOATING_CARD_HEIGHT) + 1,
                px(FLOATING_CARD_RADIUS * 2.0),
                px(FLOATING_CARD_RADIUS * 2.0),
            );
            let combined = CreateRectRgn(0, 0, 0, 0);
            if !circle.is_null() && !card.is_null() && !combined.is_null() {
                CombineRgn(combined, circle, card, RGN_OR);
                // Windows owns the combined region after a successful call.
                if SetWindowRgn(window, combined, 1) == 0 {
                    DeleteObject(combined);
                }
            } else if !combined.is_null() {
                DeleteObject(combined);
            }
            if !circle.is_null() {
                DeleteObject(circle);
            }
            if !card.is_null() {
                DeleteObject(card);
            }
        }
    }

    fn render_all(&mut self) {
        for index in 0..self.circles.len() {
            self.render(index);
        }
    }

    fn render(&mut self, index: usize) {
        let position = self.slot_position(index);
        let Some(circle) = self.circles.get_mut(index) else {
            return;
        };
        if circle
            .surface
            .as_ref()
            .is_none_or(|surface| surface.size != (self.width, self.height))
        {
            circle.surface = Surface::new((self.width, self.height));
        }
        let Some(surface) = circle.surface.as_ref() else {
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let metrics = Metrics {
            scale: self.scale,
            thickness: FLOATING_HEIGHT,
            vertical: false,
        };
        let progress = circle
            .view
            .rows
            .iter()
            .map(|row| row.progress)
            .collect::<Vec<_>>();
        let colors = self
            .config
            .as_ref()
            .map(|config| config.floating_circle_colors)
            .unwrap_or_default();
        if let Err(error) = renderer.paint_floating_panel(
            surface.dc,
            (self.width, self.height),
            metrics,
            self.appearance,
            &circle.view,
            &progress,
            colors,
            circle.hover,
            circle.pressed,
        ) {
            trace(format!("Floating circle paint failed: {error}"));
            return;
        }
        unsafe {
            let screen = GetDC(null_mut());
            let destination = POINT {
                x: position.0,
                y: position.1,
            };
            let size = SIZE {
                cx: self.width,
                cy: self.height,
            };
            let source = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            UpdateLayeredWindow(
                circle.window,
                screen,
                &destination,
                &size,
                surface.dc,
                &source,
                0,
                &blend,
                ULW_ALPHA,
            );
            ReleaseDC(null_mut(), screen);
            if !circle.shown {
                ShowWindow(circle.window, SW_SHOWNA);
                circle.shown = true;
            }
        }
    }

    fn circle_index(&self, hwnd: HWND) -> Option<usize> {
        self.circles.iter().position(|circle| circle.window == hwnd)
    }

    fn mouse_move(&mut self, hwnd: HWND) -> bool {
        let Some(index) = self.circle_index(hwnd) else {
            return false;
        };
        let track = !self.circles[index].tracking;
        self.circles[index].tracking = true;
        if let Some(mut drag) = self.drag {
            let mut cursor = POINT { x: 0, y: 0 };
            unsafe { GetCursorPos(&mut cursor) };
            let dx = cursor.x - drag.cursor.0;
            let dy = cursor.y - drag.cursor.1;
            let threshold = (DRAG_THRESHOLD_DIP * self.scale).round() as i32;
            drag.moved |= dx.abs() >= threshold || dy.abs() >= threshold;
            self.drag = Some(drag);
            if drag.moved {
                self.position = Some(clamp_position(
                    (drag.origin.0 + dx, drag.origin.1 + dy),
                    self.group_size(),
                    Some(cursor),
                ));
                self.move_windows();
                self.render_all();
            }
        } else if !self.circles[index].hover {
            self.circles[index].hover = true;
            self.render(index);
        }
        track
    }

    fn mouse_leave(&mut self, hwnd: HWND) {
        let Some(index) = self.circle_index(hwnd) else {
            return;
        };
        self.circles[index].tracking = false;
        if self.drag.is_none() && self.circles[index].hover {
            self.circles[index].hover = false;
            self.render(index);
        }
    }

    fn press(&mut self, hwnd: HWND) -> bool {
        let Some(index) = self.circle_index(hwnd) else {
            return false;
        };
        let Some(position) = self.position else {
            return false;
        };
        let mut cursor = POINT { x: 0, y: 0 };
        unsafe { GetCursorPos(&mut cursor) };
        self.drag = Some(Drag {
            cursor: (cursor.x, cursor.y),
            origin: position,
            slot: index,
            moved: false,
        });
        self.circles[index].pressed = true;
        self.render(index);
        true
    }

    fn release(&mut self, _hwnd: HWND) -> Option<Action> {
        let drag = self.drag.take()?;
        if let Some(circle) = self.circles.get_mut(drag.slot) {
            circle.pressed = false;
        }
        self.render(drag.slot);
        let position = self.position?;
        if drag.moved {
            return Some(Action::MoveFloatingCircle(PopupPosition {
                monitor: String::new(),
                x_dip: (position.0 as f32 / self.scale).round() as i32,
                y_dip: (position.1 as f32 / self.scale).round() as i32,
                screen_x: position.0,
                screen_y: position.1,
            }));
        }
        let provider = self
            .circles
            .get(drag.slot)
            .and_then(|circle| circle.view.provider);
        let page = self
            .config
            .as_ref()
            .map(|config| config.taskbar.click)
            .unwrap_or_default();
        Some(Action::Open {
            provider,
            anchor: (
                self.slot_position(drag.slot).0
                    + ((FLOATING_RING_LEFT + FLOATING_RING_DIAMETER / 2.0) * self.scale).round()
                        as i32,
                self.slot_position(drag.slot).1
                    + ((FLOATING_RING_TOP + FLOATING_RING_DIAMETER / 2.0) * self.scale).round()
                        as i32,
            ),
            page,
            dismiss_on_outside_click: true,
            floating_slot: Some(drag.slot),
        })
    }

    fn move_windows(&self) {
        for (index, circle) in self.circles.iter().enumerate() {
            if circle.window.is_null() {
                continue;
            }
            let position = self.slot_position(index);
            unsafe {
                SetWindowPos(
                    circle.window,
                    HWND_TOPMOST,
                    position.0,
                    position.1,
                    self.width,
                    self.height,
                    SWP_NOACTIVATE,
                );
            }
        }
    }

    fn destroy_circle(circle: &mut Circle) {
        circle.surface = None;
        circle.shown = false;
        if !circle.window.is_null() {
            let window = std::mem::replace(&mut circle.window, null_mut());
            unsafe { DestroyWindow(window) };
        }
    }

    fn hide(&mut self) {
        self.drag = None;
        for circle in &mut self.circles {
            Self::destroy_circle(circle);
        }
        self.circles.clear();
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.hide();
        let _ = self.controller;
    }
}

fn work_area(point: POINT) -> windows_sys::Win32::Foundation::RECT {
    unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..std::mem::zeroed()
        };
        if !monitor.is_null() && GetMonitorInfoW(monitor, &mut info) != 0 {
            info.rcWork
        } else {
            info.rcMonitor
        }
    }
}

fn clamp_position(position: (i32, i32), size: (i32, i32), cursor: Option<POINT>) -> (i32, i32) {
    let point = cursor.unwrap_or(POINT {
        x: position.0 + size.0 / 2,
        y: position.1 + size.1 / 2,
    });
    let work = work_area(point);
    (
        position
            .0
            .clamp(work.left, (work.right - size.0).max(work.left)),
        position
            .1
            .clamp(work.top, (work.bottom - size.1).max(work.top)),
    )
}
