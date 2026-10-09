//! Platform-specific implementations - Linux
//!
//! This module provides Linux-specific UI implementations using GTK/Wayland/X11.

use super::{
    Platform, PlatformColors, PlatformFonts, PlatformMotion, PlatformShapes, PlatformSpacing,
    PlatformTheme, PlatformUi, Window, WindowConfig,
};
use parking_lot::RwLock;
use shared::errors::Result;
use std::sync::Arc;

pub struct LinuxUi {
    application: Arc<RwLock<Option<LinuxApplication>>>,
}

/// Handle to the host GTK application that the UI is attached to.
///
/// Placeholder for the GTK application reference a real integration would
/// need; it is carried around so the handle stays reachable from [`LinuxUi`].
#[derive(Debug)]
pub struct LinuxApplication {
    // GTK application reference
}

impl Default for LinuxUi {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxUi {
    pub fn new() -> Self {
        Self {
            application: Arc::new(RwLock::new(None)),
        }
    }

    /// The shared slot holding the host [`LinuxApplication`], if any.
    ///
    /// The lock is exposed rather than the inner value so callers can take a
    /// read or a write guard without this accessor having to guess their
    /// intent.
    pub fn application(&self) -> &Arc<RwLock<Option<LinuxApplication>>> {
        &self.application
    }
}

impl PlatformUi for LinuxUi {
    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn create_window(&self, config: WindowConfig) -> Result<Box<dyn Window>> {
        Ok(Box::new(LinuxWindow::new(config)))
    }

    fn theme(&self) -> PlatformTheme {
        PlatformTheme {
            name: "GTK/Adwaita".to_string(),
            colors: PlatformColors {
                primary: "#3584E4".to_string(),
                on_primary: "#FFFFFF".to_string(),
                background: "#FFFFFF".to_string(),
                on_background: "#1E1E1E".to_string(),
                surface: "#F5F5F5".to_string(),
                on_surface: "#1E1E1E".to_string(),
                error: "#DC3545".to_string(),
                on_error: "#FFFFFF".to_string(),
            },
            fonts: PlatformFonts {
                family: "Inter".to_string(),
                size_scale: 1.0,
                weight_regular: 400,
                weight_medium: 500,
                weight_bold: 600,
            },
            spacing: PlatformSpacing {
                base: 8.0,
                scale: 1.0,
            },
            shapes: PlatformShapes {
                corner_radius_small: 4.0,
                corner_radius_medium: 8.0,
                corner_radius_large: 12.0,
            },
            motion: PlatformMotion {
                duration_short: 100,
                duration_medium: 200,
                duration_long: 300,
                easing_standard: "cubic-bezier(0.4, 0.0, 0.2, 1)".to_string(),
            },
        }
    }

    fn on_resume(&self) {
        // Handle Linux app focus
    }

    fn on_pause(&self) {
        // Handle Linux app unfocus
    }

    fn on_destroy(&self) {
        // Handle Linux app quit
    }

    fn announce_for_accessibility(&self, _text: &str) {
        // Use AT-SPI/ATK for accessibility
    }

    fn set_accessibility_focus(&self, _element_id: &str) {
        // Set accessibility focus on Linux
    }
}

struct LinuxWindow {
    config: WindowConfig,
}

impl LinuxWindow {
    fn new(config: WindowConfig) -> Self {
        Self { config }
    }
}

impl Window for LinuxWindow {
    fn show(&self) {
        // Show GTK window
    }

    fn hide(&self) {
        // Hide GTK window
    }

    fn close(&self) {
        // Close GTK window
    }

    fn set_title(&self, _title: &str) {
        // Set GTK window title
    }

    fn set_size(&self, _width: u32, _height: u32) {
        // Set GTK window size
    }

    fn set_min_size(&self, _width: u32, _height: u32) {
        // Set GTK window min size
    }

    fn set_max_size(&self, _width: u32, _height: u32) {
        // Set GTK window max size
    }

    fn set_fullscreen(&self, _fullscreen: bool) {
        // Set GTK fullscreen
    }

    fn set_maximized(&self, _maximized: bool) {
        // Set GTK maximized
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn set_position(&self, _x: i32, _y: i32) {
        // Set GTK window position
    }

    fn is_visible(&self) -> bool {
        true
    }

    fn is_fullscreen(&self) -> bool {
        self.config.fullscreen
    }

    fn is_maximized(&self) -> bool {
        self.config.maximized
    }
}
