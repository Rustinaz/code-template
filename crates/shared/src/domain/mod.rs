//! Core domain models and entities
//!
//! This module contains the fundamental business entities that are shared
//! across all platforms. These are pure Rust structs with no platform dependencies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for entities
pub type EntityId = Uuid;

/// Timestamp wrapper for consistent serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(pub DateTime<Utc>);

impl Default for Timestamp {
    fn default() -> Self {
        Self(Utc::now())
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

/// Base entity trait that all domain entities implement
pub trait Entity: Send + Sync + 'static {
    fn id(&self) -> EntityId;
    fn created_at(&self) -> Timestamp;
    fn updated_at(&self) -> Timestamp;
}

/// An [`Entity`] that a repository can keep secondary indexes for.
///
/// Each method returns `None` by default, so an entity only declares the indexes
/// it can actually support: `User` answers for email and username, `AppSettings`
/// only for its owner. Bounding a repository on this trait rather than on
/// [`Entity`] is what makes an index impossible to declare but never fill, which
/// is the failure mode a hand-rolled "simplified for now" lookup produces.
pub trait RepositoryEntity: Entity {
    /// Unique email address, matched case-insensitively.
    fn email(&self) -> Option<&str> {
        None
    }

    /// Unique handle, matched case-insensitively.
    fn username(&self) -> Option<&str> {
        None
    }

    /// The user this entity belongs to, for per-user lookups.
    fn owner_id(&self) -> Option<EntityId> {
        None
    }
}

/// User domain entity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: EntityId,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub preferences: UserPreferences,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Entity for User {
    fn id(&self) -> EntityId {
        self.id
    }
    fn created_at(&self) -> Timestamp {
        self.created_at
    }
    fn updated_at(&self) -> Timestamp {
        self.updated_at
    }
}

impl RepositoryEntity for User {
    fn email(&self) -> Option<&str> {
        Some(&self.email)
    }

    fn username(&self) -> Option<&str> {
        Some(&self.username)
    }
}

/// User preferences with platform-aware settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UserPreferences {
    pub theme: ThemePreference,
    pub language: String,
    pub notifications_enabled: bool,
    pub data_sync_enabled: bool,
    pub platform_specific: HashMap<String, serde_json::Value>,
}

/// Theme preference that adapts to platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
    HighContrast,
}

/// Application settings entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub id: EntityId,
    pub user_id: EntityId,
    pub window_state: WindowState,
    pub ui_scale: f32,
    pub font_scale: f32,
    pub reduced_motion: bool,
    pub accessibility: AccessibilitySettings,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Entity for AppSettings {
    fn id(&self) -> EntityId {
        self.id
    }
    fn created_at(&self) -> Timestamp {
        self.created_at
    }
    fn updated_at(&self) -> Timestamp {
        self.updated_at
    }
}

impl RepositoryEntity for AppSettings {
    fn owner_id(&self) -> Option<EntityId> {
        Some(self.user_id)
    }
}

/// Window state for responsive design
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    pub fullscreen: bool,
    pub position: Option<WindowPosition>,
}

/// Window position
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

/// Accessibility settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AccessibilitySettings {
    pub screen_reader_enabled: bool,
    pub high_contrast: bool,
    pub large_text: bool,
    pub reduce_motion: bool,
    pub focus_indicator: bool,
}

/// Device information for responsive layout
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub platform: Platform,
    pub form_factor: FormFactor,
    pub screen_size: ScreenSize,
    pub pixel_density: f32,
    pub orientation: Orientation,
    pub has_touch: bool,
    pub has_keyboard: bool,
    pub has_mouse: bool,
    pub safe_area: SafeArea,
}

/// Target platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Android,
    Ios,
    Linux,
    Windows,
    Macos,
    Web,
}

impl clap::ValueEnum for Platform {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Platform::Android,
            Platform::Ios,
            Platform::Linux,
            Platform::Windows,
            Platform::Macos,
            Platform::Web,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Platform::Android => clap::builder::PossibleValue::new("android"),
            Platform::Ios => clap::builder::PossibleValue::new("ios"),
            Platform::Linux => clap::builder::PossibleValue::new("linux"),
            Platform::Windows => clap::builder::PossibleValue::new("windows"),
            Platform::Macos => clap::builder::PossibleValue::new("macos"),
            Platform::Web => clap::builder::PossibleValue::new("web"),
        })
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Android => write!(f, "android"),
            Platform::Ios => write!(f, "ios"),
            Platform::Linux => write!(f, "linux"),
            Platform::Windows => write!(f, "windows"),
            Platform::Macos => write!(f, "macos"),
            Platform::Web => write!(f, "web"),
        }
    }
}

