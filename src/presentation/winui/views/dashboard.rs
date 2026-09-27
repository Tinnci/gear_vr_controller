//! Dashboard Tab View: Connection management, Mode switching, Real-time input telemetry

use crate::domain::i18n::I18nStrings;
use crate::domain::models::ConnectionStatus;
use crate::presentation::radial_menu::ControlMode;
use crate::presentation::winui::app::{GearVRReactorApp, ReactorMessage};
use crate::presentation::winui::components::cards::render_card;
use crate::presentation::winui::components::formatters::format_telemetry;
use crate::presentation::winui::tokens::FluentTokens;
use windows_reactor::*;

pub fn render_dashboard_view(
    app: &GearVRReactorApp,
    context: &mut ViewContext<GearVRReactorApp>,
    s: &I18nStrings,
) -> View {
    // Card 1: Connection & Bluetooth Scanning
    let connect_button = if app.connection_status == ConnectionStatus::Connected {
        Button::new()
            .style(ButtonStyle::Default)
            .on_click(context.message(ReactorMessage::Disconnect))
            .content(s.disconnect_button)
    } else {
        Button::new()
            .style(ButtonStyle::Accent)
            .on_click(context.message(ReactorMessage::Connect))
            .content(s.connect_button)
    };

    let scan_button = Button::new()
        .on_click(context.message(ReactorMessage::ToggleScan))
        .content(if app.is_scanning { s.stop_scan_button } else { s.scan_button });

    let scan_ring = if app.is_scanning {
        ProgressRing::new().is_active(true).width(FluentTokens::RING_MD).height(FluentTokens::RING_MD)
    } else {
        ProgressRing::new().is_active(false).width(FluentTokens::RING_MD).height(FluentTokens::RING_MD)
    };

    let address_box = TextBox::new()
        .text(&app.address_input)
        .placeholder_text(s.address_placeholder)
        .on_text_changed(context.callback(ReactorMessage::UpdateAddressInput));

    let connection_controls = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(FluentTokens::SPACING_MD)
        .children((
            address_box,
            connect_button,
            scan_button,
            scan_ring,
        ));

    let connection_card = render_card(
        s.conn_card_title,
        s.conn_card_desc,
        connection_controls.into(),
    );

    // Card 2: Operational Mode Selection (Segmented Control)
    let modes = [
        (ControlMode::Mouse, s.mode_air_mouse),
        (ControlMode::Touchpad, s.mode_trackpad),
        (ControlMode::Presentation, s.mode_presenter),
    ];

    let mode_buttons = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(FluentTokens::SPACING_MD)
        .children(modes.map(|(mode, label)| {
            Button::new()
                .style(if app.current_mode == mode {
                    ButtonStyle::Accent
                } else {
                    ButtonStyle::Default
                })
                .on_click(context.message(ReactorMessage::ChangeMode(mode)))
                .content(label)
        }));

    let mode_card = render_card(
        s.mode_card_title,
        s.mode_card_desc,
        mode_buttons.into(),
    );

    // Card 3: Real-Time Input Telemetry Monitor
    let (tp_text, btn_text, sample_text) = format_telemetry(app.latest_data.as_ref(), s);

    let telemetry_card = render_card(
        s.telemetry_card_title,
        s.telemetry_card_desc,
        StackPanel::new()
            .spacing(FluentTokens::SPACING_XS)
            .children((
                TextBlock::new().text(tp_text).font_size(FluentTokens::FONT_BODY),
                TextBlock::new().text(btn_text).font_size(FluentTokens::FONT_BODY),
                TextBlock::new()
                    .text(sample_text)
                    .font_size(FluentTokens::FONT_CAPTION)
                    .foreground(ThemeBrush::PrimaryText),
            ))
            .into(),
    );

    StackPanel::new()
        .spacing(FluentTokens::SPACING_XL)
        .children((
            connection_card,
            mode_card,
            telemetry_card,
        ))
        .into()
}
