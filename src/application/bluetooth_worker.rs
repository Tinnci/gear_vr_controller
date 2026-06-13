use crate::domain::models::{
    AppEvent, BluetoothCommand, ConnectionStatus, MessageSeverity, StatusMessage,
};
use crate::domain::settings::SettingsService;
use crate::infrastructure::bluetooth::BluetoothService;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::error;

pub fn spawn_bluetooth_worker(
    event_sender: mpsc::UnboundedSender<AppEvent>,
    mut command_receiver: mpsc::UnboundedReceiver<BluetoothCommand>,
    settings: Arc<Mutex<SettingsService>>,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("Failed to create tokio runtime for Bluetooth");

        rt.block_on(async move {
            let error_sender = event_sender.clone();
            let mut bt_service = BluetoothService::new(event_sender, settings);

            while let Some(cmd) = command_receiver.recv().await {
                match cmd {
                    BluetoothCommand::Connect(address) => {
                        if let Err(e) = bt_service.connect(address).await {
                            error!("Connection failed: {}", e);
                            let _ = error_sender.send(AppEvent::LogMessage(StatusMessage {
                                message: format!("Connection failed: {}", e),
                                severity: MessageSeverity::Error,
                            }));
                            let _ = error_sender
                                .send(AppEvent::ConnectionStatus(ConnectionStatus::Disconnected));
                        }
                    }
                    BluetoothCommand::Disconnect => {
                        bt_service.disconnect();
                    }
                    BluetoothCommand::StartScan => {
                        if let Err(e) = bt_service.start_scan() {
                            error!("Failed to start scan: {}", e);
                        }
                    }
                    BluetoothCommand::StopScan => {
                        if let Err(e) = bt_service.stop_scan() {
                            error!("Failed to stop scan: {}", e);
                        }
                    }
                }
            }
        });
    });
}
