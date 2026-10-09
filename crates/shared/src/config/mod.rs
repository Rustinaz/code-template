//! Configuration management for lightweight cross-platform applications
//!
//! This module handles loading, validation, and management of application
//! configuration from multiple sources (files, env vars, platform defaults).

use crate::domain::{
    DeviceInfo, FormFactor, Orientation, Platform, SafeArea, ScreenSize, ThemeTokens,
};
use crate::errors::{ConfigError, Result};
#[cfg(not(target_arch = "wasm32"))]
use config::Environment;
use config::{Config, File, FileFormat};
use directories::ProjectDirs;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

/// Global configuration instance
static GLOBAL_CONFIG: Lazy<RwLock<Option<Arc<AppConfig>>>> = Lazy::new(|| RwLock::new(None));

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppInfo,
    pub platform: PlatformConfig,
    pub ui: UiConfig,
    pub network: NetworkConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
    pub features: FeatureFlags,
}

/// Application metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub bundle_id: String,
    pub author: String,
    pub description: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
}

/// Platform configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub platform: Platform,
    pub min_version: String,
    pub target_version: String,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: ThemeConfig,
    pub font_scale: f32,
    pub ui_scale: f32,
    pub reduced_motion: bool,
    pub high_contrast: bool,
    pub language: String,
    pub font_family: String,
    pub animations_enabled: bool,
    pub responsive_breakpoints: ResponsiveBreakpoints,
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub mode: ThemeMode,
    pub custom_tokens: Option<ThemeTokens>,
    pub platform_adaptation: bool,
}

/// Theme mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
    HighContrast,
}

/// Responsive breakpoints for different form factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsiveBreakpoints {
    pub phone_max_width: u32,
    pub tablet_max_width: u32,
    pub desktop_min_width: u32,
}

impl Default for ResponsiveBreakpoints {
    fn default() -> Self {
        Self {
            phone_max_width: 599,
            tablet_max_width: 839,
            desktop_min_width: 840,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Where the app's HTTP client points. The default is the bundled backend
    /// in `apps/server`, which `scripts/dev.sh` starts on this address.
    pub base_url: String,
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
    pub enable_caching: bool,
    pub cache_max_age_secs: u64,
    pub user_agent: String,
    pub tls_verification: bool,
    pub proxy: Option<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub database_path: Option<PathBuf>,
    pub enable_encryption: bool,
    pub encryption_key: Option<String>,
    pub max_size_mb: u64,
    pub backup_enabled: bool,
    pub backup_interval_hours: u32,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub format: LogFormat,
    pub output: LogOutput,
    pub file_path: Option<PathBuf>,
    pub max_file_size_mb: u64,
    pub max_files: u32,
    pub include_timestamp: bool,
    pub include_thread_id: bool,
    pub include_file_line: bool,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Log format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Text,
    Compact,
}

/// Log output destination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogOutput {
    Stdout,
    Stderr,
    File,
    Both,
}

/// Feature flags for conditional compilation/runtime features
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeatureFlags {
    pub analytics: bool,
    pub crash_reporting: bool,
    pub auto_updates: bool,
    pub offline_mode: bool,
    pub debug_menu: bool,
    pub experimental_ui: bool,
    pub telemetry: bool,
    pub custom: HashMap<String, bool>,
}

impl AppConfig {
    /// Load configuration from all sources
    pub fn load() -> Result<Arc<Self>> {
        let config = Self::load_internal()?;
        let arc = Arc::new(config);
        *GLOBAL_CONFIG.write() = Some(arc.clone());
        Ok(arc)
    }

    /// Get the global configuration instance
    pub fn global() -> Option<Arc<Self>> {
        GLOBAL_CONFIG.read().clone()
    }

    /// Get global config or load if not initialized
    pub fn global_or_load() -> Result<Arc<Self>> {
        if let Some(config) = Self::global() {
            Ok(config)
        } else {
            Self::load()
        }
    }

