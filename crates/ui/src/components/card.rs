//! Card component.
//!
//! A card is a rounded, elevated surface that groups related content. Like
//! [`crate::components::button::Button`], all of the maths (corner radius,
//! padding, shadow geometry) lives in pure functions that compile without the
//! `egui` feature; only [`Card::show`] is egui-bound.

use super::ButtonResponse;

/// Default inner padding multiplier for a card.
const DEFAULT_PADDING_MULTIPLIER: f32 = 1.0;

/// A rounded, optionally clickable surface with a title and subtitle.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    title: String,
    subtitle: Option<String>,
    elevation: f32,
    padding_multiplier: f32,
    clickable: bool,
}

impl Card {
    /// Create a card titled `title` with no subtitle, no elevation and
    /// platform-default padding.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            elevation: 1.0,
            padding_multiplier: DEFAULT_PADDING_MULTIPLIER,
            clickable: false,
        }
    }

    /// Add a secondary line under the title.
    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set the shadow elevation, in the 0..=5 range used by Material Design 3.
    ///
    /// Values are clamped to `0.0..=5.0`.
    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = elevation.clamp(0.0, 5.0);
        self
    }

    /// Scale the platform's default inner padding.
    #[must_use]
    pub fn padding_multiplier(mut self, multiplier: f32) -> Self {
        self.padding_multiplier = multiplier.max(0.0);
        self
    }

    /// Make the whole card respond to clicks.
    #[must_use]
    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    /// The corner radius every card uses on the device described by `ctx`.
    ///
    /// Delegates straight to
    /// [`platform_adaptations::default_border_radius`](crate::platform_adaptations::default_border_radius),
    /// so cards follow each platform's shape language (12px on Android, 4px on
    /// Windows, and so on).
    #[must_use]
    pub fn corner_radius(ctx: &crate::UiContext) -> f32 {
        crate::platform_adaptations::default_border_radius(ctx.platform())
    }

    /// Inner padding in logical pixels for this card.
    #[must_use]
    pub fn padding(&self, ctx: &crate::UiContext) -> f32 {
        crate::platform_adaptations::default_padding(ctx.platform()) * self.padding_multiplier
    }

    /// Vertical offset of the drop shadow.
    #[must_use]
    pub fn shadow_offset(&self) -> f32 {
        self.elevation * 2.0
    }

    /// Blur radius of the drop shadow.
    #[must_use]
    pub fn shadow_blur(&self) -> f32 {
        self.elevation * 3.0
    }

    /// The card title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The card subtitle, if one was set.
    #[must_use]
    pub fn subtitle_(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }

    /// The configured elevation, clamped to `0.0..=5.0`.
    #[must_use]
    pub fn elevation_(&self) -> f32 {
        self.elevation
    }

    /// The configured padding multiplier.
    #[must_use]
    pub fn padding_multiplier_(&self) -> f32 {
        self.padding_multiplier
    }

    /// Whether the card reports clicks.
    #[must_use]
    pub fn is_clickable(&self) -> bool {
        self.clickable
    }
}

#[cfg(feature = "egui")]
impl Card {
    /// Draw the card header into `ui` and report whether the card was clicked.
    ///
    /// A card that is not [`Card::clickable`] still paints but always returns
    /// [`ButtonResponse::None`].
    pub fn show(&self, ui: &mut egui::Ui, ctx: &crate::UiContext) -> ButtonResponse {
        self.show_with(ui, ctx, |_ui| {})
    }

