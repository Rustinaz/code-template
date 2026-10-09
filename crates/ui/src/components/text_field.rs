//! Text field component.
//!
//! [`TextFieldState`] is deliberately egui-free: it is the *value* half of a
//! text input, and it can be created, mutated and asserted on without a single
//! backend dependency. [`TextField`] is the *view* half and is gated behind the
//! `egui` feature.
//!
//! The split matters for the template's goal: on desktop, Android, iOS and the
//! web, the validation, length limiting and masking rules are identical, so
//! they only need writing once.

use super::ButtonResponse;

/// Character used to mask a password field, one per character of the real value.
pub const PASSWORD_MASK: char = '\u{2022}';

/// The value half of a single-line text input.
///
/// All fields are public so callers can inspect or restore a saved draft;
/// prefer [`TextFieldState::new`] over a struct literal because it applies the
/// `max_length` invariant.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextFieldState {
    /// The current, unmasked value.
    pub value: String,
    /// Hint shown while the value is empty.
    pub placeholder: String,
    /// Hard cap on the number of characters.
    pub max_length: Option<usize>,
    /// When `true`, the UI masks the value and [`Self::displayed_value`] masks it too.
    pub password: bool,
    /// When `true`, the UI ignores input.
    pub disabled: bool,

    /// Set when the value changed and not yet consumed by
    /// [`TextFieldState::take_changed`].
    pending: Option<String>,
}

impl TextFieldState {
    /// Create an empty, enabled field showing `placeholder` as the hint.
    #[must_use]
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            placeholder: placeholder.into(),
            ..Self::default()
        }
    }

    /// Cap the value at `max_length` characters.
    #[must_use]
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        // `set_value` mutably borrows `self`, so the old value has to be moved
        // out before the call rather than read from inside it.
        let current = std::mem::take(&mut self.value);
        self.set_value(current);
        self
    }

    /// Mask the value.
    #[must_use]
    pub fn with_password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Enable or disable the field.
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Replace the value.
    ///
    /// Truncated to [`Self::max_length`] when one is set. A change is recorded
    /// only when the truncated value actually differs from the current one, so
    /// assigning the same text twice does not produce a spurious change event.
    pub fn set_value(&mut self, value: impl Into<String>) {
        let incoming: String = value.into();
        let next = Self::truncate(&incoming, self.max_length);
        if next != self.value {
            self.value = next.clone();
            self.pending = Some(next);
        }
    }

    /// Take the pending change, if there is one.
    ///
    /// Returns `Some(value)` at most once per modification, which makes
    /// `if let Some(new) = state.take_changed()` a reliable "did the user edit
    /// this?" test in a frame loop.
    pub fn take_changed(&mut self) -> Option<String> {
        self.pending.take()
    }

    /// Whether a change is waiting to be consumed.
    #[must_use]
    pub fn has_pending_change(&self) -> bool {
        self.pending.is_some()
    }

    /// Clear both the value and any pending change.
    pub fn clear(&mut self) {
        self.set_value(String::new());
    }

    /// The current value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The placeholder hint.
    #[must_use]
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Whether the value is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// The number of characters currently in the value.
    #[must_use]
    pub fn len(&self) -> usize {
        self.value.chars().count()
    }

    /// Whether the value reached [`Self::max_length`].
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.max_length.is_some_and(|max| self.len() >= max)
    }

    /// Whether the value is masked.
    #[must_use]
    pub fn is_password(&self) -> bool {
        self.password
    }

    /// Whether the field ignores input.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// What should be shown on screen: the value, or one [`PASSWORD_MASK`] per
    /// character when the field is a password field.
    #[must_use]
    pub fn displayed_value(&self) -> String {
        if self.password {
            std::iter::repeat_n(PASSWORD_MASK, self.value.chars().count()).collect()
        } else {
            self.value.clone()
        }
    }

    /// Truncate `value` to `max_length` characters, without splitting a
    /// multi-byte character.
    fn truncate(value: &str, max_length: Option<usize>) -> String {
        let Some(max_length) = max_length else {
            return value.to_string();
        };
        match value.char_indices().nth(max_length) {
            Some((index, _)) => value[..index].to_string(),
            None => value.to_string(),
        }
    }
}

