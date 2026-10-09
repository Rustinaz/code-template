//! App bar component.
//!
//! The app bar is the persistent strip at the top of a screen: the screen's
//! title, an optional back affordance on the leading edge, and a row of
//! actions on the trailing edge.
//!
//! Only the painting is egui-bound. Every decision the bar makes — how tall it
//! is on a given OS, whether the back affordance exists, where the title
//! sits, how much room the leading and trailing groups need — is a pure
//! function of [`Platform`] and the builder state, so a screen can be reasoned
//! about (and unit tested) without a backend.
//!
//! ```
//! use shared::domain::Platform;
//! use ui::components::app_bar::AppBar;
//!
//! let bar = AppBar::new("Inbox").back_enabled(true).action("Archive");
//! // iOS and macOS centre their titles; Material leads with them.
//! assert!(bar.centers_title(Platform::Ios));
//! assert!(!bar.centers_title(Platform::Android));
//! // A 44pt iOS title row is grown so the back arrow stays reachable.
//! assert_eq!(AppBar::touch_height_for(Platform::Ios), 48.0);
//! ```

use shared::domain::Platform;

use super::button::{Button, ButtonStyle, MIN_TOUCH_TARGET};

/// Id that [`AppBar::show`] reports when the back affordance was pressed.
///
/// Reserving one id for "go back" means a caller can branch on
/// `action.as_deref() == Some(BACK_ACTION)` instead of inventing its own
/// sentinel string, and it cannot collide with an action label because the bar
/// never returns a label for it — see [`AppBar::show`].
pub const BACK_ACTION: &str = "back";

/// Default visible label of the back affordance.
const DEFAULT_BACK_LABEL: &str = "Back";

/// Glyph drawn in front of [`DEFAULT_BACK_LABEL`].
const BACK_ICON: char = '\u{2190}';

/// The persistent top bar of a screen.
#[derive(Debug, Clone, PartialEq)]
pub struct AppBar {
    title: String,
    subtitle: Option<String>,
    actions: Vec<String>,
    back_enabled: bool,
    back_label: String,
    center_title: Option<bool>,
    elevated: bool,
}

