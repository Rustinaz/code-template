//! Platform-specific UI implementations
//!
//! This module provides platform-specific UI adaptations and
//! native component integrations.

pub mod android;
pub mod ios;
pub mod linux;
pub mod macos;
pub mod web;
pub mod windows;

use shared::domain::Platform;

/// Platform UI trait for platform-specific implementations
pub trait PlatformUi: Send + Sync {
    fn platform(&self) -> Platform;

    /// Create platform-specific window
    fn create_window(&self, config: WindowConfig) -> shared::errors::Result<Box<dyn Window>>;

    /// Get platform-specific theme
    fn theme(&self) -> PlatformTheme;

    /// Handle platform-specific lifecycle events
    fn on_resume(&self);
    fn on_pause(&self);
    fn on_destroy(&self);

    /// Platform-specific accessibility
    fn announce_for_accessibility(&self, text: &str);
    fn set_accessibility_focus(&self, element_id: &str);
}

/// Window configuration
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub resizable: bool,
    pub fullscreen: bool,
    pub maximized: bool,
    pub transparent: bool,
    pub decorations: bool,
    pub always_on_top: bool,
    pub icon: Option<String>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "App".to_string(),
            width: 800,
            height: 600,
            min_width: Some(320),
            min_height: Some(240),
            max_width: None,
            max_height: None,
            resizable: true,
            fullscreen: false,
            maximized: false,
            transparent: false,
            decorations: true,
            always_on_top: false,
            icon: None,
        }
    }
}

/// Window trait
pub trait Window: Send + Sync {
    fn show(&self);
    fn hide(&self);
    fn close(&self);
    fn set_title(&self, title: &str);
    fn set_size(&self, width: u32, height: u32);
    fn set_min_size(&self, width: u32, height: u32);
    fn set_max_size(&self, width: u32, height: u32);
    fn set_fullscreen(&self, fullscreen: bool);
    fn set_maximized(&self, maximized: bool);
    fn position(&self) -> (i32, i32);
    fn set_position(&self, x: i32, y: i32);
    fn is_visible(&self) -> bool;
    fn is_fullscreen(&self) -> bool;
    fn is_maximized(&self) -> bool;
}

/// Platform theme
#[derive(Debug, Clone)]
pub struct PlatformTheme {
    pub name: String,
    pub colors: PlatformColors,
    pub fonts: PlatformFonts,
    pub spacing: PlatformSpacing,
    pub shapes: PlatformShapes,
    pub motion: PlatformMotion,
}

#[derive(Debug, Clone)]
pub struct PlatformColors {
    pub primary: String,
    pub on_primary: String,
    pub background: String,
    pub on_background: String,
    pub surface: String,
    pub on_surface: String,
    pub error: String,
    pub on_error: String,
}

#[derive(Debug, Clone)]
pub struct PlatformFonts {
    pub family: String,
    pub size_scale: f32,
    pub weight_regular: u16,
    pub weight_medium: u16,
    pub weight_bold: u16,
}

#[derive(Debug, Clone)]
pub struct PlatformSpacing {
    pub base: f32,
    pub scale: f32,
}

#[derive(Debug, Clone)]
pub struct PlatformShapes {
    pub corner_radius_small: f32,
    pub corner_radius_medium: f32,
    pub corner_radius_large: f32,
}

#[derive(Debug, Clone)]
pub struct PlatformMotion {
    pub duration_short: u32,
    pub duration_medium: u32,
    pub duration_long: u32,
    pub easing_standard: String,
}

/// Platform factory for creating platform-specific UI
pub struct PlatformFactory;

impl PlatformFactory {
    pub fn create(platform: Platform) -> Box<dyn PlatformUi> {
        match platform {
            Platform::Android => Box::new(android::AndroidUi::new()),
            Platform::Ios => Box::new(ios::IosUi::new()),
            Platform::Linux => Box::new(linux::LinuxUi::new()),
            Platform::Windows => Box::new(windows::WindowsUi::new()),
            Platform::Macos => Box::new(macos::MacosUi::new()),
            Platform::Web => Box::new(web::WebUi::new()),
        }
    }
}

/// Platform-specific keyboard handling
pub mod keyboard {
    use shared::domain::Platform;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum KeyCode {
        Backspace,
        Tab,
        Enter,
        Escape,
        Space,
        ArrowUp,
        ArrowDown,
        ArrowLeft,
        ArrowRight,
        Home,
        End,
        PageUp,
        PageDown,
        Delete,
        F(u8),        // F1-F12
        Digit(u8),    // 0-9
        Letter(char), // A-Z
        Control,
        Meta,
        Other(&'static str),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ModifierKey {
        Shift,
        Control,
        Alt,
        Meta, // Command on macOS, Windows key on Windows
    }

    #[derive(Debug, Clone)]
    pub struct KeyEvent {
        pub code: KeyCode,
        pub modifiers: Vec<ModifierKey>,
        pub text: Option<String>,
        pub is_repeat: bool,
    }

    /// Platform-specific key mappings
    pub fn platform_key_mapping(platform: Platform, code: KeyCode) -> KeyCode {
        // Handle platform-specific key differences
        match (platform, code) {
            (Platform::Macos, KeyCode::Control) => KeyCode::Meta,
            (Platform::Macos, KeyCode::Meta) => KeyCode::Control,
            _ => code,
        }
    }
}

/// Platform-specific input handling
pub mod input {
    use shared::domain::Platform;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PointerType {
        Mouse,
        Touch,
        Pen,
        Eraser,
        Unknown,
    }

