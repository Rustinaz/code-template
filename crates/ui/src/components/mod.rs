//! UI Components - Reusable cross-platform UI components
//!
//! This module provides a set of platform-agnostic UI components
//! that adapt to the current platform's design guidelines.
//!
//! Every component follows the same split:
//!
//! * **Pure logic** - the builder, the state struct, the sizing and radius
//!   maths. This part has no `egui` dependency at all, so it compiles (and is
//!   unit tested) with the `egui` feature turned off.
//! * **Rendering** - the `show(...)` methods that take a `&mut egui::Ui`.
//!   These are gated behind `#[cfg(feature = "egui")]` because `egui` is an
//!   *optional* dependency of the `ui` crate.
//!
//! That split is what lets one Rust codebase render one UI on desktop
//! (Linux/Windows/macOS), Android, iOS and the web: the logic is written once,
//! and only the final draw call is bound to the active backend.

pub mod app_bar;
pub mod button;
pub mod card;
pub mod dialog;
pub mod navigation_bar;
pub mod scaffold;
pub mod text_field;

use crate::UiContext;
use shared::domain::ThemeTokens;

/// Base trait for all UI components
pub trait Component {
    fn render(&self, ctx: &UiContext);
    fn theme(&self) -> &ThemeTokens;
}

/// Component variant for platform adaptation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentVariant {
    Primary,
    Secondary,
    Tertiary,
    Outlined,
    Filled,
    Tonal,
    Elevated,
    Text,
}

/// Component size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentSize {
    Small,
    Medium,
    Large,
    ExtraLarge,
}

/// Component state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentState {
    Default,
    Hovered,
    Focused,
    Pressed,
    Disabled,
    Loading,
    Error,
}

/// Accessibility role for components
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityRole {
    Button,
    Link,
    TextField,
    Checkbox,
    Radio,
    Switch,
    Slider,
    Tab,
    MenuItem,
    Dialog,
    Alert,
    Status,
    Region,
    Navigation,
    Main,
    Complementary,
    ContentInfo,
    Banner,
    Search,
    Form,
    Image,
    Heading(u8), // 1-6
    List,
    ListItem,
    Table,
    Row,
    Cell,
    Grid,
    Tree,
    TreeItem,
}

/// Base component props
#[derive(Debug, Clone, Default)]
pub struct ComponentProps {
    pub variant: Option<ComponentVariant>,
    pub size: Option<ComponentSize>,
    pub state: Option<ComponentState>,
    pub accessibility_role: Option<AccessibilityRole>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
    pub test_id: Option<String>,
    pub class: Option<String>,
    pub style: Option<String>,
}

