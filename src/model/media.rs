use super::decision::Decision;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub file_size: u64,
    pub modified: Option<SystemTime>,
    pub decision: Decision,
}

impl MediaEntry {
    pub fn new(path: PathBuf, file_size: u64, modified: Option<SystemTime>) -> Self {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string());

        Self {
            path,
            file_name,
            file_size,
            modified,
            decision: Decision::Unreviewed,
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}
