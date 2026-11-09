//! Live widgets drawn inside the Windows taskbar.
//!
//! A dedicated thread owns a layered child window parented to Explorer's
//! `Shell_TrayWnd`, the model FluentFlyout and TrafficMonitor use. Parenting
//! attaches that thread's input queue to Explorer's, so it never does
//! anything but draw the strip: it must not share a thread with GPUI, whose
//! longer frames would freeze the taskbar. The bridge thread publishes
//! settings and limits here and performs the widgets' actions.

// The taskbar strip is a Windows Explorer feature; Linux keeps only the
// settings model it shares with the tray.
#![cfg_attr(not(windows), allow(dead_code))]

#[cfg(windows)]
mod floating;
#[cfg(windows)]
mod host;
pub(crate) mod model;
#[cfg(windows)]
mod obstacles;
#[cfg(windows)]
pub(crate) mod render;

use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

use crate::{
    instances::ProviderId,
    limits::ProviderLimits,
    settings::{FloatingCircleSlot, Settings, TaskbarSettings},
};

/// Horizontal space between neighboring widgets, in DIPs.
pub(crate) const WIDGET_GAP: f32 = 4.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FloatingCircleItem {
    pub(crate) slot: FloatingCircleSlot,
    pub(crate) widget: crate::settings::TaskbarWidget,
}

/// Settings the strip renders from.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Config {
    pub(crate) taskbar: TaskbarSettings,
    pub(crate) colored_icons: bool,
    pub(crate) floating_circle_enabled: bool,
    pub(crate) floating_circle_colors: crate::settings::FloatingCircleColors,
    pub(crate) floating_circle_position: Option<crate::settings::PopupPosition>,
    pub(crate) floating_circle_items: Vec<FloatingCircleItem>,
}

impl Config {
    fn from_settings(settings: &Settings) -> Self {
        let enabled = settings.enabled_providers();
        let first_visible = settings.taskbar.widgets.iter().find_map(|widget| {
            widget
                .provider()
                .filter(|provider| enabled.contains(provider))
                .map(|provider| (provider, widget.clone()))
        });
        let mut items = Vec::new();
        for (index, configured) in settings.floating_circle_slots.iter().take(2).enumerate() {
            let resolved = configured
                .provider()
                .filter(|provider| enabled.contains(provider))
                .or_else(|| {
                    (index == 0)
                        .then(|| first_visible.as_ref().map(|(provider, _)| *provider))
                        .flatten()
                })
                .or_else(|| (index == 0).then(|| enabled.first().copied()).flatten());
            let Some(provider) = resolved else { continue };
            let mut widget = settings
                .taskbar
                .widgets
                .iter()
                .find(|widget| widget.provider() == Some(provider))
                .cloned()
                .or_else(|| {
                    first_visible
                        .as_ref()
                        .filter(|(candidate, _)| *candidate == provider)
                        .map(|(_, widget)| widget.clone())
                })
                .unwrap_or_else(|| crate::settings::TaskbarWidget::for_provider(provider));
            widget.style = crate::settings::TaskbarWidgetStyle::Nested;
            widget.indicators.truncate(2);
            for indicator in &mut widget.indicators {
                indicator.limit_value = crate::settings::LimitValue::Remaining;
                indicator.color_mode = crate::settings::TrayColorMode::Status;
            }
            widget.show_icon = true;
            widget.show_labels = false;
            widget.show_values = false;
            widget.reset = crate::settings::TaskbarResetDisplay::Off;
            let mut slot = configured.clone();
            slot.provider_id = provider.id().into();
            items.push(FloatingCircleItem { slot, widget });
        }
        Self {
            taskbar: settings.taskbar.clone(),
            colored_icons: settings.use_colored_provider_icons,
            floating_circle_enabled: settings.floating_circle_enabled,
            floating_circle_colors: settings.floating_circle_colors,
            floating_circle_position: settings.floating_circle_position.clone(),
            floating_circle_items: items,
        }
    }
}

/// Something the user asked for from a widget; performed by the bridge.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Toggle the popup, opened on the widget's provider.
    Open {
        provider: Option<ProviderId>,
        anchor: (i32, i32),
        page: crate::settings::TaskbarClick,
        /// A second floating-circle click must still dismiss the popup while
        /// Settings is open; ordinary taskbar widgets retain live preview.
        dismiss_on_outside_click: bool,
        /// Which compact floating-circle slot opened the detail card.
        floating_slot: Option<usize>,
    },
    /// Open Settings on the editor of this widget.
    Edit(String),
    /// Open Settings on the Taskbar page.
    Settings,
    /// Turn the widgets off.
    Hide,
    /// Open the app's main Settings window.
    AppSettings {
        /// Taskbar/tray menus keep the quota surface visible as a live preview;
        /// the floating circle's context action opens Settings by itself.
        show_preview: bool,
    },
    Update,
    Exit,
    /// Persist the compact floating circle after a drag.
    MoveFloatingCircle(crate::settings::PopupPosition),
    /// Add or remove the optional second floating circle.
    SetSecondFloatingCircle(bool),
}

