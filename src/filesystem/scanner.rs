use crate::model::MediaEntry;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Mutex;
use std::time::SystemTime;
use walkdir::WalkDir;

const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp"];

#[derive(Debug, Clone)]
pub struct ImageFolder {
    pub path: PathBuf,
    pub image_count: usize,
    pub total_bytes: u64,
    pub latest_modified: Option<SystemTime>,
    pub images: Vec<MediaEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct ScanProgress {
    pub current_root: Option<PathBuf>,
    pub current_path: Option<PathBuf>,
    pub scanned_files: u64,
    pub images_found: usize,
    pub folders_found: usize,
    pub skipped_directories: u64,
    pub errors: usize,
    pub completed_roots: usize,
    pub total_roots: usize,
}

#[derive(Debug, Clone)]
pub struct ScanError {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct ScanReport {
    pub folders: Vec<ImageFolder>,
    pub errors: Vec<ScanError>,
    pub total_errors: usize,
    pub scanned_files: u64,
    pub skipped_directories: u64,
    pub completed_roots: Vec<PathBuf>,
    pub cancelled: bool,
}

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
    entries.sort_by(|a, b| match (a.modified, b.modified) {
        (Some(ta), Some(tb)) => tb.cmp(&ta),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.file_name.cmp(&b.file_name),
    });

    entries
}

#[cfg(test)]
pub fn index_image_folders_cancellable(
    dirs: &[PathBuf],
    cancelled: Option<&AtomicBool>,
) -> Vec<ImageFolder> {
    let default_cancel = AtomicBool::new(false);
    scan_image_folders(
        dirs,
        cancelled.unwrap_or(&default_cancel),
        &Mutex::default(),
    )
    .folders
}

pub fn scan_image_folders(
    dirs: &[PathBuf],
    cancelled: &AtomicBool,
    progress: &Mutex<ScanProgress>,
) -> ScanReport {
    let mut folders: BTreeMap<PathBuf, ImageFolder> = BTreeMap::new();
    let mut seen_files = HashSet::new();
    let mut report = ScanReport::default();
    let mut state = ScanProgress {
        total_roots: dirs.len(),
        ..Default::default()
    };

    for root in dirs {
        if cancelled.load(AtomicOrdering::Relaxed) {
            break;
        }
        state.current_root = Some(root.clone());
        let mut walker = WalkDir::new(root).follow_links(false).into_iter();
        while let Some(result) = walker.next() {
            if cancelled.load(AtomicOrdering::Relaxed) {
                break;
            }
            let entry = match result {
                Ok(entry) => entry,
                Err(error) => {
                    report.total_errors += 1;
                    if report.errors.len() < 256 {
                        report.errors.push(ScanError {
                            path: error.path().unwrap_or(root).to_path_buf(),
                            message: error.to_string(),
                        });
                    }
                    state.errors = report.total_errors;
                    *progress.lock().unwrap_or_else(|e| e.into_inner()) = state.clone();
                    continue;
                }
            };
            state.current_path = Some(entry.path().to_path_buf());
            if should_prune(&entry, root) {
                walker.skip_current_dir();
                state.skipped_directories += 1;
                continue;
            }
            let path = entry.path();
            if !entry.file_type().is_file() {
                continue;
            }
            state.scanned_files += 1;
            if state.scanned_files.is_multiple_of(128) {
                *progress.lock().unwrap_or_else(|e| e.into_inner()) = state.clone();
            }
            if !is_supported_image(path) {
                continue;
            }

            let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            if !seen_files.insert(canonical) {
                continue;
            }

            let Some(parent) = path.parent() else {
                continue;
            };
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(error) => {
                    report.total_errors += 1;
                    if report.errors.len() < 256 {
                        report.errors.push(ScanError {
                            path: path.to_path_buf(),
                            message: error.to_string(),
                        });
                    }
                    state.errors = report.total_errors;
                    continue;
                }
            };

            let modified = metadata.modified().ok();
            let folder = folders
                .entry(parent.to_path_buf())
                .or_insert_with(|| ImageFolder {
                    path: parent.to_path_buf(),
                    image_count: 0,
                    total_bytes: 0,
                    latest_modified: None,
                    images: Vec::new(),
                });
            folder.image_count += 1;
            folder.total_bytes += metadata.len();
            if modified > folder.latest_modified {
                folder.latest_modified = modified;
            }
            folder.images.push(MediaEntry::new(
                path.to_path_buf(),
                metadata.len(),
                modified,
            ));
            state.images_found += 1;
            state.folders_found = folders.len();
        }
        if cancelled.load(AtomicOrdering::Relaxed) {
            break;
        }
        report.completed_roots.push(root.clone());
        state.completed_roots += 1;
        *progress.lock().unwrap_or_else(|e| e.into_inner()) = state.clone();
    }

