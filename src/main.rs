mod admin_client;
mod admin_ipc;
mod admin_worker;
mod application;
mod domain;
mod infrastructure;
mod presentation;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--admin-worker".to_string()) {
        if let Err(e) = admin_worker::run_admin_worker() {
            eprintln!("Admin worker failed: {}", e);
        }
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Gear VR Controller"),
        ..Default::default()
    };

    eframe::run_native(
        "Gear VR Controller",
        options,
        Box::new(|cc| Ok(Box::new(presentation::GearVRApp::new(cc)))),
    )
}
