//! Button component.
//!
//! [`Button`] owns the *logic* of a platform-adaptive button (how tall it is on
//! each OS, which token colours it uses, what a screen reader should say) and,
//! when the `egui` feature is on, draws itself into an [`egui::Ui`].
//!
//! ```
//! use ui::components::{button::{Button, ButtonStyle}, ComponentSize};
//! use shared::domain::Platform;
//!
//! let button = Button::new("Save").style(ButtonStyle::Filled).size(ComponentSize::Large);
//! // Android's 48dp touch target, grown by the "large" scale factor.
//! assert_eq!(Button::height_for(Platform::Android, ComponentSize::Large), 60.0);
//! ```

use shared::domain::{Platform, TypographyTokens};

use super::{ButtonResponse, ComponentSize};

/// Smallest touch target (in logical pixels) a button may shrink to.
///
/// Android's Material accessibility guidelines put the floor at 48dp; iOS uses
/// 44pt. We use the larger of the two so one number is safe everywhere.
pub const MIN_TOUCH_TARGET: f32 = 48.0;

/// Alpha applied to a disabled button's colours.
const DISABLED_ALPHA: f32 = 0.38;

/// Visual emphasis of a [`Button`], mirroring Material Design 3's button roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonStyle {
    /// Highest emphasis: solid primary fill. The default.
    #[default]
    Filled,
    /// Medium emphasis: filled with the secondary container colour.
    Tonal,
    /// Low emphasis: transparent fill with a 1px outline.
    Outlined,
    /// Lowest emphasis: label only, no fill and no outline.
    Text,
    /// Filled with a drop shadow, for surfaces that float above the page.
    Elevated,
    /// Destructive action, wired to the `error` colour role.
    Danger,
}

impl ButtonStyle {
    /// Whether the style paints a background fill.
    #[must_use]
    pub const fn has_fill(self) -> bool {
        matches!(
            self,
            Self::Filled | Self::Tonal | Self::Elevated | Self::Danger
        )
    }

    /// Whether the style paints a 1px outline around the fill.
    #[must_use]
    pub const fn has_stroke(self) -> bool {
        matches!(self, Self::Outlined | Self::Elevated)
    }

    /// Whether the style casts a drop shadow.
    #[must_use]
    pub const fn has_shadow(self) -> bool {
        matches!(self, Self::Elevated)
    }
}

/// A platform-adaptive push button.
///
/// Build one with the builder methods, then hand it to
/// [`Button::show`] once per frame. [`Button::show`] returns a
/// [`ButtonResponse`] describing what the user did.
#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    label: String,
    style: ButtonStyle,
    size: ComponentSize,
    enabled: bool,
    icon: Option<char>,
    test_id: Option<String>,
    accessibility_label: Option<String>,
    full_width: bool,
}

