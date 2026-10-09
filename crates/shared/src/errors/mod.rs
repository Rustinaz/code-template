//! Core error types for the application
//!
//! This module defines the unified error hierarchy used across all platforms.
//! Errors are categorized by domain and provide rich context for debugging.

use thiserror::Error;

/// Main application error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Platform error: {0}")]
    Platform(#[from] PlatformError),

    #[error("UI error: {0}")]
    Ui(#[from] UiError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] SerializationError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration library error: {0}")]
    ConfigLib(#[from] config::ConfigError),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    /// Present only with the `network` feature. See the crate feature docs in
    /// `Cargo.toml` for why it is not on by default.
    #[cfg(feature = "network")]
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Opener error: {0}")]
    Opener(#[from] opener::OpenError),

    #[error("Permission denied: {resource}")]
    PermissionDenied { resource: String },

    #[error("Resource not found: {resource} (id: {id})")]
    NotFound { resource: String, id: String },

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Timeout: {operation} took longer than {timeout_ms}ms")]
    Timeout { operation: String, timeout_ms: u64 },

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Unsupported operation: {operation} on {platform}")]
    Unsupported { operation: String, platform: String },

    #[error("Multiple errors occurred: {0:?}")]
    Multiple(Vec<AppError>),
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required configuration: {key}")]
    MissingKey { key: String },

    #[error("Invalid configuration value for {key}: {value}")]
    InvalidValue { key: String, value: String },

    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Failed to parse configuration: {source}")]
    ParseError { source: anyhow::Error },

    #[error("Configuration migration failed from v{from} to v{to}: {reason}")]
    MigrationFailed { from: u32, to: u32, reason: String },

    #[error("Environment variable not set: {var}")]
    EnvVarNotSet { var: String },
}

/// Network-related errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection failed: {reason}")]
    ConnectionFailed { reason: String },

    #[error("Request timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("DNS resolution failed for {host}")]
    DnsFailed { host: String },

    #[error("SSL/TLS error: {reason}")]
    TlsError { reason: String },

    #[error("Rate limited: retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Offline: no network connectivity")]
    Offline,

    #[error("Invalid URL: {url}")]
    InvalidUrl { url: String },

    #[error("Request cancelled")]
    Cancelled,
}

/// Storage-related errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {source}")]
    Database { source: anyhow::Error },

    #[error("Migration failed: {version} - {reason}")]
    MigrationFailed { version: u32, reason: String },

    #[error("Constraint violation: {constraint}")]
    ConstraintViolation { constraint: String },

    #[error("Record not found: {table} id={id}")]
    NotFound { table: String, id: String },

    #[error("Serialization failed: {source}")]
    Serialization { source: anyhow::Error },

    #[error("Disk full: {available_bytes} bytes available")]
    DiskFull { available_bytes: u64 },

    #[error("Permission denied accessing {path}")]
    PermissionDenied { path: String },

    #[error("Corrupted data at {location}")]
    Corrupted { location: String },
}

/// Authentication-related errors
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Token invalid: {reason}")]
    TokenInvalid { reason: String },

    #[error("Session expired")]
    SessionExpired,

    #[error("User not found: {identifier}")]
    UserNotFound { identifier: String },

    #[error("Account locked: {reason}")]
    AccountLocked { reason: String },

    #[error("MFA required")]
    MfaRequired,

    #[error("OAuth error: {provider} - {message}")]
    OAuthError { provider: String, message: String },

    #[error("Biometric authentication failed: {reason}")]
    BiometricFailed { reason: String },

    #[error("Permission denied: {permission}")]
    PermissionDenied { permission: String },

    #[error("No signed-in user")]
    NotAuthenticated,
}

/// Validation errors with field-level detail
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Field '{field}': {message}")]
    Field { field: String, message: String },

    #[error("Multiple validation errors: {0:?}")]
    Multiple(Vec<ValidationError>),

    #[error("Required field missing: {field}")]
    Required { field: String },

    #[error("Field '{field}' too long: max {max} chars")]
    TooLong { field: String, max: usize },

    #[error("Field '{field}' too short: min {min} chars")]
    TooShort { field: String, min: usize },

    #[error("Field '{field}' invalid format: expected {pattern}")]
    InvalidFormat { field: String, pattern: String },

    #[error("Field '{field}' out of range: {min} - {max}")]
    OutOfRange {
        field: String,
        min: String,
        max: String,
    },

    #[error("Duplicate value for unique field: {field}")]
    Duplicate { field: String },
}

