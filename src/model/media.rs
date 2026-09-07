use std::path::PathBuf;
use std::time::SystemTime;
use chrono::{DateTime, Datelike, Local};
use serde::{Deserialize, Serialize};
use super::decision::Decision;

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

    pub fn formatted_size(&self) -> String {
        format_bytes(self.file_size)
    }

    pub fn year_month_label(&self) -> String {
        if let Some(time) = self.modified {
            let dt: DateTime<Local> = time.into();
            let month_name = match dt.month() {
                1 => "Gennaio",
                2 => "Febbraio",
                3 => "Marzo",
                4 => "Aprile",
                5 => "Maggio",
                6 => "Giugno",
                7 => "Luglio",
                8 => "Agosto",
                9 => "Settembre",
                10 => "Ottobre",
                11 => "Novembre",
                12 => "Dicembre",
                _ => "Sconosciuto",
            };
            format!("{} {}", month_name, dt.year())
        } else {
            "Data sconosciuta".to_string()
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