    #[derive(Debug, Clone)]
    pub struct PointerEvent {
        pub pointer_id: u32,
        pub pointer_type: PointerType,
        pub x: f32,
        pub y: f32,
        pub pressure: f32,
        pub tilt_x: f32,
        pub tilt_y: f32,
        pub buttons: u32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PointerButton {
        None = 0,
        Primary = 1,
        Secondary = 2,
        Middle = 4,
        Back = 8,
        Forward = 16,
    }

    /// Get platform-specific pointer behavior
    pub fn platform_pointer_behavior(platform: Platform) -> PointerBehavior {
        match platform {
            Platform::Android | Platform::Ios => PointerBehavior {
                hover_support: false,
                touch_support: true,
                pen_support: true,
                mouse_support: false,
            },
            Platform::Linux | Platform::Windows | Platform::Macos => PointerBehavior {
                hover_support: true,
                touch_support: false,
                pen_support: true,
                mouse_support: true,
            },
            Platform::Web => PointerBehavior {
                hover_support: true,
                touch_support: true,
                pen_support: true,
                mouse_support: true,
            },
        }
    }

    #[derive(Debug, Clone)]
    pub struct PointerBehavior {
        pub hover_support: bool,
        pub touch_support: bool,
        pub pen_support: bool,
        pub mouse_support: bool,
    }
}

/// Platform-specific text input
pub mod text_input {
    use shared::domain::Platform;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum InputType {
        Text,
        Password,
        Email,
        Number,
        Phone,
        Url,
        Search,
        Multiline,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum InputAction {
        None,
        Go,
        Search,
        Send,
        Next,
        Done,
        Previous,
    }

    #[derive(Debug, Clone)]
    pub struct TextInputConfig {
        pub input_type: InputType,
        pub input_action: InputAction,
        pub placeholder: String,
        pub max_length: Option<usize>,
        pub auto_correct: bool,
        pub auto_capitalize: bool,
        pub spell_check: bool,
        pub enable_ime: bool,
    }

    impl Default for TextInputConfig {
        fn default() -> Self {
            Self {
                input_type: InputType::Text,
                input_action: InputAction::Done,
                placeholder: String::new(),
                max_length: None,
                auto_correct: true,
                auto_capitalize: true,
                spell_check: true,
                enable_ime: true,
            }
        }
    }

    /// Platform-specific text input behavior
    pub fn platform_text_input_behavior(platform: Platform) -> TextInputBehavior {
        match platform {
            Platform::Android => TextInputBehavior {
                show_keyboard_on_focus: true,
                hide_keyboard_on_blur: true,
                supports_autofill: true,
                supports_ime: true,
            },
            Platform::Ios => TextInputBehavior {
                show_keyboard_on_focus: true,
                hide_keyboard_on_blur: true,
                supports_autofill: true,
                supports_ime: true,
            },
            Platform::Linux | Platform::Windows | Platform::Macos => TextInputBehavior {
                show_keyboard_on_focus: false,
                hide_keyboard_on_blur: false,
                supports_autofill: true,
                supports_ime: true,
            },
            Platform::Web => TextInputBehavior {
                show_keyboard_on_focus: false,
                hide_keyboard_on_blur: false,
                supports_autofill: true,
                supports_ime: true,
            },
        }
    }

    #[derive(Debug, Clone)]
    pub struct TextInputBehavior {
        pub show_keyboard_on_focus: bool,
        pub hide_keyboard_on_blur: bool,
        pub supports_autofill: bool,
        pub supports_ime: bool,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_factory() {
        let android_ui = PlatformFactory::create(Platform::Android);
        assert_eq!(android_ui.platform(), Platform::Android);

        let ios_ui = PlatformFactory::create(Platform::Ios);
        assert_eq!(ios_ui.platform(), Platform::Ios);
    }

    #[test]
    fn test_platform_key_mapping() {
        use keyboard::*;

        assert_eq!(
            platform_key_mapping(Platform::Macos, KeyCode::Control),
            KeyCode::Meta
        );
        assert_eq!(
            platform_key_mapping(Platform::Macos, KeyCode::Meta),
            KeyCode::Control
        );
    }
}
