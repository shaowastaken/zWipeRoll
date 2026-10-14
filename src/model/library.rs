use crate::filesystem::volumes::{volume_for_path, VolumeMount};
use crate::filesystem::ImageFolder;
use crate::model::MediaEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Decision, Session};

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("zwiperoll-resume-{}-{id}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn indexed(path: &str, names: &[&str]) -> ImageFolder {
        ImageFolder {
            path: path.into(),
            image_count: names.len(),
            total_bytes: names.len() as u64,
            latest_modified: None,
            images: names
                .iter()
                .map(|name| MediaEntry::new(Path::new(path).join(name), 1, None))
                .collect(),
        }
    }

    #[test]
    fn restarted_real_album_restores_exact_photo_and_cumulative_unique_decisions() {
        let scratch = Scratch::new();
        let library_file = scratch.0.join("library.json");
        let session_file = scratch.0.join("session.json");
        let mut catalog = LibraryCatalog::load_from(&library_file);
        catalog.upsert(indexed("album", &["a.jpg", "b.jpg", "c.jpg"]));
        let mut session = Session::default();
        session.set_items("album".into(), catalog.folders[0].images.clone());
        session.apply_decision(Decision::Keep);
        catalog.record_session_progress(&session);
        session.save_to(&session_file).unwrap();
        assert_eq!(
            Session::load_from(&session_file)
                .unwrap()
                .current_item()
                .unwrap()
                .path,
            Path::new("album/b.jpg")
        );

        let mut restored = LibraryCatalog::load_from(&library_file);
        restored.resync(vec![indexed(
            "album",
            &["c.jpg", "b.jpg", "a.jpg", "new.jpg"],
        )]);
        let progress = restored.folder_progress(Path::new("album")).unwrap();
        session.resume_items(
            "album".into(),
            restored.folders[0].images.clone(),
            Some(&progress),
            &restored.folders[0].images,
        );
        assert_eq!(
            session.current_item().unwrap().path,
            Path::new("album/b.jpg")
        );
        assert_eq!(session.items[2].decision, Decision::Keep);
        session.apply_decision(Decision::Trash);
        restored.record_session_progress(&session);
        assert_eq!(
            LibraryCatalog::load_from(&library_file).folders[0].viewed_count,
            2
        );
        session.undo();
        restored.record_session_progress(&session);
        assert_eq!(
            LibraryCatalog::load_from(&library_file).folders[0].viewed_count,
            1
        );
        // Re-reviewing the same file changes its decision, not the unique reviewed count.
        session.apply_decision(Decision::Keep);
        restored.record_session_progress(&session);
        assert_eq!(
            LibraryCatalog::load_from(&library_file).folders[0].viewed_count,
            2
        );
    }
    #[test]
    fn restarted_virtual_album_retains_own_cursor_and_updates_real_decisions() {
        let scratch = Scratch::new();
        let file = scratch.0.join("library.json");
        let mut catalog = LibraryCatalog::load_from(&file);
        catalog.upsert(indexed("first", &["a.jpg", "b.jpg"]));
        catalog.upsert(indexed("second", &["c.jpg"]));
        let first_items = catalog.record(Path::new("first")).unwrap().images.clone();
        let mut real = Session::default();
        real.set_items("first".into(), first_items.clone());
        real.current_index = 1;
        catalog.record_session_progress(&real);
        let mut virtual_session = Session::default();
        virtual_session.set_items(
            "Virtual album · January 2025".into(),
            vec![
                first_items[0].clone(),
                catalog.record(Path::new("second")).unwrap().images[0].clone(),
                first_items[1].clone(),
            ],
        );
        virtual_session.apply_decision(Decision::Keep);
        virtual_session.apply_decision(Decision::Trash);
        catalog.record_session_progress(&virtual_session);
        virtual_session.undo();
        catalog.record_session_progress(&virtual_session);
        assert_eq!(
            catalog
                .virtual_progress("January 2025")
                .unwrap()
                .viewed_count,
            1
        );
        virtual_session.apply_decision(Decision::Trash);
        catalog.record_session_progress(&virtual_session);
        let mut restored = LibraryCatalog::load_from(&file);
        assert_eq!(restored.last_virtual_album.as_deref(), Some("January 2025"));
        assert_eq!(restored.last_folder.as_deref(), Some(Path::new("first")));
        assert_eq!(restored.record(Path::new("first")).unwrap().viewed_count, 1);
        assert_eq!(
            restored.record(Path::new("second")).unwrap().images[0].decision,
            Decision::Trash
        );
        restored.resync(vec![
            indexed("first", &["b.jpg", "a.jpg"]),
            indexed("second", &["c.jpg"]),
        ]);
        let progress = restored.virtual_progress("January 2025").unwrap();
        let items: Vec<_> = restored
            .folders
            .iter()
            .flat_map(|folder| folder.images.clone())
            .collect();
        virtual_session.resume_items(
            "Virtual album · January 2025".into(),
            items.clone(),
            Some(progress),
            &items,
        );
        assert_eq!(
            virtual_session.current_item().unwrap().path,
            Path::new("first/b.jpg")
        );
        assert_eq!(virtual_session.current_index, 0);
        assert_eq!(virtual_session.stats().kept, 1);
        assert_eq!(virtual_session.stats().trashed, 1);
        restored.record_session_progress(&real);
        assert!(LibraryCatalog::load_from(&file)
            .last_virtual_album
            .is_none());
    }

    #[test]
    fn reopened_album_upsert_persists_new_photo_decision_without_losing_prior_reviews() {
        let scratch = Scratch::new();
        let file = scratch.0.join("library.json");
        let mut catalog = LibraryCatalog::load_from(&file);
        catalog.upsert(indexed("album", &["old.jpg"]));
        let mut session = Session::default();
        session.set_items("album".into(), catalog.folders[0].images.clone());
        session.apply_decision(Decision::Keep);
        catalog.record_session_progress(&session);
        catalog.upsert(indexed("album", &["old.jpg", "new.jpg"]));
        let progress = catalog.folder_progress(Path::new("album"));
        session.resume_items(
            "album".into(),
            catalog.folders[0].images.clone(),
            progress.as_ref(),
            &catalog.folders[0].images,
        );
        session.next();
        session.apply_decision(Decision::Trash);
        catalog.record_session_progress(&session);
        let restored = LibraryCatalog::load_from(&file);
        assert_eq!(restored.folders[0].images[0].decision, Decision::Keep);
        assert_eq!(
            restored.folders[0].images[1].path,
            Path::new("album/new.jpg")
        );
        assert_eq!(restored.folders[0].images[1].decision, Decision::Trash);
        assert_eq!(restored.folders[0].viewed_count, 2);
    }

    #[test]
    fn resync_and_upsert_remove_deleted_reviews_without_inventing_legacy_progress() {
        for use_resync in [true, false] {
            let scratch = Scratch::new();
            let file = scratch.0.join("library.json");
            let mut catalog = LibraryCatalog::load_from(&file);
            catalog.upsert(indexed("album", &["deleted.jpg", "remaining.jpg"]));
            let mut session = Session::default();
            session.set_items("album".into(), catalog.folders[0].images.clone());
            session.apply_decision(Decision::Keep);
            catalog.record_session_progress(&session);
            let changed = indexed("album", &["remaining.jpg", "new.jpg"]);
            if use_resync {
                catalog.resync(vec![changed]);
            } else {
                catalog.upsert(changed);
            }
            assert_eq!(catalog.folders[0].viewed_count, 0);
            session.resume_items(
                "album".into(),
                catalog.folders[0].images.clone(),
                catalog.folder_progress(Path::new("album")).as_ref(),
                &catalog.folders[0].images,
            );
            session.apply_decision(Decision::Keep);
            catalog.record_session_progress(&session);
            assert_eq!(LibraryCatalog::load_from(&file).folders[0].viewed_count, 1);
        }
    }

    #[test]
    fn virtual_refresh_drops_deleted_reviews_and_counts_new_decisions_once() {
        use chrono::TimeZone;
        let scratch = Scratch::new();
        let file = scratch.0.join("library.json");
        let date = chrono::Local
            .with_ymd_and_hms(2025, 1, 15, 12, 0, 0)
            .unwrap()
            .into();
        let dated = |names: &[&str]| {
            let mut folder = indexed("album", names);
            for image in &mut folder.images {
                image.modified = Some(date);
            }
            folder
        };
        let mut catalog = LibraryCatalog::load_from(&file);
        catalog.upsert(dated(&["deleted.jpg", "kept.jpg", "remaining.jpg"]));
        let mut session = Session::default();
        session.set_items(
            "Virtual album · January 2025".into(),
            catalog.folders[0].images.clone(),
        );
        session.apply_decision(Decision::Keep);
        session.apply_decision(Decision::Keep);
        catalog.record_session_progress(&session);
        catalog.resync(vec![dated(&["kept.jpg", "remaining.jpg", "new.jpg"])]);
        assert_eq!(
            catalog
                .virtual_progress("January 2025")
                .unwrap()
                .viewed_count,
            1
        );
        session.resume_items(
            "Virtual album · January 2025".into(),
            catalog.folders[0].images.clone(),
            catalog.virtual_progress("January 2025"),
            &catalog.folders[0].images,
        );
        session.apply_decision(Decision::Keep);
        catalog.record_session_progress(&session);
        assert_eq!(
            LibraryCatalog::load_from(&file)
                .virtual_progress("January 2025")
                .unwrap()
                .viewed_count,
            2
        );
    }

    #[test]
    fn recursive_parent_inherits_reviewed_child_decisions_on_open_and_resync() {
        for use_resync in [false, true] {
            let scratch = Scratch::new();
            let file = scratch.0.join("library.json");
            let mut catalog = LibraryCatalog::load_from(&file);
            catalog.upsert(indexed("parent/child", &["photo.jpg"]));
            let mut child = Session::default();
            child.set_items("parent/child".into(), catalog.folders[0].images.clone());
            child.apply_decision(Decision::Keep);
            catalog.record_session_progress(&child);
            // Old catalogs may contain an earlier unreviewed recursive overlap.
            let mut stale_overlap = catalog.folders[0].clone();
            stale_overlap.path = "legacy-parent".into();
            stale_overlap.images[0].decision = Decision::Unreviewed;
            stale_overlap.viewed_count = 0;
            catalog.folders.insert(0, stale_overlap);
            let photo = MediaEntry::new("parent/child/photo.jpg".into(), 1, None);
            let parent = ImageFolder {
                path: "parent".into(),
                image_count: 1,
                total_bytes: 1,
                latest_modified: None,
                images: vec![photo],
            };
            if use_resync {
                catalog.resync(vec![indexed("parent/child", &["photo.jpg"]), parent]);
            } else {
                catalog.upsert(parent);
            }
            let record = catalog.record(Path::new("parent")).unwrap();
            assert_eq!(record.images[0].decision, Decision::Keep);
            assert_eq!(record.viewed_count, 1);
            let mut session = Session::default();
            session.resume_items(
                "parent".into(),
                record.images.clone(),
                catalog.folder_progress(Path::new("parent")).as_ref(),
                &record.images,
            );
            catalog.record_session_progress(&session);
            let restored = LibraryCatalog::load_from(&file);
            assert_eq!(
                restored.record(Path::new("parent/child")).unwrap().images[0].decision,
                Decision::Keep
            );
            assert_eq!(
                restored
                    .record(Path::new("parent/child"))
                    .unwrap()
                    .viewed_count,
                1
            );
        }
    }

    #[test]
    fn real_album_reviews_and_undo_update_month_count_without_moving_month_cursor() {
        use chrono::TimeZone;
        let scratch = Scratch::new();
        let file = scratch.0.join("library.json");
        let mut folder = indexed("album", &["a.jpg", "b.jpg"]);
        for image in &mut folder.images {
            image.modified = Some(
                chrono::Local
                    .with_ymd_and_hms(2025, 1, 15, 12, 0, 0)
                    .unwrap()
                    .into(),
            );
        }
        let mut catalog = LibraryCatalog::load_from(&file);
        catalog.upsert(folder);
        catalog.record_virtual_progress("January 2025", 1, Some("album/b.jpg".into()), 0);
        let mut session = Session::default();
        session.set_items("album".into(), catalog.folders[0].images.clone());
        session.apply_decision(Decision::Keep);
        catalog.record_session_progress(&session);
        assert_eq!(
            catalog
                .virtual_progress("January 2025")
                .unwrap()
                .viewed_count,
            1
        );
        assert_eq!(
            catalog
                .virtual_progress("January 2025")
                .unwrap()
                .current_photo
                .as_deref(),
            Some(Path::new("album/b.jpg"))
        );
        session.undo();
        catalog.record_session_progress(&session);
        assert_eq!(
            LibraryCatalog::load_from(&file)
                .virtual_progress("January 2025")
                .unwrap()
                .viewed_count,
            0
        );
    }

    #[test]
    fn scoped_resync_preserves_unselected_and_partially_unreadable_folders() {
        let catalog = LibraryCatalog {
            folders: vec![
                FolderRecord {
                    path: "selected/album".into(),
                    image_count: 5,
                    total_bytes: 5,
                    latest_modified: None,
                    viewed_count: 3,
                    current_index: 2,
                    current_photo: None,
                    images: Vec::new(),
                },
                FolderRecord {
                    path: "offline/album".into(),
                    image_count: 7,
                    total_bytes: 7,
                    latest_modified: None,
                    viewed_count: 0,
                    current_index: 0,
                    current_photo: None,
                    images: Vec::new(),
                },
            ],
            ..Default::default()
        };
        let partial = ImageFolder {
            path: "selected/album".into(),
            image_count: 2,
            total_bytes: 2,
            latest_modified: None,
            images: Vec::new(),
        };
        let merged = catalog.merge_scoped(
            vec![partial.clone()],
            &["selected".into()],
            &["selected/album/unreadable.jpg".into()],
        );
        assert_eq!(
            merged
                .iter()
                .find(|f| f.path == Path::new("selected/album"))
                .unwrap()
                .image_count,
            5
        );
        assert!(merged.iter().any(|f| f.path == Path::new("offline/album")));
        let clean = catalog.merge_scoped(vec![partial], &["selected".into()], &[]);
        assert_eq!(
            clean
                .iter()
                .find(|f| f.path == Path::new("selected/album"))
                .unwrap()
                .image_count,
            2
        );
        let deleted = catalog.merge_scoped(vec![], &["selected".into()], &[]);
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].path, Path::new("offline/album"));
    }

    #[test]
    fn volume_disconnect_and_reconnect_preserve_precise_progress() {
        let mut catalog = LibraryCatalog::default();
        catalog.reconcile_volumes(&[mount("one", "R:\\")]);
        catalog.scan_roots.push("R:\\photos".into());
        catalog
            .folders
            .push(record("R:\\photos", "R:\\photos\\a.jpg"));
        catalog.last_folder = Some("R:\\photos".into());
        catalog.virtual_progress.insert(
            "2026-10".into(),
            AlbumProgress {
                current_photo: Some("R:\\photos\\a.jpg".into()),
                ..Default::default()
            },
        );
        catalog.reconcile_volumes(&[]);
        assert_eq!(
            catalog.folder_status(Path::new("R:\\photos")),
            FolderAvailability::Offline
        );
        catalog.reconcile_volumes(&[mount("one", "S:\\")]);
        assert_eq!(catalog.folders[0].path, Path::new("S:\\photos"));
        assert_eq!(
            catalog.folders[0].current_photo.as_deref(),
            Some(Path::new("S:\\photos\\a.jpg"))
        );
        assert_eq!(
            catalog.folders[0].images[0].path,
            Path::new("S:\\photos\\a.jpg")
        );
        assert_eq!(catalog.folders[0].viewed_count, 1);
        assert_eq!(
            catalog.last_folder.as_deref(),
            Some(Path::new("S:\\photos"))
        );
        assert_eq!(catalog.scan_roots, vec![PathBuf::from("S:\\photos")]);
        assert_eq!(
            catalog.virtual_progress["2026-10"].current_photo.as_deref(),
            Some(Path::new("S:\\photos\\a.jpg"))
        );
    }

    fn mount(id: &str, root: &str) -> VolumeMount {
        VolumeMount {
            id: id.into(),
            root: root.into(),
        }
    }

    #[test]
    fn swapped_volume_letters_remap_once_and_reused_letter_cannot_replace_album() {
        let mut catalog = LibraryCatalog::default();
        catalog.reconcile_volumes(&[mount("one", "R:\\"), mount("two", "S:\\")]);
        catalog.folders = vec![
            record("R:\\photos", "R:\\photos\\one.jpg"),
            record("S:\\photos", "S:\\photos\\two.jpg"),
        ];
        catalog.reconcile_volumes(&[mount("one", "S:\\"), mount("two", "R:\\")]);
        assert_eq!(
            catalog.folders[0].images[0].path,
            Path::new("S:\\photos\\one.jpg")
        );
        assert_eq!(
            catalog.folders[1].images[0].path,
            Path::new("R:\\photos\\two.jpg")
        );
        catalog.reconcile_volumes(&[mount("imposter", "S:\\"), mount("two", "R:\\")]);
        assert_eq!(
            catalog.folder_status(Path::new("S:\\photos")),
            FolderAvailability::LetterReused
        );
        assert!(!catalog.scan_allowed(Path::new("S:\\")));
        let merged = catalog.merge_scoped(
            vec![ImageFolder {
                path: "S:\\photos".into(),
                image_count: 9,
                total_bytes: 99,
                latest_modified: None,
                images: vec![MediaEntry::new("S:\\photos\\wrong.jpg".into(), 99, None)],
            }],
            &["S:\\".into()],
            &[],
        );
        assert_eq!(
            merged
                .iter()
                .find(|folder| folder.path == Path::new("S:\\photos"))
                .unwrap()
                .images[0]
                .path,
            Path::new("S:\\photos\\one.jpg")
        );
        assert_eq!(catalog.folders[0].viewed_count, 1);
    }

    fn record(path: &str, photo: &str) -> FolderRecord {
        FolderRecord {
            path: path.into(),
            image_count: 2,
            total_bytes: 42,
            latest_modified: None,
            viewed_count: 1,
            current_index: 1,
            current_photo: Some(photo.into()),
            images: vec![MediaEntry::new(photo.into(), 42, None)],
        }
    }

    #[test]
    fn old_catalog_schema_and_volume_bindings_roundtrip_without_data_loss() {
        let json = r#"{"folders":[{"path":"R:\\photos","image_count":2,"total_bytes":42,"latest_modified":null,"viewed_count":1,"current_index":1,"current_photo":"R:\\photos\\a.jpg","images":[]}],"last_folder":"R:\\photos"}"#;
        let mut catalog: LibraryCatalog = serde_json::from_str(json).unwrap();
        assert_eq!(
            catalog.folders[0].current_photo.as_deref(),
            Some(Path::new("R:\\photos\\a.jpg"))
        );
        assert!(catalog.volume_bindings.is_empty());
        catalog.reconcile_volumes(&[mount("stable-id", "R:\\")]);
        let saved = serde_json::to_string(&catalog).unwrap();
        let mut restored: LibraryCatalog = serde_json::from_str(&saved).unwrap();
        restored.reconcile_volumes(&[mount("stable-id", "T:\\")]);
        assert_eq!(
            restored.folders[0].current_photo.as_deref(),
            Some(Path::new("T:\\photos\\a.jpg"))
        );
        assert_eq!(restored.folders[0].current_index, 1);
        assert_eq!(restored.folders[0].viewed_count, 1);
        assert_eq!(
            restored.last_folder.as_deref(),
            Some(Path::new("T:\\photos"))
        );
    }

    #[test]
    fn offline_roots_reserve_destinations_and_shadow_outer_mount_moves() {
        let mut catalog = LibraryCatalog::default();
        catalog.reconcile_volumes(&[mount("one", "R:\\"), mount("two", "S:\\")]);
        catalog.folders = vec![
            record("R:\\photos", "R:\\photos\\one.jpg"),
            record("S:\\photos", "S:\\photos\\two.jpg"),
        ];
        catalog.reconcile_volumes(&[mount("two", "R:\\")]);
        assert_eq!(catalog.folders[0].path, Path::new("R:\\photos"));
        assert_eq!(catalog.folders[1].path, Path::new("S:\\photos"));
        assert!(!catalog.path_available(Path::new("R:\\photos")));
        assert!(!catalog.path_available(Path::new("S:\\photos")));
        assert_eq!(catalog.volume_bindings[0].id, "one");
        assert_eq!(catalog.volume_bindings[1].root, Path::new("S:\\"));

        let mut nested = LibraryCatalog::default();
        nested.reconcile_volumes(&[mount("outer", "R:\\"), mount("inner", "R:\\mounted\\")]);
        nested.folders = vec![
            record("R:\\normal", "R:\\normal\\a.jpg"),
            record("R:\\mounted\\photos", "R:\\mounted\\photos\\b.jpg"),
        ];
        let remaps = nested.reconcile_volumes(&[mount("outer", "S:\\")]);
        assert_eq!(nested.folders[0].path, Path::new("S:\\normal"));
        assert_eq!(nested.folders[1].path, Path::new("R:\\mounted\\photos"));
        assert_eq!(
            remap_path(Path::new("R:\\mounted\\photos\\b.jpg"), &remaps),
            Path::new("R:\\mounted\\photos\\b.jpg")
        );
        assert!(!nested.scan_allowed(Path::new("R:\\")));
    }

    #[test]
    fn blocked_destination_propagates_back_through_mount_chain() {
        let mut catalog = LibraryCatalog::default();
        catalog.reconcile_volumes(&[
            mount("one", "R:\\"),
            mount("two", "S:\\"),
            mount("three", "T:\\"),
        ]);
        catalog.folders = vec![
            record("R:\\photos", "R:\\photos\\one.jpg"),
            record("S:\\photos", "S:\\photos\\two.jpg"),
            record("T:\\photos", "T:\\photos\\three.jpg"),
        ];
        catalog.reconcile_volumes(&[mount("one", "S:\\"), mount("two", "T:\\")]);
        assert_eq!(catalog.folders[0].path, Path::new("R:\\photos"));
        assert_eq!(catalog.folders[1].path, Path::new("S:\\photos"));
        assert_eq!(catalog.folders[2].path, Path::new("T:\\photos"));
        assert_eq!(
            catalog
                .volume_bindings
                .iter()
                .map(|binding| binding.root.clone())
                .collect::<Vec<_>>(),
            vec![
                PathBuf::from("R:\\"),
                PathBuf::from("S:\\"),
                PathBuf::from("T:\\")
            ]
        );
        for folder in &catalog.folders {
            assert!(!catalog.path_available(&folder.path));
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderRecord {
    pub path: PathBuf,
    pub image_count: usize,
    pub total_bytes: u64,
    pub latest_modified: Option<SystemTime>,
    #[serde(default)]
    pub viewed_count: usize,
    #[serde(default)]
    pub current_index: usize,
    #[serde(default)]
    pub current_photo: Option<PathBuf>,
    #[serde(default)]
    pub images: Vec<MediaEntry>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LibraryCatalog {
    pub folders: Vec<FolderRecord>,
    pub last_folder: Option<PathBuf>,
    #[serde(default)]
    pub last_virtual_album: Option<String>,
    #[serde(default)]
    pub virtual_progress: HashMap<String, AlbumProgress>,
    #[serde(default)]
    pub scan_roots: Vec<PathBuf>,
    #[serde(default)]
    pub volume_bindings: Vec<VolumeMount>,
    #[serde(skip)]
    mounted_volumes: Vec<VolumeMount>,
    #[serde(skip)]
    storage_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderAvailability {
    Available,
    Offline,
    LetterReused,
    Unavailable,
}

impl FolderAvailability {
    pub fn label(self) -> &'static str {
        match self {
            Self::Available => "Available",
            Self::Offline => "Offline",
            Self::LetterReused => "Offline — drive letter belongs to another volume",
            Self::Unavailable => "Unavailable",
        }
    }
}

/// Apply the most-specific original mapping once, so swapped letters do not cascade.
pub fn remap_path(path: &Path, mappings: &[(PathBuf, PathBuf)]) -> PathBuf {
    let mounts: Vec<_> = mappings
        .iter()
        .enumerate()
        .map(|(index, (root, _))| VolumeMount {
            id: index.to_string(),
            root: root.clone(),
        })
        .collect();
    let Some(mount) = volume_for_path(path, &mounts) else {
        return path.to_path_buf();
    };
    let index: usize = mount.id.parse().expect("mapping index");
    // Windows path components are case-insensitive, unlike strip_prefix.
    let relative: PathBuf = path
        .components()
        .skip(mount.root.components().count())
        .collect();
    mappings[index].1.join(relative)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlbumProgress {
    pub viewed_count: usize,
    pub current_index: usize,
    pub current_photo: Option<PathBuf>,
}

impl LibraryCatalog {
    pub fn reconcile_volumes(&mut self, mounts: &[VolumeMount]) -> Vec<(PathBuf, PathBuf)> {
        let mut mappings: Vec<_> = self
            .volume_bindings
            .iter()
            .map(|old| {
                // Prefer the old mount if a volume has more than one mounted path.
                let new = mounts
                    .iter()
                    .find(|new| new.id == old.id && new.root == old.root)
                    .or_else(|| mounts.iter().find(|new| new.id == old.id));
                let target = new
                    .filter(|new| {
                        !self.volume_bindings.iter().any(|other| {
                            other.root == new.root
                                && other.id != old.id
                                && !mounts.iter().any(|mounted| mounted.id == other.id)
                        })
                    })
                    .map_or_else(|| old.root.clone(), |new| new.root.clone());
                (old.root.clone(), target)
            })
            .collect();
        // A blocked move reserves its original root too. Propagate that reservation
        // backwards until stable; cycles where every occupant moves (swaps) stay valid.
        loop {
            let blocked: Vec<_> = mappings
                .iter()
                .enumerate()
                .filter_map(|(index, (old, target))| {
                    (old != target
                        && mappings.iter().enumerate().any(
                            |(other_index, (other_old, other_target))| {
                                other_index != index
                                    && other_old == target
                                    && other_old == other_target
                            },
                        ))
                    .then_some(index)
                })
                .collect();
            if blocked.is_empty() {
                break;
            }
            for index in blocked {
                mappings[index].1 = mappings[index].0.clone();
            }
        }
        for folder in &mut self.folders {
            folder.path = remap_path(&folder.path, &mappings);
            if let Some(photo) = &mut folder.current_photo {
                *photo = remap_path(photo, &mappings);
            }
            for image in &mut folder.images {
                image.path = remap_path(&image.path, &mappings);
            }
        }
        if let Some(last) = &mut self.last_folder {
            *last = remap_path(last, &mappings);
        }
        for progress in self.virtual_progress.values_mut() {
            if let Some(photo) = &mut progress.current_photo {
                *photo = remap_path(photo, &mappings);
            }
        }
        for root in &mut self.scan_roots {
            *root = remap_path(root, &mappings);
        }
        for (binding, (_, target)) in self.volume_bindings.iter_mut().zip(&mappings) {
            binding.root = target.clone();
        }
        for mount in mounts {
            // An offline binding reserves its old root: never reinterpret old photos as a different disk.
            if !self
                .volume_bindings
                .iter()
                .any(|old| old.root == mount.root)
            {
                self.volume_bindings.push(mount.clone());
            }
        }
        self.mounted_volumes = mounts.to_vec();
        // Keep unchanged roots in a moving snapshot to protect more-specific offline mounts.
        if mappings.iter().any(|(old, new)| old != new) {
            mappings
        } else {
            Vec::new()
        }
    }

    pub fn folder_status(&self, path: &Path) -> FolderAvailability {
        if let Some(binding) = volume_for_path(path, &self.volume_bindings) {
            let mounted = volume_for_path(path, &self.mounted_volumes);
            if mounted.is_none() {
                return FolderAvailability::Offline;
            }
            if mounted.is_some_and(|mounted| mounted.id != binding.id) {
                return FolderAvailability::LetterReused;
            }
        }
        if path.exists() {
            FolderAvailability::Available
        } else {
            FolderAvailability::Unavailable
        }
    }

    pub fn path_available(&self, path: &Path) -> bool {
        self.folder_status(path) == FolderAvailability::Available
    }

    pub fn scan_allowed(&self, path: &Path) -> bool {
        // Reject ancestors too: scanning an outer mount must not consume a reused inner mount.
        self.volume_bindings.iter().all(|binding| {
            let intersects = volume_for_path(path, std::slice::from_ref(binding)).is_some()
                || volume_for_path(
                    &binding.root,
                    &[VolumeMount {
                        id: String::new(),
                        root: path.into(),
                    }],
                )
                .is_some();
            !intersects
                || !matches!(
                    self.folder_status(&binding.root),
                    FolderAvailability::Offline | FolderAvailability::LetterReused
                )
        })
    }
    pub fn resync_scoped(
        &mut self,
        indexed: Vec<ImageFolder>,
        completed_roots: &[PathBuf],
        error_paths: &[PathBuf],
    ) {
        let combined = self.merge_scoped(indexed, completed_roots, error_paths);
        self.resync(combined);
    }

    fn merge_scoped(
        &self,
        indexed: Vec<ImageFolder>,
        completed_roots: &[PathBuf],
        error_paths: &[PathBuf],
    ) -> Vec<ImageFolder> {
        let mut combined: Vec<_> = indexed
            .into_iter()
            .filter(|folder| self.scan_allowed(&folder.path))
            .collect();
        for old in &self.folders {
            let covered = completed_roots
                .iter()
                .any(|root| old.path.starts_with(root));
            let inaccessible = error_paths
                .iter()
                .any(|path| old.path.starts_with(path) || path.starts_with(&old.path));
            if !covered || inaccessible || !self.scan_allowed(&old.path) {
                // A partial read cannot establish which old entries were deleted.
                combined.retain(|folder| folder.path != old.path);
                combined.push(ImageFolder {
                    path: old.path.clone(),
                    image_count: old.image_count,
                    total_bytes: old.total_bytes,
                    latest_modified: old.latest_modified,
                    images: old.images.clone(),
                });
            }
        }
        combined
    }
    pub fn load() -> Self {
        let Some(project) = directories::ProjectDirs::from("com", "zWipeRoll", "zWipeRoll") else {
            return Self::default();
        };
        Self::load_from(&project.config_dir().join("library.json"))
    }

    pub fn load_from(path: &Path) -> Self {
        let mut catalog: Self = std::fs::read(path)
            .ok()
            .and_then(|json| serde_json::from_slice(&json).ok())
            .unwrap_or_default();
        catalog.storage_path = Some(path.to_owned());
        catalog
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)
    }

    pub fn save(&self) {
        if let Some(path) = &self.storage_path {
            let _ = self.save_to(path);
            return;
        }
        let Some(project) = directories::ProjectDirs::from("com", "zWipeRoll", "zWipeRoll") else {
            return;
        };
        let _ = self.save_to(&project.config_dir().join("library.json"));
    }

    pub fn resync(&mut self, indexed: Vec<ImageFolder>) {
        let previous_virtual = self.month_review_counts();
        let prior_images: Vec<_> = self
            .folders
            .iter()
            .flat_map(|record| record.images.iter().cloned())
            .collect();
        let previous: HashMap<PathBuf, FolderRecord> = self
            .folders
            .drain(..)
            .map(|record| (record.path.clone(), record))
            .collect();

        self.folders = indexed
            .into_iter()
            .map(|folder| {
                let old = previous.get(&folder.path);
                let images = Self::preserve_decisions(folder.images, &prior_images);
                let viewed_count = old.map_or_else(
                    || Self::reviewed_count(&images),
                    |record| Self::reviewed_after_refresh(record, &images),
                );
                FolderRecord {
                    path: folder.path,
                    image_count: folder.image_count,
                    total_bytes: folder.total_bytes,
                    latest_modified: folder.latest_modified,
                    viewed_count,
                    current_index: old.map_or(0, |record| {
                        record
                            .current_index
                            .min(folder.image_count.saturating_sub(1))
                    }),
                    current_photo: old.and_then(|record| record.current_photo.clone()),
                    images,
                }
            })
            .collect();
        for record in &mut self.folders {
            record.current_index = record
                .current_photo
                .as_ref()
                .and_then(|photo| record.images.iter().position(|item| &item.path == photo))
                .unwrap_or(record.current_index)
                .min(record.image_count.saturating_sub(1));
        }
        if self
            .last_folder
            .as_ref()
            .is_some_and(|last| !self.folders.iter().any(|f| &f.path == last))
        {
            self.last_folder = None;
        }
        self.refresh_virtual_counts(&previous_virtual);
        self.save();
    }

    pub fn record_progress(
        &mut self,
        folder: &Path,
        current_index: usize,
        current_photo: Option<PathBuf>,
        viewed_count: usize,
    ) {
        if let Some(record) = self.folders.iter_mut().find(|record| record.path == folder) {
            let next_index = current_index.min(record.image_count.saturating_sub(1));
            let next_viewed = record
                .viewed_count
                .max(viewed_count)
                .min(record.image_count);
            let changed = record.current_index != next_index
                || record.current_photo != current_photo
                || record.viewed_count != next_viewed
                || self.last_folder.as_deref() != Some(folder)
                || self.last_virtual_album.is_some();
            record.current_index = next_index;
            record.current_photo = current_photo;
            record.viewed_count = next_viewed;
            self.last_folder = Some(record.path.clone());
            self.last_virtual_album = None;
            if changed {
                self.save();
            }
        }
    }

    pub fn upsert(&mut self, folder: ImageFolder) {
        if !self.scan_allowed(&folder.path) {
            return;
        }
        let previous_virtual = self.month_review_counts();
        let prior_images: Vec<_> = self
            .folders
            .iter()
            .flat_map(|record| record.images.iter().cloned())
            .collect();
        let images = Self::preserve_decisions(folder.images, &prior_images);
        if let Some(record) = self
            .folders
            .iter_mut()
            .find(|record| record.path == folder.path)
        {
            record.viewed_count = Self::reviewed_after_refresh(record, &images);
            record.image_count = folder.image_count;
            record.total_bytes = folder.total_bytes;
            record.latest_modified = folder.latest_modified;
            record.images = images;
            record.current_index = record
                .current_photo
                .as_ref()
                .and_then(|photo| record.images.iter().position(|item| &item.path == photo))
                .unwrap_or(record.current_index)
                .min(record.image_count.saturating_sub(1));
            record.viewed_count = record.viewed_count.min(record.image_count);
        } else {
            self.folders.push(FolderRecord {
                path: folder.path,
                image_count: folder.image_count,
                total_bytes: folder.total_bytes,
                latest_modified: folder.latest_modified,
                viewed_count: Self::reviewed_count(&images),
                current_index: 0,
                current_photo: None,
                images,
            });
            self.folders.sort_by(|a, b| {
                b.image_count
                    .cmp(&a.image_count)
                    .then_with(|| a.path.cmp(&b.path))
            });
        }
        self.refresh_virtual_counts(&previous_virtual);
        self.save();
    }

    pub fn record(&self, path: &Path) -> Option<&FolderRecord> {
        self.folders.iter().find(|record| record.path == path)
    }

    pub fn record_virtual_progress(
        &mut self,
        label: &str,
        current_index: usize,
        current_photo: Option<PathBuf>,
        viewed_count: usize,
    ) {
        let before = self.virtual_progress.get(label).cloned();
        let last_changed = self.last_virtual_album.as_deref() != Some(label);
        let progress = self.virtual_progress.entry(label.to_owned()).or_default();
        progress.current_index = current_index;
        progress.current_photo = current_photo;
        progress.viewed_count = viewed_count;
        let changed = before.as_ref() != Some(progress) || last_changed;
        self.last_virtual_album = Some(label.to_owned());
        if changed {
            self.save();
        }
    }

    pub fn virtual_progress(&self, label: &str) -> Option<&AlbumProgress> {
        self.virtual_progress.get(label)
    }

    pub fn folder_progress(&self, path: &Path) -> Option<AlbumProgress> {
        self.record(path).map(|record| AlbumProgress {
            viewed_count: record.viewed_count,
            current_index: record.current_index,
            current_photo: record.current_photo.clone(),
        })
    }

    fn preserve_decisions(mut items: Vec<MediaEntry>, old: &[MediaEntry]) -> Vec<MediaEntry> {
        let mut decisions = HashMap::new();
        for item in old {
            let decision = decisions.entry(&item.path).or_insert(item.decision);
            // Keep the first reviewed occurrence, but never let a stale unreviewed
            // overlap erase an existing review in another album.
            if *decision == super::decision::Decision::Unreviewed {
                *decision = item.decision;
            }
        }
        for item in &mut items {
            if let Some(decision) = decisions.get(&item.path) {
                item.decision = *decision;
            }
        }
        items
    }

    fn reviewed_after_refresh(old: &FolderRecord, items: &[MediaEntry]) -> usize {
        (old.viewed_count
            .saturating_sub(Self::reviewed_count(&old.images))
            + Self::reviewed_count(items))
        .min(items.len())
    }

    fn reviewed_count(items: &[MediaEntry]) -> usize {
        items
            .iter()
            .filter(|item| item.decision != super::decision::Decision::Unreviewed)
            .count()
    }

    fn month_review_counts(&self) -> HashMap<String, (usize, usize)> {
        super::albums::month_albums(
            self.folders.iter().flat_map(|folder| &folder.images),
            super::albums::SortDirection::Ascending,
        )
        .into_iter()
        .map(|album| {
            let reviewed = album
                .images
                .iter()
                .filter(|item| item.decision != super::decision::Decision::Unreviewed)
                .count();
            (album.label, (reviewed, album.images.len()))
        })
        .collect()
    }

    fn refresh_virtual_counts(&mut self, previous: &HashMap<String, (usize, usize)>) {
        let current = self.month_review_counts();
        for (label, progress) in &mut self.virtual_progress {
            let old_reviewed = previous.get(label).map_or(0, |counts| counts.0);
            let (reviewed, total) = current.get(label).copied().unwrap_or_default();
            progress.viewed_count =
                (progress.viewed_count.saturating_sub(old_reviewed) + reviewed).min(total);
        }
    }

    pub fn record_session_progress(&mut self, session: &super::session::Session) {
        let Some(folder) = &session.folder else {
            return;
        };
        let previous_virtual = self.month_review_counts();
        let decisions: HashMap<_, _> = session
            .items
            .iter()
            .map(|item| (&item.path, item.decision))
            .collect();
        let virtual_label = folder
            .to_string_lossy()
            .strip_prefix("Virtual album · ")
            .map(str::to_owned);
        let legacy_virtual = virtual_label
            .as_deref()
            .and_then(|label| self.virtual_progress(label))
            .map_or(0, |progress| {
                let reviewed_before: std::collections::HashSet<_> = self
                    .folders
                    .iter()
                    .flat_map(|record| &record.images)
                    .filter(|item| {
                        decisions.contains_key(&item.path)
                            && item.decision != super::decision::Decision::Unreviewed
                    })
                    .map(|item| &item.path)
                    .collect();
                progress.viewed_count.saturating_sub(reviewed_before.len())
            });
        let mut changed = false;
        for record in &mut self.folders {
            let reviewed_before = record
                .images
                .iter()
                .filter(|item| item.decision != super::decision::Decision::Unreviewed)
                .count();
            let legacy_viewed = record.viewed_count.saturating_sub(reviewed_before);
            for item in &mut record.images {
                if let Some(decision) = decisions.get(&item.path) {
                    changed |= item.decision != *decision;
                    item.decision = *decision;
                }
            }
            let viewed = record
                .images
                .iter()
                .filter(|item| item.decision != super::decision::Decision::Unreviewed)
                .count();
            let viewed = (legacy_viewed + viewed).min(record.image_count);
            changed |= record.viewed_count != viewed;
            record.viewed_count = viewed;
        }
        if changed {
            self.refresh_virtual_counts(&previous_virtual);
            self.save();
        }
        let photo = session.current_item().map(|item| item.path.clone());
        let stats = session.stats();
        if let Some(label) = virtual_label {
            self.record_virtual_progress(
                &label,
                session.current_index,
                photo,
                (legacy_virtual + stats.kept + stats.trashed).min(session.items.len()),
            );
        } else {
            let viewed = self
                .record(folder)
                .map_or(stats.kept + stats.trashed, |record| record.viewed_count);
            self.record_progress(folder, session.current_index, photo, viewed);
        }
    }
}
