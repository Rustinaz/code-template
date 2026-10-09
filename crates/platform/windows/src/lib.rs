//! Windows platform integration.
//!
//! The UI is rendered by Rust (egui/eframe on the `winit` Windows backend), so
//! this crate owns only what needs the Win32 runtime: OS information, the
//! Material 3 theme defaults, and the [`ui::platform::PlatformUi`] adapter.

use std::sync::OnceLock;

use parking_lot::RwLock;
use shared::domain::Platform;
use shared::errors::Result as AppResult;
use ui::platform::{PlatformTheme, PlatformUi, Window, WindowConfig};

/// Static information about the Windows host.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WindowsInfo {
    /// Windows version, e.g. `"11"`.
    pub major_version: u32,
    /// Windows build number, e.g. `22631`.
    pub build_number: u32,
    /// Version string as reported by the platform, e.g. `"11.0.22631"`.
    pub version_string: String,
    /// DPI scale factor, 1.0 on a 100 % display.
    pub dpi_scale: f64,
}

/// Initializes the Windows platform layer.
pub fn init() -> AppResult<()> {
    tracing::info!("Initializing Windows platform integration");
    Ok(())
}

static CACHE: OnceLock<RwLock<WindowsInfo>> = OnceLock::new();

/// Reads host information, caching the result after the first call.
#[must_use]
pub fn host_info() -> WindowsInfo {
    let cache = CACHE.get_or_init(|| RwLock::new(read_host()));
    cache.read().clone()
}

#[cfg(target_os = "windows")]
fn read_host() -> WindowsInfo {
    use windows::Win32::System::SystemInformation::{GetVersionExW, OSVERSIONINFOW};

    // SAFETY: `OSVERSIONINFOW` is a plain-old-data struct, and
    // `RtlGetVersion` is the documented, version-independent way to read it.
    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    // `GetVersionExW` is deprecated but still the only one callable without
    // linking `ntdll`; the shim returns a zeroed struct on failure, which the
    // defaults above already cover.
    let ok = unsafe { GetVersionExW(&mut info) }.as_bool();
    if !ok {
        tracing::debug!("GetVersionExW failed; reporting unknown Windows version");
    }
    WindowsInfo {
        major_version: info.dwMajorVersion,
        build_number: info.dwBuildNumber,
        version_string: format!(
            "{}.{}.{}",
            info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
        ),
        dpi_scale: 1.0,
    }
}

#[cfg(not(target_os = "windows"))]
fn read_host() -> WindowsInfo {
    WindowsInfo {
        version_string: "not-windows".to_string(),
        dpi_scale: 1.0,
        ..WindowsInfo::default()
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
            family: "Segoe UI".to_string(),
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

/// [`PlatformUi`] implementation for Windows.
#[derive(Debug, Default)]
pub struct WindowsPlatformUi;

impl WindowsPlatformUi {
    /// Creates a new adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PlatformUi for WindowsPlatformUi {
    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn create_window(&self, config: WindowConfig) -> AppResult<Box<dyn Window>> {
        Ok(Box::new(WindowsWindow::new(config)))
    }

    fn theme(&self) -> PlatformTheme {
        material3_theme()
    }

    fn on_resume(&self) {
        tracing::debug!("Windows app became active");
    }

    fn on_pause(&self) {
        tracing::debug!("Windows app became inactive");
    }

    fn on_destroy(&self) {
        tracing::debug!("Windows window closed");
    }

    fn announce_for_accessibility(&self, text: &str) {
        tracing::debug!(text, "narrator announcement requested");
    }

    fn set_accessibility_focus(&self, element_id: &str) {
        tracing::debug!(element_id, "UIA focus requested");
    }
}

/// Window handle for the single Windows window.
#[derive(Debug)]
pub struct WindowsWindow {
    title: RwLock<String>,
    size: RwLock<(u32, u32)>,
    min_size: RwLock<(u32, u32)>,
    max_size: RwLock<Option<(u32, u32)>>,
    position: RwLock<(i32, i32)>,
    fullscreen: RwLock<bool>,
    maximized: RwLock<bool>,
    visible: RwLock<bool>,
}

impl WindowsWindow {
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

impl Window for WindowsWindow {
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
        assert!(!info.version_string.is_empty());
    }

    #[test]
    fn platform_ui_reports_windows() {
        let platform_ui = WindowsPlatformUi::new();
        assert_eq!(platform_ui.platform(), Platform::Windows);
        assert_eq!(platform_ui.theme().fonts.family, "Segoe UI");
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
        let window = WindowsWindow::new(config);

        window.set_size(100, 100);
        assert_eq!(window.size(), (400, 300));

        window.set_size(4000, 4000);
        assert_eq!(window.size(), (1280, 720));
    }

    #[test]
    fn window_tracks_title_and_position() {
        let window = WindowsWindow::new(WindowConfig::default());
        window.set_title("Rust App");
        window.set_position(30, 40);
        assert_eq!(window.title(), "Rust App");
        assert_eq!(window.position(), (30, 40));
    }
}
