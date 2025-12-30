use eframe::egui;

pub struct Components;

impl Components {
    pub fn brutalist_card<R>(
        ui: &mut egui::Ui,
        title: &str,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
        let bg = ui.style().visuals.widgets.noninteractive.bg_fill;

        egui::Frame::none()
            .inner_margin(egui::Margin::same(15.0))
            .stroke(stroke)
            .fill(bg)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(title).strong().size(18.0));
                    ui.add_space(8.0);
                    add_contents(ui)
                })
                .inner
            })
            .inner
    }

    pub fn status_banner(
        ui: &mut egui::Ui,
        text: &str,
        bg_color: egui::Color32,
        text_color: egui::Color32,
    ) {
        ui.add_sized(
            [ui.available_width(), 35.0],
            egui::Label::new(
                egui::RichText::new(text)
                    .color(text_color)
                    .background_color(bg_color)
                    .size(16.0)
                    .strong(),
            )
            .wrap_mode(egui::TextWrapMode::Extend),
        );
    }
}
