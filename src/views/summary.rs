use egui::{vec2, Align, Color32, CornerRadius, Layout, RichText, ScrollArea, Stroke, Ui};
use crate::filesystem::{move_to_recycle_bin, DeletionResult};
use crate::model::{Decision, Session};

pub enum SummaryAction {
    None,
    BackToSwipe,
    FinishedDeletion,
}

pub struct SummaryView {
    deletion_result: Option<DeletionResult>,
}

impl Default for SummaryView {
    fn default() -> Self {
        Self {
            deletion_result: None,
        }
    }
}

impl SummaryView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui(&mut self, ui: &mut Ui, session: &mut Session) -> SummaryAction {
        let mut action = SummaryAction::None;

        ui.horizontal(|ui| {
            if ui.button("← Back to Review").clicked() {
                action = SummaryAction::BackToSwipe;
            }

            ui.heading("🗑️ Trash Review");
        });

        ui.separator();

        let stats = session.stats();

        if let Some(res) = &self.deletion_result {
            let mut clear_result = false;
            let deleted_count = res.deleted_count;
            let freed_bytes = res.freed_bytes;
            let error_count = res.errors.len();

            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(format!(
                        "✅ Moved {} items to the Windows Recycle Bin!",
                        deleted_count
                    ))
                    .size(20.0)
                    .color(Color32::from_rgb(34, 197, 94)) // #22C55E
                    .strong(),
                );

                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!(
                        "Total space recovered: {}",
                        crate::model::media::format_bytes(freed_bytes)
                    ))
                    .size(16.0)
                    .color(Color32::from_rgb(58, 207, 191)), // Mint #3ACFBF
                );

                if error_count > 0 {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!("⚠️ {error_count} errors occurred"))
                            .color(Color32::from_rgb(239, 68, 68)),
                    );
                }

                ui.add_space(24.0);
                if ui
                    .add(
                        egui::Button::new(RichText::new("OK, Back to App").size(15.0).strong())
                            .fill(Color32::from_rgb(12, 165, 214)) // Azure #0CA5D6
                            .min_size(vec2(160.0, 40.0)),
                    )
                    .clicked()
                {
                    clear_result = true;
                    action = SummaryAction::FinishedDeletion;
                }
            });

            if clear_result {
                self.deletion_result = None;
            }

            return action;
        }

        if stats.trashed == 0 {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new("No items marked for deletion.").size(16.0));
                ui.add_space(20.0);
                if ui.button("Back to Review").clicked() {
                    action = SummaryAction::BackToSwipe;
                }
            });
            return action;
        }

        // Summary Banner
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "You are about to free {} ({} photos)",
                    crate::model::media::format_bytes(stats.trash_bytes),
                    stats.trashed
                ))
                .size(18.0)
                .color(Color32::from_rgb(12, 165, 214)) // Azure
                .strong(),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let delete_btn = egui::Button::new(
                    RichText::new(format!(
                        "🗑️ MOVE TO RECYCLE BIN ({})",
                        stats.trashed
                    ))
                    .color(crate::theme::text(ui.visuals().dark_mode))
                    .size(15.0)
                    .strong(),
                )
                .fill(Color32::from_rgb(239, 68, 68)) // Danger #EF4444
                .min_size(vec2(300.0, 42.0));

                if ui.add(delete_btn).clicked() {
                    let trash_targets: Vec<_> = session
                        .items
                        .iter()
                        .filter(|item| item.decision == Decision::Trash)
                        .map(|item| (item.path.clone(), item.file_size))
                        .collect();

                    let res = move_to_recycle_bin(&trash_targets);

                    session
                        .items
                        .retain(|item| item.decision != Decision::Trash);
                    session.current_index = session.current_index.min(session.items.len().saturating_sub(1));
                    session.undo_stack.clear();
                    session.save_session();

                    self.deletion_result = Some(res);
                }
            });
        });

        ui.add_space(10.0);
        ui.separator();

        ui.label(
            RichText::new("Review these items before moving them to the Recycle Bin:")
                .color(crate::theme::muted(ui.visuals().dark_mode)),
        );
        ui.add_space(6.0);

        ScrollArea::vertical().show(ui, |ui| {
            let mut to_keep = Vec::new();

            for (idx, item) in session.items.iter().enumerate() {
                if item.decision == Decision::Trash {
                    egui::Frame::group(ui.style())
                        .fill(crate::theme::surface_raised(ui.visuals().dark_mode))
                        .corner_radius(CornerRadius::same(8))
                        .stroke(Stroke::new(1.0, crate::theme::border(ui.visuals().dark_mode)))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("🗑️");
                                ui.label(RichText::new(&item.file_name).strong());
                                ui.label(
                                    RichText::new(format!("({})", item.formatted_size()))
                                        .color(crate::theme::muted(ui.visuals().dark_mode)),
                                );
                                ui.label(
                                    RichText::new(item.year_month_label())
                                        .color(crate::theme::muted(ui.visuals().dark_mode)),
                                );

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("⭐ Keep")
                                                    .color(crate::theme::text(ui.visuals().dark_mode)),
                                            )
                                            .fill(Color32::from_rgb(34, 197, 94)), // Keep #22C55E
                                        )
                                        .clicked()
                                    {
                                        to_keep.push(idx);
                                    }
                                });
                            });
                        });
                }
            }

            for idx in to_keep {
                if let Some(item) = session.items.get_mut(idx) {
                    item.decision = Decision::Keep;
                }
            }
        });

        action
    }
}
