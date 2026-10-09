//! Screen scaffold.
//!
//! A scaffold is the frame a screen is drawn into: an optional [`AppBar`] at
//! the top, an optional [`NavigationBar`] at the bottom, and the body that
//! fills whatever is left. Every platform has one, and every platform spells it
//! the same way, so the arithmetic of "what is left" belongs in one place
//! instead of in every screen.
//!
//! The arithmetic is deliberately a plain function of four `f32`s and two
//! optional bar heights — [`Scaffold::layout`] returns a [`ScaffoldLayout`] of
//! [`Slot`]s and needs neither egui nor a widget. Screens can therefore be
//! reasoned about, and unit tested, on a headless machine.
//!
//! ```
//! use shared::domain::Platform;
//! use ui::components::scaffold::{Slot, Scaffold};
//!
//! let scaffold = Scaffold::new();
//! // 800x600, a Material app bar (64) and a Material bottom bar (80).
//! let area = Slot::new(0.0, 0.0, 800.0, 600.0);
//! let slots = scaffold.layout(Platform::Android, area, Some(64.0), Some(80.0));
//!
//! assert_eq!(slots.body().top(), 64.0);
//! assert_eq!(slots.body().height(), 600.0 - 64.0 - 80.0);
//!
//! // A bar that is taller than the window squeezes the body to nothing
//! // instead of drawing the body underneath it.
//! let tiny = Slot::new(0.0, 0.0, 400.0, 100.0);
//! let squeezed = scaffold.layout(Platform::Android, tiny, Some(64.0), Some(80.0));
//! assert!(squeezed.body().is_empty());
//! ```

use shared::domain::Platform;

use super::app_bar::AppBar;
use super::navigation_bar::{NavSelection, NavigationBar};

/// A rectangle inside the scaffold, in screen coordinates.
///
/// A plain four-`f32` struct rather than an [`egui::Rect`] so the scaffold's
/// layout maths is a pure function that compiles and tests with the `egui`
/// feature turned off.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Slot {
    /// A slot at `x, y` with the given size.
    #[must_use]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width: width.max(0.0),
            height: height.max(0.0),
        }
    }

    /// A zero-sized slot at the origin.
    #[must_use]
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// A slot of the given size at the origin.
    #[must_use]
    pub fn sized(width: f32, height: f32) -> Self {
        Self::new(0.0, 0.0, width, height)
    }

    /// The slot moved by `dx` and `dy`.
    #[must_use]
    pub fn translated(self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    /// The slot shrunk on all four sides by `inset`.
    ///
    /// A negative `inset` is a no-op rather than a growth, because growing a
    /// slot on a screen the user is already too small to see makes no sense.
    #[must_use]
    pub fn inset(self, inset: f32) -> Self {
        let inset = inset.max(0.0);
        Self::new(
            self.x + inset,
            self.y + inset,
            self.width - inset * 2.0,
            self.height - inset * 2.0,
        )
    }

    /// Left edge.
    #[must_use]
    pub fn left(&self) -> f32 {
        self.x
    }

    /// Top edge.
    #[must_use]
    pub fn top(&self) -> f32 {
        self.y
    }

    /// Width.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Height.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Right edge.
    #[must_use]
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge.
    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Whether the slot has no area at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Whether `(x, y)` is inside the slot.
    ///
    /// The right and bottom edges are *inside* the slot, so a full-window slot
    /// contains its own far corner — a click one pixel past the edge of the
    /// screen is not inside the screen.
    #[must_use]
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.left() && x <= self.right() && y >= self.top() && y <= self.bottom()
    }
}

/// Where each part of a screen goes.
///
/// The bars are `Option` because a screen without a bar has no bar slot, which
/// is different from a bar that happens to be zero pixels tall this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaffoldLayout {
    app_bar: Option<Slot>,
    navigation_bar: Option<Slot>,
    body: Slot,
}

impl ScaffoldLayout {
    /// Build a layout from its three slots.
    #[must_use]
    pub fn new(app_bar: Option<Slot>, navigation_bar: Option<Slot>, body: Slot) -> Self {
        Self {
            app_bar,
            navigation_bar,
            body,
        }
    }

