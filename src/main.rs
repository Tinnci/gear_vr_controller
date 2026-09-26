mod admin_client;
mod admin_ipc;
mod admin_worker;
mod application;
mod domain;
mod infrastructure;
mod presentation;

use eframe::egui;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--admin-worker".to_string()) {
        if let Err(e) = admin_worker::run_admin_worker() {
            eprintln!("Admin worker failed: {}", e);
        }
        return Ok(());
    }

    if args.contains(&"--egui".to_string()) {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 600.0])
                .with_title("Gear VR Controller (Classic egui)"),
            ..Default::default()
        };

        let _ = eframe::run_native(
            "Gear VR Controller",
            options,
            Box::new(|cc| Ok(Box::new(presentation::GearVRApp::new(cc)))),
        );
        return Ok(());
    }

    // Default: Run native WinUI 3 experience via Windows Reactor
    presentation::run_reactor_app()?;
    Ok(())
}
