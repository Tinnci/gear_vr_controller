//! Radial Menu Overlay
//!
//! A pie-menu style overlay that appears when the trigger is held down.
//! Users can select options by moving their finger on the touchpad.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use std::f32::consts::PI;

/// Available control modes for the controller
/// Available control modes for the controller
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlMode {
    #[default]
    Mouse, // Air Mouse Mode (IMU cursor + TP scroll)
    Touchpad,     // Laptop Trackpad Mode (TP cursor + Button scroll)
    Presentation, // PPT/Media Mode (Buttons only)
    Settings,     // Quick Settings / Calibration
}

impl ControlMode {
    pub fn name(&self) -> &'static str {
        match self {
            ControlMode::Mouse => "Air Mouse",
            ControlMode::Touchpad => "Touchpad",
            ControlMode::Presentation => "Presenter",
            ControlMode::Settings => "Settings",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ControlMode::Mouse => "✈️",
            ControlMode::Touchpad => "🖱️",
            ControlMode::Presentation => "📽️",
            ControlMode::Settings => "⚙️",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ControlMode::Mouse => "Wave to move, Touch to scroll",
            ControlMode::Touchpad => "Laptop style control",
            ControlMode::Presentation => "PPT & Media control",
            ControlMode::Settings => "Calibration & Options",
        }
    }
}

/// Radial menu item
#[derive(Debug, Clone)]
pub struct RadialMenuItem {
    pub mode: ControlMode,
    pub angle_start: f32, // in radians
    pub angle_end: f32,   // in radians
}

/// Radial menu state and rendering
pub struct RadialMenu {
    pub is_visible: bool,
    pub center_pos: Pos2,
    pub selected_index: Option<usize>,
    pub items: Vec<RadialMenuItem>,
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub dead_zone_radius: f32,
}

impl Default for RadialMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl RadialMenu {
    pub fn new() -> Self {
        let modes = [
            ControlMode::Mouse,
            ControlMode::Touchpad,
            ControlMode::Presentation,
            ControlMode::Settings,
        ];

        let item_count = modes.len();
        let angle_per_item = 2.0 * PI / item_count as f32;

        let items: Vec<RadialMenuItem> = modes
            .iter()
            .enumerate()
            .map(|(i, &mode)| {
                let angle_start = -PI / 2.0 + (i as f32) * angle_per_item - angle_per_item / 2.0;
                let angle_end = angle_start + angle_per_item;
                RadialMenuItem {
                    mode,
                    angle_start,
                    angle_end,
                }
            })
            .collect();

        Self {
            is_visible: false,
            center_pos: Pos2::ZERO,
            selected_index: None,
            items,
            outer_radius: 120.0,
            inner_radius: 40.0,
            dead_zone_radius: 25.0,
        }
    }

    /// Show the menu at the given screen position
    pub fn show(&mut self, center: Pos2) {
        self.is_visible = true;
        self.center_pos = center;
        self.selected_index = None;
    }

    /// Hide the menu and return the selected mode (if any)
    pub fn hide(&mut self) -> Option<ControlMode> {
        self.is_visible = false;
        self.selected_index.map(|i| self.items[i].mode)
    }

    /// Update selection based on touchpad position (-1 to 1 range)
    pub fn update_selection(&mut self, touchpad_x: f64, touchpad_y: f64) {
        let distance = (touchpad_x * touchpad_x + touchpad_y * touchpad_y).sqrt();

        // Dead zone in center - no selection
        if distance < 0.3 {
            self.selected_index = None;
            return;
        }

        // Calculate angle from touchpad position
        let angle = (touchpad_y as f32).atan2(touchpad_x as f32);

        // Find which item this angle falls into
        for (i, item) in self.items.iter().enumerate() {
            let mut item_start = item.angle_start;
            let mut item_end = item.angle_end;

            // Normalize angles for comparison
            while item_start > PI {
                item_start -= 2.0 * PI;
            }
            while item_start < -PI {
                item_start += 2.0 * PI;
            }
            while item_end > PI {
                item_end -= 2.0 * PI;
            }
            while item_end < -PI {
                item_end += 2.0 * PI;
            }

            // Check if angle falls within this item's range
            let in_range = if item_start <= item_end {
                angle >= item_start && angle < item_end
            } else {
                // Wraps around -PI/PI
                angle >= item_start || angle < item_end
            };

            if in_range {
                self.selected_index = Some(i);
                return;
            }
        }
    }

