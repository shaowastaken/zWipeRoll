use egui::{vec2, Color32, CornerRadius, RichText, ScrollArea, Stroke, Ui};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalScanAction {
    None,
    Refresh,
    AddRoot,
    Start,
    Cancel,
    OpenLibrary,
}

pub struct GlobalScanView;

impl GlobalScanView {
    pub fn ui(
        ui: &mut Ui,
        roots: &mut [(PathBuf, bool)],
        running: bool,
        progress_text: &str,
        report_text: Option<&str>,
    ) -> GlobalScanAction {
        let dark = ui.visuals().dark_mode;
        let mut action = GlobalScanAction::None;
        ui.add_space(14.0);
        ui.heading(RichText::new("Global Scan").size(24.0).strong());
        ui.label(
            RichText::new(
                "Choose the drives and locations to index. Your album progress is preserved.",
            )
            .color(crate::theme::muted(dark)),
        );
        ui.add_space(18.0);

        ScrollArea::vertical()
            .id_salt("global_scan_content")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::new()
                    .fill(crate::theme::surface(dark))
                    .stroke(Stroke::new(1.0, crate::theme::border(dark)))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("Scan locations").strong().size(16.0));
                        ui.add_space(8.0);
                        ui.add_enabled_ui(!running, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("Refresh drives").clicked() {
                                    action = GlobalScanAction::Refresh;
                                }
                                if ui.button("Add location…").clicked() {
                                    action = GlobalScanAction::AddRoot;
                                }
                            });
                            ui.add_space(8.0);
                            if roots.is_empty() {
                                ui.label("No drives found. Refresh or add a location to scan.");
                            }
                            for (path, selected) in roots.iter_mut() {
                                ui.horizontal(|ui| {
                                    ui.checkbox(selected, "");
                                    ui.add(egui::Label::new(path.display().to_string()).wrap());
                                });
                            }
                        });
                        ui.add_space(12.0);
                        ui.add(egui::Label::new(RichText::new("Indexes JPG, JPEG, PNG, WebP, GIF and BMP. System folders and user AppData are excluded; symbolic links are not followed. Unreadable paths appear in the report.").small()).wrap());
                        let selected = roots.iter().filter(|(_, selected)| *selected).count();
                        ui.label(
                            RichText::new(format!("{selected} locations selected"))
                                .color(crate::theme::muted(dark)),
                        );
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            if running {
                                if ui.button("Cancel scan").clicked() {
                                    action = GlobalScanAction::Cancel;
                                }
                            } else if ui
                                .add_enabled(
                                    selected > 0,
                                    egui::Button::new(
                                        RichText::new("Scan selected locations")
                                            .color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(10, 117, 246))
                                    .min_size(vec2(180.0, 36.0)),
                                )
                                .clicked()
                            {
                                action = GlobalScanAction::Start;
                            }
                            if ui.button("Open library").clicked() {
                                action = GlobalScanAction::OpenLibrary;
                            }
                        });
                    });

                ui.add_space(16.0);
                if running || !progress_text.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        if running {
                            ui.spinner();
                        }
                        ui.add(egui::Label::new(progress_text).wrap());
                    });
                    ui.add_space(12.0);
                }
                if let Some(report) = report_text.filter(|text| !text.is_empty()) {
                    ui.separator();
                    ui.add_space(8.0);
                    ui.label(RichText::new("Scan report").strong().size(16.0));
                    ui.add_space(6.0);
                    ui.add(egui::Label::new(report).wrap());
                }
            });
        action
    }
}
