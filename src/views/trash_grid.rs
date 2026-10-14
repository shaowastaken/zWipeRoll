use crate::filesystem::{move_to_recycle_bin, DeletionResult};
use crate::imaging::TextureCache;
use crate::model::{Decision, Session};
use egui::{
    vec2, Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui, UiBuilder,
};

pub enum TrashAction {
    None,
}

#[derive(Default)]
pub struct TrashGridView {
    deletion_result: Option<DeletionResult>,
}

impl TrashGridView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        session: &mut Session,
        cache: &mut TextureCache,
        volumes: &[crate::filesystem::volumes::VolumeMount],
    ) -> TrashAction {
        let action = TrashAction::None;
        let stats = session.stats();

        ui.add_space(14.0);

        // Header: "Trash" - "Review the files marked for deletion." (Panel 6)
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading(
                    RichText::new("Trash")
                        .size(24.0)
                        .color(crate::theme::text(ui.visuals().dark_mode))
                        .strong(),
                );
                ui.label(
                    RichText::new("Review the files marked for deletion.")
                        .color(crate::theme::muted(ui.visuals().dark_mode))
                        .size(13.0),
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Empty button
                let empty_btn = egui::Button::new(
                    RichText::new("Empty")
                        .color(crate::theme::text(ui.visuals().dark_mode))
                        .strong(),
                )
                .fill(crate::theme::surface_raised(ui.visuals().dark_mode))
                .min_size(vec2(75.0, 32.0));

                if ui.add_enabled(stats.trashed > 0, empty_btn).clicked() {
                    let trash_targets: Vec<_> = session
                        .items
                        .iter()
                        .filter(|item| item.decision == Decision::Trash)
                        .map(|item| (item.path.clone(), item.file_size))
                        .collect();

                    let res = move_to_recycle_bin(&trash_targets, volumes);
                    session.items.retain(|item| {
                        item.decision != Decision::Trash
                            || res.errors.iter().any(|(path, _)| path == &item.path)
                    });
                    session.current_index = session
                        .current_index
                        .min(session.items.len().saturating_sub(1));
                    session.undo_stack.clear();
                    session.save_session();
                    self.deletion_result = Some(res);
                }

                ui.add_space(12.0);

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{} files", stats.trashed))
                            .size(13.0)
                            .color(crate::theme::text(ui.visuals().dark_mode))
                            .strong(),
                    );
                    ui.label(
                        RichText::new(crate::model::media::format_bytes(stats.trash_bytes))
                            .size(11.0)
                            .color(crate::theme::muted(ui.visuals().dark_mode)),
                    );
                });
            });
        });

        ui.add_space(16.0);

        if let Some(res) = &self.deletion_result {
            let mut close = false;
            let del_cnt = res.deleted_count;
            let freed = res.freed_bytes;
            let error_count = res.errors.len();

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "✅ Moved {} items to the Windows Recycle Bin ({} recovered)",
                            del_cnt,
                            crate::model::media::format_bytes(freed)
                        ))
                        .color(Color32::from_rgb(34, 197, 94))
                        .strong(),
                    );
                    if error_count > 0 {
                        ui.label(
                            RichText::new(format!("{error_count} files could not be moved"))
                                .color(Color32::from_rgb(239, 68, 68)),
                        );
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                });
            });

            if close {
                self.deletion_result = None;
            }
            ui.add_space(10.0);
        }

        if stats.trashed == 0 {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(
                    RichText::new("No items in Trash.")
                        .size(16.0)
                        .color(crate::theme::muted(ui.visuals().dark_mode)),
                );
                ui.add_space(8.0);
                ui.label("Use Review or press A to mark photos for deletion.");
            });
            return action;
        }

        // Thumbnail Grid Area
        let grid_area_h = (ui.available_height() - 70.0).max(200.0);

        ScrollArea::vertical()
            .max_height(grid_area_h)
            .show(ui, |ui| {
                let thumb_size = vec2(110.0, 110.0);
                let col_spacing = 10.0;
                let avail_w = ui.available_width();
                let cols = ((avail_w / (thumb_size.x + col_spacing)).floor() as usize).max(1);

                let mut to_restore = Vec::new();

                let trash_indices: Vec<usize> = session
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| item.decision == Decision::Trash)
                    .map(|(idx, _)| idx)
                    .collect();

                egui::Grid::new("trash_thumb_grid")
                    .spacing(vec2(col_spacing, col_spacing))
                    .show(ui, |ui| {
                        for (i, &idx) in trash_indices.iter().enumerate() {
                            let item = &session.items[idx];
                            cache.request_if_needed(item.path.clone(), 200, 200);

                            let (rect, resp) = ui.allocate_exact_size(thumb_size, Sense::click());
                            let is_hover = resp.hovered();

                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::same(8),
                                crate::theme::surface(ui.visuals().dark_mode),
                            );

                            if let Some(tex) = cache.get(&item.path) {
                                ui.painter().image(
                                    tex.id(),
                                    rect.shrink(2.0),
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            } else {
                                ui.painter().text(
                                    rect.center(),
                                    Align2::CENTER_CENTER,
                                    "📷",
                                    FontId::proportional(22.0),
                                    crate::theme::muted(ui.visuals().dark_mode),
                                );
                            }

                            ui.painter().rect_stroke(
                                rect,
                                CornerRadius::same(8),
                                Stroke::new(
                                    if is_hover { 2.0 } else { 1.0 },
                                    if is_hover {
                                        Color32::from_rgb(12, 165, 214)
                                    } else {
                                        crate::theme::border(ui.visuals().dark_mode)
                                    },
                                ),
                                egui::StrokeKind::Inside,
                            );

                            if resp.on_hover_text("Click to restore").clicked() {
                                to_restore.push(idx);
                            }

                            if (i + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }
                    });

                for idx in to_restore {
                    if let Some(item) = session.items.get_mut(idx) {
                        item.decision = Decision::Keep;
                    }
                }
            });

        // Bottom Action Bar: "342 files selected • 1.8 GB" | Restore All | Move to Recycle Bin
        ui.add_space(8.0);
        let bottom_h = 44.0;
        let (bar_rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), bottom_h), Sense::hover());
        ui.painter().rect_filled(
            bar_rect,
            CornerRadius::same(10),
            crate::theme::surface(ui.visuals().dark_mode),
        );

        ui.painter().text(
            Pos2::new(bar_rect.left() + 16.0, bar_rect.center().y),
            Align2::LEFT_CENTER,
            format!(
                "{} files selected  •  {}",
                stats.trashed,
                crate::model::media::format_bytes(stats.trash_bytes)
            ),
            FontId::proportional(13.0),
            crate::theme::muted(ui.visuals().dark_mode),
        );

        // Buttons at right using new_child
        let mut child_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(bar_rect.shrink2(vec2(8.0, 4.0)))
                .layout(Layout::right_to_left(Align::Center)),
        );

        let del_btn = egui::Button::new(
            RichText::new("Move to Recycle Bin")
                .color(crate::theme::text(ui.visuals().dark_mode))
                .strong(),
        )
        .fill(Color32::from_rgb(239, 68, 68))
        .min_size(vec2(160.0, 34.0));

        if child_ui.add(del_btn).clicked() {
            let trash_targets: Vec<_> = session
                .items
                .iter()
                .filter(|item| item.decision == Decision::Trash)
                .map(|item| (item.path.clone(), item.file_size))
                .collect();

            let res = move_to_recycle_bin(&trash_targets, volumes);
            session.items.retain(|item| {
                item.decision != Decision::Trash
                    || res.errors.iter().any(|(path, _)| path == &item.path)
            });
            session.current_index = session
                .current_index
                .min(session.items.len().saturating_sub(1));
            session.undo_stack.clear();
            session.save_session();
            self.deletion_result = Some(res);
        }

        child_ui.add_space(10.0);

        let restore_btn = egui::Button::new(
            RichText::new("Restore All").color(crate::theme::text(ui.visuals().dark_mode)),
        )
        .fill(crate::theme::surface_raised(ui.visuals().dark_mode))
        .min_size(vec2(100.0, 34.0));

        if child_ui.add(restore_btn).clicked() {
            for item in &mut session.items {
                if item.decision == Decision::Trash {
                    item.decision = Decision::Keep;
                }
            }
        }

        action
    }
}