    fn load_internal() -> Result<Self> {
        // `directories` cannot resolve a config directory on every target: the
        // browser has no filesystem, and Android has no XDG home. That is not an
        // error -- the built-in defaults below are a complete configuration, and
        // the file and the `APP__*` environment variables are optional overlays
        // on top of them. Requiring `ProjectDirs` here made web and Android
        // boot fail outright before the defaults were ever reached.
        let config_file = ProjectDirs::from("com", "example", "rust-crossplatform-template")
            .map(|dirs| dirs.config_dir().join("config.toml"));

        let mut builder = Config::builder()
            // Default values
            .set_default("app.name", "Rust CrossPlatform App")?
            .set_default("app.version", env!("CARGO_PKG_VERSION"))?
            .set_default("app.bundle_id", "com.example.rust-crossplatform")?
            .set_default("app.author", "Your Name")?
            .set_default("app.description", "A cross-platform Rust application")?
            // Platform defaults
            .set_default("platform.platform", Self::detect_platform().to_string())?
            .set_default("platform.min_version", Self::default_min_version())?
            .set_default("platform.target_version", Self::default_target_version())?
            // UI defaults
            .set_default("ui.theme.mode", "system")?
            .set_default("ui.font_scale", 1.0)?
            .set_default("ui.ui_scale", 1.0)?
            .set_default("ui.reduced_motion", false)?
            .set_default("ui.high_contrast", false)?
            .set_default("ui.language", "en")?
            .set_default("ui.font_family", "Inter")?
            .set_default("ui.animations_enabled", true)?
            .set_default("ui.responsive_breakpoints.phone_max_width", 599)?
            .set_default("ui.responsive_breakpoints.tablet_max_width", 839)?
            .set_default("ui.responsive_breakpoints.desktop_min_width", 840)?
            .set_default("ui.theme.platform_adaptation", true)?
            // Network defaults. The bundled backend in `apps/server` listens
            // here, so the example app can reach it with no configuration.
            .set_default("network.base_url", "http://127.0.0.1:8080")?
            .set_default("network.timeout_ms", 30000)?
            .set_default("network.max_retries", 3)?
            .set_default("network.retry_backoff_ms", 1000)?
            .set_default("network.enable_caching", true)?
            .set_default("network.cache_max_age_secs", 3600)?
            .set_default(
                "network.user_agent",
                format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            )?
            .set_default("network.tls_verification", true)?
            // Storage defaults
            .set_default("storage.enable_encryption", false)?
            .set_default("storage.max_size_mb", 100)?
            .set_default("storage.backup_enabled", true)?
            .set_default("storage.backup_interval_hours", 24)?
            // Logging defaults
            .set_default("logging.level", "info")?
            .set_default("logging.format", "text")?
            .set_default("logging.output", "stdout")?
            .set_default("logging.max_file_size_mb", 10)?
            .set_default("logging.max_files", 5)?
            .set_default("logging.include_timestamp", true)?
            .set_default("logging.include_thread_id", false)?
            .set_default("logging.include_file_line", true)?
            // Features defaults
            .set_default("features.analytics", false)?
            .set_default("features.crash_reporting", false)?
            .set_default("features.auto_updates", true)?
            .set_default("features.offline_mode", true)?
            .set_default("features.debug_menu", cfg!(debug_assertions))?
            .set_default("features.experimental_ui", false)?
            .set_default("features.telemetry", false)?
            .set_default("features.custom", HashMap::<String, bool>::new())?;

        // Load from a config file only when the target has a config directory
        // and the file is actually there.
        if let Some(path) = config_file.as_deref() {
            if path.exists() {
                let path_str = path.to_str().ok_or_else(|| ConfigError::FileNotFound {
                    path: path.display().to_string(),
                })?;
                builder = builder.add_source(File::new(path_str, FileFormat::Toml));
                info!("Loading config from: {}", path.display());
            }
        }

        // Load from environment variables (prefixed with APP__). The `config`
        // crate's environment source walks `std::env::vars_os`, which is
        // unsupported on wasm32 and panics there, so the overlay is only added
        // where a real process environment exists.
        #[cfg(not(target_arch = "wasm32"))]
        {
            builder = builder.add_source(Environment::with_prefix("APP").separator("__"));
        }

        let config = builder.build()?;
        let app_config: AppConfig = config.try_deserialize()?;

        // Validate and adjust for platform
        app_config.validate_and_adjust()?;

        info!(
            "Configuration loaded successfully for platform: {:?}",
            app_config.platform.platform
        );
        Ok(app_config)
    }