    /// Render the radial menu
    pub fn render(&self, ctx: &egui::Context) {
        if !self.is_visible {
            return;
        }

        // Create a full-screen overlay layer
        egui::Area::new(egui::Id::new("radial_menu_overlay"))
            .fixed_pos(Pos2::ZERO)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();

                // Semi-transparent background
                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(0, 0, 0, 120),
                );

                let painter = ui.painter();
                let center = self.center_pos;

                // Draw outer ring background
                painter.circle_filled(
                    center,
                    self.outer_radius,
                    Color32::from_rgba_unmultiplied(40, 40, 50, 240),
                );

                // Draw each segment
                for (i, item) in self.items.iter().enumerate() {
                    let is_selected = self.selected_index == Some(i);
                    self.draw_segment(painter, center, item, is_selected);
                }

                // Draw inner circle (dead zone / cancel area)
                painter.circle_filled(center, self.inner_radius, Color32::from_rgb(30, 30, 40));
                painter.circle_stroke(
                    center,
                    self.inner_radius,
                    Stroke::new(2.0, Color32::from_rgb(100, 100, 120)),
                );

                // Draw center icon
                let center_text = if self.selected_index.is_some() {
                    "✓"
                } else {
                    "✕"
                };
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    center_text,
                    egui::FontId::proportional(24.0),
                    Color32::WHITE,
                );

                // Draw instruction text
                let instruction = if let Some(idx) = self.selected_index {
                    format!("Release to select: {}", self.items[idx].mode.name())
                } else {
                    "Move to select, release to cancel".to_string()
                };
                painter.text(
                    center + Vec2::new(0.0, self.outer_radius + 30.0),
                    egui::Align2::CENTER_CENTER,
                    instruction,
                    egui::FontId::proportional(14.0),
                    Color32::WHITE,
                );
            });
    }

    fn draw_segment(
        &self,
        painter: &egui::Painter,
        center: Pos2,
        item: &RadialMenuItem,
        is_selected: bool,
    ) {
        let segments = 32;
        let angle_step = (item.angle_end - item.angle_start) / segments as f32;

        // Draw filled arc
        let fill_color = if is_selected {
            Color32::from_rgb(80, 140, 220)
        } else {
            Color32::from_rgb(60, 60, 80)
        };

        let mut points = Vec::new();

        // Inner arc points
        for i in 0..=segments {
            let angle = item.angle_start + i as f32 * angle_step;
            points.push(Pos2::new(
                center.x + self.inner_radius * angle.cos(),
                center.y + self.inner_radius * angle.sin(),
            ));
        }

        // Outer arc points (reverse order)
        for i in (0..=segments).rev() {
            let angle = item.angle_start + i as f32 * angle_step;
            points.push(Pos2::new(
                center.x + self.outer_radius * angle.cos(),
                center.y + self.outer_radius * angle.sin(),
            ));
        }

        painter.add(egui::Shape::convex_polygon(
            points,
            fill_color,
            Stroke::new(1.5, Color32::from_rgb(100, 100, 120)),
        ));

        // Draw divider lines between segments
        let line_start = Pos2::new(
            center.x + self.inner_radius * item.angle_start.cos(),
            center.y + self.inner_radius * item.angle_start.sin(),
        );
        let line_end = Pos2::new(
            center.x + self.outer_radius * item.angle_start.cos(),
            center.y + self.outer_radius * item.angle_start.sin(),
        );
        painter.line_segment(
            [line_start, line_end],
            Stroke::new(2.0, Color32::from_rgb(50, 50, 60)),
        );

        // Draw icon and label
        let mid_angle = (item.angle_start + item.angle_end) / 2.0;
        let label_radius = (self.inner_radius + self.outer_radius) / 2.0;
        let label_pos = Pos2::new(
            center.x + label_radius * mid_angle.cos(),
            center.y + label_radius * mid_angle.sin(),
        );

        let text_color = if is_selected {
            Color32::WHITE
        } else {
            Color32::from_rgb(200, 200, 210)
        };

        // Icon
        painter.text(
            label_pos + Vec2::new(0.0, -8.0),
            egui::Align2::CENTER_CENTER,
            item.mode.icon(),
            egui::FontId::proportional(20.0),
            text_color,
        );

        // Label
        painter.text(
            label_pos + Vec2::new(0.0, 12.0),
            egui::Align2::CENTER_CENTER,
            item.mode.name(),
            egui::FontId::proportional(11.0),
            text_color,
        );
    }
}
