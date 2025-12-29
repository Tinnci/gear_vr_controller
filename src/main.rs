mod bluetooth;
mod controller;
mod input_simulator;
mod models;
mod settings;
mod ui;

use eframe::egui;
use env_logger;
use log::info;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    info!("Starting Gear VR Controller Application");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Gear VR Controller"),
        ..Default::default()
    };

    eframe::run_native(
        "Gear VR Controller",
        options,
        Box::new(|cc| Ok(Box::new(ui::GearVRApp::new(cc)))),
    )
}

