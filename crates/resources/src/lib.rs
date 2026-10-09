//! Resources crate - Resource management for cross-platform applications
//!
//! This crate handles loading and managing application resources including:
//! - Localized strings (like Android strings.xml)
//! - Colors (like Android colors.xml)
//! - Themes (like Android themes.xml)
//! - Assets (images, fonts, raw files)
//! - Platform-specific resource configurations
//!
//! NOTE: For the lightweight version, module files are not created.
//! The main functionality is defined inline here.

// pub mod strings;
// pub mod colors;
// pub mod themes;
// pub mod assets;
// pub mod fonts;
// pub mod raw;
// pub mod loader;

use directories::ProjectDirs;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use shared::config::AppConfig;
use shared::domain::{ColorValue, ThemeTokens};
use shared::errors::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};

/// Global resource manager instance
static RESOURCE_MANAGER: Lazy<RwLock<Option<Arc<ResourceManager>>>> =
    Lazy::new(|| RwLock::new(None));

/// Resource manager for loading and caching resources
pub struct ResourceManager {
    config: Arc<AppConfig>,
    strings: RwLock<HashMap<String, StringMap>>,
    colors: RwLock<HashMap<String, ColorMap>>,
    themes: RwLock<HashMap<String, ThemeTokens>>,
    assets: RwLock<HashMap<String, AssetInfo>>,
    fonts: RwLock<HashMap<String, FontInfo>>,
    raw: RwLock<HashMap<String, Vec<u8>>>,
    resource_dirs: Vec<PathBuf>,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new(config: Arc<AppConfig>) -> Self {
        let mut dirs = Vec::new();

        // Add platform-specific resource directories
        if let Some(proj_dirs) = ProjectDirs::from("com", "example", "rust-crossplatform-template")
        {
            dirs.push(proj_dirs.data_dir().join("resources"));
        }

        // Add current directory resources for development
        dirs.push(PathBuf::from("resources"));
        dirs.push(PathBuf::from("assets"));

        Self {
            config,
            strings: RwLock::new(HashMap::new()),
            colors: RwLock::new(HashMap::new()),
            themes: RwLock::new(HashMap::new()),
            assets: RwLock::new(HashMap::new()),
            fonts: RwLock::new(HashMap::new()),
            raw: RwLock::new(HashMap::new()),
            resource_dirs: dirs,
        }
    }

    /// Initialize the global resource manager
    pub fn init_global(config: Arc<AppConfig>) -> Arc<Self> {
        let manager = Arc::new(Self::new(config));
        manager.load_all().expect("Failed to load resources");
        *RESOURCE_MANAGER.write() = Some(manager.clone());
        manager
    }

    /// Get the global resource manager
    pub fn global() -> Option<Arc<Self>> {
        RESOURCE_MANAGER.read().clone()
    }

    /// Get global or initialize
    pub fn global_or_init(config: Arc<AppConfig>) -> Arc<Self> {
        Self::global().unwrap_or_else(|| Self::init_global(config))
    }

    /// Load all resources from disk
    pub fn load_all(&self) -> Result<()> {
        self.load_strings()?;
        self.load_colors()?;
        self.load_themes()?;
        self.load_assets()?;
        self.load_fonts()?;
        info!("All resources loaded successfully");
        Ok(())
    }

    /// Load localized strings
    fn load_strings(&self) -> Result<()> {
        let locales = vec![
            "en", "es", "fr", "de", "ja", "ko", "zh", "ar", "hi", "pt", "ru",
        ];

        for locale in locales {
            for dir in &self.resource_dirs {
                let path = dir.join("strings").join(format!("strings_{}.json", locale));
                if path.exists() {
                    let content = std::fs::read_to_string(&path)?;
                    let map: HashMap<String, String> = serde_json::from_str(&content)?;
                    self.strings
                        .write()
                        .insert(locale.to_string(), StringMap { values: map });
                    debug!(
                        "Loaded strings for locale: {} from {}",
                        locale,
                        path.display()
                    );
                    break;
                }
            }
        }

        // Ensure default locale exists
        if !self.strings.read().contains_key("en") {
            self.strings
                .write()
                .insert("en".to_string(), StringMap::default());
        }

        Ok(())
    }

