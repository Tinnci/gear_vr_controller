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
    let candidates = app
        .ui
        .devices
        .iter()
        .filter(|device| device.known || device.matches_service)
        .count();
    let others = app.ui.devices.len() - candidates;
    let found: View = ScrollViewer::new()
        .max_height(260.0)
        .content(device_rows(app, context, true));
    let other_devices: View = if others == 0 {
        StackPanel::new().into()
    } else {
        Expander::new()
            .is_expanded(app.ui.other_devices_open)
            .on_is_expanded_changed(context.callback(ReactorMessage::OtherDevices))
            .slots([
                SlotView::new(
                    ExpanderSlot::Header,
                    paragraph(format!("{} ({others})", app.text(Text::OtherDevices))),
                ),
                SlotView::new(
                    ExpanderSlot::Content,
                    ScrollViewer::new()
                        .max_height(220.0)
                        .content(device_rows(app, context, false)),
                ),
            ])
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
        } else if app.ui.scan_attempted && candidates == 0 {
            app.text(Text::NoResults)
        } else {
            ""
        }),
        paragraph(app.text(Text::DiscoveryOrder)),
        found,
        other_devices,
        button(
            app,
            context,
            Text::BluetoothSettings,
            ReactorMessage::OpenBluetooth,
        ),
    ))
}
fn device_rows(
    app: &GearVRReactorApp,
    context: &ViewContext<GearVRReactorApp>,
    candidates: bool,
) -> View {
    use crate::domain::models::BluetoothAddressKind;
    StackPanel::new().spacing(12.0).keyed_children(
        app.ui
            .devices
            .iter()
            .filter(|device| (device.known || device.matches_service) == candidates)
            .map(|device| {
                let name = if device.name.is_empty() {
                    format!(
                        "{} · {:06X}",
                        app.text(if candidates {
                            Text::AppName
                        } else {
                            Text::UnnamedDevice
                        }),
                        device.address & 0xFF_FFFF
                    )
                } else {
                    device.name.clone()
                };
                let kind = match device.address_kind {
                    BluetoothAddressKind::Public => Text::PublicAddress,
                    BluetoothAddressKind::Random => Text::RandomAddress,
                    BluetoothAddressKind::Unknown => Text::UnknownAddress,
                };
                let identity = format!("{:012X} · {}", device.address, app.text(kind));
                let classification = app.text(if device.matches_service {
                    Text::ServiceMatch
                } else if device.known {
                    Text::SeenBefore
                } else {
                    Text::OtherDevices
                });
                let signal = if device.available {
                    format!("{} dBm", device.signal_strength)
                } else {
                    app.text(Text::NotSeen).to_owned()
                };
                KeyedView::new(
                    format!("{:X}-{:?}", device.address, device.address_kind),
                    Grid::new()
                        .columns([GridLength::STAR, GridLength::Auto])
                        .column_spacing(12.0)
                        .children((
                            StackPanel::new().grid_column(0).spacing(4.0).children((
                                paragraph(&name),
                                TextBlock::new()
                                    .text(identity.clone())
                                    .font_size(12.0)
                                    .foreground(ThemeBrush::PrimaryText),
                                TextBlock::new()
                                    .text(format!("{classification} · {signal}"))
                                    .font_size(12.0)
                                    .foreground(ThemeBrush::PrimaryText),
                            )),
                            button(
                                app,
                                context,
                                Text::Connect,
                                ReactorMessage::ConnectDevice(device.address),
                            )
                            .grid_column(1)
                            .is_enabled(
                                candidates
                                    && (device.available || device.known)
                                    && app.ui.connection != ConnectionStatus::Connecting,
                            )
                            .automation_name(format!(
                                "{} {name} {identity}",
                                app.text(Text::Connect)
                            )),
                        )),
                )
            }),
    )
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
