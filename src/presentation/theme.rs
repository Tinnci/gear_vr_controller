use eframe::egui;

pub struct BrutalistPalette {
    pub bg: egui::Color32,
    pub fg: egui::Color32,
    pub stroke: egui::Color32,
    pub accent_yellow: egui::Color32,
    pub accent_green: egui::Color32,
    pub accent_cyan: egui::Color32,
    pub accent_red: egui::Color32,
    pub accent_blue: egui::Color32,
}

impl BrutalistPalette {
    pub fn new(is_dark: bool) -> Self {
        if is_dark {
            Self {
                bg: egui::Color32::from_rgb(25, 25, 25),
                fg: egui::Color32::WHITE,
                stroke: egui::Color32::WHITE,
                accent_yellow: egui::Color32::from_rgb(255, 200, 0),
                accent_green: egui::Color32::from_rgb(0, 255, 127),
                accent_cyan: egui::Color32::from_rgb(0, 255, 255),
                accent_red: egui::Color32::from_rgb(255, 80, 80),
                accent_blue: egui::Color32::from_rgb(80, 80, 255),
            }
        } else {
            Self {
                bg: egui::Color32::from_rgb(245, 245, 245),
                fg: egui::Color32::BLACK,
                stroke: egui::Color32::BLACK,
                accent_yellow: egui::Color32::from_rgb(255, 220, 0),
                accent_green: egui::Color32::from_rgb(0, 255, 100),
                accent_cyan: egui::Color32::from_rgb(0, 200, 255),
                accent_red: egui::Color32::from_rgb(255, 50, 50),
                accent_blue: egui::Color32::from_rgb(50, 50, 255),
            }
        }
    }
}

pub fn configure_neubrutalism(ctx: &egui::Context, is_dark: bool) {
    let mut style = (*ctx.style()).clone();
    let palette = BrutalistPalette::new(is_dark);

    // Typography
    style
        .text_styles
        .iter_mut()
        .for_each(|(text_style, font_id)| {
            font_id.size = match text_style {
                egui::TextStyle::Heading => 28.0,
                egui::TextStyle::Body => 15.0,
                egui::TextStyle::Button => 15.0,
                _ => font_id.size,
            };
        });

    // Spacing
    style.spacing.item_spacing = egui::vec2(12.0, 12.0);
    style.spacing.button_padding = egui::vec2(16.0, 10.0);

    // Visuals
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(2.0, palette.stroke);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
    style.visuals.widgets.noninteractive.bg_fill = palette.bg;

    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(2.0, palette.stroke);
    style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
    style.visuals.widgets.inactive.bg_fill = if is_dark {
        egui::Color32::from_gray(30)
    } else {
        egui::Color32::WHITE
    };
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.fg);

    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(2.5, palette.stroke);
    style.visuals.widgets.hovered.rounding = egui::Rounding::ZERO;
    style.visuals.widgets.hovered.bg_fill = palette.accent_yellow;
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
    style.visuals.widgets.hovered.expansion = 2.0;

    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(3.0, palette.stroke);
    style.visuals.widgets.active.rounding = egui::Rounding::ZERO;
    style.visuals.widgets.active.bg_fill = palette.accent_green;
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);

    style.visuals.selection.stroke = egui::Stroke::new(1.0, palette.stroke);
    style.visuals.selection.bg_fill = palette.accent_cyan;

    style.visuals.window_rounding = egui::Rounding::ZERO;
    style.visuals.window_stroke = egui::Stroke::new(2.0, palette.stroke);
    style.visuals.window_shadow = egui::Shadow {
        offset: egui::vec2(8.0, 8.0),
        blur: 0.0,
        spread: 0.0,
        color: palette.stroke,
    };
    style.visuals.window_fill = palette.bg;

    style.visuals.panel_fill = palette.bg;
    style.visuals.override_text_color = Some(palette.fg);

    ctx.set_style(style);
}
