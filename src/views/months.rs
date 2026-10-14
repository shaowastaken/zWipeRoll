use std::collections::BTreeMap;
use egui::{vec2, Align, Color32, CornerRadius, Layout, ProgressBar, RichText, ScrollArea, Stroke, Ui};
use crate::model::{Decision, Session};

pub enum MonthAction {
    None,
    OpenSwipe(usize),
    SelectFolder,
    ScanAllPictures,
    OpenSummary,
}

#[derive(Default)]
pub struct MonthGroup {
    pub label: String,
    pub indices: Vec<usize>,
    pub total_bytes: u64,
    pub unreviewed: usize,
    pub kept: usize,
    pub trashed: usize,
}

pub struct MonthView;

impl MonthView {
    pub fn ui(ui: &mut Ui, session: &mut Session) -> MonthAction {
        let mut action = MonthAction::None;

        // Top bar
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            if ui.button("📁 Choose Folder").clicked() {
                action = MonthAction::SelectFolder;
            }

            if ui
                .button(
                    RichText::new("🔍 Scan All Drives")
                        .color(Color32::from_rgb(12, 165, 214)), // Azure #0CA5D6
                )
                .clicked()
            {
                action = MonthAction::ScanAllPictures;
            }

            if let Some(folder) = &session.folder {
                ui.label(
                    RichText::new(format!("Sorgente: {}", folder.display()))
                        .color(crate::theme::muted(ui.visuals().dark_mode)), // Mist
                );
            }

            let stats = session.stats();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if stats.trashed > 0 {
                    let summary_btn = egui::Button::new(
                        RichText::new(format!("🗑️ Trash Summary ({})", stats.trashed))
                            .color(crate::theme::text(ui.visuals().dark_mode))
                            .strong(),
                    )
                    .fill(Color32::from_rgb(239, 68, 68));

                    if ui.add(summary_btn).clicked() {
                        action = MonthAction::OpenSummary;
                    }
                }

                if stats.total > 0 {
                    let swipe_all_btn = egui::Button::new(
                        RichText::new("⚡ Inizia da Prima Foto")
                            .color(crate::theme::text(ui.visuals().dark_mode))
                            .strong(),
                    )
                    .fill(Color32::from_rgb(12, 165, 214)); // Azure #0CA5D6

                    if ui.add(swipe_all_btn).clicked() {
                        action = MonthAction::OpenSwipe(session.current_index);
                    }
                }
            });
        });

        ui.separator();

        if session.items.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.heading(
                    RichText::new("zWipeRoll")
                        .size(36.0)
                        .color(Color32::from_rgb(12, 165, 214))
                        .strong(),
                );
                ui.label(
                    RichText::new("Swipe through. Wipe what you don’t need.")
                        .size(16.0)
                        .color(Color32::from_rgb(58, 207, 191)), // Mint #3ACFBF
                );
                ui.add_space(35.0);

                ui.label(
                    RichText::new("Choose how to start:")
                        .size(18.0)
                        .color(crate::theme::text(ui.visuals().dark_mode)),
                );
                ui.add_space(20.0);

                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 24.0;

                    // Option 1: Select single folder
                    let folder_card = egui::Frame::group(ui.style())
                        .fill(crate::theme::surface_raised(ui.visuals().dark_mode)) // #182936
                        .corner_radius(CornerRadius::same(16))
                        .stroke(Stroke::new(1.0, crate::theme::border(ui.visuals().dark_mode)));

                    folder_card.show(ui, |ui| {
                        ui.set_width(280.0);
                        ui.vertical_centered(|ui| {
                            ui.add_space(15.0);
                            ui.label(RichText::new("📁").size(40.0));
                            ui.heading("Single Folder");
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Choose one folder to review manually.")
                                    .color(crate::theme::muted(ui.visuals().dark_mode))
                                    .size(13.0),
                            );
                            ui.add_space(15.0);
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Choose Folder...")
                                            .color(crate::theme::text(ui.visuals().dark_mode))
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(17, 118, 145)) // Deep Teal
                                    .min_size(vec2(200.0, 40.0)),
                                )
                                .clicked()
                            {
                                action = MonthAction::SelectFolder;
                            }
                            ui.add_space(15.0);
                        });
                    });

                    // Option 2: Automatic System Scan
                    let auto_card = egui::Frame::group(ui.style())
                        .fill(crate::theme::surface_raised(ui.visuals().dark_mode))
                        .corner_radius(CornerRadius::same(16))
                        .stroke(Stroke::new(1.5, Color32::from_rgb(12, 165, 214))); // Border Azure

                    auto_card.show(ui, |ui| {
                        ui.set_width(280.0);
                        ui.vertical_centered(|ui| {
                            ui.add_space(15.0);
                            ui.label(RichText::new("🌐").size(40.0));
                            ui.heading("All Images on This PC");
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Automatically scan mounted drives and sort images by date.")
                                    .color(crate::theme::muted(ui.visuals().dark_mode))
                                    .size(13.0),
                            );
                            ui.add_space(15.0);
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("⚡ Scansiona Tutto il PC")
                                            .color(crate::theme::text(ui.visuals().dark_mode))
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(12, 165, 214)) // Azure #0CA5D6
                                    .min_size(vec2(200.0, 40.0)),
                                )
                                .clicked()
                            {
                                action = MonthAction::ScanAllPictures;
                            }
                            ui.add_space(15.0);
                        });
                    });
                });
            });
            return action;
        }

        // Group items by Year-Month
        let mut groups: BTreeMap<String, MonthGroup> = BTreeMap::new();

        for (idx, item) in session.items.iter().enumerate() {
            let key = item.year_month_label();
            let group = groups.entry(key.clone()).or_insert_with(|| MonthGroup {
                label: key,
                ..Default::default()
            });

            group.indices.push(idx);
            group.total_bytes += item.file_size;
            match item.decision {
                Decision::Unreviewed => group.unreviewed += 1,
                Decision::Keep => group.kept += 1,
                Decision::Trash => group.trashed += 1,
            }
        }

        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(10.0);
            ui.heading("📅 Review by Month / Date");
            ui.add_space(8.0);

            for (_, group) in groups.iter().rev() {
                egui::Frame::group(ui.style())
                    .fill(crate::theme::surface_raised(ui.visuals().dark_mode))
                    .corner_radius(CornerRadius::same(12))
                    .stroke(Stroke::new(1.0, crate::theme::border(ui.visuals().dark_mode)))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.heading(&group.label);
                                ui.label(format!(
                                    "{} photos  •  {} total",
                                    group.indices.len(),
                                    crate::model::media::format_bytes(group.total_bytes)
                                ));

                                let completed = group.kept + group.trashed;
                                let progress = if !group.indices.is_empty() {
                                    completed as f32 / group.indices.len() as f32
                                } else {
                                    0.0
                                };

                                ui.add_space(4.0);
                                ui.add(
                                    ProgressBar::new(progress)
                                        .text(format!("{:.0}% revisionato", progress * 100.0))
                                        .desired_width(260.0),
                                );
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let first_unreviewed = group
                                    .indices
                                    .iter()
                                    .find(|&&idx| session.items[idx].decision == Decision::Unreviewed)
                                    .copied()
                                    .unwrap_or_else(|| *group.indices.first().unwrap_or(&0));

                                let btn_text = if group.unreviewed > 0 {
                                    format!("🧹 Review Month ({} remaining)", group.unreviewed)
                                } else {
                                    "🔍 Rivedi Foto".to_string()
                                };

                                let clean_btn = egui::Button::new(
                                    RichText::new(btn_text)
                                        .color(crate::theme::text(ui.visuals().dark_mode))
                                        .strong(),
                                )
                                .fill(if group.unreviewed > 0 {
                                    Color32::from_rgb(34, 197, 94) // Green #22C55E
                                } else {
                                    Color32::from_rgb(17, 118, 145) // Deep Teal
                                })
                                .min_size(vec2(220.0, 38.0));

                                if ui.add(clean_btn).clicked() {
                                    action = MonthAction::OpenSwipe(first_unreviewed);
                                }
                            });
                        });
                    });
                ui.add_space(8.0);
            }
        });

        action
    }
}