    /// Detect the current platform.
    ///
    /// This only supplies the default for `platform.platform` when no config
    /// file sets it. Entry points that already know their target (Android and
    /// the web) still override it, so this never has to be right on a target the
    /// host compiler is not building for -- but it must at least name the target
    /// it *is* building for, which the old Linux-only version did not.
    fn detect_platform() -> Platform {
        #[cfg(target_arch = "wasm32")]
        return Platform::Web;
        #[cfg(target_os = "android")]
        return Platform::Android;
        #[cfg(target_os = "ios")]
        return Platform::Ios;
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(target_os = "macos")]
        return Platform::Macos;
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        #[allow(unreachable_code)]
        Platform::Linux
    }

    fn default_min_version() -> String {
        match Self::detect_platform() {
            Platform::Android => "21".to_string(),
            Platform::Ios => "13.0".to_string(),
            Platform::Linux => "2.6.32".to_string(),
            Platform::Windows => "10.0.17763".to_string(),
            Platform::Macos => "10.15".to_string(),
            Platform::Web => "1.0".to_string(),
        }
    }

    fn default_target_version() -> String {
        match Self::detect_platform() {
            Platform::Android => "34".to_string(),
            Platform::Ios => "17.0".to_string(),
            Platform::Linux => "6.0".to_string(),
            Platform::Windows => "10.0.22621".to_string(),
            Platform::Macos => "14.0".to_string(),
            Platform::Web => "1.0".to_string(),
        }
    }

    /// Validate and adjust configuration for current platform
    fn validate_and_adjust(&self) -> Result<()> {
        // Validate font scale
        if self.ui.font_scale < 0.5 || self.ui.font_scale > 3.0 {
            return Err(ConfigError::InvalidValue {
                key: "ui.font_scale".to_string(),
                value: self.ui.font_scale.to_string(),
            }
            .into());
        }

        // Validate UI scale
        if self.ui.ui_scale < 0.5 || self.ui.ui_scale > 3.0 {
            return Err(ConfigError::InvalidValue {
                key: "ui.ui_scale".to_string(),
                value: self.ui.ui_scale.to_string(),
            }
            .into());
        }

        Ok(())
    }

    /// Save configuration to file.
    ///
    /// A silent no-op on targets without a config directory (the browser and
    /// Android), where the defaults are the only configuration anyway.
    pub fn save(&self) -> Result<()> {
        let Some(project_dirs) = ProjectDirs::from("com", "example", "rust-crossplatform-template")
        else {
            warn!("No config directory on this target; not saving configuration");
            return Ok(());
        };

        let config_dir = project_dirs.config_dir();
        std::fs::create_dir_all(config_dir)?;

        let config_file = config_dir.join("config.toml");
        let toml = toml::to_string_pretty(self)?;
        std::fs::write(&config_file, toml)?;

        info!("Configuration saved to: {}", config_file.display());
        Ok(())
    }

    /// Get device info for responsive layout
    pub fn device_info(&self) -> DeviceInfo {
        let platform = self.platform.platform;
        let (form_factor, screen_size) = self.calculate_form_factor();
        let orientation = self.detect_orientation();

        DeviceInfo {
            platform,
            form_factor,
            screen_size,
            pixel_density: self.ui.ui_scale,
            orientation,
            has_touch: false,
            has_keyboard: true,
            has_mouse: true,
            safe_area: self.calculate_safe_area(),
        }
    }