impl Default for Button {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Button {
    /// Create a button showing `label`, enabled, [`ButtonStyle::Filled`], medium.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            style: ButtonStyle::default(),
            size: ComponentSize::Medium,
            enabled: true,
            icon: None,
            test_id: None,
            accessibility_label: None,
            full_width: false,
        }
    }

    /// Set the visual emphasis.
    #[must_use]
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the size, which scales both height and font size.
    #[must_use]
    pub fn size(mut self, size: ComponentSize) -> Self {
        self.size = size;
        self
    }

    /// Enable or disable the button. A disabled button never reports a click.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Prefix the label with an icon glyph (egui's default font ships glyphs
    /// such as `'⚙'` and `'📁'`).
    #[must_use]
    pub fn icon(mut self, icon: char) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Give the button a stable id so UI tests can find it again.
    #[must_use]
    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    /// Override what a screen reader announces. Defaults to the visible text.
    #[must_use]
    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Stretch the button to fill the available width.
    #[must_use]
    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    /// The scale factor applied to the platform's base button height.
    ///
    /// `Small` 0.75, `Medium` 1.0, `Large` 1.25, `ExtraLarge` 1.5.
    #[must_use]
    pub fn size_scale(size: ComponentSize) -> f32 {
        match size {
            ComponentSize::Small => 0.75,
            ComponentSize::Medium => 1.0,
            ComponentSize::Large => 1.25,
            ComponentSize::ExtraLarge => 1.5,
        }
    }

    /// Height in logical pixels for `platform` at `size`.
    ///
    /// This is a pure function: it combines
    /// [`platform_adaptations::button_height`](crate::platform_adaptations::button_height)
    /// with [`Button::size_scale`] and needs neither a widget nor an egui
    /// context.
    #[must_use]
    pub fn height_for(platform: Platform, size: ComponentSize) -> f32 {
        crate::platform_adaptations::button_height(platform) * Self::size_scale(size)
    }

    /// Height for a device, taking touch ergonomics into account.
    ///
    /// When `is_mobile` is set the result is clamped up to
    /// [`MIN_TOUCH_TARGET`], because a finger is far less precise than a mouse.
    #[must_use]
    pub fn height_for_form_factor(platform: Platform, size: ComponentSize, is_mobile: bool) -> f32 {
        let height = Self::height_for(platform, size);
        if is_mobile {
            height.max(MIN_TOUCH_TARGET)
        } else {
            height
        }
    }

    /// Height for the device described by `ctx`.
    ///
    /// Delegates to [`Button::height_for_form_factor`] using
    /// [`UiContext::is_mobile`](crate::UiContext::is_mobile).
    #[must_use]
    pub fn height_for_context(&self, ctx: &crate::UiContext) -> f32 {
        Self::height_for_form_factor(ctx.platform(), self.size, ctx.is_mobile())
    }

    /// Font size for `size`, taken from the shared typography tokens.
    #[must_use]
    pub fn font_size_for(tokens: &TypographyTokens, size: ComponentSize) -> f32 {
        match size {
            ComponentSize::Small => tokens.typography_label_small(),
            ComponentSize::Medium => tokens.typography_label_medium(),
            ComponentSize::Large => tokens.typography_label_large(),
            ComponentSize::ExtraLarge => tokens.typography_title_medium(),
        }
    }

    /// The text actually painted: the icon (if any) followed by the label.
    #[must_use]
    pub fn display_text(&self) -> String {
        match self.icon {
            Some(icon) => format!("{icon} {}", self.label),
            None => self.label.clone(),
        }
    }

    /// What a screen reader should announce: the explicit accessibility label
    /// when there is one, otherwise the visible text.
    #[must_use]
    pub fn accessible_label(&self) -> String {
        self.accessibility_label
            .clone()
            .unwrap_or_else(|| self.display_text())
    }

    /// The visible label, without the icon.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The configured style.
    #[must_use]
    pub fn style_(&self) -> ButtonStyle {
        self.style
    }

    /// The configured size.
    #[must_use]
    pub fn size_(&self) -> ComponentSize {
        self.size
    }

    /// Whether the button responds to input.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// The icon glyph, if one was set.
    #[must_use]
    pub fn icon_(&self) -> Option<char> {
        self.icon
    }

    /// The stable test id, if one was set.
    #[must_use]
    pub fn test_id_(&self) -> Option<&str> {
        self.test_id.as_deref()
    }

    /// The explicit accessibility label, if one was set.
    ///
    /// Distinct from [`Button::accessible_label`], which falls back to the
    /// visible text; this returns only what was configured.
    #[must_use]
    pub fn accessibility_label_(&self) -> Option<&str> {
        self.accessibility_label.as_deref()
    }

    /// Whether the button stretches to the available width.
    #[must_use]
    pub fn is_full_width(&self) -> bool {
        self.full_width
    }

    /// Stable egui id for this button.
    ///
    /// Prefers the explicit `test_id` so UI tests stay stable even when the
    /// label is localised.
    #[cfg(feature = "egui")]
    #[must_use]
    pub fn egui_id(&self) -> egui::Id {
        match self.test_id.as_deref() {
            Some(test_id) => egui::Id::new(("ui_components::Button", test_id)),
            None => egui::Id::new(("ui_components::Button", self.label.clone())),
        }
    }

    /// `(fill, stroke, text)` colours for the current style and theme.
    #[cfg(feature = "egui")]
    #[must_use]
    fn palette(&self, ctx: &crate::UiContext) -> (egui::Color32, egui::Color32, egui::Color32) {
        use super::egui_helpers as eh;
        let colors = ctx.color_tokens();
        match self.style {
            ButtonStyle::Filled => (
                eh::token(ctx, &colors.primary),
                eh::token(ctx, &colors.primary),
                eh::token(ctx, &colors.on_primary),
            ),
            ButtonStyle::Tonal => (
                eh::token(ctx, &colors.secondary_container),
                eh::token(ctx, &colors.secondary_container),
                eh::token(ctx, &colors.on_secondary_container),
            ),
            ButtonStyle::Outlined => (
                egui::Color32::TRANSPARENT,
                eh::token(ctx, &colors.outline),
                eh::token(ctx, &colors.primary),
            ),
            ButtonStyle::Text => (
                egui::Color32::TRANSPARENT,
                egui::Color32::TRANSPARENT,
                eh::token(ctx, &colors.primary),
            ),
            ButtonStyle::Elevated => (
                eh::token(ctx, &colors.surface_container),
                eh::token(ctx, &colors.outline_variant),
                eh::token(ctx, &colors.on_surface_container),
            ),
            ButtonStyle::Danger => (
                eh::token(ctx, &colors.error),
                eh::token(ctx, &colors.error),
                eh::token(ctx, &colors.on_error),
            ),
        }
    }
}

