//! Settings Tab View: Multi-language selector and system preferences

use crate::domain::i18n::{I18nStrings, Language};
use crate::presentation::winui::app::{GearVRReactorApp, ReactorMessage};
use crate::presentation::winui::components::cards::{render_card, render_toggle_card};
use crate::presentation::winui::tokens::FluentTokens;
use windows_reactor::*;

pub fn render_settings_view(
    app: &GearVRReactorApp,
    context: &mut ViewContext<GearVRReactorApp>,
    s: &I18nStrings,
) -> View {
    // Language Selection Card (4 Languages + Auto)
    let resolved_lang = app.language.resolve();
    let auto_label = format!("Auto ({})", resolved_lang.display_name());

    let languages = [
        (Language::Auto, auto_label),
        (Language::SimplifiedChinese, "简体中文".to_string()),
        (Language::English, "English".to_string()),
        (Language::Japanese, "日本語".to_string()),
        (Language::Korean, "한국어".to_string()),
    ];

    let lang_buttons = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(FluentTokens::SPACING_MD)
        .children(languages.map(|(lang, label)| {
            Button::new()
                .style(if app.language == lang {
                    ButtonStyle::Accent
                } else {
                    ButtonStyle::Default
                })
                .on_click(context.message(ReactorMessage::SelectLanguage(lang)))
                .content(label)
        }));

    let language_card = render_card(s.language_card_title, s.language_card_desc, lang_buttons);

    let anti_sleep_row = render_toggle_card(
        s.anti_sleep_title,
        s.anti_sleep_desc,
        ToggleSwitch::new()
            .is_on(app.enable_anti_sleep)
            .on_toggled(context.callback(ReactorMessage::ToggleAntiSleep)),
    );

    let auto_profile_row = render_toggle_card(
        s.auto_profile_title,
        s.auto_profile_desc,
        ToggleSwitch::new()
            .is_on(app.enable_auto_profile)
            .on_toggled(context.callback(ReactorMessage::ToggleAutoProfile)),
    );

    let tray_row = render_toggle_card(
        s.tray_title,
        s.tray_desc,
        ToggleSwitch::new()
            .is_on(app.enable_background_tray)
            .on_toggled(context.callback(ReactorMessage::ToggleBackgroundTray)),
    );

    StackPanel::new()
        .spacing(FluentTokens::SPACING_XL)
        .children((language_card, anti_sleep_row, auto_profile_row, tray_row))
}
