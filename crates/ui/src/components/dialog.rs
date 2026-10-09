//! Dialog component.
//!
//! [`DialogState`] is pure: opening, closing and toggling a dialog needs no
//! backend. [`Dialog::show`] is gated behind the `egui` feature and is safe to
//! call unconditionally every frame — when nothing is visible it draws nothing
//! and returns an empty `Vec`.

use super::{button::Button, button::ButtonStyle};

/// Visibility of a [`Dialog`].
///
/// Kept separate from [`Dialog`] so the *definition* of a dialog (title, body,
/// actions) can live in one place while each screen owns its own open/closed
/// state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DialogState {
    /// Whether the dialog should be drawn this frame.
    pub visible: bool,
}

impl DialogState {
    /// Create a closed dialog state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the dialog is visible.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Show the dialog.
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// Hide the dialog.
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Flip the visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Set the visibility explicitly.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
}

/// A modal dialog: title, body text and a row of actions.
#[derive(Debug, Clone, PartialEq)]
pub struct Dialog {
    title: String,
    body: String,
    dismissable: bool,
    visible: bool,
    actions: Vec<String>,
}

impl Dialog {
    /// Create a dismissable dialog that is opted in.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: String::new(),
            dismissable: true,
            visible: true,
            actions: Vec::new(),
        }
    }

    /// Set the explanatory body text under the title.
    #[must_use]
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// Allow the user to dismiss the dialog with the close button or the scrim.
    #[must_use]
    pub fn dismissable(mut self, dismissable: bool) -> Self {
        self.dismissable = dismissable;
        self
    }

    /// Include this dialog at all.
    ///
    /// The dialog renders only when *both* this flag and
    /// [`DialogState::is_visible`] are set, so a screen can turn a dialog off
    /// entirely without dropping its state.
    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Append an action button.
    #[must_use]
    pub fn add_action(mut self, label: impl Into<String>) -> Self {
        self.actions.push(label.into());
        self
    }

    /// Whether anything should be drawn for `state` this frame.
    #[must_use]
    pub fn should_render(&self, state: &DialogState) -> bool {
        self.visible && state.visible
    }

    /// The configured action labels, in order.
    #[must_use]
    pub fn actions(&self) -> &[String] {
        &self.actions
    }

    /// Whether the dialog has at least one action button.
    #[must_use]
    pub fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    /// Whether the dialog can be dismissed.
    #[must_use]
    pub fn is_dismissable(&self) -> bool {
        self.dismissable
    }

    /// The dialog title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The dialog body text.
    #[must_use]
    pub fn body_text(&self) -> &str {
        &self.body
    }
}

