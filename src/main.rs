mod application;
mod domain;
mod infrastructure;
mod presentation;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    // We will initialize logging later after loading settings, or initialize a default one first.
    // For now, let's just set up a basic subscriber that might be reloaded or just simple init.
    // Actually, the requirement is to use "most standardized modern rust logging system" and "expose fields".
    // So we should load settings first, then init logging.

    // However, we might want to log startup even before settings are loaded.
    // Let's rely on the UI or Infrastructure layer to set this up.
    // For this step, I will just update the mods and keep the structure clean.

    // Temporary basic init to catch early logs if needed, or we can delegate to the App::new
    // But since we want to be "modern" and "compliant", let's do it right.
    // We'll call a setup function.

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Gear VR Controller"),
        ..Default::default()
    };

    eframe::run_native(
        "Gear VR Controller",
        options,
        Box::new(|cc| Ok(Box::new(presentation::ui::GearVRApp::new(cc)))),
    )
}
