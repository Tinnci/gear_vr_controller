//! Reusable, labelled native controls. Layout code does not persist or inject input.
use super::super::{
    app::{GearVRReactorApp, ReactorMessage},
    state::{BooleanPreference, NumericPreference},
    text::Text,
};
use windows_reactor::*;

pub fn paragraph(text: impl Into<String>) -> View {
    TextBlock::new()
        .text(text)
        .font_size(14.0)
        .text_wrapping(TextWrapping::Wrap)
        .into()
}
pub fn section(app: &GearVRReactorApp, title: Text, description: Text, content: View) -> View {
    super::cards::render_card(app.text(title), app.text(description), content)
}
pub fn button(
    app: &GearVRReactorApp,
    context: &ViewContext<GearVRReactorApp>,
    text: Text,
    message: ReactorMessage,
) -> ActionButton {
    ActionButton {
        control: Button::new().on_click(context.message(message)),
        label: app.text(text).into(),
    }
}

/// Configure a native button before assigning its content, which finalizes a Reactor view.
pub struct ActionButton {
    control: Button,
    label: String,
}
impl ActionButton {
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.control = self.control.style(style);
        self
    }
    pub fn is_enabled(mut self, enabled: bool) -> Self {
        self.control = self.control.is_enabled(enabled);
        self
    }
    pub fn grid_column(mut self, column: i32) -> Self {
        self.control = self.control.grid_column(column);
        self
    }
    pub fn automation_name(mut self, name: impl Into<String>) -> Self {
        self.control = self.control.automation_name(name);
        self
    }
}
impl From<ActionButton> for View {
    fn from(button: ActionButton) -> Self {
        button.control.content(button.label)
    }
}
pub fn toggle(
    app: &GearVRReactorApp,
    context: &ViewContext<GearVRReactorApp>,
    text: Text,
    field: BooleanPreference,
    enabled: bool,
) -> View {
    StackPanel::new().spacing(4.0).children((
        paragraph(app.text(text)),
        ToggleSwitch::new()
            .automation_name(app.text(text))
            .is_on(enabled)
            .on_toggled(context.callback(move |value| ReactorMessage::Boolean(field, value))),
    ))
}
pub fn slider(
    app: &GearVRReactorApp,
    context: &ViewContext<GearVRReactorApp>,
    text: Text,
    field: NumericPreference,
    value: f64,
    range: (f64, f64, f64),
) -> View {
    StackPanel::new().spacing(4.0).children((
        paragraph(format!("{} · {:.2}", app.text(text), value)),
        Slider::new()
            .automation_name(app.text(text))
            .minimum(range.0)
            .maximum(range.1)
            .step_frequency(range.2)
            .value(value)
            .min_width(180.0)
            .max_width(560.0)
            .on_value_changed(context.callback(move |value| ReactorMessage::Number(field, value))),
    ))
}
pub fn save_bar(app: &GearVRReactorApp, context: &ViewContext<GearVRReactorApp>) -> View {
    StackPanel::new().spacing(8.0).children((
        paragraph(if app.ui.dirty() {
            app.text(Text::Unsaved)
        } else {
            app.text(Text::Saved)
        }),
        button(app, context, Text::Save, ReactorMessage::SavePreferences)
            .style(ButtonStyle::Accent)
            .is_enabled(app.ui.dirty()),
        button(
            app,
            context,
            Text::Discard,
            ReactorMessage::DiscardPreferences,
        )
        .is_enabled(app.ui.dirty()),
    ))
}
