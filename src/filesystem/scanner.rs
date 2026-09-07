use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::model::MediaEntry;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "gif", "bmp",
];

pub fn is_supported_image(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        SUPPORTED_EXTENSIONS.contains(&ext_lower.as_str())
    } else {
        false
    }
}

pub fn scan_directory(dir: &Path) -> Vec<MediaEntry> {
    scan_directories(&[dir.to_path_buf()])
}

pub fn scan_directories(dirs: &[PathBuf]) -> Vec<MediaEntry> {
    let mut entries = Vec::new();

    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && is_supported_image(path) {
                if let Ok(meta) = entry.metadata() {
                    let size = meta.len();
                    let modified = meta.modified().ok();
                    entries.push(MediaEntry::new(path.to_path_buf(), size, modified));
                }
            }
        }
    }

    // Sort by modified date descending (newest first), fallback to filename
    entries.sort_by(|a, b| {
        match (a.modified, b.modified) {
            (Some(ta), Some(tb)) => tb.cmp(&ta),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.file_name.cmp(&b.file_name),
        }
    });

    entries
}

pub fn discover_system_picture_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(user_dirs) = directories::UserDirs::new() {
        if let Some(pic_dir) = user_dirs.picture_dir() {
            dirs.push(pic_dir.to_path_buf());
        }

        let home = user_dirs.home_dir();
        // Check common Windows picture locations including OneDrive sync
        let candidates = [
            home.join("Pictures"),
            home.join("Immagini"),
            home.join("OneDrive").join("Pictures"),
            home.join("OneDrive").join("Immagini"),
            home.join("Downloads"),
            home.join("Download"),
        ];

        for cand in candidates {
            if cand.exists() && !dirs.contains(&cand) {
                dirs.push(cand);
            }
        }
    }

    dirs
}
