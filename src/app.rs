use crate::filesystem::volumes::{discover_volume_mounts, VolumeMount};
use crate::filesystem::{
    discover_global_scan_roots, scan_directory, scan_image_folders, ImageFolder, ScanProgress,
    ScanReport,
};
use crate::imaging::TextureCache;
use crate::model::library::remap_path;
use crate::model::{LibraryCatalog, Session};
use crate::theme::ThemeMode;
use crate::views::{
    FolderSort, FoldersAction, FoldersView, GlobalScanAction, GlobalScanView, HomeView,
    LibraryMode, NavTab, SettingsView, SidebarView, SingleFolderView, SortDirection, SwipeAction,
    SwipeView, TrashGridView,
};
use egui::{FontData, FontDefinitions, FontFamily, TextureHandle, TextureOptions, Ui};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver},
    Arc, Mutex,
};

fn load_trimmed_texture(
    ctx: &egui::Context,
    name: &str,
    bytes: &[u8],
    dark_wordmark: bool,
) -> Option<TextureHandle> {
    let mut rgba = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = rgba.dimensions();
    if dark_wordmark {
        for (x, _, pixel) in rgba.enumerate_pixels_mut() {
            if x > width * 34 / 100
                && pixel[3] > 8
                && pixel[0] < 75
                && pixel[1] < 95
                && pixel[2] < 125
            {
                pixel[0] = 245;
                pixel[1] = 248;
                pixel[2] = 252;
            }
        }
    }
    let mut bounds = (width, height, 0_u32, 0_u32);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        if pixel[3] > 8 {
            bounds.0 = bounds.0.min(x);
            bounds.1 = bounds.1.min(y);
            bounds.2 = bounds.2.max(x);
            bounds.3 = bounds.3.max(y);
        }
    }
    if bounds.2 < bounds.0 || bounds.3 < bounds.1 {
        return None;
    }
    let cropped = image::imageops::crop_imm(
        &rgba,
        bounds.0,
        bounds.1,
        bounds.2 - bounds.0 + 1,
        bounds.3 - bounds.1 + 1,
    )
    .to_image();
    let size = [cropped.width() as usize, cropped.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, cropped.as_raw());
    Some(ctx.load_texture(name, color, TextureOptions::LINEAR))
}

pub struct PcSpongeApp {
    session: Session,
    cache: TextureCache,
    current_tab: NavTab,
    swipe_view: SwipeView,
    trash_view: TrashGridView,
    wordmark_texture: Option<TextureHandle>,
    wordmark_dark_texture: Option<TextureHandle>,
    cleaned_bytes_total: u64,
    theme: ThemeMode,
    first_frame: bool,
    library: LibraryCatalog,
    library_mode: LibraryMode,
    folder_sort: FolderSort,
    sort_direction: SortDirection,
    scan_receiver: Option<Receiver<ScanReport>>,
    scan_cancel: Option<Arc<AtomicBool>>,
    scan_progress: Arc<Mutex<ScanProgress>>,
    scan_roots: Vec<(PathBuf, bool)>,
    scan_report: Option<String>,
    scan_mounts: Vec<VolumeMount>,
    volume_snapshot: Vec<VolumeMount>,
    last_volume_refresh: std::time::Instant,
}