/// Platform-specific errors
#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("Android error: {source}")]
    Android { source: anyhow::Error },

    #[error("iOS error: {source}")]
    Ios { source: anyhow::Error },

    #[error("Linux error: {source}")]
    Linux { source: anyhow::Error },

    #[error("Windows error: {source}")]
    Windows { source: anyhow::Error },

    #[error("macOS error: {source}")]
    Macos { source: anyhow::Error },

    #[error("Web error: {source}")]
    Web { source: anyhow::Error },

    #[error("JNI error: {message}")]
    Jni { message: String },

    #[error("Objective-C error: {message}")]
    ObjC { message: String },

    #[error("Platform API not available: {api}")]
    ApiNotAvailable { api: String },

    #[error("Version mismatch: required {required}, found {found}")]
    VersionMismatch { required: String, found: String },
}

/// UI-related errors
#[derive(Error, Debug)]
pub enum UiError {
    #[error("Component not found: {id}")]
    ComponentNotFound { id: String },

    #[error("Layout error: {message}")]
    Layout { message: String },

    #[error("Theme error: {message}")]
    Theme { message: String },

    #[error("Animation error: {message}")]
    Animation { message: String },

    #[error("Focus error: {message}")]
    Focus { message: String },

    #[error("Accessibility error: {message}")]
    Accessibility { message: String },

    #[error("Resource loading failed: {resource}")]
    ResourceLoadFailed { resource: String },

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },
}

/// Serialization errors
#[derive(Error, Debug)]
pub enum SerializationError {
    #[error("JSON serialization failed: {source}")]
    Json { source: serde_json::Error },

    #[error("TOML serialization failed: {source}")]
    Toml { source: toml::ser::Error },

    #[error("RON serialization failed: {source}")]
    Ron { source: ron::error::Error },

    #[error("RON deserialization failed: {source}")]
    RonDeserialization { source: ron::error::SpannedError },

    #[error("Integer parsing failed: {source}")]
    ParseInt { source: std::num::ParseIntError },

    #[error("TOML deserialization failed: {source}")]
    TomlDeserialization { source: toml::de::Error },

    #[error("Binary serialization failed: {source}")]
    Binary { source: anyhow::Error },

    #[error("Deserialization failed: {source}")]
    Deserialization { source: anyhow::Error },
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, AppError>;

/// Error context for richer error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub platform: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            platform: std::env::consts::OS.to_string(),
            user_id: None,
            session_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Extension trait for adding context to errors
pub trait ErrorContextExt<T> {
    fn with_context(self, ctx: ErrorContext) -> Result<T>;
}

impl<T, E> ErrorContextExt<T> for std::result::Result<T, E>
where
    E: Into<AppError>,
{
    fn with_context(self, ctx: ErrorContext) -> Result<T> {
        self.map_err(|e| {
            let app_error: AppError = e.into();
            tracing::error!(
                operation = %ctx.operation,
                platform = %ctx.platform,
                user_id = ?ctx.user_id,
                session_id = ?ctx.session_id,
                metadata = ?ctx.metadata,
                error = %app_error,
                "Operation failed with context"
            );
            app_error
        })
    }
}

/// Macro for creating validation errors easily
#[macro_export]
macro_rules! validation_error {
    ($field:expr, $msg:expr) => {
        $crate::errors::ValidationError::Field {
            field: $field.to_string(),
            message: $msg.to_string(),
        }
    };
    (required $field:expr) => {
        $crate::errors::ValidationError::Required {
            field: $field.to_string(),
        }
    };
    (too_long $field:expr, $max:expr) => {
        $crate::errors::ValidationError::TooLong {
            field: $field.to_string(),
            max: $max,
        }
    };
    (too_short $field:expr, $min:expr) => {
        $crate::errors::ValidationError::TooShort {
            field: $field.to_string(),
            min: $min,
        }
    };
}

/// Macro for creating not found errors
#[macro_export]
macro_rules! not_found_error {
    ($resource:expr, $id:expr) => {
        $crate::errors::AppError::NotFound {
            resource: $resource.to_string(),
            id: $id.to_string(),
        }
    };
}

/// Explicit From implementations for error chaining
impl From<ron::error::SpannedError> for AppError {
    fn from(err: ron::error::SpannedError) -> Self {
        AppError::Serialization(SerializationError::RonDeserialization { source: err })
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(err: std::num::ParseIntError) -> Self {
        AppError::Serialization(SerializationError::ParseInt { source: err })
    }
}

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self {
        AppError::Serialization(SerializationError::TomlDeserialization { source: err })
    }
}
