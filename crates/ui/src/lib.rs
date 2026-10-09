//! UI crate - Cross-platform UI components for Rust applications
//!
//! This crate provides a unified UI abstraction layer focused on egui
//! with theme, layout, and navigation utilities.
//!
//! NOTE: For the lightweight version, only egui is supported.
//! Other frameworks (iced, slint, tauri) will be added in future versions.

pub mod layout;
pub mod navigation;
pub mod theme;

// Re-exports
pub use layout::*;
pub use navigation::*;
pub use theme::*;

// Components, hooks, utils, platform
// hooks and utils are still disabled: they are not part of the egui stack.
pub mod components;
pub mod platform;

pub use components::*;
pub use platform::*;

use parking_lot::RwLock;
use shared::config::AppConfig;
use shared::domain::{ColorTokens, DeviceInfo, Platform, ThemePreference, ThemeTokens};
use std::sync::Arc;

/// UI framework type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiFramework {
    Egui,
    Iced,
    Slint,
    Tauri,
    Native,
}

/// UI context for platform-aware rendering
#[derive(Clone)]
pub struct UiContext {
    pub config: Arc<RwLock<AppConfig>>,
    pub device_info: DeviceInfo,
    pub theme_tokens: ThemeTokens,
    pub framework: UiFramework,
}

impl UiContext {
    pub fn new(config: Arc<RwLock<AppConfig>>, framework: UiFramework) -> Self {
        let device_info = config.read().device_info();
        let theme_tokens = config.read().theme_tokens();

        Self {
            config,
            device_info,
            theme_tokens,
            framework,
        }
    }

    pub fn platform(&self) -> Platform {
        self.device_info.platform
    }

    pub fn is_mobile(&self) -> bool {
        matches!(
            self.device_info.form_factor,
            shared::domain::FormFactor::Phone | shared::domain::FormFactor::Tablet
        )
    }

    pub fn is_desktop(&self) -> bool {
        matches!(
            self.device_info.form_factor,
            shared::domain::FormFactor::Desktop | shared::domain::FormFactor::Tv
        )
    }

    pub fn theme_preference(&self) -> ThemePreference {
        self.config.read().ui.theme.mode.into()
    }

    pub fn color_tokens(&self) -> &ColorTokens {
        &self.theme_tokens.colors
    }

    pub fn spacing(&self) -> &shared::domain::SpacingTokens {
        &self.theme_tokens.spacing
    }

    pub fn typography(&self) -> &shared::domain::TypographyTokens {
        &self.theme_tokens.typography
    }
}

/// UI builder for creating platform-appropriate UI
pub struct UiBuilder {
    config: Arc<RwLock<AppConfig>>,
    framework: Option<UiFramework>,
    platform: Option<Platform>,
    device_info: Option<DeviceInfo>,
}

impl UiBuilder {
    pub fn new(config: Arc<RwLock<AppConfig>>) -> Self {
        Self {
            config,
            framework: None,
            platform: None,
            device_info: None,
        }
    }

    pub fn framework(mut self, framework: UiFramework) -> Self {
        self.framework = Some(framework);
        self
    }

    /// Overrides the platform the UI believes it is running on.
    ///
    /// Needed on Android, where the value baked into the config file is the
    /// build host's, not the device's.
    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Overrides the detected device information entirely.
    pub fn device_info(mut self, device_info: DeviceInfo) -> Self {
        self.device_info = Some(device_info);
        self
    }

    pub fn build(self) -> UiContext {
        // egui is the single UI backend in this template, on every target
        // including Android and iOS, so it is always the default.
        let framework = self.framework.unwrap_or(UiFramework::Egui);

        let mut context = UiContext::new(self.config, framework);
        if let Some(platform) = self.platform {
            context.device_info.platform = platform;
        }
        if let Some(device_info) = self.device_info {
            context.device_info = device_info;
        }
        context
    }
}

/// Responsive layout helpers
pub mod responsive {
    use shared::domain::{DeviceInfo, FormFactor, ScreenSize};

    pub fn grid_columns(screen_size: ScreenSize, base_columns: usize) -> usize {
        match screen_size {
            ScreenSize::Small => 1,
            ScreenSize::Medium => (base_columns as f32 * 0.5).ceil() as usize,
            ScreenSize::Large => (base_columns as f32 * 0.75).ceil() as usize,
            ScreenSize::ExtraLarge => base_columns,
        }
    }

