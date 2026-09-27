//! Microsoft Fluent Design System Tokens
//!
//! Provides standardized metrics for spacing, corner radii, padding,
//! and typography following Windows 11 Human Interface Guidelines.

use windows_reactor::Thickness;

pub struct FluentTokens;

impl FluentTokens {
    // Spacing scale (4px / 8px grid alignment)
    pub const SPACING_XXS: f64 = 2.0;
    pub const SPACING_XS: f64 = 4.0;
    pub const SPACING_SM: f64 = 6.0;
    pub const SPACING_MD: f64 = 8.0;
    pub const SPACING_LG: f64 = 10.0;
    pub const SPACING_XL: f64 = 12.0;
    pub const SPACING_XXL: f64 = 16.0;

    // Corner Radii
    pub const RADIUS_CARD: f64 = 8.0;
    pub const RADIUS_PILL: f64 = 12.0;

    // Border Thickness
    pub const BORDER_THIN: f64 = 1.0;

    // Dimensions
    pub const NAV_PANE_WIDTH: f64 = 240.0;
    pub const RING_SM: f64 = 12.0;
    pub const RING_MD: f64 = 18.0;

    // Typography Sizes
    pub const FONT_CAPTION: f64 = 12.0;
    pub const FONT_BODY: f64 = 14.0;
    pub const FONT_SUBTITLE: f64 = 16.0;
    pub const FONT_TITLE: f64 = 18.0;

    // Paddings
    pub fn card_padding() -> Thickness {
        Thickness::new(16.0, 14.0, 16.0, 14.0)
    }

    pub fn pill_padding() -> Thickness {
        Thickness::new(10.0, 4.0, 10.0, 4.0)
    }

    pub fn page_padding() -> Thickness {
        Thickness::new(24.0, 16.0, 24.0, 24.0)
    }
}
