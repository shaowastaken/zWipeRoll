use crate::model::albums::{month_albums, ordered_folders};
pub use crate::model::albums::{FolderSort, SortDirection};
use crate::model::{FolderRecord, LibraryCatalog, MediaEntry, Session};
use chrono::{DateTime, Local};
use egui::{
    vec2, Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, RichText, ScrollArea, Sense,
    Stroke, Ui,
};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryMode {
    Folders,
    Timeline,
}

pub enum FoldersAction {
    None,
    RescanGlobal,
    SelectFolder(PathBuf),
    SelectVirtualAlbum(String, Vec<MediaEntry>),
    StartReviewAll,
}
pub struct FoldersView;

impl FoldersView {
    pub fn ui(
        ui: &mut Ui,
        session: &mut Session,
        library: &LibraryCatalog,
        mode: &mut LibraryMode,
        sort: &mut FolderSort,
        direction: &mut SortDirection,
    ) -> FoldersAction {
        let indexed = &library.folders;
        let mut action = FoldersAction::None;
        let dark = ui.visuals().dark_mode;
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading(
                    RichText::new("Global Library")
                        .size(24.0)
                        .color(crate::theme::text(dark))
                        .strong(),
                );
                ui.label(
                    RichText::new("Folders containing supported images, indexed locally.")
                        .color(crate::theme::muted(dark))
                        .size(13.0),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Rescan").clicked() {
                    action = FoldersAction::RescanGlobal;
                }
                if (!session.items.is_empty() || !indexed.is_empty())
                    && ui
                        .add(
                            egui::Button::new(
                                RichText::new(if ui.available_width() < 760.0 {
                                    "Continue"
                                } else if indexed.len() > 1 {
                                    "Continue last album"
                                } else {
                                    "Continue album"
                                })
                                .color(Color32::WHITE)
                                .strong(),
                            )
                            .fill(Color32::from_rgb(10, 117, 246))
                            .min_size(vec2(132.0, 36.0)),
                        )
                        .clicked()
                {
                    action = FoldersAction::StartReviewAll;
                }
            });
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.selectable_value(mode, LibraryMode::Folders, "Real folders");
            ui.selectable_value(mode, LibraryMode::Timeline, "Month · year");
            ui.separator();
            if *mode == LibraryMode::Folders {
                egui::ComboBox::from_id_salt("folder_sort")
                    .selected_text(match sort {
                        FolderSort::Date => "Date",
                        FolderSort::Size => "Size",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(sort, FolderSort::Date, "Date");
                        ui.selectable_value(sort, FolderSort::Size, "Size");
                    });
            } else {
                ui.label(RichText::new("Chronological order").color(crate::theme::muted(dark)));
            }
            let direction_label = match direction {
                SortDirection::Ascending => "Ascending ↑",
                SortDirection::Descending => "Descending ↓",
            };
            egui::ComboBox::from_id_salt("sort_direction")
                .selected_text(direction_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(direction, SortDirection::Ascending, "Ascending ↑");
                    ui.selectable_value(direction, SortDirection::Descending, "Descending ↓");
                });
        });

        ui.add_space(18.0);
        if session.items.is_empty() && indexed.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(
                    RichText::new("No image folders found.")
                        .size(16.0)
                        .color(crate::theme::muted(dark)),
                );
            });
            return action;
        }

        let fallback;
        let folders = if indexed.is_empty() {
            fallback = session.folder.as_ref().map(|path| FolderRecord {
                path: path.clone(),
                image_count: session.items.len(),
                total_bytes: session.items.iter().map(|item| item.file_size).sum(),
                latest_modified: session.items.iter().filter_map(|item| item.modified).max(),
                viewed_count: session.stats().kept + session.stats().trashed,
                current_index: session.current_index,
                current_photo: session.current_item().map(|item| item.path.clone()),
                images: session.items.clone(),
            });
            fallback.as_slice()
        } else {
            indexed
        };

        if *mode == LibraryMode::Timeline {
            let months = month_albums(
                folders
                    .iter()
                    .filter(|folder| library.path_available(&folder.path))
                    .flat_map(|folder| folder.images.iter()),
                *direction,
            );
            Self::month_header(ui, dark);
            ScrollArea::vertical().show(ui, |ui| {
                for album in months {
                    let viewed = album.viewed_count(
                        library
                            .virtual_progress(&album.label)
                            .map_or(0, |progress| progress.viewed_count),
                    );
                    let images = album.images;
                    let total_bytes = images.iter().map(|image| image.file_size).sum();
                    let label = album.label;
                    let (row, response) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click());
                    ui.painter().rect_filled(
                        row,
                        CornerRadius::same(8),
                        if response.hovered() {
                            crate::theme::surface_raised(dark)
                        } else {
                            crate::theme::surface(dark)
                        },
                    );
                    ui.painter().text(
                        Pos2::new(row.left() + 16.0, row.center().y),
                        Align2::LEFT_CENTER,
                        format!("◫  {label}"),
                        FontId::proportional(14.0),
                        crate::theme::text(dark),
                    );
                    ui.painter().text(
                        Pos2::new(row.left() + row.width() * 0.66, row.center().y),
                        Align2::LEFT_CENTER,
                        format!("{viewed} / {} viewed", images.len()),
                        FontId::proportional(13.0),
                        crate::theme::muted(dark),
                    );
                    ui.painter().text(
                        Pos2::new(row.right() - 16.0, row.center().y),
                        Align2::RIGHT_CENTER,
                        crate::model::media::format_bytes(total_bytes),
                        FontId::proportional(12.0),
                        crate::theme::muted(dark),
                    );
                    if response.clicked() {
                        action = FoldersAction::SelectVirtualAlbum(label, images);
                    }
                    ui.add_space(4.0);
                }
            });
            return action;
        }

        let ordered = ordered_folders(folders, *sort, *direction);

        Self::header(ui, dark);
        ScrollArea::vertical().show(ui, |ui| {
            for folder in ordered {
                let available = library.path_available(&folder.path);
                let (row, response) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click());
                let response = response.on_hover_text(match &folder.current_photo {
                    Some(photo) => format!("Continue from {}", photo.display()),
                    None => format!("Start {}", folder.path.display()),
                });
                ui.painter().rect_filled(
                    row,
                    CornerRadius::same(8),
                    if response.hovered() {
                        crate::theme::surface_raised(dark)
                    } else {
                        crate::theme::surface(dark)
                    },
                );
                if library.last_folder.as_deref() == Some(folder.path.as_path()) {
                    ui.painter().rect_filled(
                        row,
                        CornerRadius::same(8),
                        crate::theme::selection(dark),
                    );
                    ui.painter().rect_stroke(
                        row,
                        CornerRadius::same(8),
                        Stroke::new(1.0, Color32::from_rgb(10, 117, 246)),
                        egui::StrokeKind::Inside,
                    );
                }
                ui.painter()
                    .with_clip_rect(egui::Rect::from_min_max(
                        row.min,
                        Pos2::new(row.left() + row.width() * 0.50, row.bottom()),
                    ))
                    .text(
                        Pos2::new(row.left() + 40.0, row.center().y),
                        Align2::LEFT_CENTER,
                        folder.path.display().to_string(),
                        FontId::proportional(13.0),
                        crate::theme::text(dark),
                    );
                let icon_rect = egui::Rect::from_center_size(
                    Pos2::new(row.left() + 22.0, row.center().y + 1.0),
                    vec2(16.0, 12.0),
                );
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(icon_rect.min - vec2(0.0, 3.0), vec2(8.0, 5.0)),
                    CornerRadius::same(2),
                    Color32::from_rgb(0, 164, 229),
                );
                ui.painter().rect_filled(
                    icon_rect,
                    CornerRadius::same(2),
                    Color32::from_rgb(63, 197, 245),
                );
                ui.painter().text(
                    Pos2::new(row.left() + row.width() * 0.52, row.center().y),
                    Align2::LEFT_CENTER,
                    format!("{} / {}", folder.viewed_count, folder.image_count),
                    FontId::proportional(13.0),
                    crate::theme::muted(dark),
                );
                ui.painter().text(
                    Pos2::new(row.left() + row.width() * 0.67, row.center().y),
                    Align2::LEFT_CENTER,
                    format!(
                        "Photo {}",
                        folder
                            .current_index
                            .saturating_add(1)
                            .min(folder.image_count)
                    ),
                    FontId::proportional(13.0),
                    crate::theme::muted(dark),
                );
                let modified = folder.latest_modified.map_or_else(
                    || "—".to_owned(),
                    |time| DateTime::<Local>::from(time).format("%d/%m/%Y").to_string(),
                );
                ui.painter().text(
                    Pos2::new(row.left() + row.width() * 0.79, row.center().y),
                    Align2::LEFT_CENTER,
                    crate::model::media::format_bytes(folder.total_bytes),
                    FontId::proportional(12.0),
                    crate::theme::muted(dark),
                );
                ui.painter().text(
                    Pos2::new(row.right() - 16.0, row.center().y),
                    Align2::RIGHT_CENTER,
                    modified,
                    FontId::proportional(12.0),
                    crate::theme::muted(dark),
                );
                if !available {
                    ui.painter().text(
                        Pos2::new(row.left() + row.width() * 0.57, row.center().y),
                        Align2::LEFT_CENTER,
                        library.folder_status(&folder.path).label(),
                        FontId::proportional(12.0),
                        crate::theme::muted(dark),
                    );
                }
                if response.clicked() && available {
                    action = FoldersAction::SelectFolder(folder.path.clone());
                }
                ui.add_space(4.0);
            }
        });
        action
    }

    fn header(ui: &mut Ui, dark: bool) {
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
        for (x, align, label) in [
            (row.left() + 20.0, Align2::LEFT_CENTER, "Folder"),
            (
                row.left() + row.width() * 0.52,
                Align2::LEFT_CENTER,
                "Viewed",
            ),
            (
                row.left() + row.width() * 0.67,
                Align2::LEFT_CENTER,
                "Current",
            ),
            (row.left() + row.width() * 0.79, Align2::LEFT_CENTER, "Size"),
            (row.right() - 16.0, Align2::RIGHT_CENTER, "Modified"),
        ] {
            ui.painter().text(
                Pos2::new(x, row.center().y),
                align,
                label,
                FontId::proportional(12.0),
                crate::theme::muted(dark),
            );
        }
    }

    fn month_header(ui: &mut Ui, dark: bool) {
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
        for (x, align, label) in [
            (row.left() + 20.0, Align2::LEFT_CENTER, "Virtual album"),
            (
                row.left() + row.width() * 0.66,
                Align2::LEFT_CENTER,
                "Progress",
            ),
            (row.right() - 16.0, Align2::RIGHT_CENTER, "Total size"),
        ] {
            ui.painter().text(
                Pos2::new(x, row.center().y),
                align,
                label,
                FontId::proportional(12.0),
                crate::theme::muted(dark),
            );
        }
    }
}
