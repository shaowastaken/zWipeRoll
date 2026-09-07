use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use super::decision::Decision;
use super::media::MediaEntry;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Session {
    pub folder: Option<PathBuf>,
    pub items: Vec<MediaEntry>,
    pub current_index: usize,
    pub undo_stack: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub total: usize,
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
            total: self.items.len(),
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
            let _ = std::fs::create_dir_all(config_dir);
            let file_path = config_dir.join("session.json");
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(file_path, json);
            }
        }
    }

    pub fn load_session() -> Option<Self> {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "zWipeRoll", "zWipeRoll") {
            let file_path = proj_dirs.config_dir().join("session.json");
            if file_path.exists() {
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    if let Ok(sess) = serde_json::from_str::<Self>(&content) {
                        return Some(sess);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
