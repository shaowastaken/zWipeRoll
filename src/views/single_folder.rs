use crate::model::Session;
use egui::{vec2, Color32, CornerRadius, RichText, Ui};

pub struct SingleFolderView;

impl SingleFolderView {
    pub fn ui(ui: &mut Ui, session: &Session) -> bool {
        let dark = ui.visuals().dark_mode;
        ui.add_space(18.0);
        ui.heading(
            RichText::new("Single Folder")
                .size(24.0)
                .color(crate::theme::text(dark))
                .strong(),
        );
        ui.label(
            RichText::new("Choose one folder and review every supported image inside it.")
                .size(13.0)
                .color(crate::theme::muted(dark)),
        );
        ui.add_space(22.0);

        let mut browse = false;
        egui::Frame::new()
            .fill(crate::theme::surface(dark))
            .stroke(egui::Stroke::new(1.0, crate::theme::border(dark)))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(18.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width().min(680.0));
                ui.label(
                    RichText::new("Selected folder")
                        .strong()
                        .color(crate::theme::text(dark)),
                );
                ui.add_space(6.0);
                let source = session.folder.as_ref().map_or_else(
                    || "No folder selected".to_owned(),
                    |path| path.display().to_string(),
                );
                ui.label(RichText::new(source).color(crate::theme::muted(dark)));
                if !session.items.is_empty() {
                    ui.label(
                        RichText::new(format!("{} images found", session.items.len()))
                            .color(crate::theme::muted(dark)),
                    );
                }
                ui.add_space(16.0);
                browse = ui
                    .add(
                        egui::Button::new(RichText::new("Choose folder…").color(Color32::WHITE))
                            .fill(Color32::from_rgb(10, 117, 246))
                            .min_size(vec2(150.0, 38.0)),
                    )
                    .clicked();
            });
        browse
    }
}