/// The view half of a single-line text input.
///
/// Build one per field (they are cheap and hold no mutable state) and call
/// [`TextField::show`] once per frame with the [`TextFieldState`] you want to
/// edit.
#[derive(Debug, Clone, PartialEq)]
pub struct TextField {
    test_id: Option<String>,
    label: Option<String>,
    full_width: bool,
    min_width: f32,
    submit_on_enter: bool,
}

impl Default for TextField {
    fn default() -> Self {
        Self::new()
    }
}

impl TextField {
    /// Create an unlabelled, shrink-to-fit text field.
    #[must_use]
    pub fn new() -> Self {
        Self {
            test_id: None,
            label: None,
            full_width: false,
            min_width: 0.0,
            submit_on_enter: true,
        }
    }

    /// Give the field a stable id so UI tests can find it again.
    #[must_use]
    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    /// Show a leading label next to the input.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Stretch the input to fill the available width.
    #[must_use]
    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    /// Never render narrower than `min_width` logical pixels.
    #[must_use]
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width.max(0.0);
        self
    }

    /// Report [`ButtonResponse::Clicked`] when the user presses Enter.
    #[must_use]
    pub fn submit_on_enter(mut self, submit_on_enter: bool) -> Self {
        self.submit_on_enter = submit_on_enter;
        self
    }

    /// The leading label, if one was set.
    #[must_use]
    pub fn label_text(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The stable test id, if one was set.
    #[must_use]
    pub fn test_id_(&self) -> Option<&str> {
        self.test_id.as_deref()
    }

    /// Whether the field stretches to the available width.
    #[must_use]
    pub fn is_full_width(&self) -> bool {
        self.full_width
    }

    /// Whether Enter submits the field.
    #[must_use]
    pub fn submits_on_enter(&self) -> bool {
        self.submit_on_enter
    }

    /// Stable egui id for this field.
    #[cfg(feature = "egui")]
    #[must_use]
    pub fn egui_id(&self) -> egui::Id {
        match self.test_id.as_deref() {
            Some(test_id) => egui::Id::new(("ui_components::TextField", test_id)),
            None => egui::Id::new(("ui_components::TextField", self.label.clone())),
        }
    }
}