impl Default for AppBar {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl AppBar {
    /// Create a bar showing `title`, with no subtitle, no actions and no back
    /// affordance.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            actions: Vec::new(),
            back_enabled: false,
            back_label: DEFAULT_BACK_LABEL.to_string(),
            center_title: None,
            elevated: false,
        }
    }

    /// Add a second line under the title.
    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Append a trailing action.
    ///
    /// The argument is both the painted label and the id [`AppBar::show`]
    /// reports, so the caller never has to keep two strings in sync.
    #[must_use]
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.actions.push(label.into());
        self
    }

    /// Draw the back affordance.
    ///
    /// Opt-in, because a bar on a root screen has nowhere to go back to and an
    /// always-visible arrow there is a dead control.
    #[must_use]
    pub fn back_enabled(mut self, back_enabled: bool) -> Self {
        self.back_enabled = back_enabled;
        self
    }

    /// Override the text next to the back arrow.
    #[must_use]
    pub fn back_label(mut self, label: impl Into<String>) -> Self {
        self.back_label = label.into();
        self
    }

    /// Force the title to be centred (`true`) or leading-aligned (`false`),
    /// overriding the platform rule from [`AppBar::centers_title_on`].
    #[must_use]
    pub fn center_title(mut self, center_title: bool) -> Self {
        self.center_title = Some(center_title);
        self
    }

    /// Separate the bar from the content with a drop shadow instead of a
    /// hairline.
    ///
    /// The two are mutually exclusive on purpose: iOS and macOS already read
    /// as "floating above the page", and a hairline on top of a shadow reads
    /// as a rendering bug.
    #[must_use]
    pub fn elevated(mut self, elevated: bool) -> Self {
        self.elevated = elevated;
        self
    }

    /// Height of the title row for `platform`, before the touch-target floor.
    ///
    /// Material's top app bar is 64dp, the iOS and macOS navigation title rows
    /// are 44pt, and a desktop title bar sits in between at 48px.
    #[must_use]
    pub fn height_for(platform: Platform) -> f32 {
        match platform {
            Platform::Android | Platform::Web => 64.0,
            Platform::Ios | Platform::Macos => 44.0,
            Platform::Linux | Platform::Windows => 48.0,
        }
    }

    /// Height the bar is actually drawn at.
    ///
    /// Grown to [`MIN_TOUCH_TARGET`] where the platform's title row is
    /// shorter, because the back affordance has to stay reachable.
    #[must_use]
    pub fn touch_height_for(platform: Platform) -> f32 {
        Self::height_for(platform).max(MIN_TOUCH_TARGET)
    }

    /// Whether a platform centres its titles by default.
    ///
    /// iOS and macOS centre window and navigation titles; Material leads with
    /// them, and so does anything that follows Material.
    #[must_use]
    pub fn centers_title_on(platform: Platform) -> bool {
        matches!(platform, Platform::Ios | Platform::Macos)
    }

    /// Whether *this* bar centres its title on `platform`, after the
    /// [`AppBar::center_title`] override is taken into account.
    #[must_use]
    pub fn centers_title(&self, platform: Platform) -> bool {
        self.center_title
            .unwrap_or_else(|| Self::centers_title_on(platform))
    }

    /// Width taken by the leading group: the back affordance plus the gap that
    /// separates it from the title, or zero when there is none.
    ///
    /// `back_width` is whatever the affordance actually needs, rather than a
    /// fixed touch target, because the label is localised and a translated
    /// "Zurück" is wider than an English "Back".
    #[must_use]
    pub fn leading_width(&self, back_width: f32, gap: f32) -> f32 {
        if self.back_enabled {
            back_width + gap
        } else {
            0.0
        }
    }

    /// Width taken by the trailing group: every action plus a gap after each
    /// one, or zero when there are no actions.
    ///
    /// The gap after the last action is included even though nothing follows
    /// it, so the last action keeps the same breathing room from the screen
    /// edge as every other action keeps from its neighbour.
    #[must_use]
    pub fn trailing_width(&self, action_widths: &[f32], gap: f32) -> f32 {
        action_widths.iter().sum::<f32>() + gap * action_widths.len() as f32
    }

    /// Number of things in the bar the user can activate: the back affordance
    /// plus the trailing actions.
    #[must_use]
    pub fn interactive_count(&self) -> usize {
        usize::from(self.back_enabled) + self.actions.len()
    }

    /// The screen title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The second title line, if one was set.
    #[must_use]
    pub fn subtitle_text(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }

    /// The trailing action labels, in order.
    #[must_use]
    pub fn actions(&self) -> &[String] {
        &self.actions
    }

    /// Whether the bar has at least one trailing action.
    #[must_use]
    pub fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    /// Whether the back affordance is drawn.
    #[must_use]
    pub fn is_back_enabled(&self) -> bool {
        self.back_enabled
    }

    /// The text next to the back arrow.
    #[must_use]
    pub fn back_label_(&self) -> &str {
        &self.back_label
    }

    /// Whether the bar is separated from the content by a shadow.
    #[must_use]
    pub fn is_elevated(&self) -> bool {
        self.elevated
    }

    /// The text a screen reader reads when focus enters the bar.
    ///
    /// The actions announce themselves separately, so this is only the title
    /// (plus the subtitle, which is otherwise decoration).
    #[must_use]
    pub fn accessible_label(&self) -> String {
        match &self.subtitle {
            Some(subtitle) => format!("{}, {subtitle}", self.title),
            None => self.title.clone(),
        }
    }

    /// The button used for the back affordance.
    ///
    /// Its accessible label is [`BACK_ACTION`] so that what a screen reader
    /// announces and what [`AppBar::show`] reports are the same string.
    #[must_use]
    pub fn back_button(&self) -> Button {
        Button::new(self.back_label.as_str())
            .icon(BACK_ICON)
            .accessibility_label(BACK_ACTION)
    }
}