impl PcSpongeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Load custom brand fonts: Manrope and Inter
        let mut fonts = FontDefinitions::default();
        let inter_bytes = include_bytes!("../assets/Inter.ttf");
        let manrope_bytes = include_bytes!("../assets/Manrope.ttf");

        fonts.font_data.insert(
            "Inter".to_owned(),
            FontData::from_static(inter_bytes).into(),
        );
        fonts.font_data.insert(
            "Manrope".to_owned(),
            FontData::from_static(manrope_bytes).into(),
        );

        if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
            prop.insert(0, "Inter".to_owned());
            prop.insert(1, "Manrope".to_owned());
        }

        cc.egui_ctx.set_fonts(fonts);

        let theme: ThemeMode = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, "zwiperoll_theme"))
            .unwrap_or_default();
        theme.apply(&cc.egui_ctx);

        let wordmark_texture = load_trimmed_texture(
            &cc.egui_ctx,
            "brand_wordmark",
            include_bytes!("../assets/logo.png"),
            false,
        );
        let wordmark_dark_texture = load_trimmed_texture(
            &cc.egui_ctx,
            "brand_wordmark_dark",
            include_bytes!("../assets/logo.png"),
            true,
        );

        // Pass Context to TextureCache so background decoders trigger immediate request_repaint()
        let cache = TextureCache::new(16, Some(cc.egui_ctx.clone()));

        // Restore previous session if available
        let mut session = Session::load_session().unwrap_or_default();
        let mut library = LibraryCatalog::load();
        let mounts = discover_volume_mounts();
        let remaps = library.reconcile_volumes(&mounts);
        remap_session(&mut session, &remaps);
        library.save();
        let mut scan_roots: Vec<_> = discover_global_scan_roots()
            .into_iter()
            .map(|root| {
                let selected = library.scan_roots.is_empty() || library.scan_roots.contains(&root);
                (root, selected)
            })
            .collect();
        for root in &library.scan_roots {
            if !scan_roots.iter().any(|(path, _)| path == root) {
                scan_roots.push((root.clone(), true));
            }
        }

        Self {
            session,
            cache,
            current_tab: NavTab::Home,
            swipe_view: SwipeView::new(),
            trash_view: TrashGridView::new(),
            wordmark_texture,
            wordmark_dark_texture,
            cleaned_bytes_total: 0,
            theme,
            first_frame: true,
            library,
            library_mode: LibraryMode::Folders,
            folder_sort: FolderSort::Date,
            sort_direction: SortDirection::Descending,
            scan_receiver: None,
            scan_cancel: None,
            scan_progress: Arc::new(Mutex::new(ScanProgress::default())),
            scan_roots,
            scan_report: None,
            scan_mounts: Vec::new(),
            volume_snapshot: mounts,
            last_volume_refresh: std::time::Instant::now(),
        }
    }

    fn refresh_volumes(&mut self) -> Vec<(PathBuf, PathBuf)> {
        let mounts = discover_volume_mounts();
        let previous = self.library.volume_bindings.clone();
        let remaps = self.library.reconcile_volumes(&mounts);
        self.volume_snapshot = mounts;
        remap_session(&mut self.session, &remaps);
        for (root, _) in &mut self.scan_roots {
            *root = remap_path(root, &remaps);
        }
        if previous != self.library.volume_bindings || !remaps.is_empty() {
            self.library.save();
            self.session.save_session();
            self.cache.clear();
        }
        self.last_volume_refresh = std::time::Instant::now();
        remaps
    }

    fn save_active_progress(&mut self) {
        self.library.record_session_progress(&self.session);
    }

    fn open_folder_dialog(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.refresh_volumes();
            if !self.library.path_available(&folder) || !self.library.scan_allowed(&folder) {
                self.scan_report = Some("Location unavailable or belongs to an offline volume. Reconnect the original volume and refresh drives.".into());
                self.current_tab = NavTab::Scan;
                return;
            }
            self.save_active_progress();
            let items = scan_directory(&folder);
            if discover_volume_mounts() != self.volume_snapshot {
                self.scan_report = Some("Volumes changed during scanning. Previous index retained; refresh drives and retry.".into());
                self.current_tab = NavTab::Scan;
                return;
            }
            self.library.upsert(ImageFolder {
                path: folder.clone(),
                image_count: items.len(),
                total_bytes: items.iter().map(|item| item.file_size).sum(),
                latest_modified: items.iter().filter_map(|item| item.modified).max(),
                images: items.clone(),
            });
            let progress = self.library.folder_progress(&folder);
            let prior = self
                .library
                .record(&folder)
                .map_or(&[][..], |record| record.images.as_slice());
            self.session
                .resume_items(folder, items, progress.as_ref(), prior);
            self.save_active_progress();
            self.cache.clear();
            self.session.save_session();
            self.current_tab = NavTab::Review;
        }
    }

    fn open_indexed_folder(&mut self, path: PathBuf) {
        let remaps = self.refresh_volumes();
        let path = remap_path(&path, &remaps);
        if !self.library.path_available(&path) || !self.library.scan_allowed(&path) {
            self.current_tab = NavTab::Folders;
            return;
        }
        self.save_active_progress();
        let items = scan_directory(&path);
        if discover_volume_mounts() != self.volume_snapshot {
            self.scan_report = Some("Volumes changed while opening the album. Previous progress retained; refresh drives and retry.".into());
            self.current_tab = NavTab::Scan;
            return;
        }
        self.library.upsert(ImageFolder {
            path: path.clone(),
            image_count: items.len(),
            total_bytes: items.iter().map(|item| item.file_size).sum(),
            latest_modified: items.iter().filter_map(|item| item.modified).max(),
            images: items.clone(),
        });
        let progress = self.library.folder_progress(&path);
        let prior = self
            .library
            .record(&path)
            .map_or(&[][..], |record| record.images.as_slice());
        self.session
            .resume_items(path, items, progress.as_ref(), prior);
        self.save_active_progress();
        self.cache.clear();
        self.session.save_session();
        self.current_tab = NavTab::Review;
    }

    fn open_virtual_album(&mut self, label: String, items: Vec<crate::model::MediaEntry>) {
        self.save_active_progress();
        let prior: Vec<_> = self
            .library
            .folders
            .iter()
            .flat_map(|folder| folder.images.iter().cloned())
            .collect();
        self.session.resume_items(
            PathBuf::from(format!("Virtual album · {label}")),
            items,
            self.library.virtual_progress(&label),
            &prior,
        );
        self.save_active_progress();
        self.cache.clear();
        self.session.save_session();
        self.current_tab = NavTab::Review;
    }

    fn start_global_scan(&mut self) {
        if self.scan_receiver.is_some() {
            return;
        }
        self.refresh_volumes();
        let roots: Vec<_> = self
            .scan_roots
            .iter()
            .filter(|(_, selected)| *selected)
            .map(|(root, _)| root.clone())
            .collect();
        if roots.is_empty() {
            return;
        }
        if roots.iter().any(|root| !self.library.scan_allowed(root)) {
            self.scan_report = Some("Scan not started: a selected location is offline, unavailable, or conflicts with a saved offline volume. Reconnect the original volume or deselect that location.".into());
            return;
        }
        self.scan_mounts = self.volume_snapshot.clone();
        self.save_active_progress();
        self.library.scan_roots = roots.clone();
        self.library.save();
        self.scan_report = None;
        self.scan_progress = Arc::new(Mutex::new(ScanProgress {
            total_roots: roots.len(),
            ..Default::default()
        }));
        let progress = Arc::clone(&self.scan_progress);
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancelled);
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let report = scan_image_folders(&roots, &worker_cancel, &progress);
            let _ = sender.send(report);
        });
        self.scan_receiver = Some(receiver);
        self.scan_cancel = Some(cancelled);
        self.current_tab = NavTab::Scan;
    }

    fn poll_global_scan(&mut self) {
        let result = match self
            .scan_receiver
            .as_ref()
            .map(|receiver| receiver.try_recv())
        {
            Some(Ok(report)) => Some(report),
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.scan_receiver = None;
                self.scan_cancel = None;
                self.scan_report = Some(
                    "Scan worker stopped unexpectedly. Previous index retained; you can retry."
                        .into(),
                );
                None
            }
            _ => None,
        };
        if let Some(report) = result {
            self.scan_receiver = None;
            self.scan_cancel = None;
            if discover_volume_mounts() != self.scan_mounts {
                self.scan_report = Some("Volumes changed during scanning. Previous index retained; refresh drives and retry.".into());
                self.refresh_volumes();
                return;
            }
            let error_paths: Vec<_> = report
                .errors
                .iter()
                .map(|error| error.path.clone())
                .collect();
            let images: usize = report.folders.iter().map(|folder| folder.image_count).sum();
            let mut summary = format!("{}\n{} completed roots · {} checked files · {} images · {} folders · {} excluded directories\n", if report.cancelled { "Scan cancelled — previous index retained." } else { "Scan finished." }, report.completed_roots.len(), report.scanned_files, images, report.folders.len(), report.skipped_directories);
            for root in &report.completed_roots {
                summary.push_str(&format!("Completed: {}\n", root.display()));
            }
            for error in &report.errors {
                summary.push_str(&format!(
                    "Could not read {}: {}\n",
                    error.path.display(),
                    error.message
                ));
            }
            summary.push_str(&format!(
                "{} read errors (showing up to 256).\n",
                report.total_errors
            ));
            if !report.cancelled && report.total_errors <= report.errors.len() {
                self.library
                    .resync_scoped(report.folders, &report.completed_roots, &error_paths);
            }
            if report.total_errors > report.errors.len() {
                summary.push_str(
                    "Index retained: too many read errors for a safe resynchronization.\n",
                );
            }
            self.scan_report = Some(summary);
        }
    }
}

