//! Platform-specific implementations - Android
//!
//! This module provides Android-specific UI implementations using JNI.

use super::{
    Platform, PlatformColors, PlatformFonts, PlatformMotion, PlatformShapes, PlatformSpacing,
    PlatformTheme, PlatformUi, Window, WindowConfig,
};
use parking_lot::RwLock;
use shared::errors::Result;
use std::sync::Arc;

pub struct AndroidUi {
    activity: Arc<RwLock<Option<AndroidActivity>>>,
}

/// Handle to the host Android activity that the UI is attached to.
///
/// Placeholder for the JNI references a real integration would need; it is
/// carried around so [`AndroidUi::set_activity`] has something to store.
#[derive(Debug)]
pub struct AndroidActivity {
    // JNI references would go here
}

impl Default for AndroidUi {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidUi {
    pub fn new() -> Self {
        Self {
            activity: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_activity(&self, activity: AndroidActivity) {
        *self.activity.write() = Some(activity);
    }

    /// The shared slot holding the attached [`AndroidActivity`], if any.
    ///
    /// Returns `None` until [`Self::set_activity`] has been called. The lock
    /// is exposed rather than the inner value so callers can take a read or a
    /// write guard without this accessor having to guess their intent.
    pub fn activity(&self) -> &Arc<RwLock<Option<AndroidActivity>>> {
        &self.activity
    }
}

impl PlatformUi for AndroidUi {
    fn platform(&self) -> Platform {
        Platform::Android
    }

    fn create_window(&self, config: WindowConfig) -> Result<Box<dyn Window>> {
        Ok(Box::new(AndroidWindow::new(config)))
    }

    fn theme(&self) -> PlatformTheme {
        PlatformTheme {
            name: "Material3".to_string(),
            colors: PlatformColors {
                primary: "#6750A4".to_string(),
                on_primary: "#FFFFFF".to_string(),
                background: "#FFFBFE".to_string(),
                on_background: "#1C1B1F".to_string(),
                surface: "#FFFBFE".to_string(),
                on_surface: "#1C1B1F".to_string(),
                error: "#BA1A1A".to_string(),
                on_error: "#FFFFFF".to_string(),
            },
            fonts: PlatformFonts {
                family: "Roboto".to_string(),
                size_scale: 1.0,
                weight_regular: 400,
                weight_medium: 500,
                weight_bold: 700,
            },
            spacing: PlatformSpacing {
                base: 4.0,
                scale: 1.0,
            },
            shapes: PlatformShapes {
                corner_radius_small: 4.0,
                corner_radius_medium: 8.0,
                corner_radius_large: 12.0,
            },
            motion: PlatformMotion {
                duration_short: 150,
                duration_medium: 250,
                duration_long: 350,
                easing_standard: "cubic-bezier(0.4, 0.0, 0.2, 1)".to_string(),
            },
        }
    }

    fn on_resume(&self) {
        // Handle Android resume
    }

    fn on_pause(&self) {
        // Handle Android pause
    }

    fn on_destroy(&self) {
        // Handle Android destroy
    }

    fn announce_for_accessibility(&self, _text: &str) {
        // Use Android's accessibility manager
    }

    fn set_accessibility_focus(&self, _element_id: &str) {
        // Set accessibility focus on Android
    }
}

struct AndroidWindow {
    config: WindowConfig,
}

impl AndroidWindow {
    fn new(config: WindowConfig) -> Self {
        Self { config }
    }
}

impl Window for AndroidWindow {
    fn show(&self) {
        // Show Android activity/window
    }

    fn hide(&self) {
        // Hide Android activity/window
    }

    fn close(&self) {
        // Close Android activity/window
    }

    fn set_title(&self, _title: &str) {
        // Set Android activity title
    }

    fn set_size(&self, _width: u32, _height: u32) {
        // Set Android window size
    }

    fn set_min_size(&self, _width: u32, _height: u32) {
        // Set Android window min size
    }

    fn set_max_size(&self, _width: u32, _height: u32) {
        // Set Android window max size
    }

    fn set_fullscreen(&self, _fullscreen: bool) {
        // Set Android fullscreen mode
    }

    fn set_maximized(&self, _maximized: bool) {
        // Not applicable on Android
    }

    fn position(&self) -> (i32, i32) {
        (0, 0) // Not applicable on Android
    }

    fn set_position(&self, _x: i32, _y: i32) {
        // Not applicable on Android
    }

    fn is_visible(&self) -> bool {
        true
    }

    fn is_fullscreen(&self) -> bool {
        self.config.fullscreen
    }

    fn is_maximized(&self) -> bool {
        false
    }
}