    /// Load color definitions
    fn load_colors(&self) -> Result<()> {
        for dir in &self.resource_dirs {
            let path = dir.join("colors").join("colors.json");
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let map: HashMap<String, String> = serde_json::from_str(&content)?;
                let mut color_map = ColorMap::new();
                for (name, value) in map {
                    if let Ok(color) = Self::parse_color(&value) {
                        color_map.colors.insert(name, color);
                    }
                }
                self.colors.write().insert("default".to_string(), color_map);
                debug!("Loaded colors from {}", path.display());
                break;
            }
        }
        Ok(())
    }

    /// Load theme definitions
    fn load_themes(&self) -> Result<()> {
        for dir in &self.resource_dirs {
            let path = dir.join("themes").join("themes.ron");
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let themes: HashMap<String, ThemeTokens> = ron::from_str(&content)?;
                for (name, theme) in themes {
                    self.themes.write().insert(name, theme);
                }
                debug!("Loaded themes from {}", path.display());
                break;
            }
        }
        Ok(())
    }

    /// Load asset metadata
    fn load_assets(&self) -> Result<()> {
        for dir in &self.resource_dirs {
            let assets_dir = dir.join("assets");
            if assets_dir.exists() {
                for entry in std::fs::read_dir(&assets_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let info = AssetInfo {
                            name: name.clone(),
                            path: path.clone(),
                            size: entry.metadata()?.len(),
                            mime_type: mime_guess::from_path(&path)
                                .first_or_octet_stream()
                                .to_string(),
                        };
                        self.assets.write().insert(name, info);
                    }
                }
                debug!("Loaded assets from {}", assets_dir.display());
            }
        }
        Ok(())
    }

    /// Load font metadata
    fn load_fonts(&self) -> Result<()> {
        for dir in &self.resource_dirs {
            let fonts_dir = dir.join("fonts");
            if fonts_dir.exists() {
                for entry in std::fs::read_dir(&fonts_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let info = FontInfo {
                            name: name.clone(),
                            path: path.clone(),
                            size: entry.metadata()?.len(),
                            family: name.clone(),
                        };
                        self.fonts.write().insert(name, info);
                    }
                }
                debug!("Loaded fonts from {}", fonts_dir.display());
            }
        }
        Ok(())
    }

    /// Parse color string to `ColorValue`.
    ///
    /// Accepts `#RGB`, `#RRGGBB` and `#RRGGBBAA`. The three-digit form is the
    /// usual shorthand where each digit is doubled (`#f0a` is `#ff00aa`), and
    /// the eight-digit form carries alpha in its last byte instead of
    /// discarding it.
    fn parse_color(s: &str) -> Result<ColorValue> {
        let s = s.trim();
        let hex = s.strip_prefix('#').ok_or_else(|| {
            shared::errors::AppError::Internal("Color must be hex format".to_string())
        })?;

        let byte = |range: std::ops::Range<usize>| -> Result<u8> {
            u8::from_str_radix(&hex[range], 16).map_err(|error| {
                shared::errors::AppError::Internal(format!("Invalid hex color {s:?}: {error}"))
            })
        };

        let (rgb, alpha) = match hex.len() {
            3 => ((byte(0..1)? * 17, byte(1..2)? * 17, byte(2..3)? * 17), 1.0),
            6 => ((byte(0..2)?, byte(2..4)?, byte(4..6)?), 1.0),
            8 => (
                (byte(0..2)?, byte(2..4)?, byte(4..6)?),
                byte(6..8)? as f32 / 255.0,
            ),
            other => {
                return Err(shared::errors::AppError::Internal(format!(
                    "Invalid hex color {s:?}: expected 3, 6 or 8 digits, got {other}"
                )));
            }
        };

        Ok(ColorValue {
            hex: s.to_string(),
            rgb,
            alpha,
        })
    }

    /// Get a localized string
    pub fn get_string(&self, key: &str, locale: Option<&str>) -> Option<String> {
        let locale = locale.unwrap_or(&self.config.ui.language);
        let fallback = "en";

        let strings = self.strings.read();
        strings
            .get(locale)
            .or_else(|| strings.get(fallback))
            .and_then(|map| map.values.get(key).cloned())
    }

    /// Get a string with formatting arguments
    pub fn get_string_formatted(
        &self,
        key: &str,
        args: &[&str],
        locale: Option<&str>,
    ) -> Option<String> {
        let template = self.get_string(key, locale)?;
        let mut result = template;
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("{{{}}}", i), arg);
        }
        Some(result)
    }

    /// Get a color by name
    pub fn get_color(&self, name: &str) -> Option<ColorValue> {
        self.colors
            .read()
            .get("default")
            .and_then(|map| map.colors.get(name).cloned())
    }

    /// Get a theme by name
    pub fn get_theme(&self, name: &str) -> Option<ThemeTokens> {
        self.themes.read().get(name).cloned()
    }

    /// Get the current theme based on config
    pub fn current_theme(&self) -> ThemeTokens {
        let theme_name = match self.config.ui.theme.mode {
            shared::config::ThemeMode::Dark => "dark",
            shared::config::ThemeMode::HighContrast => "high_contrast",
            _ => "light",
        };
        self.get_theme(theme_name)
            .unwrap_or_else(|| self.config.theme_tokens())
    }

    /// Get asset info
    pub fn get_asset(&self, name: &str) -> Option<AssetInfo> {
        self.assets.read().get(name).cloned()
    }

    /// Get asset data
    pub fn get_asset_data(&self, name: &str) -> Option<Vec<u8>> {
        let info = self.get_asset(name)?;
        std::fs::read(&info.path).ok()
    }

    /// Get font info
    pub fn get_font(&self, name: &str) -> Option<FontInfo> {
        self.fonts.read().get(name).cloned()
    }

    /// Get raw resource data
    pub fn get_raw(&self, name: &str) -> Option<Vec<u8>> {
        self.raw.read().get(name).cloned()
    }

    /// Add a resource directory
    pub fn add_resource_dir(&mut self, path: PathBuf) {
        self.resource_dirs.push(path);
    }

    /// Reload all resources (for hot reload)
    #[cfg(feature = "hot_reload")]
    pub fn reload(&self) -> Result<()> {
        self.load_all()
    }
}