#[cfg(feature = "egui")]
impl TextField {
    /// Draw the field into `ui`, updating `state` in place.
    ///
    /// Edits made this frame are written back through
    /// [`TextFieldState::set_value`], so `max_length` is enforced even when the
    /// caller forgot to configure egui's own character limit.
    ///
    /// Returns [`ButtonResponse::Clicked`] when the user pressed Enter and
    /// [`self.submit_on_enter`] is set, otherwise [`ButtonResponse::None`].
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        ctx: &crate::UiContext,
        state: &mut TextFieldState,
    ) -> ButtonResponse {
        use super::egui_helpers as eh;
        let colors = ctx.color_tokens();
        let height =
            super::button::Button::height_for_context(&super::button::Button::new(""), ctx);
        let radius = crate::platform_adaptations::default_border_radius(ctx.platform());
        let font_size = ctx.typography().body_medium.font_size;
        let horizontal_padding = ctx.spacing().space_2;
        let text_width = if self.full_width {
            (ui.available_width() - horizontal_padding * 2.0 - radius).max(48.0)
        } else {
            self.min_width.max(ui.spacing().text_edit_width)
        };

        let mut buffer = state.value.clone();

        let frame = egui::Frame::none()
            .fill(eh::token(ctx, &colors.surface))
            .rounding(radius)
            .inner_margin(egui::Margin::symmetric(
                horizontal_padding,
                ctx.spacing().space_1,
            ))
            .stroke(egui::Stroke::new(1.0_f32, eh::token(ctx, &colors.outline)));

        let interactable = !state.disabled;
        let response = frame.show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut buffer)
                    .id(self.egui_id())
                    .hint_text(state.placeholder.as_str())
                    .password(state.password)
                    .interactive(interactable)
                    .font(egui::FontId::proportional(font_size))
                    .text_color_opt(Some(if state.disabled {
                        eh::with_alpha(eh::token(ctx, &colors.on_surface_variant), 0.5)
                    } else {
                        eh::token(ctx, &colors.on_surface)
                    }))
                    .desired_width(text_width)
                    .min_size(egui::vec2(text_width, height))
                    .frame(false),
            )
        });
        let response = response.response;

        if buffer != state.value {
            state.set_value(buffer);
        }

        let enter_pressed = ui.input(|input| input.key_pressed(egui::Key::Enter));
        let submitted = self.submit_on_enter
            && interactable
            && enter_pressed
            && (response.has_focus() || response.lost_focus());

        if submitted {
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
    fn new_field_is_empty_and_enabled() {
        let state = TextFieldState::new("Email");
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);
        assert_eq!(state.value(), "");
        assert_eq!(state.placeholder(), "Email");
        assert!(!state.is_password());
        assert!(!state.is_disabled());
        assert_eq!(state.max_length, None);
        assert!(!state.has_pending_change());
    }

    #[test]
    fn set_value_records_a_single_change() {
        let mut state = TextFieldState::new("Name");
        state.set_value("Ada");
        assert_eq!(state.value(), "Ada");
        assert!(state.has_pending_change());
        assert_eq!(state.take_changed(), Some("Ada".to_string()));
        assert!(!state.has_pending_change());
        assert_eq!(state.take_changed(), None);
    }

    #[test]
    fn setting_the_same_value_twice_is_not_a_change() {
        let mut state = TextFieldState::new("Name");
        state.set_value("Ada");
        assert!(state.take_changed().is_some());

        state.set_value("Ada");
        assert!(!state.has_pending_change());
        assert_eq!(state.take_changed(), None);
    }

    #[test]
    fn set_value_truncates_to_max_length() {
        let mut state = TextFieldState::new("Code").with_max_length(4);
        state.set_value("1234567890");
        assert_eq!(state.value(), "1234");
        assert!(state.is_full());
    }

    #[test]
    fn max_length_zero_allows_nothing() {
        let mut state = TextFieldState::new("Code").with_max_length(0);
        state.set_value("anything");
        assert!(state.is_empty());
        assert!(state.is_full());
    }

    #[test]
    fn lowering_max_length_truncates_on_next_write() {
        let mut state = TextFieldState::new("Code");
        state.set_value("1234");
        assert_eq!(state.take_changed(), Some("1234".to_string()));

        state.max_length = Some(2);
        state.set_value("1234");
        assert_eq!(state.value(), "12");
        assert_eq!(state.take_changed(), Some("12".to_string()));
    }

    #[test]
    fn truncation_respects_char_boundaries() {
        // "äöü" is 3 chars but 6 bytes; slicing must not panic or split a char.
        let mut state = TextFieldState::new("Unicode").with_max_length(2);
        state.set_value("äöü");
        assert_eq!(state.value(), "äö");
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn displayed_value_masks_passwords() {
        let mut state = TextFieldState::new("Password").with_password(true);
        state.set_value("hunter2");
        assert_eq!(state.value(), "hunter2");
        assert_eq!(
            state.displayed_value(),
            "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"
        );

        let plain = TextFieldState::new("Name");
        assert_eq!(plain.displayed_value(), "");
    }

    #[test]
    fn displayed_value_of_empty_password_is_empty() {
        let state = TextFieldState::new("Password").with_password(true);
        assert_eq!(state.displayed_value(), "");
    }

    #[test]
    fn clear_empties_the_value_and_records_the_change() {
        let mut state = TextFieldState::new("Name");
        state.set_value("Ada");
        assert!(state.take_changed().is_some());

        state.clear();
        assert!(state.is_empty());
        assert_eq!(state.take_changed(), Some(String::new()));
        assert_eq!(state.take_changed(), None);
    }

    #[test]
    fn disabled_fields_keep_their_value() {
        let mut state = TextFieldState::new("Name").with_disabled(true);
        state.set_value("Ada");
        assert!(state.is_disabled());
        assert_eq!(state.value(), "Ada");
    }

    #[test]
    fn text_field_builder_defaults() {
        let field = TextField::new();
        assert_eq!(field.label_text(), None);
        assert_eq!(field.test_id_(), None);
        assert!(!field.is_full_width());
        assert!(field.submits_on_enter());
        assert_eq!(field, TextField::default());
    }

    #[test]
    fn text_field_builder_overrides() {
        let field = TextField::new()
            .label("Email")
            .test_id("email")
            .full_width(true)
            .min_width(200.0)
            .submit_on_enter(false);

        assert_eq!(field.label_text(), Some("Email"));
        assert_eq!(field.test_id_(), Some("email"));
        assert!(field.is_full_width());
        assert!(!field.submits_on_enter());
    }

    #[test]
    fn min_width_never_goes_negative() {
        assert_eq!(TextField::new().min_width(-10.0).min_width, 0.0);
    }
}
