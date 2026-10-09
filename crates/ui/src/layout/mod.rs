//! Layout system for responsive UI
//!
//! This module provides layout primitives that adapt to different
//! screen sizes, orientations, and form factors.

use crate::platform_adaptations::{default_padding, safe_area_insets};
use crate::responsive::{font_scale, grid_columns, is_compact, is_two_pane, spacing_multiplier};
use shared::domain::{DeviceInfo, Orientation, ScreenSize};

/// Layout constraints
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConstraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl Default for LayoutConstraints {
    fn default() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }
}

/// Layout direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    Ltr,
    Rtl,
}

/// Flex layout properties
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlexLayout {
    pub direction: FlexDirection,
    pub justify: JustifyContent,
    pub align_items: AlignItems,
    pub align_content: AlignContent,
    pub wrap: FlexWrap,
    pub gap: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    FlexStart,
    Center,
    FlexEnd,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    FlexStart,
    Center,
    FlexEnd,
    Stretch,
    Baseline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignContent {
    FlexStart,
    Center,
    FlexEnd,
    SpaceBetween,
    SpaceAround,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

impl Default for FlexLayout {
    fn default() -> Self {
        Self {
            direction: FlexDirection::Column,
            justify: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            align_content: AlignContent::Stretch,
            wrap: FlexWrap::NoWrap,
            gap: 0.0,
        }
    }
}

/// Grid layout properties
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridLayout {
    pub columns: usize,
    pub rows: usize,
    pub column_gap: f32,
    pub row_gap: f32,
    pub auto_flow: GridAutoFlow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridAutoFlow {
    Row,
    Column,
    Dense,
}

impl Default for GridLayout {
    fn default() -> Self {
        Self {
            columns: 1,
            rows: 1,
            column_gap: 16.0,
            row_gap: 16.0,
            auto_flow: GridAutoFlow::Row,
        }
    }
}

/// Responsive layout calculator
#[derive(Clone)]
pub struct LayoutCalculator {
    device_info: DeviceInfo,
}

impl LayoutCalculator {
    pub fn new(device_info: DeviceInfo) -> Self {
        Self { device_info }
    }

    /// Calculate grid columns for responsive layout
    pub fn grid_columns(&self, base_columns: usize) -> usize {
        grid_columns(self.device_info.screen_size, base_columns)
    }

    /// Calculate spacing for current form factor
    pub fn spacing(&self, base: f32) -> f32 {
        base * spacing_multiplier(self.device_info.form_factor)
    }

    /// Calculate font scale for current form factor
    pub fn font_scale(&self, base: f32) -> f32 {
        base * font_scale(self.device_info.form_factor)
    }

    /// Check if should use compact layout
    pub fn is_compact(&self) -> bool {
        is_compact(&self.device_info)
    }

    /// Check if should use two-pane layout
    pub fn is_two_pane(&self) -> bool {
        is_two_pane(&self.device_info)
    }

    /// Get safe area insets
    pub fn safe_area(&self) -> (f32, f32, f32, f32) {
        safe_area_insets(&self.device_info)
    }

    /// Get default padding for platform
    pub fn default_padding(&self) -> f32 {
        default_padding(self.device_info.platform)
    }

    /// Calculate responsive width
    pub fn responsive_width(&self, constraints: LayoutConstraints, fraction: f32) -> f32 {
        let available = constraints.max_width.min(f32::INFINITY);
        (available * fraction).clamp(constraints.min_width, constraints.max_width)
    }

    /// Calculate responsive height
    pub fn responsive_height(&self, constraints: LayoutConstraints, fraction: f32) -> f32 {
        let available = constraints.max_height.min(f32::INFINITY);
        (available * fraction).clamp(constraints.min_height, constraints.max_height)
    }

    /// Get breakpoint name for current screen size
    pub fn breakpoint(&self) -> &'static str {
        match self.device_info.screen_size {
            ScreenSize::Small => "sm",
            ScreenSize::Medium => "md",
            ScreenSize::Large => "lg",
            ScreenSize::ExtraLarge => "xl",
        }
    }

    /// Check if landscape orientation
    pub fn is_landscape(&self) -> bool {
        self.device_info.orientation == Orientation::Landscape
    }

    /// Check if portrait orientation
    pub fn is_portrait(&self) -> bool {
        self.device_info.orientation == Orientation::Portrait
    }
}

/// Layout builder for declarative layout construction
pub struct LayoutBuilder {
    calculator: LayoutCalculator,
    direction: LayoutDirection,
}

impl LayoutBuilder {
    pub fn new(device_info: DeviceInfo) -> Self {
        Self {
            calculator: LayoutCalculator::new(device_info),
            direction: LayoutDirection::Ltr,
        }
    }

    pub fn direction(mut self, direction: LayoutDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn flex(&self) -> FlexLayoutBuilder {
        FlexLayoutBuilder::new(self.calculator.clone())
    }

    pub fn grid(&self, base_columns: usize) -> GridLayoutBuilder {
        GridLayoutBuilder::new(self.calculator.clone(), base_columns)
    }

    pub fn constraints(
        &self,
        min_width: f32,
        max_width: f32,
        min_height: f32,
        max_height: f32,
    ) -> LayoutConstraints {
        LayoutConstraints {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }
}

/// Flex layout builder
pub struct FlexLayoutBuilder {
    calculator: LayoutCalculator,
    layout: FlexLayout,
}

impl FlexLayoutBuilder {
    fn new(calculator: LayoutCalculator) -> Self {
        Self {
            calculator,
            layout: FlexLayout::default(),
        }
    }

    pub fn direction(mut self, direction: FlexDirection) -> Self {
        self.layout.direction = direction;
        self
    }

    pub fn justify(mut self, justify: JustifyContent) -> Self {
        self.layout.justify = justify;
        self
    }

    pub fn align_items(mut self, align: AlignItems) -> Self {
        self.layout.align_items = align;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.layout.gap = self.calculator.spacing(gap);
        self
    }

    pub fn wrap(mut self, wrap: FlexWrap) -> Self {
        self.layout.wrap = wrap;
        self
    }

    pub fn build(self) -> FlexLayout {
        self.layout
    }
}

/// Grid layout builder
pub struct GridLayoutBuilder {
    calculator: LayoutCalculator,
    layout: GridLayout,
    base_columns: usize,
}

impl GridLayoutBuilder {
    fn new(calculator: LayoutCalculator, base_columns: usize) -> Self {
        let columns = calculator.grid_columns(base_columns);
        Self {
            calculator,
            layout: GridLayout {
                columns,
                ..GridLayout::default()
            },
            base_columns,
        }
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.layout.columns = self.calculator.grid_columns(columns);
        self
    }

    pub fn rows(mut self, rows: usize) -> Self {
        self.layout.rows = rows;
        self
    }

    pub fn column_gap(mut self, gap: f32) -> Self {
        self.layout.column_gap = self.calculator.spacing(gap);
        self
    }

    pub fn row_gap(mut self, gap: f32) -> Self {
        self.layout.row_gap = self.calculator.spacing(gap);
        self
    }

    pub fn auto_flow(mut self, flow: GridAutoFlow) -> Self {
        self.layout.auto_flow = flow;
        self
    }

    pub fn build(self) -> GridLayout {
        self.layout
    }

    /// The base column count this builder was created with.
    ///
    /// This is the value handed to [`LayoutCalculator::grid`], i.e. the
    /// *requested* column count before the responsive adjustment in
    /// [`LayoutCalculator::grid_columns`] was applied. Keep it around to
    /// re-derive the grid at a different window size.
    pub fn base_columns(&self) -> usize {
        self.base_columns
    }
}

/// Common layout patterns
pub mod patterns {
    use super::*;

    /// Center content both horizontally and vertically
    pub fn center() -> FlexLayout {
        FlexLayout {
            direction: FlexDirection::Column,
            justify: JustifyContent::Center,
            align_items: AlignItems::Center,
            align_content: AlignContent::Center,
            ..FlexLayout::default()
        }
    }

    /// Row with items spaced evenly
    pub fn row_spaced(gap: f32) -> FlexLayout {
        FlexLayout {
            direction: FlexDirection::Row,
            justify: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            gap,
            ..FlexLayout::default()
        }
    }

    /// Column with items spaced evenly
    pub fn column_spaced(gap: f32) -> FlexLayout {
        FlexLayout {
            direction: FlexDirection::Column,
            justify: JustifyContent::SpaceBetween,
            align_items: AlignItems::Stretch,
            gap,
            ..FlexLayout::default()
        }
    }

    /// Grid with responsive columns
    pub fn responsive_grid(
        calculator: &LayoutCalculator,
        base_columns: usize,
        gap: f32,
    ) -> GridLayout {
        GridLayout {
            columns: calculator.grid_columns(base_columns),
            column_gap: calculator.spacing(gap),
            row_gap: calculator.spacing(gap),
            ..GridLayout::default()
        }
    }

    /// Two-pane layout (master-detail)
    pub fn two_pane(
        calculator: &LayoutCalculator,
        master_width: f32,
    ) -> (LayoutConstraints, LayoutConstraints) {
        let padding = calculator.default_padding();
        let safe_area = calculator.safe_area();
        let total_width = 1200.0; // Would come from constraints

        let master = LayoutConstraints {
            min_width: master_width,
            max_width: master_width,
            min_height: 0.0,
            max_height: f32::INFINITY,
        };

        let detail = LayoutConstraints {
            min_width: 0.0,
            max_width: total_width - master_width - padding * 2.0 - safe_area.1 - safe_area.3,
            min_height: 0.0,
            max_height: f32::INFINITY,
        };

        (master, detail)
    }

    /// Bottom sheet layout
    pub fn bottom_sheet(calculator: &LayoutCalculator, height_fraction: f32) -> LayoutConstraints {
        let safe_area = calculator.safe_area();
        LayoutConstraints {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: 800.0 * height_fraction + safe_area.2,
        }
    }

    /// Modal layout
    pub fn modal(calculator: &LayoutCalculator) -> LayoutConstraints {
        // Stub awaiting the real implementation: the doubled default padding is
        // computed for callers to inset a modal by, but the constraints below
        // are still hard-coded. A real implementation would derive the modal
        // size from `calculator` and the safe-area insets instead.
        let _padding = calculator.default_padding() * 2.0;
        LayoutConstraints {
            min_width: 280.0,
            max_width: 560.0,
            min_height: 0.0,
            max_height: 600.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::domain::{DeviceInfo, FormFactor, Orientation, Platform, SafeArea, ScreenSize};

    fn create_device_info(screen_size: ScreenSize, form_factor: FormFactor) -> DeviceInfo {
        DeviceInfo {
            platform: Platform::Linux,
            form_factor,
            screen_size,
            pixel_density: 1.0,
            orientation: Orientation::Portrait,
            has_touch: false,
            has_keyboard: true,
            has_mouse: true,
            safe_area: SafeArea::default(),
        }
    }

    #[test]
    fn test_layout_calculator() {
        let device = create_device_info(ScreenSize::Small, FormFactor::Phone);
        let calc = LayoutCalculator::new(device);

        assert_eq!(calc.grid_columns(4), 1);
        assert!(calc.is_compact());
        assert!(!calc.is_two_pane());
    }

    #[test]
    fn test_responsive_grid() {
        let device = create_device_info(ScreenSize::Large, FormFactor::Desktop);
        let calc = LayoutCalculator::new(device);

        assert_eq!(calc.grid_columns(4), 3);
        assert!(!calc.is_compact());
        assert!(calc.is_two_pane());
    }

    #[test]
    fn test_flex_builder() {
        let device = create_device_info(ScreenSize::Medium, FormFactor::Tablet);
        let builder = LayoutBuilder::new(device);

        let flex = builder
            .flex()
            .direction(FlexDirection::Row)
            .justify(JustifyContent::SpaceBetween)
            .gap(16.0)
            .build();

        assert_eq!(flex.direction, FlexDirection::Row);
        assert_eq!(flex.justify, JustifyContent::SpaceBetween);
        assert_eq!(flex.gap, 16.0); // spacing_multiplier for tablet is 1.0
    }
}