/// Device form factor for responsive layouts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FormFactor {
    Phone,
    Tablet,
    Desktop,
    Tv,
    Watch,
    Foldable,
    Unknown,
}

/// Screen size category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ScreenSize {
    Small,      // < 600dp
    Medium,     // 600dp - 840dp
    Large,      // 840dp - 1200dp
    ExtraLarge, // > 1200dp
}

/// Screen orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Orientation {
    Portrait,
    Landscape,
}

/// Safe area insets for notches, system bars, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SafeArea {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

/// Navigation route for type-safe routing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Route {
    Home,
    Settings,
    Profile { user_id: EntityId },
    Detail { item_id: EntityId },
    Custom(String),
}

/// Build configuration per platform
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildConfig {
    pub bundle_id: String,
    pub version_code: u32,
    pub version_name: String,
    pub signing_config: Option<SigningConfig>,
    pub proguard_enabled: bool,
    pub shrink_resources: bool,
    pub abi_filters: Vec<String>,
}

/// Signing configuration for release builds
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningConfig {
    pub store_file: String,
    pub store_password: String,
    pub key_alias: String,
    pub key_password: String,
}

/// Theme tokens for consistent styling across platforms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ThemeTokens {
    pub colors: ColorTokens,
    pub typography: TypographyTokens,
    pub spacing: SpacingTokens,
    pub shape: ShapeTokens,
    pub motion: MotionTokens,
    pub elevation: ElevationTokens,
}

/// Color tokens following Material Design 3
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorTokens {
    // Primary
    pub primary: ColorRole,
    pub on_primary: ColorRole,
    pub primary_container: ColorRole,
    pub on_primary_container: ColorRole,

    // Secondary
    pub secondary: ColorRole,
    pub on_secondary: ColorRole,
    pub secondary_container: ColorRole,
    pub on_secondary_container: ColorRole,

    // Tertiary
    pub tertiary: ColorRole,
    pub on_tertiary: ColorRole,
    pub tertiary_container: ColorRole,
    pub on_tertiary_container: ColorRole,

    // Error
    pub error: ColorRole,
    pub on_error: ColorRole,
    pub error_container: ColorRole,
    pub on_error_container: ColorRole,

    // Background/Surface
    pub background: ColorRole,
    pub on_background: ColorRole,
    pub surface: ColorRole,
    pub on_surface: ColorRole,
    pub surface_variant: ColorRole,
    pub on_surface_variant: ColorRole,
    pub surface_container: ColorRole,
    pub on_surface_container: ColorRole,

    // Outline
    pub outline: ColorRole,
    pub outline_variant: ColorRole,

    // Inverse
    pub inverse_surface: ColorRole,
    pub inverse_on_surface: ColorRole,
    pub inverse_primary: ColorRole,

    // Shadow/Scrim
    pub shadow: ColorRole,
    pub scrim: ColorRole,
}

/// Color role with light/dark variants
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorRole {
    pub light: ColorValue,
    pub dark: ColorValue,
    pub high_contrast_light: Option<ColorValue>,
    pub high_contrast_dark: Option<ColorValue>,
}

/// Color value in multiple formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorValue {
    pub hex: String,
    pub rgb: (u8, u8, u8),
    pub alpha: f32,
}

impl Eq for ColorValue {}
impl Eq for ColorRole {}

/// Text style definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_family: String,
    pub font_weight: FontWeight,
    pub font_size: f32,
    pub line_height: f32,
    pub letter_spacing: f32,
}

impl Eq for TextStyle {}

/// Typography tokens
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypographyTokens {
    pub display_large: TextStyle,
    pub display_medium: TextStyle,
    pub display_small: TextStyle,
    pub headline_large: TextStyle,
    pub headline_medium: TextStyle,
    pub headline_small: TextStyle,
    pub title_large: TextStyle,
    pub title_medium: TextStyle,
    pub title_small: TextStyle,
    pub body_large: TextStyle,
    pub body_medium: TextStyle,
    pub body_small: TextStyle,
    pub label_large: TextStyle,
    pub label_medium: TextStyle,
    pub label_small: TextStyle,
}

impl Eq for TypographyTokens {}

/// Font weight enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontWeight {
    Thin = 100,
    ExtraLight = 200,
    Light = 300,
    Regular = 400,
    Medium = 500,
    SemiBold = 600,
    Bold = 700,
    ExtraBold = 800,
    Black = 900,
}

