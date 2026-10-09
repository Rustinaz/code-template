//! Platform-specific implementations - iOS
//!
//! This module provides iOS-specific UI implementations using Objective-C.

use super::{
    Platform, PlatformColors, PlatformFonts, PlatformMotion, PlatformShapes, PlatformSpacing,
    PlatformTheme, PlatformUi, Window, WindowConfig,
};
use parking_lot::RwLock;
use shared::errors::Result;
use std::sync::Arc;

pub struct IosUi {
    view_controller: Arc<RwLock<Option<IosViewController>>>,
}

/// Handle to the host iOS view controller that the UI is attached to.
///
/// Placeholder for the Objective-C references a real integration would need;
/// it is carried around so [`IosUi::set_view_controller`] has something to
/// store.
#[derive(Debug)]
pub struct IosViewController {
    // Objective-C references would go here
}

impl Default for IosUi {
    fn default() -> Self {
        Self::new()
    }
}

impl IosUi {
    pub fn new() -> Self {
        Self {
            view_controller: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_view_controller(&self, vc: IosViewController) {
        *self.view_controller.write() = Some(vc);
    }

    /// The shared slot holding the attached [`IosViewController`], if any.
    ///
    /// Returns `None` until [`Self::set_view_controller`] has been called. The
    /// lock is exposed rather than the inner value so callers can take a read
    /// or a write guard without this accessor having to guess their intent.
    pub fn view_controller(&self) -> &Arc<RwLock<Option<IosViewController>>> {
        &self.view_controller
    }
}

impl PlatformUi for IosUi {
    fn platform(&self) -> Platform {
        Platform::Ios
    }

    fn create_window(&self, config: WindowConfig) -> Result<Box<dyn Window>> {
        Ok(Box::new(IosWindow::new(config)))
    }

    fn theme(&self) -> PlatformTheme {
        PlatformTheme {
            name: "iOS".to_string(),
            colors: PlatformColors {
                primary: "#007AFF".to_string(),
                on_primary: "#FFFFFF".to_string(),
                background: "#FFFFFF".to_string(),
                on_background: "#000000".to_string(),
                surface: "#F2F2F7".to_string(),
                on_surface: "#000000".to_string(),
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
                corner_radius_small: 6.0,
                corner_radius_medium: 10.0,
                corner_radius_large: 16.0,
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
        // Handle iOS app becoming active
    }

    fn on_pause(&self) {
        // Handle iOS app entering background
    }

    fn on_destroy(&self) {
        // Handle iOS app termination
    }

    fn announce_for_accessibility(&self, _text: &str) {
        // Use iOS VoiceOver
    }

    fn set_accessibility_focus(&self, _element_id: &str) {
        // Set accessibility focus on iOS
    }
}

struct IosWindow {
    config: WindowConfig,
}

impl IosWindow {
    fn new(config: WindowConfig) -> Self {
        Self { config }
    }
}

impl Window for IosWindow {
    fn show(&self) {
        // Show iOS window
    }

    fn hide(&self) {
        // Hide iOS window
    }

    fn close(&self) {
        // Close iOS window
    }

    fn set_title(&self, _title: &str) {
        // Set iOS navigation bar title
    }

    fn set_size(&self, _width: u32, _height: u32) {
        // Set iOS window size
    }

    fn set_min_size(&self, _width: u32, _height: u32) {
        // Not typically used on iOS
    }

    fn set_max_size(&self, _width: u32, _height: u32) {
        // Not typically used on iOS
    }

    fn set_fullscreen(&self, _fullscreen: bool) {
        // Set iOS fullscreen
    }

    fn set_maximized(&self, _maximized: bool) {
        // Not applicable on iOS
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn set_position(&self, _x: i32, _y: i32) {
        // Not typically used on iOS
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