    pub fn spacing_multiplier(form_factor: FormFactor) -> f32 {
        match form_factor {
            FormFactor::Phone => 0.875,
            FormFactor::Tablet => 1.0,
            FormFactor::Desktop => 1.0,
            FormFactor::Tv => 1.5,
            FormFactor::Watch => 0.75,
            FormFactor::Foldable => 1.0,
            FormFactor::Unknown => 1.0,
        }
    }

    pub fn font_scale(form_factor: FormFactor) -> f32 {
        match form_factor {
            FormFactor::Phone => 1.0,
            FormFactor::Tablet => 1.0,
            FormFactor::Desktop => 1.0,
            FormFactor::Tv => 1.5,
            FormFactor::Watch => 0.875,
            FormFactor::Foldable => 1.0,
            FormFactor::Unknown => 1.0,
        }
    }

    pub fn is_compact(device_info: &DeviceInfo) -> bool {
        matches!(
            device_info.form_factor,
            FormFactor::Phone | FormFactor::Watch
        ) || device_info.screen_size == ScreenSize::Small
    }

    pub fn is_two_pane(device_info: &DeviceInfo) -> bool {
        matches!(
            device_info.form_factor,
            FormFactor::Tablet | FormFactor::Desktop | FormFactor::Foldable
        ) && device_info.screen_size >= ScreenSize::Medium
    }
}

/// Platform-specific UI adaptations
pub mod platform_adaptations {
    use shared::domain::{DeviceInfo, Platform};

    pub fn default_padding(platform: Platform) -> f32 {
        match platform {
            Platform::Android => 16.0,
            Platform::Ios => 20.0,
            Platform::Linux => 16.0,
            Platform::Windows => 12.0,
            Platform::Macos => 16.0,
            Platform::Web => 16.0,
        }
    }

    pub fn default_border_radius(platform: Platform) -> f32 {
        match platform {
            Platform::Android => 12.0,
            Platform::Ios => 10.0,
            Platform::Linux => 8.0,
            Platform::Windows => 4.0,
            Platform::Macos => 8.0,
            Platform::Web => 8.0,
        }
    }

    pub fn button_height(platform: Platform) -> f32 {
        match platform {
            Platform::Android => 48.0,
            Platform::Ios => 44.0,
            Platform::Linux => 40.0,
            Platform::Windows => 32.0,
            Platform::Macos => 36.0,
            Platform::Web => 40.0,
        }
    }

    pub fn icon_size(platform: Platform, size: IconSize) -> f32 {
        let base = match size {
            IconSize::Small => 16.0,
            IconSize::Medium => 24.0,
            IconSize::Large => 32.0,
            IconSize::ExtraLarge => 48.0,
        };

        match platform {
            Platform::Android => base,
            Platform::Ios => base * 1.125,
            Platform::Linux => base,
            Platform::Windows => base * 0.875,
            Platform::Macos => base,
            Platform::Web => base,
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum IconSize {
        Small,
        Medium,
        Large,
        ExtraLarge,
    }

    pub fn font_family(platform: Platform) -> &'static str {
        match platform {
            Platform::Android => "Roboto",
            Platform::Ios => "SF Pro",
            Platform::Linux => "Inter",
            Platform::Windows => "Segoe UI",
            Platform::Macos => "SF Pro",
            Platform::Web => "system-ui",
        }
    }

    pub fn safe_area_insets(device_info: &DeviceInfo) -> (f32, f32, f32, f32) {
        let safe = &device_info.safe_area;
        (
            safe.top as f32,
            safe.right as f32,
            safe.bottom as f32,
            safe.left as f32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_responsive_grid_columns() {
        assert_eq!(
            responsive::grid_columns(shared::domain::ScreenSize::Small, 4),
            1
        );
        assert_eq!(
            responsive::grid_columns(shared::domain::ScreenSize::Medium, 4),
            2
        );
        assert_eq!(
            responsive::grid_columns(shared::domain::ScreenSize::Large, 4),
            3
        );
        assert_eq!(
            responsive::grid_columns(shared::domain::ScreenSize::ExtraLarge, 4),
            4
        );
    }

    #[test]
    fn test_platform_button_height() {
        assert_eq!(
            platform_adaptations::button_height(shared::domain::Platform::Android),
            48.0
        );
        assert_eq!(
            platform_adaptations::button_height(shared::domain::Platform::Ios),
            44.0
        );
        assert_eq!(
            platform_adaptations::button_height(shared::domain::Platform::Macos),
            36.0
        );
    }
}