/// Spacing tokens
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpacingTokens {
    pub space_0: f32,
    pub space_1: f32,
    pub space_2: f32,
    pub space_3: f32,
    pub space_4: f32,
    pub space_5: f32,
    pub space_6: f32,
    pub space_8: f32,
    pub space_10: f32,
    pub space_12: f32,
    pub space_16: f32,
    pub space_20: f32,
    pub space_24: f32,
    pub space_32: f32,
    pub space_40: f32,
    pub space_48: f32,
    pub space_64: f32,
}

impl Eq for SpacingTokens {}

/// Shape tokens (border radius)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeTokens {
    pub none: f32,
    pub extra_small: f32,
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub extra_large: f32,
    pub full: f32,
}

impl Eq for ShapeTokens {}

/// Motion tokens (animation durations/easing)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionTokens {
    pub duration_short: u32,
    pub duration_medium: u32,
    pub duration_long: u32,
    pub easing_standard: EasingCurve,
    pub easing_emphasized: EasingCurve,
    pub easing_decelerate: EasingCurve,
    pub easing_accelerate: EasingCurve,
}

impl Eq for MotionTokens {}

/// Easing curve definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EasingCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseOutExpo,
    Custom, // Defined by cubic-bezier
}

/// Elevation tokens (shadows)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElevationTokens {
    pub level_0: Shadow,
    pub level_1: Shadow,
    pub level_2: Shadow,
    pub level_3: Shadow,
    pub level_4: Shadow,
    pub level_5: Shadow,
}

impl Eq for ElevationTokens {}

/// Shadow definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: ColorValue,
}

impl Eq for Shadow {}

impl Default for ColorTokens {
    fn default() -> Self {
        // Material 3 baseline light theme
        let primary = ColorRole {
            light: ColorValue {
                hex: "#6750A4".to_string(),
                rgb: (103, 80, 164),
                alpha: 1.0,
            },
            dark: ColorValue {
                hex: "#D0BCFF".to_string(),
                rgb: (208, 188, 255),
                alpha: 1.0,
            },
            high_contrast_light: None,
            high_contrast_dark: None,
        };
        let default_role = ColorRole::default();
        Self {
            primary,
            on_primary: default_role.clone(),
            primary_container: default_role.clone(),
            on_primary_container: default_role.clone(),
            secondary: default_role.clone(),
            on_secondary: default_role.clone(),
            secondary_container: default_role.clone(),
            on_secondary_container: default_role.clone(),
            tertiary: default_role.clone(),
            on_tertiary: default_role.clone(),
            tertiary_container: default_role.clone(),
            on_tertiary_container: default_role.clone(),
            error: default_role.clone(),
            on_error: default_role.clone(),
            error_container: default_role.clone(),
            on_error_container: default_role.clone(),
            background: default_role.clone(),
            on_background: default_role.clone(),
            surface: default_role.clone(),
            on_surface: default_role.clone(),
            surface_variant: default_role.clone(),
            on_surface_variant: default_role.clone(),
            surface_container: default_role.clone(),
            on_surface_container: default_role.clone(),
            outline: default_role.clone(),
            outline_variant: default_role.clone(),
            inverse_surface: default_role.clone(),
            inverse_on_surface: default_role.clone(),
            inverse_primary: default_role.clone(),
            shadow: default_role.clone(),
            scrim: default_role,
        }
    }
}

impl ColorTokens {
    pub fn default_light() -> Self {
        Self::default()
    }
}

impl Default for ColorRole {
    fn default() -> Self {
        Self {
            light: ColorValue {
                hex: "#000000".to_string(),
                rgb: (0, 0, 0),
                alpha: 1.0,
            },
            dark: ColorValue {
                hex: "#FFFFFF".to_string(),
                rgb: (255, 255, 255),
                alpha: 1.0,
            },
            high_contrast_light: None,
            high_contrast_dark: None,
        }
    }
}

