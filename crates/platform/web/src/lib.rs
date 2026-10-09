//! Web (WASM) platform integration.
//!
//! The UI is rendered by Rust (egui in a canvas, compiled to
//! `wasm32-unknown-unknown`), so this crate owns only what needs the browser:
//! user-agent and viewport information, the Material 3 theme defaults, and the
//! [`ui::platform::PlatformUi`] adapter.
//!
//! Build with `scripts/build-web.sh`.

use std::sync::OnceLock;

use parking_lot::RwLock;
use shared::domain::Platform;
use shared::errors::Result as AppResult;
use ui::platform::{PlatformTheme, PlatformUi, Window, WindowConfig};

/// Static information about the browser the app is running in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WebInfo {
    /// `navigator.userAgent`, when the browser exposes it.
    pub user_agent: String,
    /// `window.innerWidth` at startup, 0 when unavailable.
    pub viewport_width: u32,
    /// `window.innerHeight` at startup, 0 when unavailable.
    pub viewport_height: u32,
    /// `window.devicePixelRatio`, 1.0 when unavailable.
    pub device_pixel_ratio: f64,
    /// `true` on a coarse-pointer device such as a phone or tablet.
    pub is_touch_device: bool,
}

/// Initializes the web platform layer.
pub fn init() -> AppResult<()> {
    tracing::info!("Initializing web platform integration");
    Ok(())
}

static CACHE: OnceLock<RwLock<WebInfo>> = OnceLock::new();

/// Reads browser information, caching the result after the first call.
#[must_use]
pub fn browser_info() -> WebInfo {
    let cache = CACHE.get_or_init(|| RwLock::new(read_browser()));
    cache.read().clone()
}