impl eframe::App for PcSpongeApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.theme.apply(&ctx);
        ui.painter().rect_filled(
            ui.max_rect(),
            egui::CornerRadius::ZERO,
            crate::theme::background(self.theme.is_dark()),
        );
        if self.first_frame {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
            self.first_frame = false;
        }
        self.cache.poll_results(&ctx);
        self.poll_global_scan();
        if self.scan_receiver.is_none() && self.last_volume_refresh.elapsed().as_secs() >= 5 {
            self.refresh_volumes();
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(5));
        if self.scan_receiver.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }

        crate::window_chrome::resize_edges(&ctx);

        let drag_rect = egui::Rect::from_min_max(
            egui::pos2(
                ctx.content_rect().left() + 72.0,
                ctx.content_rect().top() + 7.0,
            ),
            egui::pos2(
                ctx.content_rect().right() - 14.0,
                ctx.content_rect().top() + 26.0,
            ),
        );
        crate::window_chrome::title_bar(ui, drag_rect, egui::Id::new("main_title_drag"));

        let wordmark_tex_id = if self.theme.is_dark() {
            self.wordmark_dark_texture.as_ref()
        } else {
            self.wordmark_texture.as_ref()
        }
        .map(|texture| texture.id());

        // Master 2-Column Layout: Fixed Sidebar (210px) + Main Content Area
        ui.horizontal_top(|ui| {
            // Left Fixed Sidebar
            let sidebar_action = SidebarView::ui(
                ui,
                self.current_tab,
                self.cleaned_bytes_total,
                wordmark_tex_id,
                self.theme,
            );
            if let Some(tab) = sidebar_action.tab {
                self.current_tab = tab;
            }
            // Right Main Content Panel
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() - 14.0).max(1.0));
                ui.set_min_height(ui.available_height());
                // Keep content controls below the native title-drag strip.
                ui.add_space(26.0);
                match self.current_tab {
                    NavTab::Home => {
                        if HomeView::ui(ui, wordmark_tex_id) {
                            self.current_tab = NavTab::Scan;
                        }
                    }
                    NavTab::Scan => {
                        let progress = self
                            .scan_progress
                            .lock()
                            .map(|p| {
                                format!(
                                    "{} / {} roots · {} files checked · {} images · {} errors\n{}",
                                    p.completed_roots,
                                    p.total_roots,
                                    p.scanned_files,
                                    p.images_found,
                                    p.errors,
                                    p.current_path
                                        .as_ref()
                                        .map_or(String::new(), |path| path.display().to_string())
                                )
                            })
                            .unwrap_or_default();
                        match GlobalScanView::ui(
                            ui,
                            &mut self.scan_roots,
                            self.scan_receiver.is_some(),
                            &progress,
                            self.scan_report.as_deref(),
                        ) {
                            GlobalScanAction::None => {}
                            GlobalScanAction::Start => self.start_global_scan(),
                            GlobalScanAction::Cancel => {
                                if let Some(cancel) = &self.scan_cancel {
                                    cancel.store(true, Ordering::Relaxed);
                                }
                            }
                            GlobalScanAction::OpenLibrary => self.current_tab = NavTab::Folders,
                            GlobalScanAction::Refresh => {
                                self.refresh_volumes();
                                let existing = self.scan_roots.clone();
                                let discovered = discover_global_scan_roots();
                                for root in discovered {
                                    if !existing.iter().any(|(path, _)| path == &root) {
                                        self.scan_roots.push((root, true));
                                    }
                                }
                            }
                            GlobalScanAction::AddRoot => {
                                if let Some(root) = rfd::FileDialog::new().pick_folder() {
                                    if !self.scan_roots.iter().any(|(path, _)| path == &root) {
                                        self.scan_roots.push((root, true));
                                    }
                                }
                            }
                        }
                    }
                    NavTab::Folders => {
                        match FoldersView::ui(
                            ui,
                            &mut self.session,
                            &self.library,
                            &mut self.library_mode,
                            &mut self.folder_sort,
                            &mut self.sort_direction,
                        ) {
                            FoldersAction::None => {}
                            FoldersAction::RescanGlobal => self.current_tab = NavTab::Scan,
                            FoldersAction::SelectFolder(path) => {
                                self.open_indexed_folder(path);
                            }
                            FoldersAction::SelectVirtualAlbum(label, items) => {
                                self.open_virtual_album(label, items);
                            }
                            FoldersAction::StartReviewAll => {
                                self.refresh_volumes();
                                match continuation_target(&self.library, self.sort_direction) {
                                    Ok(Some(ContinuationTarget::Virtual(label, images))) => self.open_virtual_album(label, images),
                                    Ok(Some(ContinuationTarget::Folder(path))) => self.open_indexed_folder(path),
                                    Ok(None) => self.current_tab = NavTab::Review,
                                    Err(()) => {
                                        self.scan_report = Some("The last virtual album is unavailable. Reconnect its volume or rescan your library.".into());
                                        self.current_tab = NavTab::Scan;
                                    }
                                }
                            }
                        }
                    }
                    NavTab::SingleFolder => {
                        if SingleFolderView::ui(ui, &self.session) {
                            self.open_folder_dialog();
                        }
                    }
                    NavTab::Review => {
                        if self.session.current_item().is_some_and(|item| !self.library.path_available(&item.path)) {
                            ui.heading("Media unavailable");
                            ui.label("Reconnect the original volume. Your album and review progress are retained.");
                            return;
                        }
                        match self.swipe_view.ui(ui, &mut self.session, &mut self.cache) {
                            SwipeAction::None => {}
                            SwipeAction::OpenTrash => {
                                self.current_tab = NavTab::Trash;
                            }
                        }
                        self.save_active_progress();
                    }
                    NavTab::Trash => {
                        if self.session.items.iter().any(|item| !self.library.path_available(&item.path)) {
                            ui.heading("Some media is unavailable");
                            ui.label("Reconnect the original volume before restoring or deleting files.");
                            return;
                        }
                        self.trash_view.ui(ui, &mut self.session, &mut self.cache, &self.library.volume_bindings);
                    }
                    NavTab::Settings => {
                        SettingsView::ui(ui, &mut self.theme);
                        self.theme.apply(&ctx);
                    }
                }
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_active_progress();
        self.session.save_session();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.save_active_progress();
        eframe::set_value(storage, "zwiperoll_theme", &self.theme);
    }
}

