//! Build configuration crate
//!
//! This crate handles build-time configuration, version management,
//! and generates build metadata for the application.
//!
//! NOTE: For the lightweight version, module files are not created.
//! The main types are defined inline here.

// pub mod version;
// pub mod metadata;
// pub mod targets;

use clap::{Parser, Subcommand};
use shared::domain::{BuildConfig, Platform, SigningConfig};
use shared::errors::Result;
use std::collections::HashMap;
use std::path::PathBuf;

/// Build-time configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildTimeConfig {
    pub version: VersionInfo,
    pub git: GitInfo,
    pub rustc: RustcInfo,
    pub target: TargetInfo,
    pub features: Vec<String>,
    pub profile: String,
    pub timestamp: String,
}

/// Version information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionInfo {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build_metadata: Option<String>,
    pub full: String,
}

/// Git information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitInfo {
    pub commit_hash: Option<String>,
    pub commit_short_hash: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub dirty: bool,
    pub commit_date: Option<String>,
}

/// Rustc information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RustcInfo {
    pub version: String,
    pub channel: String,
    pub commit_hash: Option<String>,
    pub commit_date: Option<String>,
    pub host: String,
}

/// Target information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TargetInfo {
    pub triple: String,
    pub platform: Platform,
    pub architecture: String,
    pub os: String,
    pub pointer_width: u32,
    pub endianness: String,
}

/// CLI for build configuration
#[derive(Parser, Debug)]
#[command(name = "build-config", version, about = "Build configuration manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate build metadata
    Generate {
        /// Output file path
        #[arg(short, long, default_value = "build-metadata.json")]
        output: PathBuf,
        /// Target platform
        #[arg(short, long)]
        target: Option<Platform>,
    },
    /// Show current build info
    Show {
        /// Output format (json, toml, yaml)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Validate build configuration
    Validate {
        /// Config file path
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Bump version
    Bump {
        /// Version part to bump (major, minor, patch)
        part: String,
    },
}

/// Generate build metadata at compile time
pub fn generate_build_metadata() -> BuildTimeConfig {
    BuildTimeConfig {
        version: VersionInfo {
            major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap_or(0),
            minor: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap_or(0),
            patch: env!("CARGO_PKG_VERSION_PATCH").parse().unwrap_or(0),
            pre_release: option_env!("CARGO_PKG_VERSION_PRE").map(String::from),
            build_metadata: None,
            full: env!("CARGO_PKG_VERSION").to_string(),
        },
        git: GitInfo {
            commit_hash: option_env!("VERGEN_GIT_SHA").map(String::from),
            commit_short_hash: option_env!("VERGEN_GIT_SHA_SHORT").map(String::from),
            branch: option_env!("VERGEN_GIT_BRANCH").map(String::from),
            tag: option_env!("VERGEN_GIT_DESCRIBE").map(String::from),
            dirty: option_env!("VERGEN_GIT_DIRTY")
                .map(|s| s == "true")
                .unwrap_or(false),
            commit_date: option_env!("VERGEN_GIT_COMMIT_TIMESTAMP").map(String::from),
        },
        rustc: RustcInfo {
            version: option_env!("VERGEN_RUSTC_SEMVER")
                .unwrap_or("unknown")
                .to_string(),
            channel: option_env!("VERGEN_RUSTC_CHANNEL")
                .unwrap_or("unknown")
                .to_string(),
            commit_hash: option_env!("VERGEN_RUSTC_COMMIT_HASH").map(String::from),
            commit_date: option_env!("VERGEN_RUSTC_COMMIT_DATE").map(String::from),
            host: option_env!("VERGEN_RUSTC_HOST_TRIPLE")
                .unwrap_or("unknown")
                .to_string(),
        },
        target: TargetInfo {
            triple: std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string()),
            platform: detect_platform(),
            architecture: std::env::var("CARGO_CFG_TARGET_ARCH")
                .unwrap_or_else(|_| "unknown".to_string()),
            os: std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "unknown".to_string()),
            pointer_width: std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH")
                .unwrap_or_else(|_| "64".to_string())
                .parse()
                .unwrap_or(64),
            endianness: std::env::var("CARGO_CFG_TARGET_ENDIAN").unwrap_or("little".to_string()),
        },
        features: get_enabled_features(),
        profile: std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string()),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

fn detect_platform() -> Platform {
    match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("android") => Platform::Android,
        Ok("ios") => Platform::Ios,
        Ok("linux") => Platform::Linux,
        Ok("windows") => Platform::Windows,
        Ok("macos") => Platform::Macos,
        _ => Platform::Linux,
    }
}

fn get_enabled_features() -> Vec<String> {
    // This would be populated by build script
    vec![]
}

