use gear_vr_controller_rust::{admin_worker, presentation};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--admin-worker".to_string()) {
        if let Err(e) = admin_worker::run_admin_worker() {
            eprintln!("Admin worker failed: {}", e);
        }
        return Ok(());
    }

    // Default: Run native WinUI 3 experience via Windows Reactor
    presentation::run_reactor_app()?;
    Ok(())
}
