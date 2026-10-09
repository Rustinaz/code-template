//! Navigation bar component.
//!
//! The navigation bar is the strip along the bottom (Material, the web) or the
//! side (a desktop navigation rail is the same idea rotated) of a screen that
//! switches between top-level destinations.
//!
//! Like [`AppBar`](super::app_bar::AppBar), only the painting is egui-bound.
//! The parts that are easy to get wrong — which item is selected, how many fit
//! before the bar has to overflow, how wide each destination ends up, how tall
//! the bar is on a given OS — are pure functions of the bar's own state, so
//! they can be unit tested without a backend.
//!
//! ```
//! use ui::components::navigation_bar::{NavItem, NavigationBar};
//! use shared::domain::Platform;
//!
//! let mut bar = NavigationBar::new(vec![
//!     NavItem::new("home", "Home").icon('⌂'),
//!     NavItem::new("search", "Search").icon('🔍'),
//!     NavItem::new("settings", "Settings").icon('⚙'),
//! ]);
//! bar.select(Some(0));
//!
//! // Selection wraps, so keyboard and D-pad navigation can never dead-end.
//! bar.select_next();
//! assert_eq!(bar.selected(), Some(1));
//! bar.select_previous();
//! bar.select_previous();
//! assert_eq!(bar.selected(), Some(2));
//!
//! // Material's bar is tall enough for an icon *and* a label.
//! assert_eq!(NavigationBar::bar_height(Platform::Android), 80.0);
//! // The iOS tab bar is the short one.
//! assert_eq!(NavigationBar::bar_height(Platform::Ios), 49.0);
//! ```

use shared::domain::Platform;

/// Most destinations a bar shows before the rest collapse into an overflow
/// menu.
///
/// Five is what Material's specification allows for a bottom bar on a phone:
/// a sixth destination would push the items below 72dp on a 360dp-wide screen,
/// which is under the touch-target floor.
pub const MAX_VISIBLE_ITEMS: usize = 5;

/// Narrowest a destination is ever drawn.
const MIN_ITEM_WIDTH: f32 = 56.0;

/// Widest a destination is ever drawn, even with room to spare.
///
/// A bar with two items and a 900dp window should not produce two 450dp-wide
/// buttons with a two-character label in the middle of each.
const MAX_ITEM_WIDTH: f32 = 120.0;

/// Glyph drawn on the overflow affordance.
const OVERFLOW_ICON: char = '\u{22ef}';

/// Stable id of the overflow affordance, so it can never collide with a
/// destination's own id.
const OVERFLOW_ID: &str = "\u{1}overflow";

/// What the user did with the bar during a frame.
///
/// Returned by [`NavigationBar::show`] instead of a bare `Option<usize>` so the
/// "the user asked for the overflow menu" case is not indistinguishable from
/// "the user picked a destination".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavSelection {
    /// Nothing was pressed this frame.
    #[default]
    None,
    /// The destination at this index in [`NavigationBar::items`] was pressed.
    Item(usize),
    /// The overflow affordance was pressed; open a menu of the hidden items.
    Overflow,
}

impl NavSelection {
    /// The index that was pressed, if a visible destination was pressed.
    #[must_use]
    pub fn index(&self) -> Option<usize> {
        match *self {
            Self::Item(index) => Some(index),
            Self::None | Self::Overflow => None,
        }
    }

    /// Whether nothing at all happened this frame.
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// One destination in a [`NavigationBar`].
///
/// The `id` is what the caller matches a selection back against, so it is kept
/// separate from `label`: the label is localised, the id is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavItem {
    id: String,
    label: String,
    icon: Option<char>,
    enabled: bool,
    badge: Option<u32>,
}

impl NavItem {
    /// Create an enabled destination showing `label`.
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            enabled: true,
            badge: None,
        }
    }

    /// Set the icon glyph.
    #[must_use]
    pub fn icon(mut self, icon: char) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Enable or disable the destination.
    ///
    /// A disabled destination is still drawn (so the bar does not resize as
    /// items come and go) but never reports a selection.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Show a count badge on the destination.
    #[must_use]
    pub fn badge(mut self, badge: u32) -> Self {
        self.badge = Some(badge);
        self
    }

    /// The stable id of the destination.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The visible label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The icon glyph, if one was set.
    #[must_use]
    pub fn icon_(&self) -> Option<char> {
        self.icon
    }

    /// Whether the destination responds to input.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// The count badge, if one was set.
    #[must_use]
    pub fn badge_count(&self) -> Option<u32> {
        self.badge
    }

    /// The text a screen reader reads for this destination.
    ///
    /// `selected` is appended rather than left implicit: a screen reader user
    /// tabbing through a bar hears the selected state once per destination
    /// instead of having to remember which one was picked.
    #[must_use]
    pub fn accessible_label(&self, selected: bool) -> String {
        let mut label = self.label.clone();
        if let Some(badge) = self.badge {
            label.push_str(&format!(", {badge}"));
        }
        if selected {
            label.push_str(", selected");
        }
        label
    }
}

