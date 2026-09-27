//! Reusable Fluent Card containers matching the Windows 11 SettingsCard pattern

use crate::presentation::winui::tokens::FluentTokens;
use windows_reactor::*;

/// Renders a Fluent SettingsCard container with a title, description, and arbitrary content.
pub fn render_card(title: &str, description: &str, content: View) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(FluentTokens::BORDER_THIN)
        .corner_radius(FluentTokens::RADIUS_CARD)
        .padding(FluentTokens::card_padding())
        .content(
            StackPanel::new()
                .spacing(FluentTokens::SPACING_LG)
                .children((
                    StackPanel::new()
                        .spacing(FluentTokens::SPACING_XXS)
                        .children((
                            TextBlock::new()
                                .text(title)
                                .font_size(FluentTokens::FONT_TITLE)
                                .font_weight(FontWeight::SEMI_BOLD),
                            TextBlock::new()
                                .text(description)
                                .font_size(FluentTokens::FONT_CAPTION)
                                .foreground(ThemeBrush::PrimaryText),
                        )),
                    content,
                )),
        )
}

/// Renders a Fluent SettingsCard pattern for right-aligned toggle switches.
pub fn render_toggle_card(title: &str, description: &str, toggle: ToggleSwitch) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(FluentTokens::BORDER_THIN)
        .corner_radius(FluentTokens::RADIUS_CARD)
        .padding(FluentTokens::card_padding())
        .content(
            Grid::new()
                .columns([GridLength::STAR, GridLength::Auto])
                .children((
                    StackPanel::new()
                        .grid_column(0)
                        .spacing(FluentTokens::SPACING_XXS)
                        .children((
                            TextBlock::new()
                                .text(title)
                                .font_size(FluentTokens::FONT_SUBTITLE)
                                .font_weight(FontWeight::SEMI_BOLD),
                            TextBlock::new()
                                .text(description)
                                .font_size(FluentTokens::FONT_CAPTION)
                                .foreground(ThemeBrush::PrimaryText),
                        )),
                    toggle.grid_column(1),
                )),
        )
}
