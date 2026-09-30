use gear_vr_controller_rust::domain::connection_failure::ConnectionFailure;
use serde_json::{json, Value};
use std::{
    io::{self, Write},
    time::Instant,
};

pub struct Output {
    started: Instant,
}
impl Output {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
        }
    }
    pub fn emit(&self, event: &str, data: Value) -> anyhow::Result<()> {
        let record = json!({"schema": 1, "event": event, "pid": std::process::id(), "elapsed_ms": self.started.elapsed().as_millis() as u64, "data": data});
        let mut stdout = io::stdout().lock();
        serde_json::to_writer(&mut stdout, &record)?;
        writeln!(stdout)?;
        stdout.flush()?;
        Ok(())
    }
    pub fn error(&self, error: &anyhow::Error) -> anyhow::Result<()> {
        self.emit("failed", json!({"success": false, "detail": format!("{error:#}"),
            "kind": error.downcast_ref::<ConnectionFailure>().map(|failure| format!("{:?}", failure.kind)),
            "hresult": error.downcast_ref::<windows::core::Error>().map(|error| format!("0x{:08X}", error.code().0 as u32)),
        }))
    }
}