/// Trait shim so [`Button::font_size_for`] can read typography tokens without
/// this module needing to know the concrete token field layout.
trait TypographyExt {
    fn typography_label_small(&self) -> f32;
    fn typography_label_medium(&self) -> f32;
    fn typography_label_large(&self) -> f32;
    fn typography_title_medium(&self) -> f32;
}

impl TypographyExt for TypographyTokens {
    fn typography_label_small(&self) -> f32 {
        self.label_small.font_size
    }

    fn typography_label_medium(&self) -> f32 {
        self.label_medium.font_size
    }

    fn typography_label_large(&self) -> f32 {
        self.label_large.font_size
    }

    fn typography_title_medium(&self) -> f32 {
        self.title_medium.font_size
    }
}

#[cfg(feature = "egui")]
impl Button {
    /// Width [`Button::show`] will draw at, measured without drawing.
    ///
    /// A layout that places a button from an edge — the app bar lays its
    /// actions out right to left — needs that width *before* the button is
    /// painted, and this is the only place that knows how it is derived.
    #[must_use]
    pub fn desired_width(&self, ui: &egui::Ui, ctx: &crate::UiContext) -> f32 {
        let galley = ui.fonts(|fonts| {
            fonts.layout(
                self.display_text(),
                egui::FontId::proportional(Button::font_size_for(ctx.typography(), self.size)),
                egui::Color32::PLACEHOLDER,
                f32::INFINITY,
            )
        });
        let natural = galley.size().x + ctx.spacing().space_4 * 2.0;
        if self.full_width {
            natural.max(ui.available_width())
        } else {
            natural
        }
    }