/// String map for a locale
#[derive(Debug, Clone, Default)]
pub struct StringMap {
    pub values: HashMap<String, String>,
}

impl StringMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.values.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.values.get(key)
    }
}

/// Color map
#[derive(Debug, Clone)]
pub struct ColorMap {
    pub colors: HashMap<String, ColorValue>,
}

impl ColorMap {
    pub fn new() -> Self {
        Self {
            colors: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, color: ColorValue) {
        self.colors.insert(name, color);
    }

    pub fn get(&self, name: &str) -> Option<&ColorValue> {
        self.colors.get(name)
    }
}

impl Default for ColorMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Asset information
#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub mime_type: String,
}

/// Font information
#[derive(Debug, Clone)]
pub struct FontInfo {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub family: String,
}

/// Resource builder for declarative resource definition
pub struct ResourceBuilder {
    strings: HashMap<String, HashMap<String, String>>,
    colors: HashMap<String, ColorValue>,
    themes: HashMap<String, ThemeTokens>,
}

impl ResourceBuilder {
    pub fn new() -> Self {
        Self {
            strings: HashMap::new(),
            colors: HashMap::new(),
            themes: HashMap::new(),
        }
    }

    pub fn string(mut self, locale: &str, key: &str, value: &str) -> Self {
        self.strings
            .entry(locale.to_string())
            .or_default()
            .insert(key.to_string(), value.to_string());
        self
    }

    pub fn color(mut self, name: &str, color: ColorValue) -> Self {
        self.colors.insert(name.to_string(), color);
        self
    }

    pub fn theme(mut self, name: &str, theme: ThemeTokens) -> Self {
        self.themes.insert(name.to_string(), theme);
        self
    }