/// A platform-adaptive strip of top-level destinations.
///
/// [`NavigationBar::show`] only *reports* a press; the bar never mutates its
/// own selection, because the caller owns the navigation state and has to
/// update the rest of the screen to match.
#[derive(Debug, Clone, PartialEq)]
pub struct NavigationBar {
    items: Vec<NavItem>,
    selected: Option<usize>,
    show_labels: bool,
    max_visible: usize,
}

impl Default for NavigationBar {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl NavigationBar {
    /// Create a bar over `items` with nothing selected and labels shown.
    #[must_use]
    pub fn new(items: Vec<NavItem>) -> Self {
        Self {
            items,
            selected: None,
            show_labels: true,
            max_visible: MAX_VISIBLE_ITEMS,
        }
    }

    /// Replace the destinations.
    ///
    /// A selection that points past the end of the new list is dropped, so
    /// swapping the items of a bar can never leave it pointing at nothing.
    #[must_use]
    pub fn items(mut self, items: Vec<NavItem>) -> Self {
        self.items = items;
        self.selected = self.selected.filter(|index| *index < self.items.len());
        self
    }

    /// Select the destination at `index`, or [`None`] to clear the selection.
    ///
    /// Out-of-range indices and disabled destinations are ignored rather than
    /// stored, so [`NavigationBar::selected`] only ever names a destination that
    /// can actually be drawn as selected.
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index.filter(|index| self.is_selectable(*index));
    }