#[cfg(target_arch = "wasm32")]
fn read_browser() -> WebInfo {
    use web_sys::window;

    let Some(win) = window() else {
        return WebInfo::default();
    };
    let navigator = win.navigator();

    // `Window::inner_width`/`inner_height` are declared as
    // `Result<JsValue, JsValue>`, so the number has to be pulled out of the
    // `JsValue` by hand. A page that reports a negative or nonsensical size
    // falls back to zero rather than wrapping around when cast to `u32`.
    let viewport = |value: Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>| {
        value
            .ok()
            .and_then(|value| value.as_f64())
            .filter(|size| size.is_finite() && *size > 0.0)
            .unwrap_or(0.0) as u32
    };

    WebInfo {
        user_agent: navigator.user_agent().unwrap_or_default(),
        viewport_width: viewport(win.inner_width()),
        viewport_height: viewport(win.inner_height()),
        device_pixel_ratio: win.device_pixel_ratio(),
        is_touch_device: navigator.max_touch_points() > 0,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_browser() -> WebInfo {
    WebInfo {
        user_agent: "not-a-browser".to_string(),
        device_pixel_ratio: 1.0,
        ..WebInfo::default()
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
            family: "system-ui".to_string(),
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

/// [`PlatformUi`] implementation for the browser.
#[derive(Debug, Default)]
pub struct WebPlatformUi;

impl WebPlatformUi {
    /// Creates a new adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PlatformUi for WebPlatformUi {
    fn platform(&self) -> Platform {
        Platform::Web
    }

    fn create_window(&self, config: WindowConfig) -> AppResult<Box<dyn Window>> {
        Ok(Box::new(WebWindow::new(config)))
    }

    fn theme(&self) -> PlatformTheme {
        material3_theme()
    }

    fn on_resume(&self) {
        tracing::debug!("page became visible");
    }

    fn on_pause(&self) {
        tracing::debug!("page was hidden");
    }

    fn on_destroy(&self) {
        tracing::debug!("page unloaded");
    }

    fn announce_for_accessibility(&self, text: &str) {
        #[cfg(target_arch = "wasm32")]
        live_region_announce(text);
        #[cfg(not(target_arch = "wasm32"))]
        tracing::debug!(text, "live-region announcement requested");
    }

    fn set_accessibility_focus(&self, element_id: &str) {
        #[cfg(target_arch = "wasm32")]
        focus_element(element_id);
        #[cfg(not(target_arch = "wasm32"))]
        tracing::debug!(element_id, "element focus requested");
    }
}

/// Pushes `text` into a polite ARIA live region so screen readers read it.
#[cfg(target_arch = "wasm32")]
fn live_region_announce(text: &str) {
    use wasm_bindgen::prelude::*;

    const REGION_ID: &str = "rust-live-region";

    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(body) = document.body() else { return };

    // Reuse the region if the page already has one, otherwise create it. Every
    // DOM call is fallible and none of these failures are worth panicking a
    // render loop over, so each is dropped in favour of doing nothing.
    let region = match document.get_element_by_id(REGION_ID) {
        Some(existing) => existing,
        None => {
            let Ok(element) = document.create_element("div") else {
                return;
            };
            let Ok(element) = element.dyn_into::<web_sys::HtmlElement>() else {
                return;
            };
            let _ = element.set_attribute("id", REGION_ID);
            let _ = element.set_attribute("role", "status");
            let _ = element.set_attribute("aria-live", "polite");
            // Visually hidden but still announced.
            let _ = element.set_attribute(
                "style",
                "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap",
            );
            let _ = body.append_child(element.as_ref());
            // Widen back to Element so both arms of the match agree; the
            // append_child above is what made the HtmlElement worth having.
            element.into()
        }
    };

    // Clearing first is what makes a repeated identical message re-announce.
    region.set_inner_html("");
    region.append_child(&document.create_text_node(text)).ok();
}

/// Moves DOM focus to the element with `id`, if it exists.
#[cfg(target_arch = "wasm32")]
fn focus_element(id: &str) {
    use wasm_bindgen::prelude::*;

    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(element) = document.get_element_by_id(id) else {
        return;
    };
    // `focus` is a method of `HtmlElement`; an element without a tabindex or a
    // focusable role is not one, so the downcast is expected to fail sometimes.
    if let Ok(element) = element.dyn_into::<web_sys::HtmlElement>() {
        let _ = element.focus();
    }
}

/// Window handle for the single browser canvas.
#[derive(Debug)]
pub struct WebWindow {
    title: RwLock<String>,
    size: RwLock<(u32, u32)>,
    min_size: RwLock<(u32, u32)>,
    fullscreen: RwLock<bool>,
    visible: RwLock<bool>,
}

impl WebWindow {
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

impl Window for WebWindow {
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
        #[cfg(target_arch = "wasm32")]
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            document.set_title(title);
        }
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
        // A browser canvas always fills its container.
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn set_position(&self, _x: i32, _y: i32) {
        // The document owns scroll position.
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
    fn browser_info_is_readable_off_browser() {
        let info = browser_info();
        assert!(!info.user_agent.is_empty());
    }

    #[test]
    fn platform_ui_reports_web() {
        let platform_ui = WebPlatformUi::new();
        assert_eq!(platform_ui.platform(), Platform::Web);
        assert_eq!(platform_ui.theme().fonts.family, "system-ui");
    }

    #[test]
    fn window_clamps_to_minimum_size() {
        // Only a horizontal floor, so this checks that the two axes clamp
        // independently: the width is pushed up to 320 while the height, which
        // has no minimum here, stays where it was put.
        let config = WindowConfig {
            min_width: Some(320),
            min_height: None,
            ..WindowConfig::default()
        };
        let window = WebWindow::new(config);
        window.set_size(100, 100);
        assert_eq!(window.size(), (320, 100));
    }

    #[test]
    fn window_tracks_title() {
        let window = WebWindow::new(WindowConfig::default());
        window.set_title("Rust App");
        assert_eq!(window.title(), "Rust App");
    }
}
