use super::{FolderRecord, MediaEntry};
use chrono::{DateTime, Datelike, Local};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderSort {
    Date,
    Size,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

pub fn ordered_folders(
    folders: &[FolderRecord],
    sort: FolderSort,
    direction: SortDirection,
) -> Vec<&FolderRecord> {
    let mut ordered: Vec<_> = folders.iter().collect();
    ordered.sort_by(|a, b| {
        let order = match sort {
            FolderSort::Date => a.latest_modified.cmp(&b.latest_modified),
            FolderSort::Size => a.total_bytes.cmp(&b.total_bytes),
        }
        .then_with(|| a.path.cmp(&b.path));
        if direction == SortDirection::Descending {
            order.reverse()
        } else {
            order
        }
    });
    ordered
}

pub struct MonthAlbum {
    pub label: String,
    pub images: Vec<MediaEntry>,
}

impl MonthAlbum {
    pub fn viewed_count(&self, saved_count: usize) -> usize {
        let known_reviewed = self
            .images
            .iter()
            .filter(|image| image.decision != super::Decision::Unreviewed)
            .count();
        saved_count.max(known_reviewed).min(self.images.len())
    }
}

/// Undated images stay in real folders; no calendar date is invented for them.
pub fn month_albums<'a>(
    images: impl IntoIterator<Item = &'a MediaEntry>,
    direction: SortDirection,
) -> Vec<MonthAlbum> {
    let mut months: BTreeMap<(i32, u32), Vec<MediaEntry>> = BTreeMap::new();
    let mut seen_paths = HashSet::new();
    for image in images {
        // Recursive single-folder albums may overlap globally indexed child folders.
        // Keep the first catalog occurrence, including its persisted decision.
        if !seen_paths.insert(&image.path) {
            continue;
        }
        if let Some(modified) = image.modified {
            let date = DateTime::<Local>::from(modified);
            months
                .entry((date.year(), date.month()))
                .or_default()
                .push(image.clone());
        }
    }
    let mut months: Vec<_> = months.into_iter().collect();
    if direction == SortDirection::Descending {
        months.reverse();
    }
    months
        .into_iter()
        .map(|((year, month), mut images)| {
            images.sort_by(|a, b| {
                a.modified
                    .cmp(&b.modified)
                    .then_with(|| a.path.cmp(&b.path))
            });
            if direction == SortDirection::Descending {
                images.reverse();
            }
            let name = [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ][(month - 1) as usize];
            MonthAlbum {
                label: format!("{name} {year}"),
                images,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    #[test]
    fn month_progress_includes_real_folder_decisions_without_saved_month_progress() {
        let mut reviewed = image("kept.jpg", 2025, 1);
        reviewed.decision = crate::model::Decision::Keep;
        let pending = image("pending.jpg", 2025, 1);
        let album = month_albums([&reviewed, &pending], SortDirection::Ascending).remove(0);
        assert_eq!(album.viewed_count(0), 1);
        assert_eq!(album.viewed_count(2), 2);
        assert_eq!(album.viewed_count(99), 2);
    }
    fn image(path: &str, year: i32, month: u32) -> MediaEntry {
        MediaEntry::new(
            path.into(),
            1,
            Some(
                Local
                    .with_ymd_and_hms(year, month, 15, 12, 0, 0)
                    .unwrap()
                    .into(),
            ),
        )
    }
    #[test]
    fn month_photo_ties_are_deterministic_across_input_order() {
        let a = image("a.jpg", 2025, 1);
        let z = image("z.jpg", 2025, 1);
        let albums = month_albums([&z, &a], SortDirection::Ascending);
        assert_eq!(
            albums[0]
                .images
                .iter()
                .map(|x| x.file_name.as_str())
                .collect::<Vec<_>>(),
            ["a.jpg", "z.jpg"]
        );
        let descending = month_albums([&a, &z], SortDirection::Descending);
        assert_eq!(
            descending[0]
                .images
                .iter()
                .map(|x| x.file_name.as_str())
                .collect::<Vec<_>>(),
            ["z.jpg", "a.jpg"]
        );
    }
    #[test]
    fn overlapping_real_albums_contribute_each_photo_once() {
        let mut first = image("parent/child/shared.jpg", 2025, 1);
        first.decision = crate::model::Decision::Keep;
        let duplicate = image("parent/child/shared.jpg", 2025, 1);
        let other = image("parent/other.jpg", 2025, 1);
        let albums = month_albums([&first, &duplicate, &other], SortDirection::Ascending);
        assert_eq!(
            albums[0]
                .images
                .iter()
                .map(|photo| photo.path.to_str().unwrap())
                .collect::<Vec<_>>(),
            ["parent/child/shared.jpg", "parent/other.jpg"]
        );
        assert_eq!(albums[0].images[0].decision, crate::model::Decision::Keep);
    }
    #[test]
    fn months_cross_years_and_reverse_photos_without_inventing_undated_months() {
        let jan = image("jan.jpg", 2025, 1);
        let dec = image("dec.jpg", 2024, 12);
        let jan_late = MediaEntry::new(
            "late.jpg".into(),
            2,
            jan.modified
                .map(|t| t + std::time::Duration::from_secs(3600)),
        );
        let undated = MediaEntry::new("undated.jpg".into(), 3, None);
        for (direction, labels, january) in [
            (
                SortDirection::Ascending,
                ["December 2024", "January 2025"],
                ["jan.jpg", "late.jpg"],
            ),
            (
                SortDirection::Descending,
                ["January 2025", "December 2024"],
                ["late.jpg", "jan.jpg"],
            ),
        ] {
            let albums = month_albums([&jan_late, &undated, &dec, &jan], direction);
            assert_eq!(
                albums.iter().map(|a| a.label.as_str()).collect::<Vec<_>>(),
                labels
            );
            let january_album = albums.iter().find(|a| a.label == "January 2025").unwrap();
            assert_eq!(
                january_album
                    .images
                    .iter()
                    .map(|a| a.file_name.as_str())
                    .collect::<Vec<_>>(),
                january
            );
        }
        assert!(month_albums([&undated], SortDirection::Ascending).is_empty());
    }
    #[test]
    fn real_folders_sort_date_size_unknowns_and_ties_in_both_directions() {
        let folder = |path: &str, bytes, modified| FolderRecord {
            path: path.into(),
            image_count: 0,
            total_bytes: bytes,
            latest_modified: modified,
            viewed_count: 0,
            current_index: 0,
            current_photo: None,
            images: Vec::new(),
        };
        let old = image("old.jpg", 2024, 12).modified;
        let new = image("new.jpg", 2025, 1).modified;
        let folders = [
            folder("z", 2, old),
            folder("b", 1, new),
            folder("a", 2, old),
            folder("unknown", 0, None),
        ];
        for (sort, direction, expected) in [
            (
                FolderSort::Date,
                SortDirection::Ascending,
                ["unknown", "a", "z", "b"],
            ),
            (
                FolderSort::Date,
                SortDirection::Descending,
                ["b", "z", "a", "unknown"],
            ),
            (
                FolderSort::Size,
                SortDirection::Ascending,
                ["unknown", "b", "a", "z"],
            ),
            (
                FolderSort::Size,
                SortDirection::Descending,
                ["z", "a", "b", "unknown"],
            ),
        ] {
            assert_eq!(
                ordered_folders(&folders, sort, direction)
                    .iter()
                    .map(|a| a.path.to_str().unwrap())
                    .collect::<Vec<_>>(),
                expected
            );
        }
        assert!(ordered_folders(&[], FolderSort::Date, SortDirection::Ascending).is_empty());
    }
}