impl Default for TypographyTokens {
    fn default() -> Self {
        let base = TextStyle {
            font_family: "Inter".to_string(),
            font_weight: FontWeight::Regular,
            font_size: 14.0,
            line_height: 20.0,
            letter_spacing: 0.0,
        };
        Self {
            display_large: TextStyle {
                font_size: 57.0,
                line_height: 64.0,
                font_weight: FontWeight::Regular,
                ..base.clone()
            },
            display_medium: TextStyle {
                font_size: 45.0,
                line_height: 52.0,
                font_weight: FontWeight::Regular,
                ..base.clone()
            },
            display_small: TextStyle {
                font_size: 36.0,
                line_height: 44.0,
                font_weight: FontWeight::Regular,
                ..base.clone()
            },
            headline_large: TextStyle {
                font_size: 32.0,
                line_height: 40.0,
                font_weight: FontWeight::Regular,
                ..base.clone()
            },
            headline_medium: TextStyle {
                font_size: 28.0,
                line_height: 36.0,
                font_weight: FontWeight::Regular,
                ..base.clone()
            },
            headline_small: TextStyle {
                font_size: 24.0,
                line_height: 32.0,
                font_weight: FontWeight::Regular,
                ..base.clone()
            },
            title_large: TextStyle {
                font_size: 22.0,
                line_height: 28.0,
                font_weight: FontWeight::Regular,
                ..base.clone()
            },
            title_medium: TextStyle {
                font_size: 16.0,
                line_height: 24.0,
                font_weight: FontWeight::Medium,
                ..base.clone()
            },
            title_small: TextStyle {
                font_size: 14.0,
                line_height: 20.0,
                font_weight: FontWeight::Medium,
                ..base.clone()
            },
            body_large: TextStyle {
                font_size: 16.0,
                line_height: 24.0,
                font_weight: FontWeight::Regular,
                ..base.clone()
            },
            body_medium: TextStyle {
                font_size: 14.0,
                line_height: 20.0,
                font_weight: FontWeight::Regular,
                ..base.clone()
            },
            body_small: TextStyle {
                font_size: 12.0,
                line_height: 16.0,
                font_weight: FontWeight::Regular,
                ..base.clone()
            },
            label_large: TextStyle {
                font_size: 14.0,
                line_height: 20.0,
                font_weight: FontWeight::Medium,
                ..base.clone()
            },
            label_medium: TextStyle {
                font_size: 12.0,
                line_height: 16.0,
                font_weight: FontWeight::Medium,
                ..base.clone()
            },
            label_small: TextStyle {
                font_size: 11.0,
                line_height: 16.0,
                font_weight: FontWeight::Medium,
                ..base.clone()
            },
        }
    }
}

impl Default for SpacingTokens {
    fn default() -> Self {
        Self {
            space_0: 0.0,
            space_1: 4.0,
            space_2: 8.0,
            space_3: 12.0,
            space_4: 16.0,
            space_5: 20.0,
            space_6: 24.0,
            space_8: 32.0,
            space_10: 40.0,
            space_12: 48.0,
            space_16: 64.0,
            space_20: 80.0,
            space_24: 96.0,
            space_32: 128.0,
            space_40: 160.0,
            space_48: 192.0,
            space_64: 256.0,
        }
    }
}

impl Default for ShapeTokens {
    fn default() -> Self {
        Self {
            none: 0.0,
            extra_small: 4.0,
            small: 8.0,
            medium: 12.0,
            large: 16.0,
            extra_large: 28.0,
            full: 9999.0,
        }
    }
}

impl Default for MotionTokens {
    fn default() -> Self {
        Self {
            duration_short: 150,
            duration_medium: 250,
            duration_long: 350,
            easing_standard: EasingCurve::EaseInOut,
            easing_emphasized: EasingCurve::EaseOutExpo,
            easing_decelerate: EasingCurve::EaseOut,
            easing_accelerate: EasingCurve::EaseIn,
        }
    }
}

impl Default for ElevationTokens {
    fn default() -> Self {
        let base_color = ColorValue {
            hex: "#000000".to_string(),
            rgb: (0, 0, 0),
            alpha: 0.3,
        };
        Self {
            level_0: Shadow {
                offset_x: 0.0,
                offset_y: 0.0,
                blur_radius: 0.0,
                spread_radius: 0.0,
                color: base_color.clone(),
            },
            level_1: Shadow {
                offset_x: 0.0,
                offset_y: 1.0,
                blur_radius: 2.0,
                spread_radius: 0.0,
                color: base_color.clone(),
            },
            level_2: Shadow {
                offset_x: 0.0,
                offset_y: 3.0,
                blur_radius: 6.0,
                spread_radius: 0.0,
                color: base_color.clone(),
            },
            level_3: Shadow {
                offset_x: 0.0,
                offset_y: 6.0,
                blur_radius: 12.0,
                spread_radius: 0.0,
                color: base_color.clone(),
            },
            level_4: Shadow {
                offset_x: 0.0,
                offset_y: 10.0,
                blur_radius: 20.0,
                spread_radius: 0.0,
                color: base_color.clone(),
            },
            level_5: Shadow {
                offset_x: 0.0,
                offset_y: 15.0,
                blur_radius: 30.0,
                spread_radius: 0.0,
                color: base_color.clone(),
            },
        }
    }
}
