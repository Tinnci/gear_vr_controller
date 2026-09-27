//! Draft preferences are saved explicitly as one validated snapshot.
use super::super::{
    app::{modes, GearVRReactorApp, ReactorMessage},
    components::controls::{button, paragraph, save_bar, section, toggle},
    state::{BindingSlot, BooleanPreference as Bool},
    text::{action_text, mode_text, Text},
};
use crate::domain::{bindings::ButtonAction, i18n::Language};
use windows_reactor::*;

pub fn render_settings_view(
    app: &GearVRReactorApp,
    context: &mut ViewContext<GearVRReactorApp>,
) -> View {
    let language = app.ui.language();
    let labels = Language::ALL.map(|language_option| match language_option {
        Language::Auto => app.text(Text::SystemLanguage),
        Language::SimplifiedChinese => "简体中文",
        Language::English => "English",
        Language::Japanese => "日本語",
        Language::Korean => "한국어",
    });
    let language_picker = ComboBox::new()
        .items_source(labels)
        .max_width(360.0)
        .selected_index(Language::ALL.iter().position(|value| *value == language))
        .automation_name(app.text(Text::Language))
        .on_selection_changed(context.callback(ReactorMessage::Language))
        .slot(ComboBoxSlot::Header, paragraph(app.text(Text::Language)));
    StackPanel::new().spacing(20.0).children((
        language_picker,
        bindings(app, context),
        background(app, context),
        save_bar(app, context),
    ))
}
fn bindings(app: &GearVRReactorApp, context: &ViewContext<GearVRReactorApp>) -> View {
    let language = app.ui.language();
    let binding = app.ui.draft.bindings.for_mode(app.ui.binding_mode);
    let fields = [
        (BindingSlot::Trigger, Text::Trigger, binding.trigger),
        (BindingSlot::Touchpad, Text::TouchPress, binding.touchpad),
        (BindingSlot::Back, Text::Back, binding.back),
        (BindingSlot::Home, Text::Home, binding.home),
    ];
    let pickers = StackPanel::new()
        .spacing(14.0)
        .children(fields.map(|(slot, label, action)| {
            ComboBox::new()
                .items_source(ButtonAction::ALL.map(|action| action_text(action).get(language)))
                .selected_index(ButtonAction::ALL.iter().position(|value| *value == action))
                .max_width(360.0)
                .automation_name(app.text(label))
                .on_selection_changed(
                    context.callback(move |index| ReactorMessage::Binding(slot, index)),
                )
                .slot(ComboBoxSlot::Header, paragraph(app.text(label)))
        }));
    section(
        app,
        Text::Bindings,
        Text::BindingsHint,
        StackPanel::new().spacing(14.0).children((
            ComboBox::new()
                .items_source(modes().map(|mode| mode_text(mode).get(language)))
                .max_width(360.0)
                .automation_name(app.text(Text::Mode))
                .selected_index(
                    modes()
                        .iter()
                        .position(|value| *value == app.ui.binding_mode),
                )
                .on_selection_changed(context.callback(ReactorMessage::BindingMode))
                .slot(ComboBoxSlot::Header, paragraph(app.text(Text::Mode))),
            pickers,
            paragraph(if binding.has_duplicate_shortcuts() {
                app.text(Text::DuplicateBindings)
            } else {
                ""
            }),
            button(
                app,
                context,
                Text::RestoreBindings,
                ReactorMessage::RestoreBindings,
            ),
        )),
    )
}
fn background(app: &GearVRReactorApp, context: &ViewContext<GearVRReactorApp>) -> View {
    let draft = &app.ui.draft;
    section(
        app,
        Text::Background,
        Text::ReconnectHint,
        StackPanel::new().spacing(14.0).children((
            toggle(
                app,
                context,
                Text::AutoConnect,
                Bool::AutoConnect,
                draft.auto_connect,
            ),
            toggle(
                app,
                context,
                Text::AutoReconnect,
                Bool::AutoReconnect,
                draft.auto_reconnect,
            ),
            toggle(
                app,
                context,
                Text::AutoProfile,
                Bool::AutoProfile,
                draft.auto_profile,
            ),
            paragraph(app.text(Text::AutoProfileHint)),
            toggle(
                app,
                context,
                Text::AntiSleep,
                Bool::AntiSleep,
                draft.anti_sleep,
            ),
            toggle(app, context, Text::Tray, Bool::Tray, draft.tray),
            paragraph(app.text(Text::TrayHint)),
        )),
    )
}