    /// The app bar's slot, if the screen has one.
    #[must_use]
    pub fn app_bar(&self) -> Option<Slot> {
        self.app_bar
    }

    /// The navigation bar's slot, if the screen has one.
    #[must_use]
    pub fn navigation_bar(&self) -> Option<Slot> {
        self.navigation_bar
    }

    /// The body: everything the bars did not take.
    #[must_use]
    pub fn body(&self) -> Slot {
        self.body
    }
}

/// What a screen did with its scaffold during a frame.
///
/// Collected from the bars so a screen's frame handler reads as one `match`
/// instead of two nested ones.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScaffoldResponse {
    /// The app bar action that was pressed, or [`None`].
    ///
    /// [`super::app_bar::BACK_ACTION`] is reported for the back affordance.
    pub app_bar_action: Option<String>,
    /// What happened in the navigation bar.
    pub navigation: NavSelection,
}

impl ScaffoldResponse {
    /// Build a response from a bar action and a navigation result.
    #[must_use]
    pub fn new(app_bar_action: Option<String>, navigation: NavSelection) -> Self {
        Self {
            app_bar_action,
            navigation,
        }
    }

    /// Whether the back affordance was pressed.
    #[must_use]
    pub fn went_back(&self) -> bool {
        self.app_bar_action.as_deref() == Some(super::app_bar::BACK_ACTION)
    }

    /// Whether nothing at all happened this frame.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.app_bar_action.is_none() && self.navigation.is_none()
    }
}

/// The frame a screen is drawn into.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scaffold {
    app_bar: Option<AppBar>,
    navigation_bar: Option<NavigationBar>,
    body_padding: Option<f32>,
}

impl Scaffold {
    /// A scaffold with no bars and the platform's default body padding.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Put `app_bar` at the top of the screen.
    #[must_use]
    pub fn app_bar(mut self, app_bar: AppBar) -> Self {
        self.app_bar = Some(app_bar);
        self
    }

    /// Put `navigation_bar` at the bottom of the screen.
    #[must_use]
    pub fn navigation_bar(mut self, navigation_bar: NavigationBar) -> Self {
        self.navigation_bar = Some(navigation_bar);
        self
    }

    /// Override the padding around the body.
    ///
    /// Negative values are clamped to zero, so a caller cannot pull the body's
    /// content on top of the bars.
    #[must_use]
    pub fn body_padding(mut self, padding: f32) -> Self {
        self.body_padding = Some(padding.max(0.0));
        self
    }

    /// Padding around the body, for `platform`.
    #[must_use]
    pub fn body_padding_for(&self, platform: Platform) -> f32 {
        self.body_padding
            .unwrap_or_else(|| crate::platform_adaptations::default_padding(platform))
    }

    /// The app bar, if the screen has one.
    #[must_use]
    pub fn app_bar_ref(&self) -> Option<&AppBar> {
        self.app_bar.as_ref()
    }

    /// The navigation bar, if the screen has one.
    #[must_use]
    pub fn navigation_bar_ref(&self) -> Option<&NavigationBar> {
        self.navigation_bar.as_ref()
    }

    /// Whether the screen has an app bar.
    #[must_use]
    pub fn has_app_bar(&self) -> bool {
        self.app_bar.is_some()
    }

    /// Whether the screen has a navigation bar.
    #[must_use]
    pub fn has_navigation_bar(&self) -> bool {
        self.navigation_bar.is_some()
    }