#[cfg(feature = "egui")]
impl AppBar {
    /// Draw the bar into `ui` and report what the user pressed.
    ///
    /// Returns the label of the action that was pressed, [`BACK_ACTION`] for
    /// the back affordance, or `None` when nothing happened this frame.
    ///
    /// The title is laid out across the whole bar and the buttons are painted
    /// on top of it, which is what makes a centred title actually centred
    /// rather than centred in the space left over next to the actions.
    pub fn show(&self, ui: &mut egui::Ui, ctx: &crate::UiContext) -> Option<String> {
        use super::egui_helpers as eh;
        let colors = ctx.color_tokens();
        let platform = ctx.platform();
        let height = Self::touch_height_for(platform);
        let gap = ctx.spacing().space_2;

        let available_width = ui.available_width();
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(available_width, height), egui::Sense::hover());

        // The bars are laid out from the two edges inward, so the widths of the
        // leading and trailing groups have to be known before anything is drawn.
        let back_width = if self.back_enabled {
            self.back_button().desired_width(ui, ctx)
        } else {
            0.0
        };
        let action_widths: Vec<f32> = self
            .actions
            .iter()
            .map(|label| {
                Button::new(label.as_str())
                    .style(ButtonStyle::Text)
                    .desired_width(ui, ctx)
            })
            .collect();
        let leading = self.leading_width(back_width, gap);
        let trailing = self.trailing_width(&action_widths, gap);

        let shadow = eh::token(ctx, &colors.shadow);
        if self.elevated {
            ui.painter().add(
                egui::epaint::Shadow {
                    offset: egui::vec2(0.0, 2.0),
                    blur: 6.0,
                    spread: 0.0,
                    color: egui::Color32::from_rgba_unmultiplied(
                        shadow.r(),
                        shadow.g(),
                        shadow.b(),
                        64,
                    ),
                }
                .as_shape(rect, 0.0),
            );
        }
        ui.painter().rect(
            rect,
            0.0,
            eh::token(ctx, &colors.surface),
            egui::Stroke::NONE,
        );
        if !self.elevated {
            // Material separates the bar from the content with a 1dp hairline
            // instead of a shadow; picking one is what keeps it from reading as
            // two borders.
            ui.painter().hline(
                rect.left()..=rect.right(),
                rect.bottom(),
                egui::Stroke::new(1.0_f32, eh::token(ctx, &colors.outline_variant)),
            );
        }

        // The title is painted straight onto the bar rather than laid out in a
        // sub-`Ui`, so a centred title is centred on the whole bar and not on
        // whatever happens to be left over next to the actions.
        let title_font = egui::FontId::proportional(ctx.typography().title_large.font_size);
        let subtitle_font = egui::FontId::proportional(ctx.typography().body_small.font_size);
        let title_galley = ui.fonts(|fonts| {
            fonts.layout(
                self.title.clone(),
                title_font,
                egui::Color32::PLACEHOLDER,
                f32::INFINITY,
            )
        });
        let subtitle_galley = self.subtitle.as_deref().map(|subtitle| {
            ui.fonts(|fonts| {
                fonts.layout(
                    subtitle.to_string(),
                    subtitle_font,
                    egui::Color32::PLACEHOLDER,
                    f32::INFINITY,
                )
            })
        });

        let line_gap = ctx.spacing().space_1;
        let title_height = title_galley.size().y;
        let title_width = title_galley.size().x;
        let block_height = title_height
            + subtitle_galley
                .as_ref()
                .map_or(0.0, |galley| galley.size().y + line_gap);
        let block_top = rect.center().y - block_height / 2.0;

        let title_x = if self.centers_title(platform) {
            rect.center().x - title_width / 2.0
        } else {
            rect.left() + leading
        };
        // A leading title only gets the room the buttons did not take, so a
        // long one is clipped by the actions rather than sliding underneath
        // them. A centred title keeps the whole bar: that is what makes it
        // centred rather than centred in the leftover space.
        let title_area = if self.centers_title(platform) {
            rect
        } else {
            egui::Rect::from_min_max(
                egui::pos2(title_x, rect.min.y),
                egui::pos2((rect.right() - trailing).max(title_x), rect.max.y),
            )
        };
        let painter = ui.painter().with_clip_rect(title_area);
        painter.galley(
            egui::pos2(title_x, block_top),
            title_galley,
            eh::token(ctx, &colors.on_surface),
        );
        if let Some(galley) = subtitle_galley {
            painter.galley(
                egui::pos2(title_x, block_top + title_height + line_gap),
                galley,
                eh::token(ctx, &colors.on_surface_variant),
            );
        }

