//! Utility functions and helpers for UI
//!
//! This module provides common utilities for UI development.
//! 
//! NOTE: Individual utility modules are defined inline here.
//! External module files will be created in future versions.

// pub mod animation;
// pub mod gesture;
// pub mod focus;
// pub mod keyboard;
// pub mod clipboard;
// pub mod image;
// pub mod format;
// pub mod validation;
// pub mod accessibility;

use shared::domain::{ColorValue, ThemeTokens};
use std::collections::HashMap;

/// Animation utilities
pub mod animation {
    use std::time::Duration;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Easing {
        Linear,
        EaseIn,
        EaseOut,
        EaseInOut,
        EaseOutExpo,
        EaseOutBack,
        Custom(fn(f32) -> f32),
    }

    impl Easing {
        pub fn apply(&self, t: f32) -> f32 {
            match self {
                Easing::Linear => t,
                Easing::EaseIn => t * t,
                Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
                Easing::EaseInOut => {
                    if t < 0.5 {
                        2.0 * t * t
                    } else {
                        1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                    }
                }
                Easing::EaseOutExpo => {
                    if t == 1.0 { 1.0 } else { 1.0 - 2.0_f32.powf(-10.0 * t) }
                }
                Easing::EaseOutBack => {
                    let c1 = 1.70158;
                    let c3 = c1 + 1.0;
                    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
                }
                Easing::Custom(f) => f(t),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Animation {
        pub duration: Duration,
        pub easing: Easing,
        pub delay: Duration,
        pub iterations: u32,
        pub direction: AnimationDirection,
        pub fill_mode: FillMode,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AnimationDirection {
        Normal,
        Reverse,
        Alternate,
        AlternateReverse,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FillMode {
        None,
        Forwards,
        Backwards,
        Both,
    }

    impl Default for Animation {
        fn default() -> Self {
            Self {
                duration: Duration::from_millis(250),
                easing: Easing::EaseInOut,
                delay: Duration::ZERO,
                iterations: 1,
                direction: AnimationDirection::Normal,
                fill_mode: FillMode::Forwards,
            }
        }
    }

    /// Drives `update` from 0 to 1 over the animation's duration.
    ///
    /// A placeholder for a real animation loop. On native targets it runs on a
    /// worker thread. A browser has neither threads nor a usable `std` clock,
    /// so there it applies the animation's final state directly; a real web app
    /// would drive the loop from `requestAnimationFrame` instead.
    pub fn animate<F>(animation: Animation, update: F)
    where
        F: FnMut(f32) + Send + 'static,
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let start = web_time::Instant::now();
            let duration = animation.duration;

            std::thread::spawn(move || {
                let mut update = update;
                loop {
                    let elapsed = start.elapsed();
                    if elapsed >= duration {
                        update(1.0);
                        break;
                    }
                    let progress = elapsed.as_secs_f32() / duration.as_secs_f32();
                    let eased = animation.easing.apply(progress);
                    update(eased);
                    std::thread::sleep(Duration::from_millis(16));
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = animation;
            let mut update = update;
            update(1.0);
        }
    }

    pub fn spring(mass: f32, stiffness: f32, damping: f32) -> SpringAnimation {
        SpringAnimation { mass, stiffness, damping }
    }

    #[derive(Debug, Clone)]
    pub struct SpringAnimation {
        mass: f32,
        stiffness: f32,
        damping: f32,
    }

    impl SpringAnimation {
        pub fn simulate(&self, from: f32, to: f32, velocity: f32, dt: f32) -> (f32, f32) {
            let displacement = to - from;
            let spring_force = -self.stiffness * displacement;
            let damping_force = -self.damping * velocity;
            let acceleration = (spring_force + damping_force) / self.mass;
            let new_velocity = velocity + acceleration * dt;
            let new_position = from + new_velocity * dt;
            (new_position, new_velocity)
        }
    }
}

/// Gesture recognition utilities
pub mod gesture {
    use shared::domain::{DeviceInfo, Platform};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum GestureType {
        Tap,
        DoubleTap,
        LongPress,
        Pan,
        Pinch,
        Rotation,
        Swipe,
        Fling,
    }

    #[derive(Debug, Clone)]
    pub struct GestureConfig {
        pub tap_timeout_ms: u32,
        pub double_tap_timeout_ms: u32,
        pub long_press_timeout_ms: u32,
        pub pan_threshold: f32,
        pub pinch_threshold: f32,
        pub swipe_velocity_threshold: f32,
    }

    impl Default for GestureConfig {
        fn default() -> Self {
            Self {
                tap_timeout_ms: 180,
                double_tap_timeout_ms: 300,
                long_press_timeout_ms: 500,
                pan_threshold: 8.0,
                pinch_threshold: 0.1,
                swipe_velocity_threshold: 500.0,
            }
        }
    }

    pub fn platform_gesture_config(platform: Platform) -> GestureConfig {
        match platform {
            Platform::Android => GestureConfig {
                long_press_timeout_ms: 500,
                ..Default::default()
            },
            Platform::Ios => GestureConfig {
                long_press_timeout_ms: 500,
                ..Default::default()
            },
            _ => GestureConfig::default(),
        }
    }
}

/// Focus management utilities
pub mod focus {
    use std::collections::HashMap;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FocusDirection {
        Next,
        Previous,
        Up,
        Down,
        Left,
        Right,
    }

    #[derive(Debug, Clone)]
    pub struct FocusManager {
        elements: HashMap<String, FocusableElement>,
        current: Option<String>,
        trap_stack: Vec<String>,
    }

    #[derive(Debug, Clone)]
    pub struct FocusableElement {
        pub id: String,
        pub order: usize,
        pub focusable: bool,
        pub tab_index: i32,
    }

    impl FocusManager {
        pub fn new() -> Self {
            Self {
                elements: HashMap::new(),
                current: None,
                trap_stack: Vec::new(),
            }
        }

        pub fn register(&mut self, id: String, order: usize, focusable: bool) {
            self.elements.insert(id.clone(), FocusableElement {
                id,
                order,
                focusable,
                tab_index: if focusable { 0 } else { -1 },
            });
        }

        pub fn unregister(&mut self, id: &str) {
            self.elements.remove(id);
            if self.current.as_deref() == Some(id) {
                self.current = None;
            }
        }

        pub fn focus(&mut self, id: &str) -> bool {
            if let Some(el) = self.elements.get(id) {
                if el.focusable {
                    self.current = Some(id.to_string());
                    return true;
                }
            }
            false
        }

        pub fn move_focus(&mut self, direction: FocusDirection) -> Option<String> {
            let current_order = self.current.as_ref()
                .and_then(|id| self.elements.get(id).map(|e| e.order));
            
            if let Some(order) = current_order {
                let next = self.elements.values()
                    .filter(|e| e.focusable && match direction {
                        FocusDirection::Next => e.order > order,
                        FocusDirection::Previous => e.order < order,
                        FocusDirection::Up | FocusDirection::Down | FocusDirection::Left | FocusDirection::Right => {
                            // Would need spatial layout info
                            e.order != order
                        }
                    })
                    .min_by_key(|e| match direction {
                        FocusDirection::Next | FocusDirection::Down | FocusDirection::Right => e.order,
                        FocusDirection::Previous | FocusDirection::Up | FocusDirection::Left => std::usize::MAX - e.order,
                    });

                if let Some(next) = next {
                    self.current = Some(next.id.clone());
                    return Some(next.id.clone());
                }
            }
            None
        }

        pub fn current(&self) -> Option<&str> {
            self.current.as_deref()
        }

        pub fn trap_focus(&mut self, container_id: &str) {
            self.trap_stack.push(container_id.to_string());
        }

        pub fn release_focus_trap(&mut self) {
            self.trap_stack.pop();
        }
    }

    impl Default for FocusManager {
        fn default() -> Self {
            Self::new()
        }
    }
}

/// Keyboard utilities
pub mod keyboard {
    use shared::domain::Platform;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Key {
        Backspace, Tab, Enter, Escape, Space,
        ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
        Home, End, PageUp, PageDown, Delete,
        F(u8), Digit(u8), Letter(char),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Modifier {
        Shift, Control, Alt, Meta,
    }

    #[derive(Debug, Clone)]
    pub struct KeyEvent {
        pub key: Key,
        pub modifiers: Vec<Modifier>,
        pub text: Option<String>,
    }

    #[derive(Debug, Clone)]
    pub struct Shortcut {
        pub key: Key,
        pub modifiers: Vec<Modifier>,
        pub action: String,
    }

    impl Shortcut {
        pub fn matches(&self, event: &KeyEvent) -> bool {
            self.key == event.key && self.modifiers == event.modifiers
        }
    }

    pub fn platform_shortcuts(platform: Platform) -> Vec<Shortcut> {
        match platform {
            Platform::Macos => vec![
                Shortcut { key: Key::Letter('c'), modifiers: vec![Modifier::Meta], action: "copy".to_string() },
                Shortcut { key: Key::Letter('v'), modifiers: vec![Modifier::Meta], action: "paste".to_string() },
                Shortcut { key: Key::Letter('x'), modifiers: vec![Modifier::Meta], action: "cut".to_string() },
                Shortcut { key: Key::Letter('z'), modifiers: vec![Modifier::Meta], action: "undo".to_string() },
                Shortcut { key: Key::Letter('a'), modifiers: vec![Modifier::Meta], action: "select_all".to_string() },
            ],
            _ => vec![
                Shortcut { key: Key::Letter('c'), modifiers: vec![Modifier::Control], action: "copy".to_string() },
                Shortcut { key: Key::Letter('v'), modifiers: vec![Modifier::Control], action: "paste".to_string() },
                Shortcut { key: Key::Letter('x'), modifiers: vec![Modifier::Control], action: "cut".to_string() },
                Shortcut { key: Key::Letter('z'), modifiers: vec![Modifier::Control], action: "undo".to_string() },
                Shortcut { key: Key::Letter('a'), modifiers: vec![Modifier::Control], action: "select_all".to_string() },
            ],
        }
    }
}

/// Clipboard utilities
pub mod clipboard {
    use shared::domain::Platform;

    pub async fn read_text() -> Option<String> {
        #[cfg(target_arch = "wasm32")]
        {
            // Use navigator.clipboard.readText()
            None
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Use platform clipboard (arboard, etc.)
            None
        }
    }

    pub async fn write_text(text: &str) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            // Use navigator.clipboard.writeText()
            false
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Use platform clipboard
            false
        }
    }

    pub fn platform_clipboard_behavior(platform: Platform) -> ClipboardBehavior {
        match platform {
            Platform::Android | Platform::Ios => ClipboardBehavior {
                auto_grant_permission: true,
                supports_rich_text: false,
                supports_images: false,
            },
            Platform::Linux | Platform::Windows | Platform::Macos => ClipboardBehavior {
                auto_grant_permission: false,
                supports_rich_text: true,
                supports_images: true,
            },
            Platform::Web => ClipboardBehavior {
                auto_grant_permission: false,
                supports_rich_text: true,
                supports_images: true,
            },
        }
    }

    #[derive(Debug, Clone)]
    pub struct ClipboardBehavior {
        pub auto_grant_permission: bool,
        pub supports_rich_text: bool,
        pub supports_images: bool,
    }
}

/// Image utilities
pub mod image {
    use std::path::Path;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ImageFormat {
        Png, Jpeg, Webp, Gif, Bmp, Ico, Svg,
    }

    #[derive(Debug, Clone)]
    pub struct ImageInfo {
        pub width: u32,
        pub height: u32,
        pub format: ImageFormat,
        pub size_bytes: u64,
        pub has_alpha: bool,
    }

    pub fn load_image_info(path: &Path) -> Option<ImageInfo> {
        // Would use image crate
        None
    }

    pub fn resize_image(data: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
        // Would use image crate
        None
    }

    pub fn optimize_image(data: &[u8], quality: u8) -> Option<Vec<u8>> {
        // Would use image crate
        None
    }

    pub fn generate_placeholder(width: u32, height: u32, color: &str) -> Vec<u8> {
        // Generate SVG placeholder
        format!(
            r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg"><rect width="100%" height="100%" fill="{}"/></svg>"#,
            width, height, color
        ).into_bytes()
    }
}

/// Formatting utilities
pub mod format {
    use chrono::{DateTime, Utc, Duration};
    use shared::domain::Timestamp;

    pub fn format_number(num: f64, locale: &str) -> String {
        // Would use ICU or similar
        num.to_string()
    }

    pub fn format_currency(amount: f64, currency: &str, locale: &str) -> String {
        format!("{}{:.2}", currency, amount)
    }

    pub fn format_date(date: Timestamp, locale: &str, format: DateFormat) -> String {
        let dt: DateTime<Utc> = date.into();
        match format {
            DateFormat::Short => dt.format("%m/%d/%Y").to_string(),
            DateFormat::Medium => dt.format("%b %d, %Y").to_string(),
            DateFormat::Long => dt.format("%B %d, %Y").to_string(),
            DateFormat::Full => dt.format("%A, %B %d, %Y").to_string(),
            DateFormat::Relative => format_relative(dt),
        }
    }

    pub fn format_time(time: Timestamp, locale: &str, use_24h: bool) -> String {
        let dt: DateTime<Utc> = time.into();
        if use_24h {
            dt.format("%H:%M").to_string()
        } else {
            dt.format("%I:%M %p").to_string()
        }
    }

    pub fn format_relative(date: DateTime<Utc>) -> String {
        let now = Utc::now();
        let diff = now.signed_duration_since(date);
        
        if diff < Duration::seconds(60) {
            "just now".to_string()
        } else if diff < Duration::minutes(60) {
            format!("{}m ago", diff.num_minutes())
        } else if diff < Duration::hours(24) {
            format!("{}h ago", diff.num_hours())
        } else if diff < Duration::days(7) {
            format!("{}d ago", diff.num_days())
        } else if diff < Duration::days(30) {
            format!("{}w ago", diff.num_weeks())
        } else if diff < Duration::days(365) {
            format!("{}mo ago", diff.num_days() / 30)
        } else {
            format!("{}y ago", diff.num_days() / 365)
        }
    }

    pub fn format_file_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit = 0;
        while size >= 1024.0 && unit < UNITS.len() - 1 {
            size /= 1024.0;
            unit += 1;
        }
        format!("{:.1} {}", size, UNITS[unit])
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DateFormat {
        Short, Medium, Long, Full, Relative,
    }
}

/// Validation utilities
pub mod validation {
    use regex::Regex;

    pub fn validate_email(email: &str) -> bool {
        let email_regex = Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
        email_regex.is_match(email)
    }

    pub fn validate_url(url: &str) -> bool {
        url::Url::parse(url).is_ok()
    }

    pub fn validate_phone(phone: &str) -> bool {
        let phone_regex = Regex::new(r"^\+?[1-9]\d{1,14}$").unwrap();
        phone_regex.is_match(&phone.replace([' ', '-', '(', ')'], ""))
    }

    pub fn validate_password(password: &str, min_length: usize) -> Vec<String> {
        let mut errors = Vec::new();
        if password.len() < min_length {
            errors.push(format!("Password must be at least {} characters", min_length));
        }
        if !password.chars().any(|c| c.is_uppercase()) {
            errors.push("Password must contain an uppercase letter".to_string());
        }
        if !password.chars().any(|c| c.is_lowercase()) {
            errors.push("Password must contain a lowercase letter".to_string());
        }
        if !password.chars().any(|c| c.is_numeric()) {
            errors.push("Password must contain a number".to_string());
        }
        if !password.chars().any(|c| !c.is_alphanumeric()) {
            errors.push("Password must contain a special character".to_string());
        }
        errors
    }

    pub fn sanitize_html(html: &str) -> String {
        // Would use ammonia or similar
        html.to_string()
    }

    pub fn validate_required(value: &str, field_name: &str) -> Option<String> {
        if value.trim().is_empty() {
            Some(format!("{} is required", field_name))
        } else {
            None
        }
    }

    pub fn validate_length(value: &str, min: usize, max: usize, field_name: &str) -> Option<String> {
        let len = value.chars().count();
        if len < min {
            Some(format!("{} must be at least {} characters", field_name, min))
        } else if len > max {
            Some(format!("{} must be at most {} characters", field_name, max))
        } else {
            None
        }
    }
}

/// Accessibility utilities
pub mod accessibility {
    use shared::domain::AccessibilitySettings;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AnnouncementPriority {
        Polite,
        Assertive,
    }

    pub fn announce(text: &str, priority: AnnouncementPriority) {
        // Platform-specific announcement
    }

    pub fn check_accessibility_settings() -> AccessibilitySettings {
        // Would check system accessibility settings
        AccessibilitySettings::default()
    }

    pub fn prefers_reduced_motion() -> bool {
        // Check system preference
        false
    }

    pub fn prefers_high_contrast() -> bool {
        // Check system preference
        false
    }

    pub fn screen_reader_active() -> bool {
        // Check if screen reader is running
        false
    }

    pub fn apply_accessibility_settings(settings: &AccessibilitySettings) {
        // Apply settings to UI
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_easing() {
        use animation::Easing;
        assert_eq!(Easing::Linear.apply(0.5), 0.5);
        assert!(Easing::EaseIn.apply(0.5) < 0.5);
        assert!(Easing::EaseOut.apply(0.5) > 0.5);
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format::format_file_size(500), "500.0 B");
        assert_eq!(format::format_file_size(1500), "1.5 KB");
        assert_eq!(format::format_file_size(1500000), "1.4 MB");
    }

    #[test]
    fn test_validate_email() {
        assert!(validation::validate_email("test@example.com"));
        assert!(!validation::validate_email("invalid"));
        assert!(!validation::validate_email("@example.com"));
    }

    #[test]
    fn test_validate_password() {
        let errors = validation::validate_password("weak", 8);
        assert!(!errors.is_empty());
        
        let errors = validation::validate_password("StrongPass123!", 8);
        assert!(errors.is_empty());
    }
}