    /// Move the selection to the next destination, wrapping at the end.
    ///
    /// A no-op when there is nothing to select, so a bar with no items does not
    /// need a guard at the call site.
    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let next = match self.selected {
            None => 0,
            Some(current) if current + 1 >= self.items.len() => 0,
            Some(current) => current + 1,
        };
        self.selected = Some(next);
    }

    /// Move the selection to the previous destination, wrapping at the start.
    pub fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let previous = match self.selected {
            None => self.items.len() - 1,
            Some(0) => self.items.len() - 1,
            Some(current) => current - 1,
        };
        self.selected = Some(previous);
    }

    /// Show the text label under every icon.
    ///
    /// Turning labels off leaves icons only, which is what a navigation rail on
    /// a short desktop window uses.
    #[must_use]
    pub fn show_labels(mut self, show_labels: bool) -> Self {
        self.show_labels = show_labels;
        self
    }

    /// Override how many destinations fit before the rest overflow.
    ///
    /// Clamped to at least one, because a bar that can display nothing has no
    /// way to let the user reach its destinations.
    #[must_use]
    pub fn max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible.max(1);
        self
    }

    /// Height of the bar for `platform`.
    ///
    /// Material's bottom navigation bar is 80dp because it stacks a 24dp icon
    /// over a label, the iOS tab bar is 49pt, and a desktop rail is a compact
    /// 64px row.
    #[must_use]
    pub fn bar_height(platform: Platform) -> f32 {
        match platform {
            Platform::Android | Platform::Web => 80.0,
            Platform::Ios => 49.0,
            Platform::Linux | Platform::Windows | Platform::Macos => 64.0,
        }
    }

    /// Number of destinations the bar shows before overflowing.
    #[must_use]
    pub fn visible_len(&self) -> usize {
        self.items.len().min(self.max_visible)
    }

    /// How many destinations overflow into a menu.
    #[must_use]
    pub fn overflow_len(&self) -> usize {
        self.items.len().saturating_sub(self.visible_len())
    }

    /// Whether any destination is hidden behind the overflow affordance.
    #[must_use]
    pub fn has_overflow(&self) -> bool {
        self.overflow_len() > 0
    }

    /// Width of one destination inside `available_width`.
    ///
    /// Fills the width when the destinations are narrow, caps at
    /// [`MAX_ITEM_WIDTH`] when they are few, and never drops below
    /// [`MIN_ITEM_WIDTH`] — an over-wide window should not shrink the bar
    /// below the touch-target floor.
    #[must_use]
    pub fn item_width(available_width: f32, visible: usize) -> f32 {
        if visible == 0 {
            return 0.0;
        }
        let share = available_width / visible as f32;
        share
            .clamp(MIN_ITEM_WIDTH, MAX_ITEM_WIDTH)
            .min(available_width.max(0.0))
    }

    /// Whether the destination at `index` can be drawn as selected.
    ///
    /// A disabled destination is skipped rather than shown unselected, so a
    /// selection never lands on a control the user cannot activate.
    #[must_use]
    pub fn is_selectable(&self, index: usize) -> bool {
        self.items.get(index).is_some_and(NavItem::is_enabled)
    }

    /// The currently selected index.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// The id of the currently selected destination.
    #[must_use]
    pub fn selected_id(&self) -> Option<&str> {
        self.selected
            .and_then(|index| self.items.get(index))
            .map(NavItem::id)
    }

    /// The destination at `index`.
    #[must_use]
    pub fn item(&self, index: usize) -> Option<&NavItem> {
        self.items.get(index)
    }

    /// Every destination, including the ones that overflow.
    #[must_use]
    pub fn items_(&self) -> &[NavItem] {
        &self.items
    }

    /// Number of destinations, including the ones that overflow.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the bar has no destinations at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Whether the text label is painted under each icon.
    #[must_use]
    pub fn has_labels(&self) -> bool {
        self.show_labels
    }

    /// Whether `index` is drawn as the selected destination.
    #[must_use]
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected == Some(index)
    }

    /// The text a screen reader reads when focus enters the bar as a whole.
    #[must_use]
    pub fn accessible_label(&self) -> String {
        match self.selected_id() {
            Some(id) => format!("Navigation, {id} selected"),
            None => "Navigation".to_string(),
        }
    }

    /// Distance from the top of the bar to the centre of the icon — and of the
    /// selection pill, which only ever wraps the icon.
    ///
    /// Sits above the middle of the bar when labels are shown, because
    /// centring the pill over the icon *and* the label pushes it down onto the
    /// gap between them.
    #[must_use]
    pub fn icon_center_from_top(&self, height: f32) -> f32 {
        if self.show_labels {
            height * 0.32
        } else {
            height * 0.5
        }
    }

    /// Distance from the top of the bar to the centre of the label.
    ///
    /// Matches the icon's position when labels are hidden, so the cell does
    /// not have a stray line of text on it.
    #[must_use]
    pub fn label_center_from_top(&self, height: f32) -> f32 {
        if self.show_labels {
            height * 0.78
        } else {
            height * 0.5
        }
    }
}

#[cfg(feature = "egui")]
impl NavigationBar {
    /// Draw the bar into `ui` and report what the user pressed.
    ///
    /// A press on an enabled destination comes back as
    /// [`NavSelection::Item`]; a press on the overflow affordance (drawn only
    /// when destinations do not fit) as [`NavSelection::Overflow`]. The bar
    /// never changes its own selection — the caller does that with
    /// [`NavigationBar::select`] and rebuilds the screen.
    pub fn show(&self, ui: &mut egui::Ui, ctx: &crate::UiContext) -> NavSelection {
        use super::egui_helpers as eh;
        let colors = ctx.color_tokens();
        let height = Self::bar_height(ctx.platform());
        let visible = self.visible_len();
        let available_width = ui.available_width();
        let width = Self::item_width(available_width, visible);
        // Material's active indicator is a fully rounded pill, so twice the
        // component radius is already the pill's own radius.
        let indicator: egui::Rounding =
            (crate::platform_adaptations::default_border_radius(ctx.platform()) * 2.0).into();
        let icon_center = self.icon_center_from_top(height);
        let label_center = self.label_center_from_top(height);
        // `FontId` is not `Copy` in egui 0.29, so the sizes are kept and the
        // ids rebuilt per call site.
        let icon_size = ctx.typography().body_large.font_size;
        let label_size = ctx.typography().label_medium.font_size;

        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(available_width, height), egui::Sense::hover());
        ui.painter().rect(
            rect,
            0.0,
            eh::token(ctx, &colors.surface),
            egui::Stroke::NONE,
        );
        // Material separates the bar from the content above it with a hairline
        // instead of a shadow; only one of the two, or it reads as a seam.
        ui.painter().hline(
            rect.left()..=rect.right(),
            rect.top(),
            egui::Stroke::new(1.0_f32, eh::token(ctx, &colors.outline_variant)),
        );

