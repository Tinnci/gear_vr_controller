use super::{
    args::{Args, Command},
    output::Output,
};
use anyhow::{ensure, Result};
use gear_vr_controller_rust::{
    application::event_bus::EventSender,
    domain::models::AppEvent,
    infrastructure::bluetooth::{
        connection::{BleConnection, ConnectionConfig},
        diagnostics, protocol,
        scanner::BleScanner,
    },
};
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use windows::{
    Devices::{
        Bluetooth::{
            BluetoothAdapter, BluetoothConnectionStatus, BluetoothLEDevice,
            GenericAttributeProfile::{GattCharacteristic, GattValueChangedEventArgs},
        },
        Enumeration::DevicePairingResultStatus,
    },
    Foundation::TypedEventHandler,
};

pub async fn run(args: &Args, output: &Output) -> Result<()> {
    match args.command {
        Command::Scan => scan(args, output).await,
        Command::Status => status(args, output).await,
        Command::Pair => pair(args, output).await,
        Command::Connect => connect(args, output).await,
    }
}

async fn scan(args: &Args, output: &Output) -> Result<()> {
    diagnostics::adapter_snapshot("cli_scan").await;
    let (sender, mut events) = EventSender::channel(256);
    let mut scanner = BleScanner::new(sender.clone());
    scanner.start(None, args.all, vec![], None)?;
    let mut devices = Vec::new();
    let until = Instant::now() + Duration::from_secs(args.seconds);
    while Instant::now() < until {
        scanner.poll();
        while let Ok(event) = events.try_recv() {
            if let AppEvent::DevicesUpdated(snapshot) = event {
                devices = snapshot;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    scanner.stop()?;
    while let Ok(event) = events.try_recv() {
        if let AppEvent::DevicesUpdated(snapshot) = event {
            devices = snapshot;
        }
    }
    ensure!(!sender.take_overflow(), "Scan event queue overflowed");
    let devices: Vec<_> = devices
        .iter()
        .map(|device| {
            json!({
                "name": device.name, "address": format!("{:012X}", device.address),
                "address_type": format!("{:?}", device.address_kind).to_lowercase(),
                "rssi": device.signal_strength, "matches_service": device.matches_service,
                "available": device.available,
            })
        })
        .collect();
    output.emit(
        "scan_result",
        json!({"devices": devices, "duration_seconds": args.seconds}),
    )
}

struct Device(BluetoothLEDevice);
impl Drop for Device {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}
async fn open(args: &Args) -> Result<Device> {
    let address = args
        .address
        .ok_or_else(|| anyhow::anyhow!("Missing device address"))?;
    let operation = match args.address_type {
        Some(kind) => {
            BluetoothLEDevice::FromBluetoothAddressWithBluetoothAddressTypeAsync(address, kind)?
        }
        None => BluetoothLEDevice::FromBluetoothAddressAsync(address)?,
    };
    Ok(Device(operation.await?))
}
fn device_status(device: &BluetoothLEDevice, output: &Output) -> Result<()> {
    let information = device.DeviceInformation()?;
    let pairing = information.Pairing()?;
    output.emit(
        "device_status",
        json!({
            "address": format!("{:012X}", device.BluetoothAddress()?),
            "name": information.Name()?.to_string(), "device_id": information.Id()?.to_string(),
            "connection_status": device.ConnectionStatus()?.0,
            "address_type": device.BluetoothAddressType()?.0,
            "enabled": information.IsEnabled()?, "paired": pairing.IsPaired()?,
            "can_pair": pairing.CanPair()?, "protection_level": pairing.ProtectionLevel()?.0,
            "access_status": device.DeviceAccessInformation()?.CurrentStatus()?.0,
        }),
    )
}
async fn status(args: &Args, output: &Output) -> Result<()> {
    diagnostics::adapter_snapshot("cli_status").await;
    let adapter = BluetoothAdapter::GetDefaultAsync()?.await?;
    let radio = adapter.GetRadioAsync()?.await?;
    output.emit("adapter_status", json!({
        "radio_name": radio.Name()?.to_string(), "radio_state": radio.State()?.0,
        "low_energy": adapter.IsLowEnergySupported()?, "central": adapter.IsCentralRoleSupported()?,
    }))?;
    if args.address.is_some() {
        device_status(&open(args).await?.0, output)?;
    }
    Ok(())
}
async fn pair(args: &Args, output: &Output) -> Result<()> {
    diagnostics::adapter_snapshot("cli_pair").await;
    let device = open(args).await?;
    device_status(&device.0, output)?;
    let pairing = device.0.DeviceInformation()?.Pairing()?;
    if pairing.IsPaired()? {
        output.emit("pair_result", json!({"success": true, "status": "already_paired", "status_code": DevicePairingResultStatus::AlreadyPaired.0}))?;
        return Ok(());
    }
    ensure!(
        pairing.CanPair()?,
        "Windows reports that this device cannot currently be paired"
    );
    output.emit(
        "pair_started",
        json!({"message": "Windows may request pairing confirmation"}),
    )?;
    let operation = pairing.PairAsync()?;
    let result = operation.await?;
    let status = result.Status()?;
    let success = matches!(
        status,
        DevicePairingResultStatus::Paired | DevicePairingResultStatus::AlreadyPaired
    );
    tracing::info!(
        event = "cli.pairing.result",
        status = status.0,
        success,
        "Windows pairing completed"
    );
    output.emit(
        "pair_result",
        json!({"success": success, "status_code": status.0, "status": pairing_status_name(status)}),
    )?;
    device_status(&device.0, output)?;
    ensure!(
        success,
        "Windows pairing returned {} ({})",
        pairing_status_name(status),
        status.0
    );
    Ok(())
}

fn pairing_status_name(status: DevicePairingResultStatus) -> &'static str {
    match status {
        DevicePairingResultStatus::Paired => "paired",
        DevicePairingResultStatus::NotReadyToPair => "not_ready_to_pair",
        DevicePairingResultStatus::NotPaired => "not_paired",
        DevicePairingResultStatus::AlreadyPaired => "already_paired",
        DevicePairingResultStatus::ConnectionRejected => "connection_rejected",
        DevicePairingResultStatus::TooManyConnections => "too_many_connections",
        DevicePairingResultStatus::HardwareFailure => "hardware_failure",
        DevicePairingResultStatus::AuthenticationTimeout => "authentication_timeout",
        DevicePairingResultStatus::AuthenticationNotAllowed => "authentication_not_allowed",
        DevicePairingResultStatus::AuthenticationFailure => "authentication_failure",
        DevicePairingResultStatus::NoSupportedProfiles => "no_supported_profiles",
        DevicePairingResultStatus::ProtectionLevelCouldNotBeMet => {
            "protection_level_could_not_be_met"
        }
        DevicePairingResultStatus::AccessDenied => "access_denied",
        DevicePairingResultStatus::InvalidCeremonyData => "invalid_ceremony_data",
        DevicePairingResultStatus::PairingCanceled => "pairing_canceled",
        DevicePairingResultStatus::OperationAlreadyInProgress => "operation_already_in_progress",
        DevicePairingResultStatus::RequiredHandlerNotRegistered => {
            "required_handler_not_registered"
        }
        DevicePairingResultStatus::RejectedByHandler => "rejected_by_handler",
        DevicePairingResultStatus::RemoteDeviceHasAssociation => "remote_device_has_association",
        DevicePairingResultStatus::Failed => "failed",
        _ => "unknown",
    }
}

async fn connect(args: &Args, output: &Output) -> Result<()> {
    let address = args
        .address
        .ok_or_else(|| anyhow::anyhow!("Missing device address"))?;
    let (sender, mut events) = EventSender::channel(256);
    let connection = BleConnection::new(sender.clone(), ConnectionConfig::default());
    let pending = connection.connect_with_address_type(address, args.address_type);
    tokio::pin!(pending);
    let result = loop {
        tokio::select! {
            result = &mut pending => break result?,
            Some(event) = events.recv() => if let AppEvent::LogMessage(message) = event {
                output.emit("progress", json!({"message": message.message, "severity": format!("{:?}", message.severity)}))?;
            },
        }
    };
    ensure!(!sender.take_overflow(), "Connection event queue overflowed");
    device_status(&result.device, output)?;
    output.emit(
        "connected",
        json!({"capture_seconds": args.seconds, "service": protocol::SERVICE_UUID}),
    )?;
    let capture = PacketCapture::attach(&result.data_characteristic)?;
    let until = Instant::now() + Duration::from_secs(args.seconds);
    while Instant::now() < until {
        tokio::time::sleep(Duration::from_secs(1)).await;
        output.emit("capture_progress", capture.summary())?;
        ensure!(
            result.device.ConnectionStatus()? == BluetoothConnectionStatus::Connected,
            "Device disconnected during capture"
        );
    }
    let summary = capture.summary();
    let packets = capture.valid.load(Ordering::Relaxed);
    drop(capture); // Unsubscribe before ConnectionResult closes its device and session.
    drop(result);
    output.emit("capture_result", summary)?;
    ensure!(
        packets > 0,
        "GATT initialization succeeded but no valid controller packets arrived"
    );
    Ok(())
}

struct PacketCapture {
    characteristic: GattCharacteristic,
    token: i64,
    valid: Arc<AtomicU64>,
    invalid: Arc<AtomicU64>,
    last_ms: Arc<AtomicU64>,
    started: Instant,
}
impl PacketCapture {
    fn attach(characteristic: &GattCharacteristic) -> Result<Self> {
        let valid = Arc::new(AtomicU64::new(0));
        let invalid = Arc::new(AtomicU64::new(0));
        let last_ms = Arc::new(AtomicU64::new(0));
        let (valid_sink, invalid_sink, last_sink) =
            (valid.clone(), invalid.clone(), last_ms.clone());
        let started = Instant::now();
        let handler = TypedEventHandler::new(
            move |_: windows::core::Ref<GattCharacteristic>,
                  args: windows::core::Ref<GattValueChangedEventArgs>| {
                let packet = args
                    .as_ref()
                    .and_then(|args| args.CharacteristicValue().ok())
                    .and_then(|value| protocol::parse_data_packet(&value).ok());
                if packet.is_some() {
                    valid_sink.fetch_add(1, Ordering::Relaxed);
                    last_sink.store(started.elapsed().as_millis() as u64 + 1, Ordering::Relaxed);
                } else {
                    invalid_sink.fetch_add(1, Ordering::Relaxed);
                }
                Ok(())
            },
        );
        let token = characteristic.ValueChanged(&handler)?;
        Ok(Self {
            characteristic: characteristic.clone(),
            token,
            valid,
            invalid,
            last_ms,
            started,
        })
    }
    fn summary(&self) -> serde_json::Value {
        let elapsed = self.started.elapsed();
        let valid = self.valid.load(Ordering::Relaxed);
        let last = self.last_ms.load(Ordering::Relaxed);
        json!({"valid_packets": valid, "invalid_packets": self.invalid.load(Ordering::Relaxed),
            "elapsed_ms": elapsed.as_millis() as u64,
            "packets_per_second": valid as f64 / elapsed.as_secs_f64().max(0.001),
            "last_packet_age_ms": (last > 0).then(|| (elapsed.as_millis() as u64).saturating_sub(last - 1)),
        })
    }
}
impl Drop for PacketCapture {
    fn drop(&mut self) {
        let _ = self.characteristic.RemoveValueChanged(self.token);
    }
}