    /// Like [`Card::show`], but draws `body` inside the card frame.
    ///
    /// The returned [`ButtonResponse`] covers the *whole* card frame, not just
    /// the header, so a click anywhere inside the card counts.
    pub fn show_with<R>(
        &self,
        ui: &mut egui::Ui,
        ctx: &crate::UiContext,
        body: impl FnOnce(&mut egui::Ui) -> R,
    ) -> ButtonResponse {
        use super::egui_helpers as eh;
        let colors = ctx.color_tokens();
        let padding = self.padding(ctx);

        let shadow_color = eh::token(ctx, &colors.shadow);
        let frame = egui::Frame::none()
            .fill(eh::token(ctx, &colors.surface))
            .rounding(Card::corner_radius(ctx))
            .inner_margin(egui::Margin::symmetric(padding, padding * 0.75))
            .stroke(egui::Stroke::new(
                1.0_f32,
                eh::token(ctx, &colors.outline_variant),
            ))
            .shadow(egui::epaint::Shadow {
                offset: egui::vec2(0.0, self.shadow_offset()),
                blur: self.shadow_blur(),
                spread: 0.0,
                color: egui::Color32::from_rgba_unmultiplied(
                    shadow_color.r(),
                    shadow_color.g(),
                    shadow_color.b(),
                    48,
                ),
            });

        let inner = frame.show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(self.title.as_str())
                        .size(ctx.typography().title_medium.font_size)
                        .strong()
                        .color(eh::token(ctx, &colors.on_surface)),
                );
                if let Some(subtitle) = &self.subtitle {
                    ui.label(
                        egui::RichText::new(subtitle.as_str())
                            .size(ctx.typography().body_small.font_size)
                            .color(eh::token(ctx, &colors.on_surface_variant)),
                    );
                }
                body(ui);
            });
        });

        let sense = if self.clickable {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let id = egui::Id::new(("ui_components::Card", self.title.as_str()));
        let mut response = ui.interact(inner.response.rect, id, sense);

        if self.clickable {
            // `on_hover_cursor` consumes the response, so it goes before the
            // final read of it but after everything else.
            response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        response.widget_info(|| {
            let mut info = egui::WidgetInfo::new(egui::WidgetType::Button);
            info.label = Some(self.title.clone());
            info.enabled = self.clickable;
            info
        });

        if self.clickable && response.clicked() {
            ButtonResponse::Clicked
        } else {
            ButtonResponse::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_defaults() {
        let card = Card::new("Billing");
        assert_eq!(card.title(), "Billing");
        assert_eq!(card.subtitle_(), None);
        assert!(!card.is_clickable());
        assert_close(card.padding_multiplier_(), 1.0);
        assert_close(card.elevation_(), 1.0);
    }

    #[test]
    fn card_builder_sets_fields() {
        let card = Card::new("Billing")
            .subtitle("Updated every month")
            .elevation(3.0)
            .padding_multiplier(1.5)
            .clickable(true);

        assert_eq!(card.subtitle_(), Some("Updated every month"));
        assert_close(card.elevation_(), 3.0);
        assert_close(card.padding_multiplier_(), 1.5);
        assert!(card.is_clickable());
    }

    #[test]
    fn elevation_is_clamped_to_the_material_range() {
        assert_close(Card::new("x").elevation(-4.0).elevation_(), 0.0);
        assert_close(Card::new("x").elevation(99.0).elevation_(), 5.0);
    }

    #[test]
    fn padding_multiplier_never_goes_negative() {
        assert_close(
            Card::new("x")
                .padding_multiplier(-2.0)
                .padding_multiplier_(),
            0.0,
        );
        assert_close(
            Card::new("x").padding_multiplier(0.0).padding_multiplier_(),
            0.0,
        );
    }

    #[test]
    fn shadow_geometry_grows_with_elevation() {
        let flat = Card::new("x").elevation(0.0);
        assert_close(flat.shadow_offset(), 0.0);
        assert_close(flat.shadow_blur(), 0.0);

        let high = Card::new("x").elevation(4.0);
        assert_close(high.shadow_offset(), 8.0);
        assert_close(high.shadow_blur(), 12.0);
        assert!(high.shadow_offset() > flat.shadow_offset());
        assert!(high.shadow_blur() > flat.shadow_blur());
    }

    #[test]
    fn cards_with_equal_settings_are_equal() {
        let a = Card::new("A").subtitle("s").clickable(true);
        let b = Card::new("A").subtitle("s").clickable(true);
        assert_eq!(a, b);

        let c = Card::new("A").subtitle("s").clickable(false);
        assert_ne!(a, c);
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {expected}, got {actual}"
        );
    }
}