    fn calculate_form_factor(&self) -> (FormFactor, ScreenSize) {
        // In a real app, this would come from platform APIs
        // For now, estimate from config
        let width = 800; // Would come from window manager

        let form_factor = if width < self.ui.responsive_breakpoints.phone_max_width as i32 {
            FormFactor::Phone
        } else if width < self.ui.responsive_breakpoints.tablet_max_width as i32 {
            FormFactor::Tablet
        } else {
            FormFactor::Desktop
        };

        let screen_size = if width < 600 {
            ScreenSize::Small
        } else if width < 840 {
            ScreenSize::Medium
        } else if width < 1200 {
            ScreenSize::Large
        } else {
            ScreenSize::ExtraLarge
        };

        (form_factor, screen_size)
    }

    fn detect_orientation(&self) -> Orientation {
        // Would come from platform APIs
        Orientation::Landscape
    }

    fn calculate_safe_area(&self) -> SafeArea {
        SafeArea::default()
    }

    /// Get theme tokens (merged with custom if provided)
    pub fn theme_tokens(&self) -> ThemeTokens {
        self.ui.theme.custom_tokens.clone().unwrap_or_default()
    }

    /// Check if a feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "analytics" => self.features.analytics,
            "crash_reporting" => self.features.crash_reporting,
            "auto_updates" => self.features.auto_updates,
            "offline_mode" => self.features.offline_mode,
            "debug_menu" => self.features.debug_menu,
            "experimental_ui" => self.features.experimental_ui,
            "telemetry" => self.features.telemetry,
            _ => self.features.custom.get(feature).copied().unwrap_or(false),
        }
    }
}