    /// Divide `area` between the bars and the body.
    ///
    /// `app_bar_height` and `navigation_bar_height` are passed in rather than
    /// read off the bars so this stays a pure function: the bar owns *how tall*
    /// it is, and the scaffold owns *where* things go.
    ///
    /// The bars are clamped to the area, so on a window shorter than the two
    /// bars together the body collapses to nothing instead of the bars being
    /// drawn on top of each other.
    #[must_use]
    pub fn layout(
        &self,
        platform: Platform,
        area: Slot,
        app_bar_height: Option<f32>,
        navigation_bar_height: Option<f32>,
    ) -> ScaffoldLayout {
        let app_bar_height = match self.app_bar.as_ref() {
            Some(_) => app_bar_height
                .unwrap_or(AppBar::touch_height_for(platform))
                .max(0.0),
            None => 0.0,
        };
        let navigation_bar_height = match self.navigation_bar.as_ref() {
            Some(_) => navigation_bar_height
                .unwrap_or_else(|| NavigationBar::bar_height(platform))
                .max(0.0),
            None => 0.0,
        };

        // The app bar comes first, so it is the one that survives when the
        // window cannot fit both: a title and a back affordance are how the
        // user gets out, a bottom bar is not.
        let app_bar_slot = self.app_bar.as_ref().map(|_| {
            let height = app_bar_height.min(area.height());
            Slot::new(area.left(), area.top(), area.width(), height)
        });

        let top = area.top() + app_bar_slot.map_or(0.0, |slot| slot.height());
        let remaining = (area.bottom() - top).max(0.0);

        let navigation_bar_slot = self.navigation_bar.as_ref().map(|_| {
            let height = navigation_bar_height.min(remaining);
            Slot::new(area.left(), area.bottom() - height, area.width(), height)
        });

        let body_height =
            (remaining - navigation_bar_slot.map_or(0.0, |slot| slot.height())).max(0.0);
        let body = Slot::new(area.left(), top, area.width(), body_height);

        ScaffoldLayout::new(app_bar_slot, navigation_bar_slot, body)
    }
}

#[cfg(feature = "egui")]
impl Scaffold {
    /// Draw the bars, hand `body` the leftover rect, and report what the user
    /// pressed.
    ///
    /// The scaffold claims the whole of `ui`'s remaining space first and then
    /// carves the bars out of it, so a screen never has to work out where its
    /// own content goes.
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        ctx: &crate::UiContext,
        body: impl FnOnce(&mut egui::Ui),
    ) -> ScaffoldResponse {
        use super::egui_helpers as eh;
        let colors = ctx.color_tokens();
        let platform = ctx.platform();

        let origin = ui.next_widget_position();
        let available = ui.available_size();
        let (area_rect, _) = ui.allocate_exact_size(available, egui::Sense::hover());
        let area = Slot::new(
            area_rect.left() - origin.x,
            area_rect.top() - origin.y,
            area_rect.width(),
            area_rect.height(),
        );

        let slots = self.layout(
            platform,
            area,
            self.app_bar
                .as_ref()
                .map(|_| AppBar::touch_height_for(platform)),
            self.navigation_bar
                .as_ref()
                .map(|_| NavigationBar::bar_height(platform)),
        );

        // The body sits on the window's own background, so a screen that draws
        // nothing is still the theme's background and not a hole.
        ui.painter_at(area_rect).rect(
            area_rect,
            0.0,
            eh::token(ctx, &colors.background),
            egui::Stroke::NONE,
        );

        let mut app_bar_action = None;
        if let (Some(app_bar), Some(slot)) = (self.app_bar.as_ref(), slots.app_bar()) {
            ui.allocate_new_ui(child(slot, origin), |ui| {
                app_bar_action = app_bar.show(ui, ctx);
            });
        }

        let mut navigation = NavSelection::None;
        if let (Some(bar), Some(slot)) = (self.navigation_bar.as_ref(), slots.navigation_bar()) {
            ui.allocate_new_ui(child(slot, origin), |ui| {
                navigation = bar.show(ui, ctx);
            });
        }

        let padded = slots.body().inset(self.body_padding_for(platform));
        if !padded.is_empty() {
            ui.allocate_new_ui(child(padded, origin), body);
        } else if slots.body().is_empty() {
            // Nothing fits; still call `body` so the screen's own state machine
            // runs exactly once per frame, whatever the window is doing.
            body(ui);
        }

        ScaffoldResponse::new(app_bar_action, navigation)
    }
}

