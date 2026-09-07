use std::collections::BTreeMap;
use egui::{
    vec2, Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui,
};
use crate::model::Session;

pub enum FoldersAction {
    None,
    BrowseFolder,
    SelectMonth(usize),
    StartReviewAll,
}

pub struct FoldersView;

impl FoldersView {
    pub fn ui(ui: &mut Ui, session: &mut Session) -> FoldersAction {
        let mut action = FoldersAction::None;

        ui.add_space(14.0);

        // Header: "Select a Folder" (Panel 3)
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading(
                    RichText::new("Select a Folder")
                        .size(24.0)
                        .color(Color32::WHITE)
                        .strong(),
                );
                ui.label(
                    RichText::new("Choose a folder to start cleaning your photos.")
                        .color(Color32::from_rgb(142, 171, 192))
                        .size(13.0),
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if !session.items.is_empty() {
                    let open_btn = egui::Button::new(
                        RichText::new("Open Folder")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(12, 165, 214))
                    .min_size(vec2(120.0, 36.0));

                    if ui.add(open_btn).clicked() {
                        action = FoldersAction::StartReviewAll;
                    }
                }
            });
        });

        ui.add_space(16.0);

        // Path input bar + Browse button
        let bar_h = 38.0;
        let (bar_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), bar_h), Sense::hover());
        ui.painter().rect_filled(
            bar_rect,
            CornerRadius::same(10),
            Color32::from_rgb(20, 34, 46),
        );
        ui.painter().rect_stroke(
            bar_rect,
            CornerRadius::same(10),
            Stroke::new(1.0, Color32::from_rgb(38, 58, 76)),
            egui::StrokeKind::Inside,
        );

        let folder_text = if let Some(p) = &session.folder {
            p.display().to_string()
        } else {
            "Nessuna cartella selezionata".to_string()
        };

        ui.painter().text(
            Pos2::new(bar_rect.left() + 14.0, bar_rect.center().y),
            Align2::LEFT_CENTER,
            format!("📁  {}", folder_text),
            FontId::proportional(13.0),
            Color32::from_rgb(220, 235, 245),
        );

        // Browse... button at right of path bar
        let btn_w = 95.0;
        let btn_rect = Rect::from_min_size(
            Pos2::new(bar_rect.right() - btn_w - 6.0, bar_rect.top() + 4.0),
            vec2(btn_w, 30.0),
        );
        let btn_resp = ui.allocate_rect(btn_rect, Sense::click());
        let is_hover = btn_resp.hovered();

        ui.painter().rect_filled(
            btn_rect,
            CornerRadius::same(8),
            if is_hover {
                Color32::from_rgb(45, 70, 92)
            } else {
                Color32::from_rgb(32, 50, 68)
            },
        );
        ui.painter().text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            "Browse...",
            FontId::proportional(13.0),
            Color32::WHITE,
        );

        if btn_resp.clicked() {
            action = FoldersAction::BrowseFolder;
        }

        ui.add_space(20.0);

        if session.items.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(
                    RichText::new("Nessuna cartella caricata.")
                        .size(16.0)
                        .color(Color32::from_rgb(142, 171, 192)),
                );
                ui.add_space(12.0);
                if ui
                    .add(
                        egui::Button::new(RichText::new("Seleziona una Cartella...").size(14.0))
                            .fill(Color32::from_rgb(12, 165, 214))
                            .min_size(vec2(200.0, 40.0)),
                    )
                    .clicked()
                {
                    action = FoldersAction::BrowseFolder;
                }
            });
            return action;
        }

        // Table Header: Name | Photos | Size | Modified
        let header_h = 30.0;
        let (hdr_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), header_h), Sense::hover());
        ui.painter().text(
            Pos2::new(hdr_rect.left() + 20.0, hdr_rect.center().y),
            Align2::LEFT_CENTER,
            "Name",
            FontId::proportional(12.0),
            Color32::from_rgb(142, 171, 192),
        );
        ui.painter().text(
            Pos2::new(hdr_rect.left() + 320.0, hdr_rect.center().y),
            Align2::LEFT_CENTER,
            "Photos",
            FontId::proportional(12.0),
            Color32::from_rgb(142, 171, 192),
        );
        ui.painter().text(
            Pos2::new(hdr_rect.left() + 450.0, hdr_rect.center().y),
            Align2::LEFT_CENTER,
            "Size",
            FontId::proportional(12.0),
            Color32::from_rgb(142, 171, 192),
        );
        ui.painter().text(
            Pos2::new(hdr_rect.right() - 80.0, hdr_rect.center().y),
            Align2::RIGHT_CENTER,
            "Action",
            FontId::proportional(12.0),
            Color32::from_rgb(142, 171, 192),
        );

        // Group by Year-Month
        let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (idx, item) in session.items.iter().enumerate() {
            groups.entry(item.year_month_label()).or_default().push(idx);
        }

        ScrollArea::vertical().show(ui, |ui| {
            for (label, indices) in groups.iter().rev() {
                let row_h = 46.0;
                let (row_rect, row_resp) =
                    ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::click());
                let row_hover = row_resp.hovered();

                let bg = if row_hover {
                    Color32::from_rgb(26, 42, 56)
                } else {
                    Color32::from_rgb(18, 30, 42)
                };

                ui.painter().rect_filled(row_rect, CornerRadius::same(8), bg);

                // Icon + Name
                ui.painter().text(
                    Pos2::new(row_rect.left() + 16.0, row_rect.center().y),
                    Align2::LEFT_CENTER,
                    format!("📁  {}", label),
                    FontId::proportional(14.0),
                    Color32::WHITE,
                );

                // Photo count
                ui.painter().text(
                    Pos2::new(row_rect.left() + 320.0, row_rect.center().y),
                    Align2::LEFT_CENTER,
                    format!("{} photos", indices.len()),
                    FontId::proportional(13.0),
                    Color32::from_rgb(187, 209, 219),
                );

                // Total size
                let total_size: u64 = indices.iter().map(|&i| session.items[i].file_size).sum();
                ui.painter().text(
                    Pos2::new(row_rect.left() + 450.0, row_rect.center().y),
                    Align2::LEFT_CENTER,
                    crate::model::media::format_bytes(total_size),
                    FontId::proportional(13.0),
                    Color32::from_rgb(187, 209, 219),
                );

                // Clean/Open button
                let first_idx = *indices.first().unwrap_or(&0);
                let btn_pill = Rect::from_center_size(
                    Pos2::new(row_rect.right() - 60.0, row_rect.center().y),
                    vec2(75.0, 26.0),
                );
                ui.painter().rect_filled(
                    btn_pill,
                    CornerRadius::same(13),
                    Color32::from_rgb(12, 165, 214),
                );
                ui.painter().text(
                    btn_pill.center(),
                    Align2::CENTER_CENTER,
                    "Clean →",
                    FontId::proportional(12.0),
                    Color32::WHITE,
                );

                if row_resp.clicked() {
                    action = FoldersAction::SelectMonth(first_idx);
                }

                ui.add_space(4.0);
            }
        });

        action
    }
}
