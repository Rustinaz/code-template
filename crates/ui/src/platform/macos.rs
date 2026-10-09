//! Platform-specific implementations - macOS
//!
//! This module provides macOS-specific UI implementations using AppKit/SwiftUI.

use super::{
    Platform, PlatformColors, PlatformFonts, PlatformMotion, PlatformShapes, PlatformSpacing,
    PlatformTheme, PlatformUi, Window, WindowConfig,
};
use parking_lot::RwLock;
use shared::errors::Result;
use std::sync::Arc;

pub struct MacosUi {
    app_delegate: Arc<RwLock<Option<MacosAppDelegate>>>,
}

/// Handle to the host AppKit application delegate that the UI is attached to.
///
/// Placeholder for the AppKit delegate reference a real integration would
/// need; it is carried around so the delegate stays reachable from
/// [`MacosUi`].
#[derive(Debug)]
pub struct MacosAppDelegate {
    // AppKit/AppDelegate reference
}

impl Default for MacosUi {
    fn default() -> Self {
        Self::new()
    }
}

impl MacosUi {
    pub fn new() -> Self {
        Self {
            app_delegate: Arc::new(RwLock::new(None)),
        }
    }

    /// The shared slot holding the host [`MacosAppDelegate`], if any.
    ///
    /// The lock is exposed rather than the inner value so callers can take a
    /// read or a write guard without this accessor having to guess their
    /// intent.
    pub fn app_delegate(&self) -> &Arc<RwLock<Option<MacosAppDelegate>>> {
        &self.app_delegate
    }
}

impl PlatformUi for MacosUi {
    fn platform(&self) -> Platform {
        Platform::Macos
    }

    fn create_window(&self, config: WindowConfig) -> Result<Box<dyn Window>> {
        Ok(Box::new(MacosWindow::new(config)))
    }

    fn theme(&self) -> PlatformTheme {
        PlatformTheme {
            name: "macOS".to_string(),
            colors: PlatformColors {
                primary: "#007AFF".to_string(),
                on_primary: "#FFFFFF".to_string(),
                background: "#FFFFFF".to_string(),
                on_background: "#1D1D1F".to_string(),
                surface: "#F5F5F7".to_string(),
                on_surface: "#1D1D1F".to_string(),
                error: "#FF3B30".to_string(),
                on_error: "#FFFFFF".to_string(),
            },
            fonts: PlatformFonts {
                family: "SF Pro".to_string(),
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
                duration_short: 150,
                duration_medium: 250,
                duration_long: 350,
                easing_standard: "cubic-bezier(0.25, 0.1, 0.25, 1.0)".to_string(),
            },
        }
    }

    fn on_resume(&self) {
        // Handle macOS app activation
    }

    fn on_pause(&self) {
        // Handle macOS app deactivation
    }

    fn on_destroy(&self) {
        // Handle macOS app termination
    }

    fn announce_for_accessibility(&self, _text: &str) {
        // Use VoiceOver
    }

    fn set_accessibility_focus(&self, _element_id: &str) {
        // Set accessibility focus on macOS
    }
}

struct MacosWindow {
    config: WindowConfig,
}

impl MacosWindow {
    fn new(config: WindowConfig) -> Self {
        Self { config }
    }
}

impl Window for MacosWindow {
    fn show(&self) {
        // Show AppKit window
    }

    fn hide(&self) {
        // Hide AppKit window
    }

    fn close(&self) {
        // Close AppKit window
    }

    fn set_title(&self, _title: &str) {
        // Set AppKit window title
    }

    fn set_size(&self, _width: u32, _height: u32) {
        // Set AppKit window size
    }

    fn set_min_size(&self, _width: u32, _height: u32) {
        // Set AppKit window min size
    }

    fn set_max_size(&self, _width: u32, _height: u32) {
        // Set AppKit window max size
    }

    fn set_fullscreen(&self, _fullscreen: bool) {
        // Set AppKit fullscreen
    }

    fn set_maximized(&self, _maximized: bool) {
        // Set AppKit zoomed
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn set_position(&self, _x: i32, _y: i32) {
        // Set AppKit window position
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