/// Configuration builder for programmatic configuration
pub struct ConfigBuilder {
    config: AppConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        let config = AppConfig::load_internal().unwrap_or_else(|_| Self::default_config());
        Self { config }
    }

    fn default_config() -> AppConfig {
        AppConfig {
            app: AppInfo {
                name: "Rust CrossPlatform App".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                bundle_id: "com.example.rust-crossplatform".to_string(),
                author: "Your Name".to_string(),
                description: "A cross-platform Rust application".to_string(),
                homepage: None,
                repository: None,
            },
            platform: PlatformConfig {
                platform: AppConfig::detect_platform(),
                min_version: AppConfig::default_min_version(),
                target_version: AppConfig::default_target_version(),
            },
            ui: UiConfig {
                theme: ThemeConfig {
                    mode: ThemeMode::System,
                    custom_tokens: None,
                    platform_adaptation: true,
                },
                font_scale: 1.0,
                ui_scale: 1.0,
                reduced_motion: false,
                high_contrast: false,
                language: "en".to_string(),
                font_family: "Inter".to_string(),
                animations_enabled: true,
                responsive_breakpoints: ResponsiveBreakpoints::default(),
            },
            network: NetworkConfig {
                base_url: "http://127.0.0.1:8080".to_string(),
                timeout_ms: 30000,
                max_retries: 3,
                retry_backoff_ms: 1000,
                enable_caching: true,
                cache_max_age_secs: 3600,
                user_agent: format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
                tls_verification: true,
                proxy: None,
            },
            storage: StorageConfig {
                database_path: None,
                enable_encryption: false,
                encryption_key: None,
                max_size_mb: 100,
                backup_enabled: true,
                backup_interval_hours: 24,
            },
            logging: LoggingConfig {
                level: LogLevel::Info,
                format: LogFormat::Text,
                output: LogOutput::Stdout,
                file_path: None,
                max_file_size_mb: 10,
                max_files: 5,
                include_timestamp: true,
                include_thread_id: false,
                include_file_line: true,
            },
            features: FeatureFlags::default(),
        }
    }

    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.config.app.name = name.into();
        self
    }

    pub fn bundle_id(mut self, id: impl Into<String>) -> Self {
        self.config.app.bundle_id = id.into();
        self
    }

    pub fn theme_mode(mut self, mode: ThemeMode) -> Self {
        self.config.ui.theme.mode = mode;
        self
    }

    pub fn font_scale(mut self, scale: f32) -> Self {
        self.config.ui.font_scale = scale.clamp(0.5, 3.0);
        self
    }

    /// Turns a feature flag on or off.
    ///
    /// Well-known names are routed to their typed field so that
    /// [`AppConfig::is_feature_enabled`] sees them; anything else goes into
    /// `features.custom`. Without the routing, `.feature("analytics", true)`
    /// would write to `custom` while the reader checks the typed `analytics`
    /// field, and the call would silently do nothing.
    pub fn feature(mut self, name: impl Into<String>, enabled: bool) -> Self {
        let name = name.into();
        match name.as_str() {
            "analytics" => self.config.features.analytics = enabled,
            "crash_reporting" => self.config.features.crash_reporting = enabled,
            "auto_updates" => self.config.features.auto_updates = enabled,
            "offline_mode" => self.config.features.offline_mode = enabled,
            "debug_menu" => self.config.features.debug_menu = enabled,
            "experimental_ui" => self.config.features.experimental_ui = enabled,
            "telemetry" => self.config.features.telemetry = enabled,
            _ => {
                self.config.features.custom.insert(name, enabled);
            }
        }
        self
    }

    pub fn build(self) -> AppConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .app_name("Test App")
            .bundle_id("com.test.app")
            .theme_mode(ThemeMode::Dark)
            .font_scale(1.2)
            .feature("analytics", true)
            .build();

        assert_eq!(config.app.name, "Test App");
        assert_eq!(config.app.bundle_id, "com.test.app");
        assert_eq!(config.ui.theme.mode, ThemeMode::Dark);
        assert_eq!(config.ui.font_scale, 1.2);
        assert!(config.is_feature_enabled("analytics"));
    }

    #[test]
    fn test_config_builder_routes_unknown_features_to_custom() {
        let config = ConfigBuilder::new().feature("beta_dashboard", true).build();

        assert!(config.is_feature_enabled("beta_dashboard"));
        assert!(config.features.custom.contains_key("beta_dashboard"));
    }

    #[test]
    fn test_config_builder_known_feature_is_not_duplicated_into_custom() {
        let config = ConfigBuilder::new().feature("analytics", true).build();

        assert!(config.features.analytics);
        assert!(
            !config.features.custom.contains_key("analytics"),
            "a known feature should live in its typed field, not in custom"
        );
    }

    #[test]
    fn test_unknown_feature_defaults_to_disabled() {
        assert!(!ConfigBuilder::new().build().is_feature_enabled("nope"));
    }

    #[test]
    fn test_font_scale_is_clamped() {
        assert_eq!(
            ConfigBuilder::new().font_scale(0.1).build().ui.font_scale,
            0.5
        );
        assert_eq!(
            ConfigBuilder::new().font_scale(9.0).build().ui.font_scale,
            3.0
        );
    }

    #[test]
    fn test_theme_mode_default() {
        assert_eq!(ThemeMode::default(), ThemeMode::System);
    }

    #[test]
    fn test_responsive_breakpoints_default() {
        let bp = ResponsiveBreakpoints::default();
        assert_eq!(bp.phone_max_width, 599);
        assert_eq!(bp.tablet_max_width, 839);
        assert_eq!(bp.desktop_min_width, 840);
    }

    #[test]
    fn detect_platform_names_the_host_target() {
        // Guards the regression where this matched Linux only. The Android and
        // web entry points override the platform, but a default that says Linux
        // on every target is still wrong and hides the mistake.
        let expected = if cfg!(target_arch = "wasm32") {
            Platform::Web
        } else if cfg!(target_os = "android") {
            Platform::Android
        } else if cfg!(target_os = "ios") {
            Platform::Ios
        } else if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::Macos
        } else {
            Platform::Linux
        };
        assert_eq!(AppConfig::detect_platform(), expected);
    }

    #[test]
    fn config_loads_without_a_config_file() {
        // The built-in defaults are a complete configuration. A missing
        // `config.toml` -- always the case in a browser, and on Android -- must
        // not fail start-up; that bug stopped both targets from booting at all.
        let config = AppConfig::load().expect("the built-in defaults should always load");
        assert_eq!(config.network.base_url, "http://127.0.0.1:8080");
        assert_eq!(config.platform.platform, AppConfig::detect_platform());
    }
}