impl ComponentProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn variant(mut self, variant: ComponentVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    pub fn size(mut self, size: ComponentSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn state(mut self, state: ComponentState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn accessibility_role(mut self, role: AccessibilityRole) -> Self {
        self.accessibility_role = Some(role);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn test_id(mut self, id: impl Into<String>) -> Self {
        self.test_id = Some(id.into());
        self
    }
}

/// What a component reports back to the caller after a frame.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ButtonResponse {
    /// The user did not interact with the button this frame.
    #[default]
    None,
    /// The button was clicked this frame.
    Clicked,
    /// Long-press / secondary action was triggered this frame.
    LongPressed,
}

impl ButtonResponse {
    /// Whether the user activated the component this frame.
    ///
    /// A [`ButtonResponse::LongPressed`] counts as an activation: callers that
    /// only care about "did the user do *something*" can ignore the distinction
    /// between a tap and a long press.
    #[must_use]
    pub fn is_clicked(&self) -> bool {
        matches!(self, Self::Clicked | Self::LongPressed)
    }

    /// Whether nothing at all happened this frame.
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// egui glue shared by every component's `show` implementation.
///
/// This module is *entirely* egui-bound, so it is gated as a whole. It resolves
/// the template's platform-independent design tokens (see
/// [`shared::domain::ColorTokens`]) into concrete [`egui::Color32`]s for the
/// currently selected theme.
#[cfg(feature = "egui")]
pub(crate) mod egui_helpers {
    use crate::UiContext;
    use egui;
    use shared::domain::{ColorRole, ColorValue, ThemePreference};

    /// Pick the light/dark/high-contrast variant of a token for `preference`.
    ///
    /// [`ThemePreference::System`] resolves to the light variant, matching
    /// `theme::ThemeManager::color_for_mode`: the actual system look is decided
    /// upstream when the egui `Style` is built.
    #[must_use]
    pub fn role_value(role: &ColorRole, preference: ThemePreference) -> &ColorValue {
        match preference {
            ThemePreference::Light | ThemePreference::System => &role.light,
            ThemePreference::Dark => &role.dark,
            ThemePreference::HighContrast => {
                role.high_contrast_light.as_ref().unwrap_or(&role.light)
            }
        }
    }

    /// Convert a token value into an egui colour.
    #[must_use]
    pub fn value(value: &ColorValue) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(
            value.rgb.0,
            value.rgb.1,
            value.rgb.2,
            (value.alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }

    /// Resolve a token role for `preference` into an egui colour.
    #[must_use]
    pub fn role(role: &ColorRole, preference: ThemePreference) -> egui::Color32 {
        value(role_value(role, preference))
    }

    /// Resolve a token role using the preference stored on `ctx`.
    ///
    /// The parameter is named `role_ref` rather than `role` because a
    /// parameter called `role` would shadow the [`role`] function it calls.
    #[must_use]
    pub fn token(ctx: &UiContext, role_ref: &ColorRole) -> egui::Color32 {
        role(role_ref, ctx.theme_preference())
    }

    /// Re-target a colour's alpha, keeping its hue.
    #[must_use]
    pub fn with_alpha(color: egui::Color32, alpha: f32) -> egui::Color32 {
        color.gamma_multiply(alpha.clamp(0.0, 1.0))
    }

    /// Lighten a fill while hovered and darken it while pressed.
    ///
    /// Kept deliberately gentle: Material surfaces change tone by ~6% on hover
    /// and ~12% on press, and going further makes the widget look like a
    /// different button than its siblings.
    #[must_use]
    pub fn state_fill(fill: egui::Color32, hovered: bool, pressed: bool) -> egui::Color32 {
        if pressed {
            fill.linear_multiply(0.88)
        } else if hovered {
            fill.lerp_to_gamma(egui::Color32::WHITE, 0.06)
        } else {
            fill
        }
    }
}

// Re-export the shared component types so `ui::components::*` gives callers a
// single import for the whole library.
pub use app_bar::{AppBar, BACK_ACTION};
pub use button::{Button, ButtonStyle, MIN_TOUCH_TARGET};
pub use card::Card;
pub use dialog::{Dialog, DialogState};
pub use navigation_bar::{NavItem, NavigationBar};
pub use scaffold::Scaffold;
pub use text_field::{TextField, TextFieldState};
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_response_defaults_to_none() {
        assert_eq!(ButtonResponse::default(), ButtonResponse::None);
        assert!(ButtonResponse::default().is_none());
    }

    #[test]
    fn button_response_is_clicked_covers_tap_and_long_press() {
        assert!(ButtonResponse::Clicked.is_clicked());
        assert!(ButtonResponse::LongPressed.is_clicked());
        assert!(!ButtonResponse::None.is_clicked());
    }

    #[test]
    fn button_response_long_press_is_not_none() {
        assert!(!ButtonResponse::LongPressed.is_none());
        assert!(!ButtonResponse::Clicked.is_none());
    }

    #[test]
    fn button_response_is_copy_and_eq() {
        let response = ButtonResponse::Clicked;
        let copied = response.clone();
        assert_eq!(response, copied);
    }

    #[test]
    fn component_props_builder_stores_every_field() {
        let props = ComponentProps::new()
            .variant(ComponentVariant::Tonal)
            .size(ComponentSize::Large)
            .state(ComponentState::Hovered)
            .accessibility_role(AccessibilityRole::Button)
            .accessibility_label("Save")
            .test_id("save-button");

        assert_eq!(props.variant, Some(ComponentVariant::Tonal));
        assert_eq!(props.size, Some(ComponentSize::Large));
        assert_eq!(props.state, Some(ComponentState::Hovered));
        assert_eq!(props.accessibility_role, Some(AccessibilityRole::Button));
        assert_eq!(props.accessibility_label.as_deref(), Some("Save"));
        assert_eq!(props.test_id.as_deref(), Some("save-button"));
    }

    #[test]
    fn component_props_default_is_empty() {
        let props = ComponentProps::new();
        assert!(props.variant.is_none());
        assert!(props.size.is_none());
        assert!(props.state.is_none());
        assert!(props.accessibility_role.is_none());
        assert!(props.accessibility_label.is_none());
        assert!(props.test_id.is_none());
    }
}
