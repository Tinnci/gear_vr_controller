use anyhow::Result;
use interprocess::local_socket::{
    traits::ListenerExt, GenericFilePath, ListenerOptions, Stream as LocalStream, ToFsName,
};
use interprocess::TryClone;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::Command;
use tracing::{error, info};

// Unique name for the named pipe
pub const PIPE_NAME: &str = "@gear_vr_admin_worker";

#[derive(Serialize, Deserialize, Debug)]
pub enum AdminCommand {
    Ping,
    RemoveGhostDevice(String), // InstanceId
    RestartBluetoothService,
    Quit,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AdminResponse {
    Pong,
    Success(String),
    Error(String),
}

/// Run the admin worker loop (this runs in the Elevated process)
pub fn run_admin_worker() -> Result<()> {
    // Setup logging for the worker (maybe to a file since no console)
    let log_path = std::env::temp_dir().join("gear_vr_admin.log");
    let file = std::fs::File::create(log_path).ok();
    tracing_subscriber::fmt()
        .with_writer(move || -> Box<dyn std::io::Write + Send + Sync> {
            if let Some(f) = file.as_ref().and_then(|f| f.try_clone().ok()) {
                Box::new(f)
            } else {
                Box::new(std::io::stdout())
            }
        })
        .init();

    info!("Admin worker started");

    let pipe_name = PIPE_NAME.to_fs_name::<GenericFilePath>()?;
    let listener = ListenerOptions::new().name(pipe_name).create_sync()?;

    info!("Listing on named pipe...");

    for conn in listener.incoming().filter_map(|x| x.ok()) {
        info!("Client connected");
        if let Err(e) = handle_connection(conn) {
            error!("Connection error: {}", e);
        }
    }

    Ok(())
}

fn handle_connection(mut stream: LocalStream) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut buffer = String::new();

    loop {
        buffer.clear();
        match reader.read_line(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(_) => {
                if let Ok(cmd) = serde_json::from_str::<AdminCommand>(&buffer) {
                    info!("Received command: {:?}", cmd);
                    let response = execute_command(cmd);
                    let json = serde_json::to_string(&response)? + "\n";
                    stream.write_all(json.as_bytes())?;
                    stream.flush()?;

                    // If quit, we can exit the process
                    if let AdminCommand::Quit = serde_json::from_str(&buffer).unwrap() {
                        std::process::exit(0);
                    }
                }
            }
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

fn execute_command(cmd: AdminCommand) -> AdminResponse {
    match cmd {
        AdminCommand::Ping => AdminResponse::Pong,
        AdminCommand::RemoveGhostDevice(instance_id) => {
            info!("Removing device: {}", instance_id);
            // pnputil /remove-device "InstanceID"
            match Command::new("pnputil")
                .args(&["/remove-device", &instance_id])
                .output()
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if output.status.success() {
                        AdminResponse::Success(stdout.to_string())
                    } else {
                        AdminResponse::Error(stdout.to_string())
                    }
                }
                Err(e) => AdminResponse::Error(e.to_string()),
            }
        }
        AdminCommand::RestartBluetoothService => {
            info!("Restarting Bluetooth service...");
            // powershell -Command "Restart-Service bthserv -Force"
            match Command::new("powershell")
                .args(&["-Command", "Restart-Service bthserv -Force"])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        AdminResponse::Success("Bluetooth service restarted".to_string())
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        AdminResponse::Error(stderr.to_string())
                    }
                }
                Err(e) => AdminResponse::Error(e.to_string()),
            }
        }
        AdminCommand::Quit => AdminResponse::Success("Quitting".to_string()),
    }
}