    pub fn build_strings(self) -> HashMap<String, StringMap> {
        self.strings
            .into_iter()
            .map(|(locale, values)| (locale, StringMap { values }))
            .collect()
    }

    pub fn build_colors(self) -> ColorMap {
        ColorMap {
            colors: self.colors,
        }
    }

    pub fn build_themes(self) -> HashMap<String, ThemeTokens> {
        self.themes
    }
}

impl Default for ResourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience macros for resource access
#[macro_export]
macro_rules! string {
    ($key:expr) => {
        $crate::resources::ResourceManager::global()
            .and_then(|rm| rm.get_string($key, None))
            .unwrap_or_else(|| $key.to_string())
    };
    ($key:expr, $locale:expr) => {
        $crate::resources::ResourceManager::global()
            .and_then(|rm| rm.get_string($key, Some($locale)))
            .unwrap_or_else(|| $key.to_string())
    };
    ($key:expr, $($args:expr),*) => {
        $crate::resources::ResourceManager::global()
            .and_then(|rm| rm.get_string_formatted($key, &[$($args),*], None))
            .unwrap_or_else(|| $key.to_string())
    };
}

#[macro_export]
macro_rules! color {
    ($name:expr) => {
        $crate::resources::ResourceManager::global()
            .and_then(|rm| rm.get_color($name))
            .unwrap_or_else(|| $crate::shared::domain::ColorValue {
                hex: "#000000".to_string(),
                rgb: (0, 0, 0),
                alpha: 1.0,
            })
    };
}

#[macro_export]
macro_rules! theme {
    ($name:expr) => {
        $crate::resources::ResourceManager::global().and_then(|rm| rm.get_theme($name))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_builder() {
        let builder = ResourceBuilder::new()
            .string("en", "hello", "Hello")
            .string("es", "hello", "Hola")
            .color(
                "primary",
                shared::domain::ColorValue {
                    hex: "#FF0000".to_string(),
                    rgb: (255, 0, 0),
                    alpha: 1.0,
                },
            );

        let strings = builder.build_strings();
        assert_eq!(
            strings.get("en").unwrap().get("hello"),
            Some(&"Hello".to_string())
        );
    }

    #[test]
    fn test_parse_color() {
        let color = ResourceManager::parse_color("#FF0000").unwrap();
        assert_eq!(color.hex, "#FF0000");
        assert_eq!(color.rgb, (255, 0, 0));
        assert_eq!(color.alpha, 1.0);

        let color = ResourceManager::parse_color("#F00").unwrap();
        assert_eq!(color.rgb, (255, 0, 0));
        assert_eq!(color.alpha, 1.0);
    }

    #[test]
    fn test_parse_color_reads_alpha_byte() {
        // The eight-digit form is #RRGGBBAA; the alpha used to be dropped.
        let color = ResourceManager::parse_color("#FF000080").unwrap();
        assert_eq!(color.rgb, (255, 0, 0));
        assert!((color.alpha - 128.0 / 255.0).abs() < 1e-6);

        let opaque = ResourceManager::parse_color("#FF0000FF").unwrap();
        assert_eq!(opaque.alpha, 1.0);
        let clear = ResourceManager::parse_color("#FF000000").unwrap();
        assert_eq!(clear.alpha, 0.0);
    }

    #[test]
    fn test_parse_color_rejects_bad_input() {
        // No leading '#'.
        assert!(ResourceManager::parse_color("FF0000").is_err());
        // Right prefix, wrong length.
        assert!(ResourceManager::parse_color("#FF00").is_err());
        assert!(ResourceManager::parse_color("#FF000").is_err());
        assert!(ResourceManager::parse_color("#FF0000FFFFF").is_err());
        // Right length, not hex.
        assert!(ResourceManager::parse_color("#GGGGGG").is_err());
        // Note `#FFF` is *not* an error: it is the three-digit shorthand for
        // `#ffffff`.
        assert_eq!(
            ResourceManager::parse_color("#FFF").unwrap().rgb,
            (255, 255, 255)
        );
    }
}
