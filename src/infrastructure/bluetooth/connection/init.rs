use super::BleConnection;
use crate::domain::models::MessageSeverity;
use crate::infrastructure::bluetooth::protocol::{COMMAND_DELAY_MS, INIT_SEQUENCE};
use anyhow::Result;
use tracing::info;
use windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristic;
use windows::Storage::Streams::DataWriter;

impl BleConnection {
    /// Send initialization commands to the controller.
    pub(super) async fn send_init_commands(&self, cmd_char: &GattCharacteristic) -> Result<()> {
        info!(
            event = "ble.init.started",
            "Controller initialization started"
        );
        self.send_log("Initializing controller...", MessageSeverity::Info);

        for (step, (command, repeat)) in INIT_SEQUENCE.iter().enumerate() {
            for iteration in 0..*repeat {
                let writer = DataWriter::new()?;
                writer.WriteBytes(command.as_bytes())?;
                let buffer = writer.DetachBuffer()?;

                let status = super::super::diagnostics::operation("write_init_command", async {
                    cmd_char.WriteValueAsync(&buffer)?.await
                })
                .await?;
                tracing::debug!(
                    event = "ble.init.command.result",
                    step,
                    iteration,
                    status = status.0,
                    "Controller initialization command result"
                );
                super::gatt::check_status("Write controller initialization command", status)?;
                tokio::time::sleep(tokio::time::Duration::from_millis(COMMAND_DELAY_MS)).await;
            }
        }

        info!(
            event = "ble.init.finished",
            "Controller initialization finished"
        );
        Ok(())
    }
}