    /// Draw the button into `ui` and report what the user did.
    ///
    /// The widget is sized from the platform's button height, the caller's
    /// [`ComponentSize`] and — on phones and tablets — the
    /// [`MIN_TOUCH_TARGET`] floor. Disabled buttons still occupy their space but
    /// always return [`ButtonResponse::None`].
    pub fn show(&mut self, ui: &mut egui::Ui, ctx: &crate::UiContext) -> ButtonResponse {
        use super::egui_helpers as eh;
        let colors = ctx.color_tokens();
        let height = self.height_for_context(ctx);
        let radius = crate::platform_adaptations::default_border_radius(ctx.platform());
        let font_size = Button::font_size_for(ctx.typography(), self.size);

        let text = self.display_text();
        let galley = ui.fonts(|fonts| {
            fonts.layout(
                text,
                egui::FontId::proportional(font_size),
                egui::Color32::PLACEHOLDER,
                f32::INFINITY,
            )
        });

        let width = self.desired_width(ui, ctx);

        let sense = if self.enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };

        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), sense);
        let mut response = ui.interact(rect, self.egui_id(), sense);
        let hovered = response.hovered();
        let pressed = response.is_pointer_button_down_on();

        let (fill, stroke_color, text_color) = self.palette(ctx);
        let (fill, stroke_color, text_color) = if self.enabled {
            (
                eh::state_fill(fill, hovered, pressed),
                if self.style.has_stroke() {
                    eh::state_fill(stroke_color, hovered, false)
                } else {
                    stroke_color
                },
                text_color,
            )
        } else {
            (
                eh::with_alpha(fill, 0.15),
                eh::with_alpha(stroke_color, DISABLED_ALPHA),
                eh::with_alpha(text_color, DISABLED_ALPHA),
            )
        };

        if self.enabled && self.style.has_shadow() {
            let shadow = egui::epaint::Shadow {
                offset: egui::vec2(0.0, 2.0),
                blur: 4.0,
                spread: 0.0,
                color: {
                    let shadow = eh::token(ctx, &colors.shadow);
                    egui::Color32::from_rgba_unmultiplied(shadow.r(), shadow.g(), shadow.b(), 96)
                },
            };
            ui.painter().add(shadow.as_shape(rect, radius));
        }

        let stroke = if self.style.has_stroke() {
            egui::Stroke::new(1.0_f32, stroke_color)
        } else {
            egui::Stroke::NONE
        };
        ui.painter().rect(rect, radius, fill, stroke);

        let galley_pos = egui::pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(galley_pos, galley, text_color);

        if self.enabled {
            // `on_hover_cursor` consumes the response, so it goes after every
            // other read of it.
            response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        response.widget_info(|| {
            let mut info = egui::WidgetInfo::new(egui::WidgetType::Button);
            info.label = Some(self.accessible_label());
            info.enabled = self.enabled;
            info
        });

        if !self.enabled {
            return ButtonResponse::None;
        }
        if response.long_touched() || response.secondary_clicked() {
            ButtonResponse::LongPressed
        } else if response.clicked() {
            ButtonResponse::Clicked
        } else {
            ButtonResponse::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn size_scale_follows_material_steps() {
        assert_close(Button::size_scale(ComponentSize::Small), 0.75);
        assert_close(Button::size_scale(ComponentSize::Medium), 1.0);
        assert_close(Button::size_scale(ComponentSize::Large), 1.25);
        assert_close(Button::size_scale(ComponentSize::ExtraLarge), 1.5);
    }

    #[test]
    fn height_for_matches_platform_baseline_at_medium() {
        for platform in [
            Platform::Android,
            Platform::Ios,
            Platform::Linux,
            Platform::Windows,
            Platform::Macos,
            Platform::Web,
        ] {
            assert_close(
                Button::height_for(platform, ComponentSize::Medium),
                crate::platform_adaptations::button_height(platform),
            );
        }
    }

    #[test]
    fn height_for_scales_per_size() {
        // Android: 48dp base.
        assert_close(
            Button::height_for(Platform::Android, ComponentSize::Small),
            36.0,
        );
        assert_close(
            Button::height_for(Platform::Android, ComponentSize::Medium),
            48.0,
        );
        assert_close(
            Button::height_for(Platform::Android, ComponentSize::Large),
            60.0,
        );
        assert_close(
            Button::height_for(Platform::Android, ComponentSize::ExtraLarge),
            72.0,
        );
        // Windows has the shortest baseline, 32px.
        assert_close(
            Button::height_for(Platform::Windows, ComponentSize::ExtraLarge),
            48.0,
        );
    }

    #[test]
    fn height_for_form_factor_enforces_touch_target_on_mobile() {
        // Windows desktop small buttons are allowed to be 24px tall.
        assert_close(
            Button::height_for_form_factor(Platform::Windows, ComponentSize::Small, false),
            24.0,
        );
        // The same button on a phone is grown to the 48px touch target.
        assert_close(
            Button::height_for_form_factor(Platform::Windows, ComponentSize::Small, true),
            MIN_TOUCH_TARGET,
        );
        // iOS already sits at 44dp, below the floor, so it grows too.
        assert_close(
            Button::height_for_form_factor(Platform::Ios, ComponentSize::Medium, true),
            MIN_TOUCH_TARGET,
        );
    }

    #[test]
    fn height_for_form_factor_leaves_tall_buttons_alone() {
        assert_close(
            Button::height_for_form_factor(Platform::Android, ComponentSize::ExtraLarge, true),
            72.0,
        );
        assert_close(
            Button::height_for_form_factor(Platform::Android, ComponentSize::Medium, true),
            48.0,
        );
    }

    #[test]
    fn font_size_for_tracks_component_size() {
        let tokens = TypographyTokens::default();
        let small = Button::font_size_for(&tokens, ComponentSize::Small);
        let medium = Button::font_size_for(&tokens, ComponentSize::Medium);
        let large = Button::font_size_for(&tokens, ComponentSize::Large);
        assert_close(small, 11.0);
        assert_close(medium, 12.0);
        assert_close(large, 14.0);
        assert!(small < medium && medium < large);
    }

    #[test]
    fn display_text_includes_icon_when_present() {
        assert_eq!(Button::new("Save").display_text(), "Save");
        assert_eq!(Button::new("Save").icon('💾').display_text(), "💾 Save");
    }

    #[test]
    fn accessible_label_prefers_explicit_override() {
        let button = Button::new("Save").accessibility_label("Save the document");
        assert_eq!(button.accessible_label(), "Save the document");

        let plain = Button::new("Save");
        assert_eq!(plain.accessible_label(), "Save");
        assert_eq!(plain.accessibility_label_(), None);

        let with_icon = Button::new("Save").icon('💾');
        assert_eq!(with_icon.accessible_label(), "💾 Save");
    }

    #[test]
    fn builder_defaults_are_medium_filled_enabled() {
        let button = Button::new("Go");
        assert_eq!(button.label(), "Go");
        assert_eq!(button.style_(), ButtonStyle::Filled);
        assert_eq!(button.size_(), ComponentSize::Medium);
        assert!(button.is_enabled());
        assert_eq!(button.icon_(), None);
        assert!(!button.is_full_width());
    }

    #[test]
    fn builder_overrides_every_field() {
        let button = Button::new("Delete")
            .style(ButtonStyle::Danger)
            .size(ComponentSize::Small)
            .enabled(false)
            .icon('🗑')
            .test_id("delete")
            .accessibility_label("Delete the file")
            .full_width(true);

        assert_eq!(button.style_(), ButtonStyle::Danger);
        assert_eq!(button.size_(), ComponentSize::Small);
        assert!(!button.is_enabled());
        assert_eq!(button.icon_(), Some('🗑'));
        assert_eq!(button.test_id_(), Some("delete"));
        assert_eq!(button.accessibility_label_(), Some("Delete the file"));
        assert!(button.is_full_width());
    }

    #[test]
    fn default_button_matches_builder() {
        assert_eq!(Button::default(), Button::new(""));
    }

    #[test]
    fn button_style_reports_chrome() {
        assert!(ButtonStyle::Filled.has_fill());
        assert!(!ButtonStyle::Filled.has_stroke());
        assert!(ButtonStyle::Outlined.has_stroke());
        assert!(!ButtonStyle::Outlined.has_fill());
        assert!(!ButtonStyle::Text.has_fill());
        assert!(ButtonStyle::Elevated.has_shadow());
        assert!(!ButtonStyle::Filled.has_shadow());
        assert!(ButtonStyle::Danger.has_fill());
    }

    #[test]
    fn button_style_default_is_filled() {
        assert_eq!(ButtonStyle::default(), ButtonStyle::Filled);
    }
}
