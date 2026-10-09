//! Theme system for cross-platform UI
//!
//! This module provides theming capabilities that adapt to platform
//! design guidelines while maintaining brand consistency.

use parking_lot::RwLock;
use shared::config::ThemeMode;
use shared::domain::{
    ColorRole, ColorTokens, ColorValue, ElevationTokens, MotionTokens, ShapeTokens, SpacingTokens,
    TextStyle, ThemePreference, ThemeTokens, TypographyTokens,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Theme manager for handling theme changes and platform adaptations
pub struct ThemeManager {
    tokens: Arc<RwLock<ThemeTokens>>,
    mode: Arc<RwLock<ThemeMode>>,
    platform_overrides: Arc<RwLock<HashMap<String, ThemeTokens>>>,
    listeners: Arc<RwLock<Vec<Box<dyn ThemeChangeListener>>>>,
}

impl ThemeManager {
    pub fn new(tokens: ThemeTokens) -> Self {
        Self {
            tokens: Arc::new(RwLock::new(tokens)),
            mode: Arc::new(RwLock::new(ThemeMode::System)),
            platform_overrides: Arc::new(RwLock::new(HashMap::new())),
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Builder-style form of [`Self::set_mode`].
    pub fn with_mode(self, mode: ThemeMode) -> Self {
        self.set_mode(mode);
        self
    }

    pub fn current_tokens(&self) -> ThemeTokens {
        self.tokens.read().clone()
    }

    pub fn current_mode(&self) -> ThemeMode {
        *self.mode.read()
    }

    pub fn set_mode(&self, mode: ThemeMode) {
        *self.mode.write() = mode;
        self.apply_mode(mode);
        self.notify_listeners();
    }

    pub fn set_tokens(&self, tokens: ThemeTokens) {
        *self.tokens.write() = tokens;
        self.notify_listeners();
    }

    pub fn update_colors(&self, colors: ColorTokens) {
        let mut tokens = self.tokens.write();
        tokens.colors = colors;
        self.notify_listeners();
    }

    pub fn update_typography(&self, typography: TypographyTokens) {
        let mut tokens = self.tokens.write();
        tokens.typography = typography;
        self.notify_listeners();
    }

    pub fn update_spacing(&self, spacing: SpacingTokens) {
        let mut tokens = self.tokens.write();
        tokens.spacing = spacing;
        self.notify_listeners();
    }

    pub fn update_shapes(&self, shapes: ShapeTokens) {
        let mut tokens = self.tokens.write();
        tokens.shape = shapes;
        self.notify_listeners();
    }

    pub fn update_motion(&self, motion: MotionTokens) {
        let mut tokens = self.tokens.write();
        tokens.motion = motion;
        self.notify_listeners();
    }

    pub fn update_elevation(&self, elevation: ElevationTokens) {
        let mut tokens = self.tokens.write();
        tokens.elevation = elevation;
        self.notify_listeners();
    }

    pub fn add_platform_override(&self, platform: String, tokens: ThemeTokens) {
        self.platform_overrides.write().insert(platform, tokens);
    }

    pub fn get_platform_override(&self, platform: &str) -> Option<ThemeTokens> {
        self.platform_overrides.read().get(platform).cloned()
    }

    pub fn add_listener(&self, listener: Box<dyn ThemeChangeListener>) {
        self.listeners.write().push(listener);
    }

    fn apply_mode(&self, _mode: ThemeMode) {
        // Stub awaiting the real implementation: this would apply the theme mode
        // by switching between light/dark color tokens.
    }

    fn notify_listeners(&self) {
        let tokens = self.current_tokens();
        let mode = self.current_mode();
        for listener in self.listeners.read().iter() {
            listener.on_theme_change(&tokens, mode);
        }
    }

    /// Get a color for the current theme mode
    pub fn color(&self, role: &ColorRole) -> ColorValue {
        let mode = self.current_mode();
        match mode {
            ThemeMode::Light | ThemeMode::System => role.light.clone(),
            ThemeMode::Dark => role.dark.clone(),
            ThemeMode::HighContrast => {
                // Prefer high contrast variants if available
                if matches!(mode, ThemeMode::HighContrast) {
                    role.high_contrast_light
                        .clone()
                        .unwrap_or(role.light.clone())
                } else {
                    role.light.clone()
                }
            }
        }
    }

    /// Get a color for a specific theme mode
    pub fn color_for_mode(&self, role: &ColorRole, mode: ThemePreference) -> ColorValue {
        match mode {
            ThemePreference::Light => role.light.clone(),
            ThemePreference::Dark => role.dark.clone(),
            ThemePreference::HighContrast => {
                role.high_contrast_dark.clone().unwrap_or(role.dark.clone())
            }
            ThemePreference::System => {
                // Would check system theme
                role.light.clone()
            }
        }
    }
}

/// Listener for theme changes
pub trait ThemeChangeListener: Send + Sync {
    fn on_theme_change(&self, tokens: &ThemeTokens, mode: ThemeMode);
}

/// Theme builder for creating custom themes
pub struct ThemeBuilder {
    tokens: ThemeTokens,
}

impl ThemeBuilder {
    pub fn new() -> Self {
        Self {
            tokens: ThemeTokens::default(),
        }
    }

    pub fn from_tokens(tokens: ThemeTokens) -> Self {
        Self { tokens }
    }

    pub fn primary_color(mut self, light: ColorValue, dark: ColorValue) -> Self {
        self.tokens.colors.primary = ColorRole {
            light,
            dark,
            high_contrast_light: None,
            high_contrast_dark: None,
        };
        self
    }

    pub fn secondary_color(mut self, light: ColorValue, dark: ColorValue) -> Self {
        self.tokens.colors.secondary = ColorRole {
            light,
            dark,
            high_contrast_light: None,
            high_contrast_dark: None,
        };
        self
    }

    pub fn font_family(mut self, family: String) -> Self {
        for style in self.all_text_styles_mut() {
            style.font_family = family.clone();
        }
        self
    }

    pub fn base_font_size(mut self, size: f32) -> Self {
        let ratio = size / self.tokens.typography.body_medium.font_size;
        for style in self.all_text_styles_mut() {
            style.font_size *= ratio;
            style.line_height *= ratio;
        }
        self
    }

    pub fn spacing_scale(mut self, scale: f32) -> Self {
        let spacing = &mut self.tokens.spacing;
        spacing.space_1 *= scale;
        spacing.space_2 *= scale;
        spacing.space_3 *= scale;
        spacing.space_4 *= scale;
        spacing.space_5 *= scale;
        spacing.space_6 *= scale;
        spacing.space_8 *= scale;
        spacing.space_10 *= scale;
        spacing.space_12 *= scale;
        spacing.space_16 *= scale;
        spacing.space_20 *= scale;
        spacing.space_24 *= scale;
        spacing.space_32 *= scale;
        spacing.space_40 *= scale;
        spacing.space_48 *= scale;
        spacing.space_64 *= scale;
        self
    }

    pub fn border_radius_scale(mut self, scale: f32) -> Self {
        let shape = &mut self.tokens.shape;
        shape.extra_small *= scale;
        shape.small *= scale;
        shape.medium *= scale;
        shape.large *= scale;
        shape.extra_large *= scale;
        self
    }

    pub fn motion_speed(mut self, speed: f32) -> Self {
        let motion = &mut self.tokens.motion;
        motion.duration_short = (motion.duration_short as f32 / speed) as u32;
        motion.duration_medium = (motion.duration_medium as f32 / speed) as u32;
        motion.duration_long = (motion.duration_long as f32 / speed) as u32;
        self
    }

    fn all_text_styles_mut(&mut self) -> Vec<&mut TextStyle> {
        vec![
            &mut self.tokens.typography.display_large,
            &mut self.tokens.typography.display_medium,
            &mut self.tokens.typography.display_small,
            &mut self.tokens.typography.headline_large,
            &mut self.tokens.typography.headline_medium,
            &mut self.tokens.typography.headline_small,
            &mut self.tokens.typography.title_large,
            &mut self.tokens.typography.title_medium,
            &mut self.tokens.typography.title_small,
            &mut self.tokens.typography.body_large,
            &mut self.tokens.typography.body_medium,
            &mut self.tokens.typography.body_small,
            &mut self.tokens.typography.label_large,
            &mut self.tokens.typography.label_medium,
            &mut self.tokens.typography.label_small,
        ]
    }

    pub fn build(self) -> ThemeTokens {
        self.tokens
    }
}

impl Default for ThemeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Predefined theme presets
pub mod presets {
    use super::*;
    use shared::domain::ColorValue;

    /// Material Design 3 baseline theme
    pub fn material3() -> ThemeTokens {
        ThemeBuilder::new()
            .primary_color(
                ColorValue {
                    hex: "#6750A4".to_string(),
                    rgb: (103, 80, 164),
                    alpha: 1.0,
                },
                ColorValue {
                    hex: "#D0BCFF".to_string(),
                    rgb: (208, 188, 255),
                    alpha: 1.0,
                },
            )
            .secondary_color(
                ColorValue {
                    hex: "#625B71".to_string(),
                    rgb: (98, 91, 113),
                    alpha: 1.0,
                },
                ColorValue {
                    hex: "#CCC2DC".to_string(),
                    rgb: (204, 194, 220),
                    alpha: 1.0,
                },
            )
            .build()
    }

    /// iOS-inspired theme
    pub fn ios() -> ThemeTokens {
        ThemeBuilder::new()
            .primary_color(
                ColorValue {
                    hex: "#007AFF".to_string(),
                    rgb: (0, 122, 255),
                    alpha: 1.0,
                },
                ColorValue {
                    hex: "#0A84FF".to_string(),
                    rgb: (10, 132, 255),
                    alpha: 1.0,
                },
            )
            .font_family("SF Pro".to_string())
            .border_radius_scale(0.8)
            .build()
    }

    /// Android Material You theme
    pub fn material_you() -> ThemeTokens {
        ThemeBuilder::new()
            .primary_color(
                ColorValue {
                    hex: "#6750A4".to_string(),
                    rgb: (103, 80, 164),
                    alpha: 1.0,
                },
                ColorValue {
                    hex: "#D0BCFF".to_string(),
                    rgb: (208, 188, 255),
                    alpha: 1.0,
                },
            )
            .font_family("Roboto".to_string())
            .build()
    }

    /// High contrast theme for accessibility
    pub fn high_contrast() -> ThemeTokens {
        let mut tokens = ThemeBuilder::new().build();
        // Override with high contrast colors
        tokens.colors.primary.high_contrast_light = Some(ColorValue {
            hex: "#0000FF".to_string(),
            rgb: (0, 0, 255),
            alpha: 1.0,
        });
        tokens.colors.primary.high_contrast_dark = Some(ColorValue {
            hex: "#FFFF00".to_string(),
            rgb: (255, 255, 0),
            alpha: 1.0,
        });
        tokens.colors.background.high_contrast_light = Some(ColorValue {
            hex: "#FFFFFF".to_string(),
            rgb: (255, 255, 255),
            alpha: 1.0,
        });
        tokens.colors.background.high_contrast_dark = Some(ColorValue {
            hex: "#000000".to_string(),
            rgb: (0, 0, 0),
            alpha: 1.0,
        });
        tokens
    }

    /// Minimal theme
    pub fn minimal() -> ThemeTokens {
        ThemeBuilder::new()
            .primary_color(
                ColorValue {
                    hex: "#000000".to_string(),
                    rgb: (0, 0, 0),
                    alpha: 1.0,
                },
                ColorValue {
                    hex: "#FFFFFF".to_string(),
                    rgb: (255, 255, 255),
                    alpha: 1.0,
                },
            )
            .spacing_scale(0.875)
            .build()
    }
}

/// CSS-in-Rust style definitions for web targets
/// Turns theme tokens into a CSS custom-property stylesheet.
///
/// wasm32 only: the other targets hand the tokens to egui, which has no CSS.
#[cfg(target_arch = "wasm32")]
pub mod css {
    // A nested module does not inherit the parent's `use`, so every type the
    // emitters below name has to be brought in explicitly.
    use super::{
        ColorRole, ColorTokens, MotionTokens, ShapeTokens, SpacingTokens, ThemeTokens,
        TypographyTokens,
    };

    pub fn generate_css(tokens: &ThemeTokens) -> String {
        let mut css = String::new();
        css.push_str(":root {\n");

        // Colors
        css.push_str("  /* Colors */\n");
        add_color_vars(&mut css, "primary", &tokens.colors.primary);
        add_color_vars(&mut css, "secondary", &tokens.colors.secondary);
        add_color_vars(&mut css, "background", &tokens.colors.background);
        add_color_vars(&mut css, "surface", &tokens.colors.surface);
        add_color_vars(&mut css, "error", &tokens.colors.error);

        // Spacing
        css.push_str("  /* Spacing */\n");
        add_spacing_vars(&mut css, &tokens.spacing);

        // Typography
        css.push_str("  /* Typography */\n");
        add_typography_vars(&mut css, &tokens.typography);

        // Shapes
        css.push_str("  /* Shapes */\n");
        add_shape_vars(&mut css, &tokens.shape);

        // Motion
        css.push_str("  /* Motion */\n");
        add_motion_vars(&mut css, &tokens.motion);

        css.push_str("}\n");

        // Dark mode
        css.push_str("@media (prefers-color-scheme: dark) {\n");
        css.push_str("  :root {\n");
        add_dark_color_vars(&mut css, &tokens.colors);
        css.push_str("  }\n");
        css.push_str("}\n");

        css
    }

    fn add_color_vars(css: &mut String, prefix: &str, role: &ColorRole) {
        css.push_str(&format!("  --{}-light: {};\n", prefix, role.light.hex));
        css.push_str(&format!("  --{}-dark: {};\n", prefix, role.dark.hex));
        css.push_str(&format!(
            "  --{}-rgb-light: {}, {}, {};\n",
            prefix, role.light.rgb.0, role.light.rgb.1, role.light.rgb.2
        ));
        css.push_str(&format!(
            "  --{}-rgb-dark: {}, {}, {};\n",
            prefix, role.dark.rgb.0, role.dark.rgb.1, role.dark.rgb.2
        ));
    }

    fn add_dark_color_vars(css: &mut String, colors: &ColorTokens) {
        css.push_str(&format!("    --primary: {};\n", colors.primary.dark.hex));
        css.push_str(&format!(
            "    --secondary: {};\n",
            colors.secondary.dark.hex
        ));
        css.push_str(&format!(
            "    --background: {};\n",
            colors.background.dark.hex
        ));
        css.push_str(&format!("    --surface: {};\n", colors.surface.dark.hex));
        css.push_str(&format!("    --error: {};\n", colors.error.dark.hex));
    }

    fn add_spacing_vars(css: &mut String, spacing: &SpacingTokens) {
        css.push_str(&format!("  --space-1: {}px;\n", spacing.space_1));
        css.push_str(&format!("  --space-2: {}px;\n", spacing.space_2));
        css.push_str(&format!("  --space-3: {}px;\n", spacing.space_3));
        css.push_str(&format!("  --space-4: {}px;\n", spacing.space_4));
        css.push_str(&format!("  --space-5: {}px;\n", spacing.space_5));
        css.push_str(&format!("  --space-6: {}px;\n", spacing.space_6));
        css.push_str(&format!("  --space-8: {}px;\n", spacing.space_8));
    }

    fn add_typography_vars(css: &mut String, typography: &TypographyTokens) {
        css.push_str(&format!(
            "  --font-family: {};\n",
            typography.body_medium.font_family
        ));
        css.push_str(&format!(
            "  --font-size-base: {}px;\n",
            typography.body_medium.font_size
        ));
        css.push_str(&format!(
            "  --line-height-base: {};\n",
            typography.body_medium.line_height / typography.body_medium.font_size
        ));
    }

    fn add_shape_vars(css: &mut String, shape: &ShapeTokens) {
        css.push_str(&format!("  --radius-sm: {}px;\n", shape.small));
        css.push_str(&format!("  --radius-md: {}px;\n", shape.medium));
        css.push_str(&format!("  --radius-lg: {}px;\n", shape.large));
        css.push_str(&format!("  --radius-xl: {}px;\n", shape.extra_large));
    }

    fn add_motion_vars(css: &mut String, motion: &MotionTokens) {
        css.push_str(&format!(
            "  --duration-short: {}ms;\n",
            motion.duration_short
        ));
        css.push_str(&format!(
            "  --duration-medium: {}ms;\n",
            motion.duration_medium
        ));
        css.push_str(&format!("  --duration-long: {}ms;\n", motion.duration_long));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_builder() {
        let tokens = ThemeBuilder::new()
            .primary_color(
                ColorValue {
                    hex: "#FF0000".to_string(),
                    rgb: (255, 0, 0),
                    alpha: 1.0,
                },
                ColorValue {
                    hex: "#FF6666".to_string(),
                    rgb: (255, 102, 102),
                    alpha: 1.0,
                },
            )
            .base_font_size(16.0)
            .spacing_scale(1.25)
            .build();

        assert_eq!(tokens.colors.primary.light.hex, "#FF0000");
        assert_eq!(tokens.typography.body_medium.font_size, 16.0);
        assert_eq!(tokens.spacing.space_4, 20.0); // 16 * 1.25
    }

    #[test]
    fn test_theme_presets() {
        let material = presets::material3();
        assert_eq!(material.colors.primary.light.hex, "#6750A4");

        let ios = presets::ios();
        assert_eq!(ios.typography.body_medium.font_family, "SF Pro");

        let hc = presets::high_contrast();
        assert!(hc.colors.primary.high_contrast_light.is_some());
    }
}