/// Turn a slot in scaffold-local coordinates into a screen-space rect.
#[cfg(feature = "egui")]
fn child(slot: Slot, origin: egui::Pos2) -> egui::UiBuilder {
    egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
        egui::pos2(origin.x + slot.left(), origin.y + slot.top()),
        egui::vec2(slot.width(), slot.height()),
    ))
}

#[cfg(test)]
mod tests {
    use super::super::{app_bar::BACK_ACTION, navigation_bar::NavItem};
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    /// [`assert_close`] with a label, for assertions inside a loop over several
    /// platforms where a bare failure would not say which one failed.
    fn assert_close_named(actual: f32, expected: f32, what: &str) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "{what}: expected {expected}, got {actual}"
        );
    }

    fn phone() -> Slot {
        Slot::new(0.0, 0.0, 400.0, 800.0)
    }

    fn bar() -> AppBar {
        AppBar::new("Inbox")
    }

    fn nav() -> NavigationBar {
        NavigationBar::new(vec![
            NavItem::new("home", "Home"),
            NavItem::new("search", "Search"),
        ])
    }

    #[test]
    fn default_scaffold_has_no_bars() {
        let scaffold = Scaffold::new();
        assert!(!scaffold.has_app_bar());
        assert!(!scaffold.has_navigation_bar());
        assert_eq!(Scaffold::default(), scaffold);
    }

    #[test]
    fn slot_edges() {
        let slot = Slot::new(10.0, 20.0, 100.0, 200.0);
        assert_close(slot.left(), 10.0);
        assert_close(slot.top(), 20.0);
        assert_close(slot.width(), 100.0);
        assert_close(slot.height(), 200.0);
        assert_close(slot.right(), 110.0);
        assert_close(slot.bottom(), 220.0);
        assert!(!slot.is_empty());
    }

    #[test]
    fn a_slot_never_has_negative_size() {
        let slot = Slot::new(0.0, 0.0, -5.0, -5.0);
        assert_close(slot.width(), 0.0);
        assert_close(slot.height(), 0.0);
        assert!(slot.is_empty());
    }

    #[test]
    fn zero_and_sized_slots() {
        assert_eq!(Slot::zero(), Slot::new(0.0, 0.0, 0.0, 0.0));
        assert!(Slot::zero().is_empty());
        assert_eq!(Slot::sized(4.0, 5.0), Slot::new(0.0, 0.0, 4.0, 5.0));
    }

    #[test]
    fn insetting_shrinks_on_all_four_sides() {
        let slot = Slot::new(0.0, 0.0, 100.0, 100.0).inset(10.0);
        assert_close(slot.left(), 10.0);
        assert_close(slot.top(), 10.0);
        assert_close(slot.width(), 80.0);
        assert_close(slot.height(), 80.0);
    }

    #[test]
    fn a_negative_inset_does_not_grow_a_slot() {
        let slot = Slot::new(0.0, 0.0, 100.0, 100.0);
        assert_eq!(slot.inset(-10.0), slot);
    }

    #[test]
    fn insetting_more_than_the_slot_collapses_it() {
        let slot = Slot::new(0.0, 0.0, 100.0, 100.0).inset(60.0);
        assert!(slot.is_empty());
        // The top-left corner still moved, so the body does not jump back to
        // the screen edge when it collapses.
        assert_close(slot.top(), 60.0);
    }

    #[test]
    fn translating_moves_without_resizing() {
        let slot = Slot::new(10.0, 10.0, 20.0, 30.0).translated(5.0, -5.0);
        assert_close(slot.left(), 15.0);
        assert_close(slot.top(), 5.0);
        assert_close(slot.width(), 20.0);
        assert_close(slot.height(), 30.0);
    }

    #[test]
    fn hit_testing_includes_the_far_edge() {
        let slot = Slot::new(0.0, 0.0, 100.0, 100.0);
        assert!(slot.contains(0.0, 0.0));
        assert!(slot.contains(100.0, 100.0));
        assert!(slot.contains(50.0, 50.0));
        assert!(!slot.contains(-0.5, 50.0));
        assert!(!slot.contains(50.0, 100.5));
    }

    #[test]
    fn the_bars_take_their_platform_heights_off_the_body() {
        let scaffold = Scaffold::new().app_bar(bar()).navigation_bar(nav());
        let slots = scaffold.layout(Platform::Android, phone(), None, None);

        let app_bar = slots.app_bar().expect("the screen has an app bar");
        assert_close(app_bar.top(), 0.0);
        assert_close(
            app_bar.height(),
            AppBar::touch_height_for(Platform::Android),
        );

        let navigation_bar = slots.navigation_bar().expect("the screen has a bar");
        assert_close(navigation_bar.bottom(), 800.0);
        assert_close(navigation_bar.height(), 80.0);

        let body = slots.body();
        assert_close(body.top(), app_bar.height());
        assert_close(body.height(), 800.0 - 64.0 - 80.0);
        // The body butts up against both bars: no gap, no overlap.
        assert_close(body.bottom(), navigation_bar.top());
    }

    #[test]
    fn the_body_fills_the_window_without_bars() {
        let slots = Scaffold::new().layout(Platform::Android, phone(), None, None);
        assert!(slots.app_bar().is_none());
        assert!(slots.navigation_bar().is_none());
        assert_eq!(slots.body(), phone());
    }

    #[test]
    fn bars_are_dropped_when_the_screen_has_none() {
        let scaffold = Scaffold::new();
        // Even when the caller offers a height, a screen with no bar has no
        // slot: that is what stops a stale height from eating the body.
        let slots = scaffold.layout(Platform::Android, phone(), Some(64.0), Some(80.0));
        assert!(slots.app_bar().is_none());
        assert!(slots.navigation_bar().is_none());
        assert_eq!(slots.body(), phone());
    }

    #[test]
    fn a_window_too_short_for_both_bars_gives_the_room_to_the_bars() {
        // 100px of window cannot hold a 64px app bar and an 80px bar.
        let scaffold = Scaffold::new().app_bar(bar()).navigation_bar(nav());
        let slots = scaffold.layout(Platform::Android, Slot::sized(400.0, 100.0), None, None);

        let app_bar = slots.app_bar().expect("the screen has an app bar");
        let navigation_bar = slots.navigation_bar().expect("the screen has a bar");
        assert_close(app_bar.height(), 64.0);
        // The navigation bar is clipped to what is left rather than drawn on
        // top of the app bar.
        assert_close(navigation_bar.height(), 36.0);
        assert_close(navigation_bar.top(), app_bar.bottom());
        assert!(slots.body().is_empty());
    }

    #[test]
    fn an_app_bar_taller_than_the_window_squeezes_out_the_navigation_bar() {
        let scaffold = Scaffold::new().app_bar(bar()).navigation_bar(nav());
        let slots = scaffold.layout(Platform::Android, Slot::sized(400.0, 50.0), None, None);

        // The app bar survives, because it is how the user gets out.
        assert_close(slots.app_bar().map_or(0.0, |slot| slot.height()), 50.0);
        assert!(slots.navigation_bar().is_some_and(|slot| slot.is_empty()));
        assert!(slots.body().is_empty());
    }

    #[test]
    fn bars_keep_the_full_width_of_the_window() {
        let scaffold = Scaffold::new().app_bar(bar()).navigation_bar(nav());
        let area = Slot::new(12.0, 34.0, 400.0, 800.0);
        let slots = scaffold.layout(Platform::Android, area, None, None);

        assert_close(slots.app_bar().map_or(0.0, |slot| slot.left()), 12.0);
        assert_close(slots.app_bar().map_or(0.0, |slot| slot.width()), 400.0);
        assert_close(slots.body().left(), 12.0);
        assert_close(slots.body().width(), 400.0);
    }

    #[test]
    fn an_explicit_bar_height_wins_over_the_platform_default() {
        let scaffold = Scaffold::new().app_bar(bar()).navigation_bar(nav());
        let slots = scaffold.layout(Platform::Android, phone(), Some(100.0), Some(20.0));
        assert_close(slots.app_bar().map_or(0.0, |slot| slot.height()), 100.0);
        assert_close(
            slots.navigation_bar().map_or(0.0, |slot| slot.height()),
            20.0,
        );
        assert_close(slots.body().height(), 800.0 - 100.0 - 20.0);
    }

    #[test]
    fn each_platform_gets_its_own_bar_heights() {
        let scaffold = Scaffold::new().app_bar(bar()).navigation_bar(nav());
        for (platform, app_bar, navigation_bar) in [
            (Platform::Android, 64.0, 80.0),
            (Platform::Ios, 48.0, 49.0),
            (Platform::Macos, 48.0, 64.0),
            (Platform::Linux, 48.0, 64.0),
            (Platform::Windows, 48.0, 64.0),
            (Platform::Web, 64.0, 80.0),
        ] {
            let slots = scaffold.layout(platform, phone(), None, None);
            assert_close_named(
                slots.app_bar().map_or(0.0, |slot| slot.height()),
                app_bar,
                &format!("{platform:?} app bar"),
            );
            assert_close_named(
                slots.navigation_bar().map_or(0.0, |slot| slot.height()),
                navigation_bar,
                &format!("{platform:?} navigation bar"),
            );
            assert_close_named(
                slots.body().height(),
                800.0 - app_bar - navigation_bar,
                &format!("{platform:?} body"),
            );
        }
    }

    #[test]
    fn body_padding_falls_back_to_the_platform_default() {
        assert_close(Scaffold::new().body_padding_for(Platform::Android), 16.0);
        assert_close(Scaffold::new().body_padding_for(Platform::Ios), 20.0);
        assert_close(Scaffold::new().body_padding_for(Platform::Windows), 12.0);
    }

    #[test]
    fn body_padding_can_be_overridden() {
        assert_close(
            Scaffold::new()
                .body_padding(0.0)
                .body_padding_for(Platform::Ios),
            0.0,
        );
        assert_close(
            Scaffold::new()
                .body_padding(32.0)
                .body_padding_for(Platform::Ios),
            32.0,
        );
    }

    #[test]
    fn a_negative_body_padding_is_clamped_to_zero() {
        assert_close(
            Scaffold::new()
                .body_padding(-20.0)
                .body_padding_for(Platform::Ios),
            0.0,
        );
    }

    #[test]
    fn the_body_still_gets_room_for_its_padding_on_a_normal_window() {
        let scaffold = Scaffold::new().body_padding(16.0);
        let slots = scaffold.layout(Platform::Android, phone(), None, None);
        let inner = slots
            .body()
            .inset(scaffold.body_padding_for(Platform::Android));
        assert_eq!(inner, Slot::new(16.0, 16.0, 368.0, 768.0));
    }

    #[test]
    fn the_builder_keeps_both_bars() {
        let scaffold = Scaffold::new()
            .app_bar(bar())
            .navigation_bar(nav())
            .body_padding(8.0);
        assert!(scaffold.has_app_bar());
        assert!(scaffold.has_navigation_bar());
        assert_eq!(scaffold.app_bar_ref().map(AppBar::title), Some("Inbox"));
        assert_eq!(
            scaffold.navigation_bar_ref().map(NavigationBar::len),
            Some(2)
        );
    }

    #[test]
    fn scaffold_response_reports_the_back_action() {
        let response = ScaffoldResponse::new(Some(BACK_ACTION.to_string()), NavSelection::None);
        assert!(response.went_back());
        assert!(!response.is_none());

        let other = ScaffoldResponse::new(Some("Archive".to_string()), NavSelection::None);
        assert!(!other.went_back());
    }

    #[test]
    fn an_idle_scaffold_response_is_none() {
        let response = ScaffoldResponse::default();
        assert!(response.is_none());
        assert!(!response.went_back());
        assert_eq!(
            ScaffoldResponse::new(None, NavSelection::None),
            ScaffoldResponse::default()
        );
    }

    #[test]
    fn a_navigation_press_is_not_none() {
        let response = ScaffoldResponse::new(None, NavSelection::Item(1));
        assert!(!response.is_none());
        assert_eq!(response.navigation, NavSelection::Item(1));
    }
}
