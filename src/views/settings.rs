use crate::theme::ThemeMode;
use egui::{CornerRadius, RichText, Ui};

pub struct SettingsView;

impl SettingsView {
    pub fn ui(ui: &mut Ui, theme: &mut ThemeMode) {
        ui.add_space(20.0);
        ui.heading(
            RichText::new("Settings")
                .size(24.0)
                .color(crate::theme::text(ui.visuals().dark_mode))
                .strong(),
        );
        ui.add_space(16.0);

        egui::Frame::group(ui.style())
            .fill(crate::theme::surface(ui.visuals().dark_mode))
            .corner_radius(CornerRadius::same(12))
            .show(ui, |ui| {
                ui.set_width(450.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Appearance").strong().color(ui.visuals().text_color()));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("Theme");
                        ui.selectable_value(theme, ThemeMode::Light, "Light");
                        ui.selectable_value(theme, ThemeMode::Dark, "Dark");
                    });
                    ui.add_space(16.0);
                    ui.label(RichText::new("Startup & Storage").strong().color(crate::theme::text(ui.visuals().dark_mode)));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("zWipeRoll works locally and never uploads your photos.")
                            .size(13.0)
                            .color(crate::theme::muted(ui.visuals().dark_mode)),
                    );
                    ui.add_space(12.0);

                    ui.label(RichText::new("Performance & Decoding").strong().color(crate::theme::text(ui.visuals().dark_mode)));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Background decoding: 3 worker threads with downscaling enabled.")
                            .size(13.0)
                            .color(crate::theme::muted(ui.visuals().dark_mode)),
                    );
                    ui.add_space(12.0);

                    ui.label(RichText::new("Shortcuts").strong().color(crate::theme::text(ui.visuals().dark_mode)));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("• A / Swipe Left / Red button: Delete\n• D / Swipe Right / Green button: Keep\n• Arrow keys ← →: Browse photos\n• Ctrl + Z / U: Undo")
                            .size(13.0)
                            .color(crate::theme::muted(ui.visuals().dark_mode)),
                    );
                });
            });
    }
}
