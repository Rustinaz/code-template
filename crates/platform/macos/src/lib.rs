//! macOS platform integration.
//!
//! The UI is rendered by Rust (egui/eframe on the `winit` macOS backend), so
//! this crate owns only what needs the AppKit runtime: OS information, the
//! Material 3 theme defaults, and the [`ui::platform::PlatformUi`] adapter.

use std::sync::OnceLock;

use parking_lot::RwLock;
use shared::domain::Platform;
use shared::errors::Result as AppResult;
use ui::platform::{PlatformTheme, PlatformUi, Window, WindowConfig};

/// Static information about the macOS host.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MacosInfo {
    /// macOS version, e.g. `"14.5"`.
    pub os_version: String,
    /// CPU architecture the binary was compiled for.
    pub arch: &'static str,
    /// Number of logical CPUs, 0 when it could not be determined.
    pub cpu_count: usize,
}

/// Initializes the macOS platform layer.
pub fn init() -> AppResult<()> {
    tracing::info!("Initializing macOS platform integration");
    Ok(())
}

static CACHE: OnceLock<RwLock<MacosInfo>> = OnceLock::new();

/// Reads host information, caching the result after the first call.
#[must_use]
pub fn host_info() -> MacosInfo {
    let cache = CACHE.get_or_init(|| RwLock::new(read_host()));
    cache.read().clone()
}

#[cfg(target_os = "macos")]
fn read_host() -> MacosInfo {
    MacosInfo {
        os_version: read_os_version(),
        arch: std::env::consts::ARCH,
        cpu_count: std::thread::available_parallelism().map_or(0, |n| n.get()),
    }
}

/// Reads `sw_vers -productVersion`, e.g. `"14.5"`.
///
/// `NSProcessInfo.operatingSystemVersion` would avoid the subprocess, but it
/// needs an Objective-C message send, which this template does not ship without
/// a macOS host to test it on. `sw_vers` is the documented command-line
/// equivalent, and `host_info` caches the result, so it runs at most once per
/// process.
#[cfg(target_os = "macos")]
fn read_os_version() -> String {
    std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}

#[cfg(not(target_os = "macos"))]
fn read_host() -> MacosInfo {
    MacosInfo {
        arch: std::env::consts::ARCH,
        cpu_count: std::thread::available_parallelism().map_or(0, |n| n.get()),
        ..MacosInfo::default()
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

/// [`PlatformUi`] implementation for macOS.
#[derive(Debug, Default)]
pub struct MacosPlatformUi;

impl MacosPlatformUi {
    /// Creates a new adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PlatformUi for MacosPlatformUi {
    fn platform(&self) -> Platform {
        Platform::Macos
    }

    fn create_window(&self, config: WindowConfig) -> AppResult<Box<dyn Window>> {
        Ok(Box::new(MacosWindow::new(config)))
    }

    fn theme(&self) -> PlatformTheme {
        material3_theme()
    }

    fn on_resume(&self) {
        tracing::debug!("macOS app became active");
    }

    fn on_pause(&self) {
        tracing::debug!("macOS app became inactive");
    }

    fn on_destroy(&self) {
        tracing::debug!("macOS window closed");
    }

    fn announce_for_accessibility(&self, text: &str) {
        tracing::debug!(text, "VoiceOver announcement requested");
    }

    fn set_accessibility_focus(&self, element_id: &str) {
        tracing::debug!(element_id, "VoiceOver focus requested");
    }
}

/// Window handle for the single macOS window.
#[derive(Debug)]
pub struct MacosWindow {
    title: RwLock<String>,
    size: RwLock<(u32, u32)>,
    min_size: RwLock<(u32, u32)>,
    max_size: RwLock<Option<(u32, u32)>>,
    position: RwLock<(i32, i32)>,
    fullscreen: RwLock<bool>,
    maximized: RwLock<bool>,
    visible: RwLock<bool>,
}

impl MacosWindow {
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
            max_size: RwLock::new(config.max_width.zip(config.max_height)),
            position: RwLock::new((0, 0)),
            fullscreen: RwLock::new(config.fullscreen),
            maximized: RwLock::new(config.maximized),
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

impl Window for MacosWindow {
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
        let max = *self.max_size.read();
        let mut w = width.max(min_w);
        let mut h = height.max(min_h);
        if let Some((max_w, max_h)) = max {
            w = w.min(max_w);
            h = h.min(max_h);
        }
        *self.size.write() = (w, h);
    }

    fn set_min_size(&self, width: u32, height: u32) {
        *self.min_size.write() = (width, height);
    }

    fn set_max_size(&self, width: u32, height: u32) {
        *self.max_size.write() = Some((width, height));
    }

    fn set_fullscreen(&self, fullscreen: bool) {
        *self.fullscreen.write() = fullscreen;
    }

    fn set_maximized(&self, maximized: bool) {
        *self.maximized.write() = maximized;
    }

    fn position(&self) -> (i32, i32) {
        *self.position.read()
    }

    fn set_position(&self, x: i32, y: i32) {
        *self.position.write() = (x, y);
    }

    fn is_visible(&self) -> bool {
        *self.visible.read()
    }

    fn is_fullscreen(&self) -> bool {
        *self.fullscreen.read()
    }

    fn is_maximized(&self) -> bool {
        *self.maximized.read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_info_is_readable_off_device() {
        let info = host_info();
        assert_eq!(info.arch, std::env::consts::ARCH);
    }

    #[test]
    fn platform_ui_reports_macos() {
        let platform_ui = MacosPlatformUi::new();
        assert_eq!(platform_ui.platform(), Platform::Macos);
        assert_eq!(platform_ui.theme().fonts.family, "SF Pro");
    }

    #[test]
    fn window_clamps_between_min_and_max() {
        let config = WindowConfig {
            min_width: Some(400),
            min_height: Some(300),
            max_width: Some(1280),
            max_height: Some(720),
            ..WindowConfig::default()
        };
        let window = MacosWindow::new(config);

        window.set_size(100, 100);
        assert_eq!(window.size(), (400, 300));

        window.set_size(4000, 4000);
        assert_eq!(window.size(), (1280, 720));
    }

    #[test]
    fn window_tracks_title_and_position() {
        let window = MacosWindow::new(WindowConfig::default());
        window.set_title("Rust App");
        window.set_position(10, 20);
        assert_eq!(window.title(), "Rust App");
        assert_eq!(window.position(), (10, 20));
    }
}
