//! Core crate - Shared business logic for lightweight cross-platform Rust applications
//!
//! This crate contains all platform-agnostic business logic including:
//! - Domain models and entities
//! - Error types
//! - Configuration management
//! - Service traits and in-memory implementations

use std::sync::Arc;

pub mod api;
pub mod config;
pub mod domain;
pub mod errors;
pub mod services;

// Re-export commonly used types
pub use api::{endpoints, ErrorResponse, HealthResponse, RegisterRequest, UserList};
pub use config::{AppConfig, ConfigBuilder, ThemeMode};
pub use domain::{
    AccessibilitySettings, AppSettings, ColorRole, ColorTokens, ColorValue, DeviceInfo,
    EasingCurve, ElevationTokens, Entity, EntityId, FontWeight, FormFactor, MotionTokens,
    Orientation, Platform, SafeArea, ScreenSize, Shadow, ShapeTokens, SpacingTokens, TextStyle,
    ThemePreference, ThemeTokens, Timestamp, TypographyTokens, User, UserPreferences,
    WindowPosition, WindowState,
};
pub use errors::{
    AppError, AuthError, ConfigError, ErrorContext, ErrorContextExt, NetworkError, PlatformError,
    Result, SerializationError, StorageError, UiError, ValidationError,
};
pub use services::{
    defaults::{
        DefaultAnalyticsService, DefaultAuthService, DefaultNotificationService,
        DefaultPlatformService, DefaultSettingsService, DefaultStorageService, DefaultUserService,
    },
    AuthService, NetworkService, PlatformService, ServiceContainer, SettingsService,
    StorageService, UserService,
};

/// The [`NetworkService`] this build compiles: `reqwest` on native targets,
/// [`StubNetworkService`] everywhere else. Re-exported under one name so calling
/// code does not have to know which target it was built for.
#[cfg(feature = "network")]
pub use services::defaults::DefaultNetworkService;
#[cfg(not(feature = "network"))]
pub use services::defaults::StubNetworkService as DefaultNetworkService;

/// A blocking, TLS-free HTTP/1.1 client: the one the example app uses to reach
/// the bundled backend in `apps/server`.
///
/// It is separate from [`DefaultNetworkService`] on purpose. That one pulls in
/// `reqwest` and a TLS stack behind the `network` feature, which is why it is
/// off by default and cannot be cross-compiled to some targets. This client
/// speaks plain HTTP over [`std::net`], so it compiles for desktop, Android,
/// iOS, Windows and macOS with no C toolchain and no new dependency.
///
/// A browser tab has no raw sockets, so the web build has no HTTP client. The
/// example app hides its backend panel there rather than shipping a fake one.
#[cfg(not(target_arch = "wasm32"))]
pub use services::defaults::HttpNetworkService;

/// Initialize the core crate with configuration.
///
/// Use this from an async context. Entry points that cannot await — Android's
/// `android_main`, for instance — should call [`init_blocking`] instead.
pub async fn init(config: Option<Arc<AppConfig>>) -> Result<Arc<AppConfig>> {
    init_blocking(config)
}

/// Initialize the core crate without an async runtime.
///
/// Split out from [`init`] because Android calls into the app from
/// `android_main`, which is a plain `extern "C"` function and cannot await.
/// The two functions do exactly the same work.
pub fn init_blocking(config: Option<Arc<AppConfig>>) -> Result<Arc<AppConfig>> {
    let config = match config {
        Some(c) => c,
        None => AppConfig::load()?,
    };

    // Initialize tracing. A second call panics inside `init()`, which is fine:
    // the process only ever boots once.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init()
        .ok();

    tracing::info!(
        "Core crate initialized for platform: {:?}",
        config.platform.platform
    );
    Ok(config)
}

/// Core crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Core crate name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Convert ThemeMode to ThemePreference
impl From<crate::config::ThemeMode> for crate::domain::ThemePreference {
    fn from(mode: crate::config::ThemeMode) -> Self {
        match mode {
            crate::config::ThemeMode::System => Self::System,
            crate::config::ThemeMode::Light => Self::Light,
            crate::config::ThemeMode::Dark => Self::Dark,
            crate::config::ThemeMode::HighContrast => Self::HighContrast,
        }
    }
}