        let mut pressed = None;
        let top = rect.center().y - height / 2.0;

        if self.back_enabled {
            let slot = egui::Rect::from_min_size(
                egui::pos2(rect.left(), top),
                egui::vec2(back_width, height),
            );
            ui.allocate_new_ui(slot_ui(slot), |ui| {
                if self.back_button().show(ui, ctx).is_clicked() {
                    pressed = Some(BACK_ACTION.to_string());
                }
            });
        }

        // Trailing actions, laid out right to left from the screen edge.
        let mut cursor = rect.right();
        for (label, width) in self.actions.iter().zip(&action_widths).rev() {
            cursor -= width;
            let slot =
                egui::Rect::from_min_size(egui::pos2(cursor, top), egui::vec2(*width, height));
            ui.allocate_new_ui(slot_ui(slot), |ui| {
                if Button::new(label.as_str())
                    .style(ButtonStyle::Text)
                    .show(ui, ctx)
                    .is_clicked()
                {
                    pressed = Some(label.clone());
                }
            });
            cursor -= gap;
        }

        pressed
    }
}

/// A child `Ui` confined to one fixed slot of the bar.
///
/// Centring the button inside the slot matters because the bar's own height
/// ([`AppBar::touch_height_for`]) and the button's (its platform height) are
/// not the same number on iOS.
#[cfg(feature = "egui")]
fn slot_ui(slot: egui::Rect) -> egui::UiBuilder {
    egui::UiBuilder::new()
        .max_rect(slot)
        .layout(egui::Layout::centered_and_justified(
            egui::Direction::TopDown,
        ))
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
    fn app_bar_defaults() {
        let bar = AppBar::new("Inbox");
        assert_eq!(bar.title(), "Inbox");
        assert_eq!(bar.subtitle_text(), None);
        assert_eq!(bar.actions(), [] as [String; 0]);
        assert!(!bar.has_actions());
        assert!(!bar.is_back_enabled());
        assert!(!bar.is_elevated());
        assert_eq!(bar.interactive_count(), 0);
    }

    #[test]
    fn default_app_bar_matches_builder() {
        assert_eq!(AppBar::default(), AppBar::new(""));
    }

    #[test]
    fn builder_sets_every_field() {
        let bar = AppBar::new("Inbox")
            .subtitle("3 unread")
            .action("Archive")
            .action("Delete")
            .back_enabled(true)
            .back_label("Zurück")
            .center_title(false)
            .elevated(true);

        assert_eq!(bar.subtitle_text(), Some("3 unread"));
        assert_eq!(bar.actions(), ["Archive".to_string(), "Delete".to_string()]);
        assert!(bar.is_back_enabled());
        assert_eq!(bar.back_label_(), "Zurück");
        assert!(bar.is_elevated());
    }

    #[test]
    fn actions_keep_insertion_order() {
        let bar = AppBar::new("x").action("One").action("Two").action("Three");
        let labels: Vec<&str> = bar.actions().iter().map(String::as_str).collect();
        assert_eq!(labels, ["One", "Two", "Three"]);
    }

    #[test]
    fn interactive_count_counts_the_back_affordance() {
        assert_eq!(AppBar::new("x").interactive_count(), 0);
        assert_eq!(
            AppBar::new("x").action("A").action("B").interactive_count(),
            2
        );
        assert_eq!(
            AppBar::new("x")
                .action("A")
                .back_enabled(true)
                .interactive_count(),
            2
        );
    }

    #[test]
    fn height_for_follows_each_platforms_title_row() {
        assert_close(AppBar::height_for(Platform::Android), 64.0);
        assert_close(AppBar::height_for(Platform::Web), 64.0);
        assert_close(AppBar::height_for(Platform::Ios), 44.0);
        assert_close(AppBar::height_for(Platform::Macos), 44.0);
        assert_close(AppBar::height_for(Platform::Linux), 48.0);
        assert_close(AppBar::height_for(Platform::Windows), 48.0);
    }

    #[test]
    fn touch_height_grows_a_short_title_row_to_the_touch_target() {
        // The 44pt iOS title row is too short for a 48dp touch target.
        assert_close(AppBar::touch_height_for(Platform::Ios), MIN_TOUCH_TARGET);
        assert_close(AppBar::touch_height_for(Platform::Macos), MIN_TOUCH_TARGET);
        // Android's 64dp bar is already comfortable.
        assert_close(AppBar::touch_height_for(Platform::Android), 64.0);
    }

    #[test]
    fn title_alignment_follows_the_platform_unless_overridden() {
        let bar = AppBar::new("x");
        assert!(bar.centers_title(Platform::Ios));
        assert!(bar.centers_title(Platform::Macos));
        assert!(!bar.centers_title(Platform::Android));
        assert!(!bar.centers_title(Platform::Linux));
        assert!(!bar.centers_title(Platform::Windows));
        assert!(!bar.centers_title(Platform::Web));

        // The builder wins in both directions.
        assert!(!bar.clone().center_title(false).centers_title(Platform::Ios));
        assert!(bar.center_title(true).centers_title(Platform::Android));
    }

    #[test]
    fn leading_width_is_zero_without_a_back_affordance() {
        assert_close(AppBar::new("x").leading_width(64.0, 8.0), 0.0);
        assert_close(
            AppBar::new("x").back_enabled(true).leading_width(64.0, 8.0),
            72.0,
        );
    }

    #[test]
    fn trailing_width_sums_the_measured_actions() {
        assert_close(AppBar::new("x").trailing_width(&[], 8.0), 0.0);
        assert_close(AppBar::new("x").trailing_width(&[40.0], 8.0), 48.0);
        assert_close(AppBar::new("x").trailing_width(&[40.0, 56.0], 8.0), 112.0);
    }

    #[test]
    fn the_leading_and_trailing_groups_do_not_overlap_the_title_room() {
        // A back affordance (64 + 8) plus two actions (56 + 8 each) leaves the
        // title the rest of a 400px bar, which is what the split is for.
        let bar = AppBar::new("Inbox")
            .back_enabled(true)
            .action("Archive")
            .action("Delete");
        let leading = bar.leading_width(64.0, 8.0);
        let trailing = bar.trailing_width(&[56.0, 56.0], 8.0);
        assert_close(leading, 72.0);
        assert_close(trailing, 128.0);
        assert_close(leading + trailing, 200.0);
        assert!(leading + trailing < 400.0);
    }

    #[test]
    fn accessible_label_includes_the_subtitle() {
        assert_eq!(AppBar::new("Inbox").accessible_label(), "Inbox");
        assert_eq!(
            AppBar::new("Inbox").subtitle("3 unread").accessible_label(),
            "Inbox, 3 unread"
        );
    }

    #[test]
    fn back_button_announces_the_back_action_id() {
        let button = AppBar::new("x").back_button();
        assert_eq!(button.accessible_label(), BACK_ACTION);
        assert!(button.label().contains("Back"));
        assert!(button.is_enabled());
    }

    #[test]
    fn back_label_is_configurable() {
        let bar = AppBar::new("x").back_label("Retour");
        assert_eq!(bar.back_label_(), "Retour");
        assert!(bar.back_button().label().contains("Retour"));
    }

    #[test]
    fn bars_with_equal_settings_are_equal() {
        let a = AppBar::new("Inbox").action("A").back_enabled(true);
        let b = AppBar::new("Inbox").action("A").back_enabled(true);
        assert_eq!(a, b);

        let c = AppBar::new("Inbox").action("A").back_enabled(false);
        assert_ne!(a, c);
    }
}