/// Platform-specific build configuration
pub fn platform_build_config(platform: Platform) -> BuildConfig {
    match platform {
        Platform::Android => BuildConfig {
            bundle_id: "com.example.rust-crossplatform".to_string(),
            version_code: 1,
            version_name: env!("CARGO_PKG_VERSION").to_string(),
            signing_config: None,
            proguard_enabled: true,
            shrink_resources: true,
            abi_filters: vec!["arm64-v8a".to_string(), "x86_64".to_string()],
        },
        Platform::Ios => BuildConfig {
            bundle_id: "com.example.rust-crossplatform".to_string(),
            version_code: 1,
            version_name: env!("CARGO_PKG_VERSION").to_string(),
            signing_config: None,
            proguard_enabled: false,
            shrink_resources: false,
            abi_filters: vec!["arm64".to_string()],
        },
        Platform::Linux => BuildConfig {
            bundle_id: "com.example.rust-crossplatform".to_string(),
            version_code: 1,
            version_name: env!("CARGO_PKG_VERSION").to_string(),
            signing_config: None,
            proguard_enabled: false,
            shrink_resources: false,
            abi_filters: vec!["x86_64".to_string(), "aarch64".to_string()],
        },
        Platform::Windows => BuildConfig {
            bundle_id: "com.example.rust-crossplatform".to_string(),
            version_code: 1,
            version_name: env!("CARGO_PKG_VERSION").to_string(),
            signing_config: None,
            proguard_enabled: false,
            shrink_resources: false,
            abi_filters: vec!["x64".to_string(), "arm64".to_string()],
        },
        Platform::Macos => BuildConfig {
            bundle_id: "com.example.rust-crossplatform".to_string(),
            version_code: 1,
            version_name: env!("CARGO_PKG_VERSION").to_string(),
            signing_config: None,
            proguard_enabled: false,
            shrink_resources: false,
            abi_filters: vec!["x86_64".to_string(), "arm64".to_string()],
        },
        Platform::Web => BuildConfig {
            bundle_id: "com.example.rust-crossplatform".to_string(),
            version_code: 1,
            version_name: env!("CARGO_PKG_VERSION").to_string(),
            signing_config: None,
            proguard_enabled: false,
            shrink_resources: false,
            abi_filters: vec!["wasm32".to_string()],
        },
    }
}

/// Signing configuration helpers
pub mod signing {
    use super::*;

    pub fn load_signing_config(path: &PathBuf) -> Result<SigningConfig> {
        let content = std::fs::read_to_string(path)?;
        let config: SigningConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_signing_config(config: &SigningConfig, path: &PathBuf) -> Result<()> {
        let content = toml::to_string_pretty(config)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn android_signing_config(
        store_file: String,
        store_password: String,
        key_alias: String,
        key_password: String,
    ) -> SigningConfig {
        SigningConfig {
            store_file,
            store_password,
            key_alias,
            key_password,
        }
    }

    pub fn ios_signing_config(
        team_id: String,
        profile_path: String,
        certificate: String,
    ) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("team_id".to_string(), team_id);
        map.insert("profile_path".to_string(), profile_path);
        map.insert("certificate".to_string(), certificate);
        map
    }
}

/// Build targets configuration
pub mod targets {
    use super::*;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct BuildTarget {
        pub platform: Platform,
        pub rust_target: String,
        pub artifacts: Vec<String>,
        pub environment: HashMap<String, String>,
        pub dependencies: Vec<String>,
    }

    pub fn all_targets() -> Vec<BuildTarget> {
        vec![
            BuildTarget {
                platform: Platform::Android,
                rust_target: "aarch64-linux-android".to_string(),
                artifacts: vec!["libapp.so".to_string(), "app.aar".to_string()],
                environment: HashMap::new(),
                dependencies: vec!["android-ndk".to_string()],
            },
            BuildTarget {
                platform: Platform::Ios,
                rust_target: "aarch64-apple-ios".to_string(),
                artifacts: vec!["app.framework".to_string(), "app.ipa".to_string()],
                environment: HashMap::new(),
                dependencies: vec!["xcode".to_string(), "ios-sdk".to_string()],
            },
            BuildTarget {
                platform: Platform::Linux,
                rust_target: "x86_64-unknown-linux-gnu".to_string(),
                artifacts: vec!["app".to_string(), "app.AppImage".to_string()],
                environment: HashMap::new(),
                dependencies: vec!["gcc".to_string(), "glibc".to_string()],
            },
            BuildTarget {
                platform: Platform::Windows,
                rust_target: "x86_64-pc-windows-msvc".to_string(),
                artifacts: vec!["app.exe".to_string(), "app.msi".to_string()],
                environment: HashMap::new(),
                dependencies: vec!["visual-studio".to_string(), "windows-sdk".to_string()],
            },
            BuildTarget {
                platform: Platform::Macos,
                rust_target: "x86_64-apple-darwin".to_string(),
                artifacts: vec!["app.app".to_string(), "app.dmg".to_string()],
                environment: HashMap::new(),
                dependencies: vec!["xcode".to_string(), "macos-sdk".to_string()],
            },
            BuildTarget {
                platform: Platform::Web,
                rust_target: "wasm32-unknown-unknown".to_string(),
                artifacts: vec!["app.js".to_string(), "app.wasm".to_string()],
                environment: HashMap::new(),
                dependencies: vec!["wasm-pack".to_string()],
            },
        ]
    }

    pub fn target_for_platform(platform: Platform) -> Option<BuildTarget> {
        all_targets().into_iter().find(|t| t.platform == platform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let metadata = generate_build_metadata();
        assert!(!metadata.version.full.is_empty());
        assert_eq!(metadata.profile, "debug");
    }

    #[test]
    fn test_platform_build_config() {
        let android_config = platform_build_config(Platform::Android);
        assert!(android_config.proguard_enabled);
        assert!(android_config.shrink_resources);

        let ios_config = platform_build_config(Platform::Ios);
        assert!(!ios_config.proguard_enabled);
    }

    #[test]
    fn test_targets() {
        let targets = targets::all_targets();
        assert_eq!(targets.len(), 6);

        let android = targets::target_for_platform(Platform::Android);
        assert!(android.is_some());
        assert_eq!(android.unwrap().rust_target, "aarch64-linux-android");
    }
}
