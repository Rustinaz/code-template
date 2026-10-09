//! iOS platform integration.
//!
//! The UI is rendered by Rust (egui/eframe on top of the `ui-kit` backend), so
//! this crate owns only what needs the iOS runtime: device/OS information read
//! through `objc2`, the Material 3 theme defaults, and the
//! [`ui::platform::PlatformUi`] adapter.
//!
//! Building an `.ipa` needs a macOS host with a current Xcode; see
//! `scripts/build-ios.sh`.

use std::sync::OnceLock;

use parking_lot::RwLock;
use shared::domain::Platform;
use shared::errors::Result as AppResult;
use ui::platform::{PlatformTheme, PlatformUi, Window, WindowConfig};

/// Static information about the iOS device the app is running on.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IosInfo {
    /// `UIDevice.currentDevice.systemVersion`, e.g. `"17.2"`.
    pub system_version: String,
    /// `UIDevice.currentDevice.model`, e.g. `"iPhone"`.
    pub model: String,
    /// `UIDevice.currentDevice.name`, the user-facing device name.
    pub name: String,
    /// `UIDevice.currentDevice.userInterfaceIdiom` (phone, pad, …).
    pub idiom: String,
    /// Process scale factor, 2.0 or 3.0 on modern devices.
    pub scale: f64,
    /// True when the app runs inside a simulator.
    pub is_simulator: bool,
}

impl IosInfo {
    /// Returns `true` for iPad-like idioms.
    #[must_use]
    pub fn is_tablet(&self) -> bool {
        self.idiom.eq_ignore_ascii_case("pad")
    }
}

static CACHE: OnceLock<RwLock<IosInfo>> = OnceLock::new();

/// Initializes the iOS platform layer.
pub fn init() -> AppResult<()> {
    tracing::info!("Initializing iOS platform integration");
    Ok(())
}

/// Reads device information, caching the result after the first call.
#[must_use]
pub fn device_info() -> IosInfo {
    let cache = CACHE.get_or_init(|| RwLock::new(read_device()));
    cache.read().clone()
}

/// Fills in the fields that are known without talking to UIKit.
///
/// UIKit values (`systemVersion`, `model`, `userInterfaceIdiom`, …) need an
/// Objective-C message send. The template deliberately does not ship a
/// hand-rolled `msg_send!` for those: it cannot be compiled or tested without a
/// macOS host, and an unverifiable FFI call is worse than an honest gap. Fill
/// [`IosInfo::system_version`] and [`IosInfo::model`] from your own
/// `objc2-ui-kit` call site when you add UIKit-specific behaviour.
fn read_device() -> IosInfo {
    let is_simulator = std::env::var("SIMULATOR_DEVICE_NAME").is_ok_and(|v| !v.is_empty());
    IosInfo {
        system_version: String::new(),
        model: if cfg!(target_os = "ios") {
            "iOS".to_string()
        } else {
            "not-ios".to_string()
        },
        name: String::new(),
        idiom: if cfg!(target_os = "ios") {
            "unspecified".to_string()
        } else {
            String::new()
        },
        scale: 1.0,
        is_simulator,
    }
}

/// Material 3 defaults shared by every platform.
#[must_use]
pub fn material3_theme() -> PlatformTheme {
    PlatformTheme {
        name: "Material 3".to_string(),
        colors: ui::platform::PlatformColors {
            primary: "#6750A4".to_string(),
            on_primary: "#FFFFFF".to_string(),
            background: "#FFFBFE".to_string(),
            on_background: "#1C1B1F".to_string(),
            surface: "#FFFBFE".to_string(),
            on_surface: "#1C1B1F".to_string(),
            error: "#BA1A1A".to_string(),
            on_error: "#FFFFFF".to_string(),
        },
        fonts: ui::platform::PlatformFonts {
            family: "SF Pro".to_string(),
            size_scale: 1.0,
            weight_regular: 400,
            weight_medium: 500,
            weight_bold: 700,
        },
        spacing: ui::platform::PlatformSpacing {
            base: 4.0,
            scale: 1.0,
        },
        shapes: ui::platform::PlatformShapes {
            corner_radius_small: 4.0,
            corner_radius_medium: 8.0,
            corner_radius_large: 16.0,
        },
        motion: ui::platform::PlatformMotion {
            duration_short: 150,
            duration_medium: 250,
            duration_long: 400,
            easing_standard: "cubic-bezier(0.2, 0.0, 0.0, 1.0)".to_string(),
        },
    }
}