static ACTIONS: Mutex<Vec<Action>> = Mutex::new(Vec::new());
static UPDATE_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Mirrors the tray menu's update item into the widgets' menu.
pub fn set_update_available(available: bool) {
    UPDATE_AVAILABLE.store(available, Ordering::SeqCst);
}

#[cfg(windows)]
fn update_available() -> bool {
    UPDATE_AVAILABLE.load(Ordering::SeqCst)
}
static BRIDGE_THREAD: AtomicU32 = AtomicU32::new(0);

/// Remembers the thread that drains [`take_actions`] so a click can wake it.
pub fn register_bridge_thread() {
    #[cfg(windows)]
    BRIDGE_THREAD.store(
        unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() },
        Ordering::SeqCst,
    );
}

fn post_action(action: Action) {
    if let Ok(mut actions) = ACTIONS.lock() {
        actions.push(action);
    }
    #[cfg(windows)]
    {
        let thread = BRIDGE_THREAD.load(Ordering::SeqCst);
        if thread != 0 {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                    thread,
                    windows_sys::Win32::UI::WindowsAndMessaging::WM_NULL,
                    0,
                    0,
                );
            }
        }
    }
}

pub fn take_actions() -> Vec<Action> {
    ACTIONS
        .lock()
        .map(|mut actions| std::mem::take(&mut *actions))
        .unwrap_or_default()
}

/// Queue a clean application exit through the bridge so workers and persisted
/// state are flushed before the process terminates.
pub(crate) fn request_exit() {
    post_action(Action::Exit);
}

pub(crate) fn request_second_floating_circle(enabled: bool) {
    post_action(Action::SetSecondFloatingCircle(enabled));
}

/// The floating circle owns its own toggle. Its mouse press must not also be
/// interpreted as a generic click outside the quota card.
#[cfg(windows)]
pub(crate) fn point_is_floating_circle(x: i32, y: i32) -> bool {
    floating::contains_screen_point(x, y)
}

/// Publishes the latest settings. The host thread starts on the first
/// publish that enables widgets and lives for the rest of the process.
pub fn publish_settings(settings: &Settings) {
    #[cfg(windows)]
    {
        let config = Config::from_settings(settings);
        host::publish(host::Message::Config(config.clone()));
        floating::publish(floating::Message::Config(config));
    }
    #[cfg(not(windows))]
    let _ = settings;
}

/// Whether the taskbar uses the light theme. The taskbar follows the
/// "Windows mode" (`SystemUsesLightTheme`), not the app mode the popup uses.
#[cfg(windows)]
pub(crate) fn uses_light_theme() -> bool {
    use windows_sys::Win32::{
        Foundation::ERROR_SUCCESS,
        System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
    };
    let wide = |value: &str| value.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let subkey = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let name = wide("SystemUsesLightTheme");
    // Windows 10 and 11 default to a dark taskbar when the value is missing.
    let mut value = 0u32;
    let mut size = size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut value as *mut u32).cast(),
            &mut size,
        )
    };
    status == ERROR_SUCCESS && value != 0
}

#[cfg(not(windows))]
pub(crate) fn uses_light_theme() -> bool {
    false
}

/// A widget strip rendered for Settings, exactly as the taskbar draws it.
pub(crate) struct Preview {
    pub(crate) png: Vec<u8>,
    /// Size in DIPs.
    pub(crate) width: f32,
    pub(crate) height: f32,
}

/// Renders `widgets` side by side on a transparent strip `height` DIPs tall.
#[cfg(windows)]
pub(crate) fn preview(
    widgets: &[crate::settings::TaskbarWidget],
    limits: &ProviderLimits,
    appearance: model::Appearance,
    scale: f32,
    height: f32,
) -> Option<Preview> {
    use std::cell::RefCell;
    thread_local! {
        static RENDERER: RefCell<Option<render::Renderer>> = const { RefCell::new(None) };
    }
    let views = widgets
        .iter()
        .map(|widget| model::resolve(widget, limits, appearance))
        .collect::<Vec<_>>();
    if views.is_empty() {
        return None;
    }
    let metrics = render::Metrics {
        scale: scale.max(0.5),
        thickness: height,
        vertical: false,
    };
    let snapshot = RENDERER.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = render::Renderer::new().ok();
        }
        render::snapshot(slot.as_mut()?, &views, metrics, appearance, None)
    })?;
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, snapshot.width, snapshot.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&snapshot.rgba).ok()?;
    }
    Some(Preview {
        png,
        width: snapshot.width as f32 / metrics.scale,
        height: snapshot.height as f32 / metrics.scale,
    })
}

#[cfg(not(windows))]
pub(crate) fn preview(
    _widgets: &[crate::settings::TaskbarWidget],
    _limits: &ProviderLimits,
    _appearance: model::Appearance,
    _scale: f32,
    _height: f32,
) -> Option<Preview> {
    None
}