    let mut indexed: Vec<_> = folders.into_values().collect();
    indexed.sort_by(|a, b| {
        b.image_count
            .cmp(&a.image_count)
            .then_with(|| a.path.cmp(&b.path))
    });
    report.folders = indexed;
    report.scanned_files = state.scanned_files;
    report.skipped_directories = state.skipped_directories;
    report.cancelled = cancelled.load(AtomicOrdering::Relaxed);
    *progress.lock().unwrap_or_else(|e| e.into_inner()) = state;
    report
}

fn should_prune(entry: &walkdir::DirEntry, root: &Path) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
    let at_volume_root = root.parent().is_none() && entry.depth() == 1;
    let system_folder = at_volume_root
        && matches!(
            name.as_str(),
            "windows"
                | "program files"
                | "program files (x86)"
                | "programdata"
                | "$recycle.bin"
                | "system volume information"
                | "recovery"
                | "windowsapps"
        );
    let user_appdata = name == "appdata"
        && entry
            .path()
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .is_some_and(|name| name.eq_ignore_ascii_case("Users"));
    system_folder || user_appdata
}

pub fn discover_global_scan_roots() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(windows)]
    for letter in b'A'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if root.exists() {
            dirs.push(root);
        }
    }

    #[cfg(not(windows))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_only_folders_that_contain_images() {
        let root = std::env::temp_dir().join(format!("zwiperoll-index-{}", std::process::id()));
        let nested = root.join("album");
        let excluded = root
            .join("Users")
            .join("test-user")
            .join("AppData")
            .join("cache");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(&excluded).unwrap();
        std::fs::write(root.join("cover.jpg"), [1_u8, 2, 3]).unwrap();
        std::fs::write(nested.join("photo.png"), [1_u8, 2, 3, 4]).unwrap();
        std::fs::write(nested.join("notes.txt"), b"not an image").unwrap();
        std::fs::write(excluded.join("software-icon.png"), [1_u8, 2]).unwrap();

        let folders = index_image_folders_cancellable(std::slice::from_ref(&root), None);
        assert_eq!(folders.len(), 2);
        assert!(folders
            .iter()
            .any(|folder| folder.path == root && folder.image_count == 1));
        assert!(folders
            .iter()
            .any(|folder| folder.path == nested && folder.image_count == 1));
        assert_eq!(
            folders
                .iter()
                .map(|folder| folder.images.len())
                .sum::<usize>(),
            2
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn scans_selected_roots_without_hiding_albums_named_like_software() {
        let root = std::env::temp_dir().join(format!("zwiperoll-selected-{}", std::process::id()));
        let selected = root.join("selected");
        let other = root.join("other");
        let album = selected.join("target");
        std::fs::create_dir_all(&album).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(album.join("photo.JPG"), [1_u8, 2, 3]).unwrap();
        std::fs::write(other.join("not-selected.png"), [1_u8]).unwrap();
        let progress = Mutex::default();
        let report = scan_image_folders(
            &[selected.clone(), album],
            &AtomicBool::new(false),
            &progress,
        );
        assert_eq!(report.folders.len(), 1);
        assert_eq!(report.folders[0].image_count, 1);
        assert_eq!(report.folders[0].total_bytes, 3);
        assert_eq!(progress.lock().unwrap().images_found, 1);
        assert_eq!(report.total_errors, 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reports_missing_roots_and_honors_cancellation() {
        let missing =
            std::env::temp_dir().join(format!("zwiperoll-missing-{}", std::process::id()));
        let report = scan_image_folders(
            std::slice::from_ref(&missing),
            &AtomicBool::new(false),
            &Mutex::default(),
        );
        assert_eq!(report.total_errors, 1);
        assert_eq!(report.errors[0].path, missing);
        let cancelled = scan_image_folders(&[missing], &AtomicBool::new(true), &Mutex::default());
        assert!(cancelled.cancelled);
        assert!(cancelled.completed_roots.is_empty());
        assert!(cancelled.folders.is_empty());
    }
}