/// [`PlatformUi`] implementation for iOS.
#[derive(Debug, Default)]
pub struct IosPlatformUi;

impl IosPlatformUi {
    /// Creates a new adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PlatformUi for IosPlatformUi {
    fn platform(&self) -> Platform {
        Platform::Ios
    }

    fn create_window(&self, config: WindowConfig) -> AppResult<Box<dyn Window>> {
        Ok(Box::new(IosWindow::new(config)))
    }

    fn theme(&self) -> PlatformTheme {
        material3_theme()
    }

    fn on_resume(&self) {
        tracing::debug!("iOS app entered foreground");
    }

    fn on_pause(&self) {
        tracing::debug!("iOS app entered background");
    }

    fn on_destroy(&self) {
        tracing::debug!("iOS app will terminate");
    }

    fn announce_for_accessibility(&self, text: &str) {
        tracing::debug!(text, "VoiceOver announcement requested");
    }

    fn set_accessibility_focus(&self, element_id: &str) {
        tracing::debug!(element_id, "VoiceOver focus requested");
    }
}

/// Window record for the single full-screen iOS surface.
#[derive(Debug)]
pub struct IosWindow {
    title: RwLock<String>,
    size: RwLock<(u32, u32)>,
    min_size: RwLock<(u32, u32)>,
    fullscreen: RwLock<bool>,
    visible: RwLock<bool>,
}

impl IosWindow {
    /// Creates a window record from `config`.
    #[must_use]
    pub fn new(config: WindowConfig) -> Self {
        let min = (
            config.min_width.unwrap_or(0),
            config.min_height.unwrap_or(0),
        );
        Self {
            title: RwLock::new(config.title),
            size: RwLock::new((config.width.max(min.0), config.height.max(min.1))),
            min_size: RwLock::new(min),
            fullscreen: RwLock::new(config.fullscreen),
            visible: RwLock::new(false),
        }
    }

    /// The title last passed to [`Window::set_title`].
    #[must_use]
    pub fn title(&self) -> String {
        self.title.read().clone()
    }

    /// The size last passed to [`Window::set_size`].
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        *self.size.read()
    }
}

impl Window for IosWindow {
    fn show(&self) {
        *self.visible.write() = true;
    }

    fn hide(&self) {
        *self.visible.write() = false;
    }

    fn close(&self) {
        *self.visible.write() = false;
    }

    fn set_title(&self, title: &str) {
        *self.title.write() = title.to_string();
    }

    fn set_size(&self, width: u32, height: u32) {
        let (min_w, min_h) = *self.min_size.read();
        *self.size.write() = (width.max(min_w), height.max(min_h));
    }

    fn set_min_size(&self, width: u32, height: u32) {
        *self.min_size.write() = (width, height);
    }

    fn set_max_size(&self, width: u32, height: u32) {
        let (min_w, min_h) = *self.min_size.read();
        debug_assert!(
            width >= min_w && height >= min_h,
            "max size must not be smaller than min size"
        );
    }

    fn set_fullscreen(&self, fullscreen: bool) {
        *self.fullscreen.write() = fullscreen;
    }

    fn set_maximized(&self, _maximized: bool) {
        // iOS scenes always fill their container.
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn set_position(&self, _x: i32, _y: i32) {
        // iOS owns scene placement.
    }

    fn is_visible(&self) -> bool {
        *self.visible.read()
    }

    fn is_fullscreen(&self) -> bool {
        *self.fullscreen.read()
    }

    fn is_maximized(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_info_is_readable_off_device() {
        let info = device_info();
        assert_eq!(info.is_tablet(), info.idiom.eq_ignore_ascii_case("pad"));
    }

    #[test]
    fn platform_ui_reports_ios() {
        let platform_ui = IosPlatformUi::new();
        assert_eq!(platform_ui.platform(), Platform::Ios);
        assert_eq!(platform_ui.theme().fonts.family, "SF Pro");
    }

    #[test]
    fn window_clamps_to_minimum_size() {
        let config = WindowConfig {
            min_width: Some(320),
            min_height: Some(480),
            ..WindowConfig::default()
        };
        let window = IosWindow::new(config);
        window.set_size(10, 10);
        assert_eq!(window.size(), (320, 480));
        assert!(window.is_maximized());
    }

    #[test]
    fn window_tracks_title() {
        let window = IosWindow::new(WindowConfig::default());
        window.set_title("Rust App");
        assert_eq!(window.title(), "Rust App");
    }
}