        let on_surface = eh::token(ctx, &colors.on_surface);
        let inactive = eh::token(ctx, &colors.on_surface_variant);
        let active_fill = eh::token(ctx, &colors.secondary_container);
        let mut pressed = NavSelection::None;

        // Destinations are laid out from the leading edge so the selection
        // indicator does not shift as items are added or removed.
        for index in 0..visible {
            let item = match self.items.get(index) {
                Some(item) => item,
                None => break,
            };
            let cell = egui::Rect::from_min_size(
                egui::pos2(rect.left() + index as f32 * width, rect.min.y),
                egui::vec2(width, height),
            );
            let selected = self.is_selected(index);

            if selected {
                // Material 3 marks the active destination with a pill behind
                // the icon, not by recolouring the whole cell.
                let pill = egui::Rect::from_center_size(
                    egui::pos2(cell.center().x, rect.min.y + icon_center),
                    egui::vec2(width * 0.6, height * 0.42),
                );
                ui.painter().rect_filled(pill, indicator, active_fill);
            }

            let label_color = if selected { on_surface } else { inactive };
            if let Some(icon) = item.icon {
                ui.painter().text(
                    egui::pos2(cell.center().x, rect.min.y + icon_center),
                    egui::Align2::CENTER_CENTER,
                    icon.to_string(),
                    egui::FontId::proportional(icon_size),
                    label_color,
                );
            }
            if self.show_labels {
                ui.painter().text(
                    egui::pos2(cell.center().x, rect.min.y + label_center),
                    egui::Align2::CENTER_CENTER,
                    item.label(),
                    egui::FontId::proportional(label_size),
                    label_color,
                );
            }
            if let Some(badge) = item.badge_count() {
                ui.painter().text(
                    egui::pos2(cell.right() - 6.0, cell.top() + 6.0),
                    egui::Align2::CENTER_CENTER,
                    badge.to_string(),
                    egui::FontId::proportional(label_size),
                    on_surface,
                );
            }

            let id = egui::Id::new(("ui_components::NavigationBar", item.id()));
            let sense = if item.is_enabled() {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            };
            let mut response = ui.interact(cell, id, sense);
            if item.is_enabled() {
                response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
            }
            response.widget_info(|| {
                let mut info = egui::WidgetInfo::new(egui::WidgetType::Button);
                info.label = Some(item.accessible_label(selected));
                info.enabled = item.is_enabled();
                info
            });
            if item.is_enabled() && response.clicked() {
                pressed = NavSelection::Item(index);
            }
        }

        if self.has_overflow() {
            let cell = egui::Rect::from_min_size(
                egui::pos2(rect.left() + visible as f32 * width, rect.min.y),
                egui::vec2(width, height),
            );
            ui.painter().text(
                egui::pos2(cell.center().x, rect.min.y + icon_center),
                egui::Align2::CENTER_CENTER,
                OVERFLOW_ICON.to_string(),
                egui::FontId::proportional(icon_size),
                inactive,
            );
            let id = egui::Id::new(("ui_components::NavigationBar", OVERFLOW_ID));
            let mut response = ui.interact(cell, id, egui::Sense::click());
            response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
            response.widget_info(|| {
                let mut info = egui::WidgetInfo::new(egui::WidgetType::Button);
                info.label = Some(format!("More destinations, {} hidden", self.overflow_len()));
                info
            });
            if response.clicked() {
                pressed = NavSelection::Overflow;
            }
        }

        pressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::button::MIN_TOUCH_TARGET;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn bar_with_three() -> NavigationBar {
        NavigationBar::new(vec![
            NavItem::new("home", "Home").icon('\u{2302}'),
            NavItem::new("search", "Search").icon('\u{1f50d}'),
            NavItem::new("settings", "Settings").icon('\u{2699}'),
        ])
    }

    fn items(count: usize) -> NavigationBar {
        NavigationBar::new(
            (0..count)
                .map(|index| NavItem::new(format!("id{index}"), format!("Item {index}")))
                .collect(),
        )
    }

    #[test]
    fn new_bar_has_no_selection_and_shows_labels() {
        let bar = bar_with_three();
        assert_eq!(bar.len(), 3);
        assert!(!bar.is_empty());
        assert_eq!(bar.selected(), None);
        assert_eq!(bar.selected_id(), None);
        assert!(bar.has_labels());
    }

    #[test]
    fn default_bar_is_empty() {
        let bar = NavigationBar::default();
        assert!(bar.is_empty());
        assert_eq!(NavigationBar::new(Vec::new()), bar);
    }