fn remap_session(session: &mut Session, remaps: &[(PathBuf, PathBuf)]) {
    if let Some(folder) = &mut session.folder {
        *folder = remap_path(folder, remaps);
    }
    for item in &mut session.items {
        item.path = remap_path(&item.path, remaps);
    }
}

enum ContinuationTarget {
    Folder(PathBuf),
    Virtual(String, Vec<crate::model::MediaEntry>),
}

fn continuation_target(
    library: &LibraryCatalog,
    direction: SortDirection,
) -> Result<Option<ContinuationTarget>, ()> {
    if let Some(label) = &library.last_virtual_album {
        let albums = crate::model::albums::month_albums(
            library
                .folders
                .iter()
                .filter(|folder| library.path_available(&folder.path))
                .flat_map(|folder| folder.images.iter()),
            direction,
        );
        return albums
            .into_iter()
            .find(|album| &album.label == label)
            .map(|album| Some(ContinuationTarget::Virtual(album.label, album.images)))
            .ok_or(());
    }
    Ok(library
        .last_folder
        .clone()
        .or_else(|| library.folders.first().map(|folder| folder.path.clone()))
        .map(ContinuationTarget::Folder))
}

#[cfg(test)]
mod volume_session_tests {
    use super::*;

    use crate::model::{Decision, MediaEntry};

