//! Connection-first control page. Device discovery results are selectable.
use super::super::{
    app::{modes, GearVRReactorApp, ReactorMessage},
    components::controls::{button, paragraph, section},
    state::Page,
    text::{action_text, mode_text, Text},
};
use crate::domain::models::{ConnectionStatus, OutputTarget};
use windows_reactor::*;

pub fn render_dashboard_view(
    app: &GearVRReactorApp,
    context: &mut ViewContext<GearVRReactorApp>,
) -> View {
    StackPanel::new().spacing(20.0).children((
        paragraph(app.text(Text::ControlHint)),
        connection(app, context),
        mode(app, context),
        advanced_connection(app, context),
    ))
}
fn connection(app: &GearVRReactorApp, context: &ViewContext<GearVRReactorApp>) -> View {
    let status = match app.ui.connection {
        ConnectionStatus::Connecting => Text::Connecting,
        ConnectionStatus::Connected => Text::Connected,
        _ => Text::Disconnected,
    };
    let controls = if app.ui.connection == ConnectionStatus::Connecting {
        StackPanel::new().spacing(8.0).children((
            ProgressRing::new().is_active(true).width(20.0).height(20.0),
            button(
                app,
                context,
                Text::CancelConnect,
                ReactorMessage::Disconnect,
            ),
        ))
    } else if app.ui.connected() {
        let target = if app.ui.output == OutputTarget::Desktop {
            OutputTarget::Paused
        } else {
            OutputTarget::Desktop
        };
        StackPanel::new().spacing(8.0).children((
            paragraph(app.text(match app.ui.output {
                OutputTarget::Desktop => Text::InputActive,
                OutputTarget::Preview => Text::TestActive,
                _ => Text::InputPaused,
            })),
            button(
                app,
                context,
                if target == OutputTarget::Desktop {
                    Text::Resume
                } else {
                    Text::Pause
                },
                ReactorMessage::SetOutput(target),
            )
            .style(ButtonStyle::Accent)
            .is_enabled(app.ui.can_output()),
            button(
                app,
                context,
                Text::Tuning,
                ReactorMessage::Navigate(Page::Tuning),
            ),
            button(app, context, Text::Disconnect, ReactorMessage::Disconnect),
        ))
    } else {
        discovery(app, context)
    };
    section(
        app,
        if app.ui.connected() {
            Text::AppName
        } else {
            Text::AddDevice
        },
        status,
        controls,
    )
}
fn discovery(app: &GearVRReactorApp, context: &ViewContext<GearVRReactorApp>) -> View {
    let found = StackPanel::new()
        .spacing(12.0)
        .keyed_children(app.ui.devices.iter().map(|device| {
            let name = if device.name.is_empty() {
                app.text(Text::UnnamedDevice).to_string()
            } else {
                device.name.clone()
            };
            KeyedView::new(
                format!("{:X}", device.address),
                Grid::new()
                    .columns([GridLength::STAR, GridLength::Auto])
                    .column_spacing(12.0)
                    .children((
                        Border::new().grid_column(0).content(paragraph(&name)),
                        button(
                            app,
                            context,
                            Text::Connect,
                            ReactorMessage::ConnectDevice(device.address),
                        )
                        .grid_column(1)
                        .automation_name(format!(
                            "{} {}",
                            app.text(Text::Connect),
                            name
                        )),
                    )),
            )
        }));
    let found: View = if app.ui.devices.is_empty() {
        StackPanel::new().into()
    } else {
        ScrollViewer::new().max_height(260.0).content(found)
    };
    let recent: View = app
        .ui
        .last_address
        .filter(|address| {
            !app.ui
                .devices
                .iter()
                .any(|device| device.address == *address)
        })
        .map(|address| {
            button(
                app,
                context,
                Text::LastDevice,
                ReactorMessage::ConnectDevice(address),
            )
            .into()
        })
        .unwrap_or_else(|| StackPanel::new().into());
    StackPanel::new().spacing(12.0).children((
        paragraph(app.text(Text::WakeHint)),
        recent,
        button(
            app,
            context,
            if app.ui.scanning {
                Text::StopSearch
            } else {
                Text::Search
            },
            ReactorMessage::ToggleScan,
        )
        .style(ButtonStyle::Accent)
        .is_enabled(!app.ui.scan_pending),
        paragraph(if app.ui.scanning {
            app.text(Text::Searching)
        } else if app.ui.scan_attempted && app.ui.devices.is_empty() {
            app.text(Text::NoResults)
        } else {
            ""
        }),
        found,
        button(
            app,
            context,
            Text::BluetoothSettings,
            ReactorMessage::OpenBluetooth,
        ),
    ))
}
fn mode(app: &GearVRReactorApp, context: &ViewContext<GearVRReactorApp>) -> View {
    let language = app.ui.language();
    let controls = RadioButtons::new()
        .items_source(modes().map(|mode| mode_text(mode).get(language)))
        .automation_name(app.text(Text::Mode))
        .selected_index(modes().iter().position(|mode| *mode == app.ui.mode))
        .max_columns(3)
        .on_selection_changed(context.callback(ReactorMessage::Mode));
    let bindings = app.ui.saved.bindings.for_mode(app.ui.mode);
    let hint = match app.ui.mode {
        crate::domain::models::ControlMode::Touchpad => Text::TouchHint,
        crate::domain::models::ControlMode::Presentation => Text::PresenterHint,
        _ => Text::AirHint,
    };
    let mapping = [
        (Text::Trigger, bindings.trigger),
        (Text::TouchPress, bindings.touchpad),
        (Text::Back, bindings.back),
        (Text::Home, bindings.home),
    ];
    let mapping = StackPanel::new()
        .spacing(4.0)
        .children(mapping.map(|(key, action)| {
            paragraph(format!(
                "{} → {}",
                app.text(key),
                app.text(action_text(action))
            ))
        }));
    section(
        app,
        Text::Mode,
        hint,
        StackPanel::new().spacing(12.0).children((
            controls,
            mapping,
            paragraph(app.text(Text::ModeHold)),
            paragraph(app.text(if app.ui.automatic_mode {
                Text::AutomaticMode
            } else {
                Text::ManualMode
            })),
        )),
    )
}
fn advanced_connection(app: &GearVRReactorApp, context: &ViewContext<GearVRReactorApp>) -> View {
    Expander::new()
        .is_expanded(app.ui.advanced_connection_open)
        .on_is_expanded_changed(context.callback(ReactorMessage::AdvancedConnection))
        .slots([
            SlotView::new(
                ExpanderSlot::Header,
                paragraph(app.text(Text::AdvancedConnection)),
            ),
            SlotView::new(
                ExpanderSlot::Content,
                StackPanel::new().spacing(12.0).children((
                    paragraph(app.text(Text::AddressHint)),
                    TextBox::new()
                        .text(&app.ui.address)
                        .placeholder_text("2C41A1001234")
                        .automation_name(app.text(Text::Address))
                        .max_width(360.0)
                        .on_text_changed(context.callback(ReactorMessage::Address)),
                    button(app, context, Text::Connect, ReactorMessage::ConnectAddress).is_enabled(
                        !app.ui.connected() && app.ui.connection != ConnectionStatus::Connecting,
                    ),
                )),
            ),
        ])
}
