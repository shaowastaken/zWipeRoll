use egui::{Color32, CornerRadius, RichText, Ui};

pub struct SettingsView;

impl SettingsView {
    pub fn ui(ui: &mut Ui) {
        ui.add_space(20.0);
        ui.heading(
            RichText::new("Settings")
                .size(24.0)
                .color(Color32::WHITE)
                .strong(),
        );
        ui.add_space(16.0);

        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(20, 34, 46))
            .corner_radius(CornerRadius::same(12))
            .show(ui, |ui| {
                ui.set_width(450.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Startup & Storage").strong().color(Color32::WHITE));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("zWipeRoll lavora in locale senza inviare dati in rete.")
                            .size(13.0)
                            .color(Color32::from_rgb(142, 171, 192)),
                    );
                    ui.add_space(12.0);

                    ui.label(RichText::new("Performance & Decoding").strong().color(Color32::WHITE));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Background decoders: 3 thread pool con downscaling attivo.")
                            .size(13.0)
                            .color(Color32::from_rgb(142, 171, 192)),
                    );
                    ui.add_space(12.0);

                    ui.label(RichText::new("Shortcuts").strong().color(Color32::WHITE));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("• A / Swipe Left / Cerchio Rosso: Cestina\n• D / Swipe Right / Cerchio Verde: Conserva\n• Frecce ← →: Scorri avanti e indietro\n• Ctrl + Z / U: Undo")
                            .size(13.0)
                            .color(Color32::from_rgb(187, 209, 219)),
                    );
                });
            });
    }
}
