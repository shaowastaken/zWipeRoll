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
            if ui.button("← Torna alla Revisione").clicked() {
                action = SummaryAction::BackToSwipe;
            }

            ui.heading("🗑️ Revisione Cestino");
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
                        "✅ Eliminati con successo {} elementi nel Cestino di Windows!",
                        deleted_count
                    ))
                    .size(20.0)
                    .color(Color32::from_rgb(34, 197, 94)) // #22C55E
                    .strong(),
                );

                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!(
                        "Spazio totale recuperato: {}",
                        crate::model::media::format_bytes(freed_bytes)
                    ))
                    .size(16.0)
                    .color(Color32::from_rgb(58, 207, 191)), // Mint #3ACFBF
                );

                if error_count > 0 {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!("⚠️ Si sono verificati {} errori", error_count))
                            .color(Color32::from_rgb(239, 68, 68)),
                    );
                }

                ui.add_space(24.0);
                if ui
                    .add(
                        egui::Button::new(RichText::new("OK, Torna all'App").size(15.0).strong())
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
                ui.label(RichText::new("Nessun elemento contrassegnato per il cestino.").size(16.0));
                ui.add_space(20.0);
                if ui.button("Torna allo Swipe").clicked() {
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
                    "Stai per liberare {} ({} foto)",
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
                        "🗑️ CONFERMA ED ELIMINA NEL CESTINO ({})",
                        stats.trashed
                    ))
                    .color(Color32::WHITE)
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
            RichText::new("Controlla gli elementi prima di eliminarli definitivamente:")
                .color(Color32::from_rgb(187, 209, 219)),
        );
        ui.add_space(6.0);

        ScrollArea::vertical().show(ui, |ui| {
            let mut to_keep = Vec::new();

            for (idx, item) in session.items.iter().enumerate() {
                if item.decision == Decision::Trash {
                    egui::Frame::group(ui.style())
                        .fill(Color32::from_rgb(24, 41, 54))
                        .corner_radius(CornerRadius::same(8))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(45, 68, 85)))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("🗑️");
                                ui.label(RichText::new(&item.file_name).strong());
                                ui.label(
                                    RichText::new(format!("({})", item.formatted_size()))
                                        .color(Color32::from_rgb(187, 209, 219)),
                                );
                                ui.label(
                                    RichText::new(item.year_month_label())
                                        .color(Color32::from_rgb(187, 209, 219)),
                                );

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("⭐ Salva (Conserva)")
                                                    .color(Color32::WHITE),
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
