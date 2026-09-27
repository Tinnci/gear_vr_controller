//! Native WinUI 3 TitleBar component matching Windows Settings / Microsoft Store pattern

use crate::domain::i18n::I18nStrings;
use crate::domain::models::ConnectionStatus;
use crate::presentation::winui::app::{GearVRReactorApp, ReactorMessage};
use crate::presentation::winui::tokens::FluentTokens;
use windows_reactor::*;

pub fn render_title_bar(
    app: &GearVRReactorApp,
    context: &mut ViewContext<GearVRReactorApp>,
    s: &I18nStrings,
) -> View {
    let (status_badge_text, ring_active) = match app.connection_status {
        ConnectionStatus::Connected => (s.status_connected.to_string(), false),
        ConnectionStatus::Connecting => (s.status_connecting.to_string(), true),
        ConnectionStatus::Disconnected => {
            if app.is_scanning {
                (s.scan_button.to_string(), true)
            } else {
                (s.status_disconnected.to_string(), false)
            }
        }
        ConnectionStatus::Error => (s.status_error.to_string(), false),
    };

    let status_icon: View = if ring_active {
        ProgressRing::new()
            .is_active(true)
            .width(FluentTokens::RING_SM)
            .height(FluentTokens::RING_SM)
            .into()
    } else {
        InfoBadge::new().into()
    };

    let status_pill = Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(FluentTokens::BORDER_THIN)
        .corner_radius(FluentTokens::RADIUS_PILL)
        .padding(FluentTokens::pill_padding())
        .content(
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(FluentTokens::SPACING_SM)
                .children((
                    status_icon,
                    TextBlock::new()
                        .text(status_badge_text)
                        .font_size(FluentTokens::FONT_CAPTION)
                        .font_weight(FontWeight::SEMI_BOLD),
                )),
        );

    let quick_bt_btn = Button::new()
        .style(ButtonStyle::Subtle)
        .on_click(context.message(ReactorMessage::OpenBtSettings))
        .content(s.open_bt_settings);

    let title_bar_right = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(FluentTokens::SPACING_MD)
        .children((status_pill, quick_bt_btn));

    // Dynamic subtitle reflecting current page and device state
    let dynamic_subtitle = match app.selected_tab {
        0 => match app.connection_status {
            ConnectionStatus::Connected => {
                if let Some(data) = &app.latest_data {
                    format!(
                        "{} - {} ({} ms)",
                        s.nav_dashboard, s.status_connected, data.timestamp
                    )
                } else {
                    format!("{} - {}", s.nav_dashboard, s.status_connected)
                }
            }
            ConnectionStatus::Connecting => {
                format!("{} - {}", s.nav_dashboard, s.status_connecting)
            }
            ConnectionStatus::Disconnected => {
                if app.is_scanning {
                    format!("{} - {}", s.nav_dashboard, s.scan_button)
                } else {
                    format!("{} - {}", s.nav_dashboard, s.app_subtitle)
                }
            }
            ConnectionStatus::Error => format!("{} - {}", s.nav_dashboard, s.status_error),
        },
        1 => format!("{} - {}", s.nav_calibration, s.touch_cal_status),
        2 => format!("{} - {}", s.nav_settings, app.language.display_name()),
        _ => format!("{} - {}", s.nav_diagnostics, s.imu_diag_title),
    };

    TitleBar::new()
        .grid_row(0)
        .title(s.app_title)
        .subtitle(dynamic_subtitle)
        .preferred_height(WindowTitleBarHeight::Tall)
        .is_back_button_visible(false)
        .is_pane_toggle_button_visible(true)
        .on_pane_toggle_requested(context.message(ReactorMessage::TogglePane))
        .slot(TitleBarSlot::RightHeader, title_bar_right)
}
