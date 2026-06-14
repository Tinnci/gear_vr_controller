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
        info!("Sending initialization commands...");
        self.send_log("Initializing controller...", MessageSeverity::Info);

        for (command, repeat) in INIT_SEQUENCE {
            for _ in 0..*repeat {
                let writer = DataWriter::new()?;
                writer.WriteBytes(command.as_bytes())?;
                let buffer = writer.DetachBuffer()?;

                let _ = cmd_char.WriteValueAsync(&buffer)?;
                tokio::time::sleep(tokio::time::Duration::from_millis(COMMAND_DELAY_MS)).await;
            }
        }

        info!("Initialization commands sent");
        Ok(())
    }
}