    #[test]
    fn continue_targets_last_virtual_album_instead_of_previous_real_folder() {
        use chrono::TimeZone;
        let folder = std::env::current_dir().unwrap();
        let photo = MediaEntry::new(
            folder.join("resume-test.jpg"),
            1,
            Some(
                chrono::Local
                    .with_ymd_and_hms(2025, 2, 15, 12, 0, 0)
                    .unwrap()
                    .into(),
            ),
        );
        let mut library = LibraryCatalog::default();
        library.last_folder = Some(folder.clone());
        library.last_virtual_album = Some("February 2025".into());
        library.folders = vec![crate::model::FolderRecord {
            path: folder.clone(),
            image_count: 1,
            total_bytes: 1,
            latest_modified: photo.modified,
            viewed_count: 0,
            current_index: 0,
            current_photo: Some(photo.path.clone()),
            images: vec![photo.clone()],
        }];
        match continuation_target(&library, SortDirection::Descending).unwrap() {
            Some(ContinuationTarget::Virtual(label, items)) => {
                assert_eq!(label, "February 2025");
                assert_eq!(items[0].path, photo.path);
            }
            _ => panic!("Continue must prefer the last virtual album"),
        }
        library.last_virtual_album = Some("January 2024".into());
        assert!(continuation_target(&library, SortDirection::Ascending).is_err());
        library.last_virtual_album = None;
        assert!(
            matches!(continuation_target(&library, SortDirection::Ascending).unwrap(),
            Some(ContinuationTarget::Folder(path)) if path == folder)
        );
    }

    #[test]
    fn drive_swap_preserves_session_decisions_and_position() {
        let mut session = Session {
            folder: Some("E:\\album".into()),
            items: vec![
                MediaEntry::new("E:\\album\\one.jpg".into(), 10, None),
                MediaEntry::new("F:\\other\\two.jpg".into(), 20, None),
            ],
            current_index: 1,
            undo_stack: vec![0],
        };
        session.items[0].decision = Decision::Keep;
        remap_session(
            &mut session,
            &[
                ("E:\\".into(), "F:\\".into()),
                ("F:\\".into(), "E:\\".into()),
            ],
        );
        assert_eq!(session.folder, Some("F:\\album".into()));
        assert_eq!(session.items[0].path, PathBuf::from("F:\\album\\one.jpg"));
        assert_eq!(session.items[1].path, PathBuf::from("E:\\other\\two.jpg"));
        assert_eq!(session.items[0].decision, Decision::Keep);
        assert_eq!(session.current_index, 1);
        assert_eq!(session.undo_stack, vec![0]);
    }
}