#[cfg(feature = "egui")]
impl Dialog {
    /// Draw the dialog (if visible) and report which action labels were pressed.
    ///
    /// Safe to call every frame: when [`Dialog::should_render`] is false this
    /// draws nothing and returns an empty `Vec`.
    ///
    /// Returns the labels of the actions pressed this frame, in the order they
    /// were declared. Dismissing the dialog (close button or scrim) closes
    /// `state` and returns an empty `Vec`.
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        ctx: &crate::UiContext,
        state: &mut DialogState,
    ) -> Vec<String> {
        use super::egui_helpers as eh;

        if !self.should_render(state) {
            return Vec::new();
        }
        let colors = ctx.color_tokens();
        let screen = ui.ctx().screen_rect();

        // Modal scrim: painted first so the dialog window lands on top of it.
        let scrim = eh::with_alpha(eh::token(ctx, &colors.scrim), 0.6);
        ui.painter().rect_filled(screen, 0.0, scrim);

        if self.dismissable {
            let scrim_id = egui::Id::new(("ui_components::DialogScrim", self.title.as_str()));
            if ui
                .interact(screen, scrim_id, egui::Sense::click())
                .clicked()
            {
                state.close();
                return Vec::new();
            }
        }

        let radius = crate::platform_adaptations::default_border_radius(ctx.platform());
        let padding = crate::platform_adaptations::default_padding(ctx.platform());
        let width = screen.width().clamp(240.0, 480.0);

        let mut open = true;
        let mut window = egui::Window::new(self.title.as_str())
            .id(egui::Id::new((
                "ui_components::Dialog",
                self.title.as_str(),
            )))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .frame(
                // `Frame::window` is egui's own default, so only the platform
                // radius and padding are replaced.
                egui::Frame::window(ui.style())
                    .rounding(radius)
                    .inner_margin(egui::Margin::symmetric(padding, padding * 0.75)),
            )
            .fixed_size(egui::vec2(width, 0.0));
        if self.dismissable {
            window = window.open(&mut open);
        }

        let mut pressed: Vec<String> = Vec::new();
        let mut buttons: Vec<Button> = self
            .actions
            .iter()
            .map(|label| {
                let style = if label.eq_ignore_ascii_case("cancel") {
                    ButtonStyle::Text
                } else {
                    ButtonStyle::Filled
                };
                Button::new(label.as_str()).style(style).full_width(true)
            })
            .collect();

        window.show(ui.ctx(), |ui| {
            ui.set_width(width);
            if !self.body.is_empty() {
                ui.label(
                    egui::RichText::new(self.body.as_str())
                        .size(ctx.typography().body_medium.font_size)
                        .color(eh::token(ctx, &colors.on_surface_variant)),
                );
                ui.add_space(ctx.spacing().space_3);
            }

            if !buttons.is_empty() {
                ui.horizontal(|ui| {
                    for mut button in buttons.drain(..) {
                        if button.show(ui, ctx).is_clicked() {
                            pressed.push(button.label().to_string());
                        }
                    }
                });
            }
        });

        if self.dismissable && !open {
            state.close();
        }

        pressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_state_starts_closed() {
        let state = DialogState::new();
        assert!(!state.is_visible());
        assert_eq!(state, DialogState::default());
    }

    #[test]
    fn open_close_and_toggle() {
        let mut state = DialogState::new();

        state.open();
        assert!(state.is_visible());
        state.open();
        assert!(state.is_visible(), "open is idempotent");

        state.close();
        assert!(!state.is_visible());
        state.close();
        assert!(!state.is_visible(), "close is idempotent");

        state.toggle();
        assert!(state.is_visible());
        state.toggle();
        assert!(!state.is_visible());
    }

    #[test]
    fn set_visible_matches_open_and_close() {
        let mut state = DialogState::new();
        state.set_visible(true);
        assert!(state.is_visible());
        state.set_visible(false);
        assert!(!state.is_visible());
    }

    #[test]
    fn dialog_defaults() {
        let dialog = Dialog::new("Delete file");
        assert_eq!(dialog.title(), "Delete file");
        assert_eq!(dialog.body_text(), "");
        assert!(dialog.is_dismissable());
        assert!(!dialog.has_actions());
        assert!(dialog.actions().is_empty());
        assert!(dialog.visible);
    }

    #[test]
    fn dialog_builder_sets_fields() {
        let dialog = Dialog::new("Delete file")
            .body("This cannot be undone.")
            .dismissable(false)
            .visible(false)
            .add_action("Cancel")
            .add_action("Delete");

        assert_eq!(dialog.body_text(), "This cannot be undone.");
        assert!(!dialog.is_dismissable());
        assert!(!dialog.visible);
        assert!(dialog.has_actions());
        assert_eq!(
            dialog.actions(),
            ["Cancel".to_string(), "Delete".to_string()]
        );
    }

    #[test]
    fn should_render_needs_both_switches() {
        let dialog = Dialog::new("x");
        let mut state = DialogState::new();

        // Dialog opted in, state closed.
        assert!(!dialog.should_render(&state));

        state.open();
        assert!(dialog.should_render(&state));

        // Dialog opted out entirely.
        assert!(!dialog.visible(false).should_render(&state));
    }

    #[test]
    fn actions_keep_insertion_order() {
        let dialog = Dialog::new("x")
            .add_action("One")
            .add_action("Two")
            .add_action("Three");
        let labels: Vec<&str> = dialog.actions().iter().map(String::as_str).collect();
        assert_eq!(labels, ["One", "Two", "Three"]);
    }
}