/// Publishes the latest limits snapshot.
pub fn publish_limits(limits: &ProviderLimits) {
    #[cfg(windows)]
    {
        host::publish(host::Message::Limits(limits.clone()));
        floating::publish(floating::Message::Limits(limits.clone()));
    }
    #[cfg(not(windows))]
    let _ = limits;
}

/// Publishes provider-scoped read failures to the floating quota surface.
/// Ordinary taskbar widgets keep their existing presentation; the floating
/// circle needs this state so unavailable data is never mistaken for 0%.
pub fn publish_provider_errors(errors: std::collections::HashMap<ProviderId, String>) {
    #[cfg(windows)]
    floating::publish(floating::Message::ProviderErrors(errors));
    #[cfg(not(windows))]
    let _ = errors;
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::{
        limits::{LimitWindow, RateLimits},
        settings::{ProviderKind, TaskbarWidget, TaskbarWidgetStyle},
    };

    fn sample() -> (Vec<TaskbarWidget>, ProviderLimits) {
        let codex = ProviderId::primary(ProviderKind::Codex);
        let window = |used| LimitWindow {
            used_percent: Some(used),
            resets_at: Some(chrono::Utc::now() + chrono::Duration::minutes(133)),
            duration_minutes: Some(300),
        };
        let limits = ProviderLimits::from_entries([(
            codex,
            RateLimits {
                primary: window(58),
                secondary: window(5),
                ..RateLimits::default()
            },
        )]);
        let widgets = TaskbarWidgetStyle::ALL
            .into_iter()
            .map(|style| TaskbarWidget {
                style,
                ..TaskbarWidget::for_provider(codex)
            })
            .collect();
        (widgets, limits)
    }

    #[test]
    fn every_style_lays_out_on_a_vertical_taskbar() {
        let (widgets, limits) = sample();
        let appearance = model::Appearance {
            light: false,
            colored_icons: true,
            accent: [0, 120, 212],
        };
        let mut renderer = render::Renderer::new().expect("Direct2D is available");
        let metrics = render::Metrics {
            scale: 1.5,
            thickness: 62.0,
            vertical: true,
        };
        for widget in &widgets {
            for show in [false, true] {
                let widget = TaskbarWidget {
                    show_icon: show,
                    show_labels: show,
                    show_values: show,
                    show_pace: show,
                    ..widget.clone()
                };
                let view = model::resolve(&widget, &limits, appearance);
                let length = renderer
                    .measure(&view, metrics)
                    .expect("the widget lays out");
                assert!(
                    (20.0..200.0).contains(&length),
                    "{:?} is {length} DIPs tall",
                    widget.style
                );
            }
        }
    }

    #[test]
    fn previews_render_every_style_at_common_scales() {
        let (widgets, limits) = sample();
        for light in [false, true] {
            for scale in [1.0, 1.25, 1.5, 2.0] {
                let appearance = model::Appearance {
                    light,
                    colored_icons: true,
                    accent: [0, 120, 212],
                };
                let preview =
                    preview(&widgets, &limits, appearance, scale, 48.0).expect("the strip renders");
                assert!(preview.width > 200.0);
                assert!((preview.height - 48.0).abs() < 1.0);
                if let Ok(dir) = std::env::var("TASKBAR_PREVIEW_DIR") {
                    let name =
                        format!("strip-{}-{scale}.png", if light { "light" } else { "dark" });
                    std::fs::write(std::path::Path::new(&dir).join(name), &preview.png).unwrap();
                }
            }
        }
    }

    #[test]
    fn floating_circle_reuses_a_compact_nested_widget() {
        let mut settings = Settings::default();
        settings.taskbar.widgets.clear();
        settings.instances[0].enabled = true;
        let provider = settings.enabled_providers()[0];
        let config = Config::from_settings(&settings);
        let widget = &config
            .floating_circle_items
            .first()
            .expect("an enabled provider supplies the circle")
            .widget;
        assert_eq!(widget.provider(), Some(provider));
        assert_eq!(widget.style, TaskbarWidgetStyle::Nested);
        assert!(widget.indicators.len() <= 2);
        assert!(widget.show_icon);
        assert!(!widget.show_labels);
        assert!(!widget.show_values);
        assert!(widget.indicators.iter().all(|indicator| {
            indicator.limit_value == crate::settings::LimitValue::Remaining
                && indicator.color_mode == crate::settings::TrayColorMode::Status
        }));
        assert_eq!(widget.reset, crate::settings::TaskbarResetDisplay::Off);
    }

    #[test]
    fn floating_circle_config_keeps_at_most_two_bound_slots() {
        let mut settings = Settings::default();
        settings.instances[0].enabled = true;
        let provider = settings.enabled_providers()[0];
        settings.floating_circle_slots = vec![
            crate::settings::FloatingCircleSlot::for_provider(provider),
            crate::settings::FloatingCircleSlot {
                provider_id: provider.id().into(),
            },
            crate::settings::FloatingCircleSlot::for_provider(provider),
        ];

        let config = Config::from_settings(&settings);

        assert_eq!(config.floating_circle_items.len(), 2);
        assert_eq!(
            config.floating_circle_items[1].slot.provider(),
            Some(provider)
        );
    }
}
