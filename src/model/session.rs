use super::decision::Decision;
use super::media::MediaEntry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Session {
    pub folder: Option<PathBuf>,
    pub items: Vec<MediaEntry>,
    pub current_index: usize,
    pub undo_stack: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub unreviewed: usize,
    pub kept: usize,
    pub trashed: usize,
    pub trash_bytes: u64,
    pub total_bytes: u64,
}

impl Session {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_items(&mut self, folder: PathBuf, items: Vec<MediaEntry>) {
        self.folder = Some(folder);
        self.items = items;
        self.current_index = 0;
        self.undo_stack.clear();
    }

    pub fn current_item(&self) -> Option<&MediaEntry> {
        self.items.get(self.current_index)
    }

    pub fn resume_items(
        &mut self,
        folder: PathBuf,
        mut items: Vec<MediaEntry>,
        progress: Option<&super::library::AlbumProgress>,
        prior_items: &[MediaEntry],
    ) {
        let decisions: std::collections::HashMap<_, _> = prior_items
            .iter()
            .map(|item| (&item.path, item.decision))
            .collect();
        for item in &mut items {
            if let Some(decision) = decisions.get(&item.path) {
                item.decision = *decision;
            }
        }
        let index = progress.map_or(0, |progress| {
            progress
                .current_photo
                .as_ref()
                .and_then(|photo| items.iter().position(|item| &item.path == photo))
                .unwrap_or(progress.current_index)
                .min(items.len().saturating_sub(1))
        });
        self.set_items(folder, items);
        self.current_index = index;
    }

    pub fn apply_decision(&mut self, decision: Decision) -> bool {
        if self.current_index < self.items.len() {
            self.items[self.current_index].decision = decision;
            self.undo_stack.push(self.current_index);
            if self.current_index + 1 < self.items.len() {
                self.current_index += 1;
            }
            true
        } else {
            false
        }
    }

    pub fn undo(&mut self) -> Option<usize> {
        if let Some(idx) = self.undo_stack.pop() {
            if let Some(item) = self.items.get_mut(idx) {
                item.decision = Decision::Unreviewed;
            }
            self.current_index = idx;
            Some(idx)
        } else {
            None
        }
    }

    pub fn next(&mut self) {
        if self.current_index + 1 < self.items.len() {
            self.current_index += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
        }
    }

    pub fn stats(&self) -> SessionStats {
        let mut stats = SessionStats {
            ..Default::default()
        };

        for item in &self.items {
            stats.total_bytes += item.file_size;
            match item.decision {
                Decision::Unreviewed => stats.unreviewed += 1,
                Decision::Keep => stats.kept += 1,
                Decision::Trash => {
                    stats.trashed += 1;
                    stats.trash_bytes += item.file_size;
                }
            }
        }
        stats
    }

    pub fn save_session(&self) {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "zWipeRoll", "zWipeRoll") {
            let config_dir = proj_dirs.config_dir();
            let file_path = config_dir.join("session.json");
            let _ = self.save_to(&file_path);
        }
    }

    pub fn load_session() -> Option<Self> {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "zWipeRoll", "zWipeRoll") {
            let file_path = proj_dirs.config_dir().join("session.json");
            return Self::load_from(&file_path);
        }
        None
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)
    }

    pub fn load_from(path: &Path) -> Option<Self> {
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_keeps_photo_and_decisions_after_reorder() {
        let mut session = Session::default();
        let mut prior = vec![MediaEntry::new("album/a.jpg".into(), 1, None)];
        prior[0].decision = Decision::Keep;
        let items = vec![
            MediaEntry::new("album/b.jpg".into(), 2, None),
            prior[0].clone(),
        ];
        let progress = super::super::library::AlbumProgress {
            current_index: 0,
            current_photo: Some("album/a.jpg".into()),
            viewed_count: 1,
        };
        session.resume_items("album".into(), items, Some(&progress), &prior);
        assert_eq!(
            session.current_item().unwrap().path,
            PathBuf::from("album/a.jpg")
        );
        assert_eq!(session.current_index, 1);
        assert_eq!(session.stats().kept, 1);
    }

    #[test]
    fn test_decision_and_undo() {
        let mut session = Session::new();
        let items = vec![
            MediaEntry::new(PathBuf::from("photo1.jpg"), 1024 * 1024, None),
            MediaEntry::new(PathBuf::from("photo2.jpg"), 2 * 1024 * 1024, None),
            MediaEntry::new(PathBuf::from("photo3.jpg"), 3 * 1024 * 1024, None),
        ];
        session.set_items(PathBuf::from("."), items);

        assert_eq!(session.current_index, 0);

        session.apply_decision(Decision::Keep);
        assert_eq!(session.current_index, 1);
        assert_eq!(session.items[0].decision, Decision::Keep);

        session.apply_decision(Decision::Trash);
        assert_eq!(session.current_index, 2);
        assert_eq!(session.items[1].decision, Decision::Trash);

        let stats = session.stats();
        assert_eq!(stats.kept, 1);
        assert_eq!(stats.trashed, 1);
        assert_eq!(stats.unreviewed, 1);
        assert_eq!(stats.trash_bytes, 2 * 1024 * 1024);

        let undone = session.undo();
        assert_eq!(undone, Some(1));
        assert_eq!(session.current_index, 1);
        assert_eq!(session.items[1].decision, Decision::Unreviewed);

        let stats_after_undo = session.stats();
        assert_eq!(stats_after_undo.trashed, 0);
        assert_eq!(stats_after_undo.unreviewed, 2);
    }

    #[test]
    fn missing_current_photo_uses_clamped_index_and_empty_album_is_safe() {
        let progress = super::super::library::AlbumProgress {
            viewed_count: 2,
            current_index: 9,
            current_photo: Some("deleted.jpg".into()),
        };
        let mut session = Session::default();
        session.resume_items(
            "album".into(),
            vec![MediaEntry::new("remaining.jpg".into(), 1, None)],
            Some(&progress),
            &[],
        );
        assert_eq!(
            session.current_item().unwrap().path,
            PathBuf::from("remaining.jpg")
        );
        assert_eq!(session.current_index, 0);
        session.resume_items("album".into(), vec![], Some(&progress), &[]);
        assert!(session.current_item().is_none());
        assert_eq!(session.current_index, 0);
    }
}