    #[test]
    fn nav_item_builder_sets_fields() {
        let item = NavItem::new("inbox", "Inbox")
            .icon('\u{2709}')
            .badge(3)
            .enabled(false);
        assert_eq!(item.id(), "inbox");
        assert_eq!(item.label(), "Inbox");
        assert_eq!(item.icon_(), Some('\u{2709}'));
        assert_eq!(item.badge_count(), Some(3));
        assert!(!item.is_enabled());
    }

    #[test]
    fn nav_item_defaults() {
        let item = NavItem::new("home", "Home");
        assert_eq!(item.icon_(), None);
        assert_eq!(item.badge_count(), None);
        assert!(item.is_enabled());
    }

    #[test]
    fn select_stores_a_valid_index() {
        let mut bar = bar_with_three();
        bar.select(Some(2));
        assert_eq!(bar.selected(), Some(2));
        assert_eq!(bar.selected_id(), Some("settings"));
        assert!(bar.is_selected(2));
        assert!(!bar.is_selected(1));

        bar.select(None);
        assert_eq!(bar.selected(), None);
    }

    #[test]
    fn select_ignores_an_out_of_range_index() {
        let mut bar = bar_with_three();
        bar.select(Some(9));
        assert_eq!(bar.selected(), None);
    }

    #[test]
    fn select_ignores_a_disabled_destination() {
        let mut bar = NavigationBar::new(vec![
            NavItem::new("home", "Home"),
            NavItem::new("admin", "Admin").enabled(false),
        ]);
        bar.select(Some(1));
        assert_eq!(bar.selected(), None);
        assert!(!bar.is_selectable(1));
        assert!(bar.is_selectable(0));
        assert!(!bar.is_selectable(7));
    }

    #[test]
    fn select_next_starts_at_the_first_destination() {
        let mut bar = bar_with_three();
        bar.select_next();
        assert_eq!(bar.selected(), Some(0));
        bar.select_next();
        assert_eq!(bar.selected(), Some(1));
    }

    #[test]
    fn select_next_wraps_past_the_end() {
        let mut bar = bar_with_three();
        bar.select(Some(2));
        bar.select_next();
        assert_eq!(bar.selected(), Some(0));
    }

    #[test]
    fn select_previous_wraps_past_the_start() {
        let mut bar = bar_with_three();
        bar.select(Some(0));
        bar.select_previous();
        assert_eq!(bar.selected(), Some(2));
    }

    #[test]
    fn select_previous_starts_at_the_last_destination() {
        let mut bar = bar_with_three();
        bar.select_previous();
        assert_eq!(bar.selected(), Some(2));
    }

    #[test]
    fn stepping_an_empty_bar_does_nothing() {
        let mut bar = NavigationBar::new(Vec::new());
        bar.select_next();
        assert_eq!(bar.selected(), None);
        bar.select_previous();
        assert_eq!(bar.selected(), None);
    }

    #[test]
    fn overflow_starts_beyond_five_destinations() {
        let mut bar = items(MAX_VISIBLE_ITEMS);
        assert_eq!(bar.len(), MAX_VISIBLE_ITEMS);
        assert_eq!(bar.visible_len(), MAX_VISIBLE_ITEMS);
        assert_eq!(bar.overflow_len(), 0);
        assert!(!bar.has_overflow());

        bar = items(MAX_VISIBLE_ITEMS + 1);
        assert_eq!(bar.overflow_len(), 1);
        assert!(bar.has_overflow());
    }

    #[test]
    fn every_item_is_still_reachable_when_the_bar_overflows() {
        let mut bar = items(8);
        assert_eq!(bar.len(), 8);
        assert_eq!(bar.visible_len(), MAX_VISIBLE_ITEMS);
        assert_eq!(bar.overflow_len(), 3);
        // Overflow only changes the painting, never the addressing.
        assert_eq!(bar.item(7).map(NavItem::id), Some("id7"));

        bar.select(Some(7));
        assert_eq!(bar.selected(), Some(7));
    }

    #[test]
    fn max_visible_override_is_clamped_to_at_least_one() {
        let bar = bar_with_three().max_visible(0);
        assert_eq!(bar.visible_len(), 1);
        assert_eq!(bar.overflow_len(), 2);

        let bar = bar_with_three().max_visible(2);
        assert_eq!(bar.visible_len(), 2);
        assert_eq!(bar.overflow_len(), 1);
    }

    #[test]
    fn item_width_splits_the_available_width() {
        // Room to go round: every destination gets an exact share.
        assert_close(NavigationBar::item_width(500.0, 5), 100.0);
        assert_close(NavigationBar::item_width(900.0, 9), 100.0);
        // Too much room: the cap keeps two destinations from becoming two
        // 450dp-wide buttons.
        assert_close(NavigationBar::item_width(900.0, 5), 120.0);
        assert_close(NavigationBar::item_width(360.0, 3), 120.0);
    }

    #[test]
    fn item_width_never_drops_below_the_touch_target() {
        // Five destinations in a 200px window would be 40px each, under the
        // 48px floor, so the bar overflows-width rather than shrinking.
        assert!(NavigationBar::item_width(200.0, 5) >= MIN_TOUCH_TARGET);
        assert_close(NavigationBar::item_width(200.0, 5), MIN_ITEM_WIDTH);
    }

    #[test]
    fn item_width_is_zero_without_a_visible_destination() {
        assert_close(NavigationBar::item_width(800.0, 0), 0.0);
    }

    #[test]
    fn item_width_never_exceeds_the_available_width() {
        // A window narrower than one destination's floor still gets clamped to
        // the window, or the bar overflows its own parent.
        assert_close(NavigationBar::item_width(30.0, 1), 30.0);
    }

    #[test]
    fn bar_height_follows_each_platform() {
        assert_close(NavigationBar::bar_height(Platform::Android), 80.0);
        assert_close(NavigationBar::bar_height(Platform::Web), 80.0);
        assert_close(NavigationBar::bar_height(Platform::Ios), 49.0);
        assert_close(NavigationBar::bar_height(Platform::Linux), 64.0);
        assert_close(NavigationBar::bar_height(Platform::Windows), 64.0);
        assert_close(NavigationBar::bar_height(Platform::Macos), 64.0);
    }

    #[test]
    fn the_icon_sits_above_the_middle_when_a_label_is_shown() {
        let bar = NavigationBar::new(vec![NavItem::new("home", "Home")]);
        let height = NavigationBar::bar_height(Platform::Android);
        // Icon and pill share a position, because the pill only wraps the icon.
        assert!(bar.icon_center_from_top(height) < height * 0.5);
        assert!(bar.label_center_from_top(height) > height * 0.5);
        assert!(bar.icon_center_from_top(height) < bar.label_center_from_top(height));
    }

    #[test]
    fn hiding_labels_centres_the_icon() {
        let bar = NavigationBar::new(vec![NavItem::new("home", "Home")]);
        let height = NavigationBar::bar_height(Platform::Android);
        let icon_only = bar.show_labels(false);
        assert_close(icon_only.icon_center_from_top(height), height * 0.5);
        // Nothing is painted where the label used to be.
        assert_close(icon_only.label_center_from_top(height), height * 0.5);
    }

    #[test]
    fn replacing_the_items_drops_a_dangling_selection() {
        let mut bar = bar_with_three();
        bar.select(Some(2));
        bar = bar.items(vec![NavItem::new("home", "Home")]);
        assert_eq!(bar.selected(), None);

        bar.select(Some(0));
        bar = bar.items(vec![
            NavItem::new("home", "Home"),
            NavItem::new("search", "Search"),
        ]);
        assert_eq!(bar.selected(), Some(0));
    }

    #[test]
    fn accessible_label_appends_the_selection_state() {
        let home = NavItem::new("home", "Home");
        assert_eq!(home.accessible_label(false), "Home");
        assert_eq!(home.accessible_label(true), "Home, selected");

        let badge = NavItem::new("inbox", "Inbox").badge(3);
        assert_eq!(badge.accessible_label(false), "Inbox, 3");
        assert_eq!(badge.accessible_label(true), "Inbox, 3, selected");
    }

    #[test]
    fn bar_announces_the_selected_destination_by_id() {
        assert_eq!(bar_with_three().accessible_label(), "Navigation");

        let mut bar = bar_with_three();
        bar.select(Some(1));
        assert_eq!(bar.accessible_label(), "Navigation, search selected");
    }

    #[test]
    fn nav_selection_reports_its_index() {
        assert_eq!(NavSelection::None.index(), None);
        assert_eq!(NavSelection::Overflow.index(), None);
        assert_eq!(NavSelection::Item(2).index(), Some(2));
        assert!(NavSelection::None.is_none());
        assert!(!NavSelection::Item(0).is_none());
        assert_eq!(NavSelection::default(), NavSelection::None);
    }
}
