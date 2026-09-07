use std::path::PathBuf;
use egui::{FontData, FontDefinitions, FontFamily, TextureHandle, TextureOptions, Ui};
use crate::filesystem::{discover_system_picture_dirs, scan_directories, scan_directory};
use crate::imaging::TextureCache;
use crate::model::Session;
use crate::views::{
    FoldersAction, FoldersView, HomeAction, HomeView, NavTab, SettingsView, SidebarView,
    SwipeAction, SwipeView, TrashGridView,
};

pub struct PcSpongeApp {
    session: Session,
    cache: TextureCache,
    current_tab: NavTab,
    swipe_view: SwipeView,
    trash_view: TrashGridView,
    logo_texture: Option<TextureHandle>,
    cleaned_bytes_total: u64,
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

        // Dark macOS Window Theme
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(13, 22, 31);     // macOS Content Background
        visuals.window_fill = egui::Color32::from_rgb(18, 30, 42);    // Card Surface
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(18, 30, 42);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(24, 41, 56);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(17, 118, 145);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(12, 165, 214);
        cc.egui_ctx.set_visuals(visuals);

        // Load brand logo as texture
        let logo_bytes = include_bytes!("../assets/logo.png");
        let logo_texture = if let Ok(img) = image::load_from_memory(logo_bytes) {
            let size = [img.width() as usize, img.height() as usize];
            let rgba = img.to_rgba8();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            Some(cc.egui_ctx.load_texture("brand_logo", color_image, TextureOptions::LINEAR))
        } else {
            None
        };

        // Pass Context to TextureCache so background decoders trigger immediate request_repaint()
        let cache = TextureCache::new(16, Some(cc.egui_ctx.clone()));

        // Restore previous session if available
        let session = Session::load_session().unwrap_or_default();

        Self {
            session,
            cache,
            current_tab: NavTab::Scan,
            swipe_view: SwipeView::new(),
            trash_view: TrashGridView::new(),
            logo_texture,
            cleaned_bytes_total: 0,
        }
    }

    fn open_folder_dialog(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            let items = scan_directory(&folder);
            self.session.set_items(folder, items);
            self.cache.clear();
            self.session.save_session();
            self.current_tab = NavTab::Folders;
        }
    }

    fn scan_all_pictures(&mut self) {
        let picture_dirs = discover_system_picture_dirs();
        let items = scan_directories(&picture_dirs);
        let pseudo_path = PathBuf::from("Tutto il PC (Immagini, OneDrive, Download)");
        self.session.set_items(pseudo_path, items);
        self.cache.clear();
        self.session.save_session();
        self.current_tab = NavTab::Folders;
    }
}

impl eframe::App for PcSpongeApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.cache.poll_results(&ctx);

        let logo_tex_id = self.logo_texture.as_ref().map(|t| t.id());

        // Master 2-Column Layout: Fixed Sidebar (210px) + Main Content Area
        ui.horizontal(|ui| {
            // Left Fixed Sidebar
            if let Some(tab) = SidebarView::ui(
                ui,
                self.current_tab,
                self.cleaned_bytes_total,
                logo_tex_id,
            ) {
                self.current_tab = tab;
            }

            // Right Main Content Panel
            ui.vertical(|ui| {
                ui.add_space(8.0);
                match self.current_tab {
                    NavTab::Scan => {
                        match HomeView::ui(ui, logo_tex_id, false, self.session.items.len()) {
                            HomeAction::None => {}
                            HomeAction::GetStarted => {
                                if self.session.items.is_empty() {
                                    self.scan_all_pictures();
                                } else {
                                    self.current_tab = NavTab::Review;
                                }
                            }
                            HomeAction::ScanDisks => {
                                self.scan_all_pictures();
                            }
                        }
                    }
                    NavTab::Folders => {
                        match FoldersView::ui(ui, &mut self.session) {
                            FoldersAction::None => {}
                            FoldersAction::BrowseFolder => self.open_folder_dialog(),
                            FoldersAction::SelectMonth(idx) => {
                                self.session.current_index = idx;
                                self.current_tab = NavTab::Review;
                            }
                            FoldersAction::StartReviewAll => {
                                self.current_tab = NavTab::Review;
                            }
                        }
                    }
                    NavTab::Review => {
                        match self.swipe_view.ui(ui, &mut self.session, &mut self.cache) {
                            SwipeAction::None => {}
                            SwipeAction::OpenTrash => {
                                self.current_tab = NavTab::Trash;
                            }
                        }
                    }
                    NavTab::Trash => {
                        self.trash_view.ui(ui, &mut self.session, &mut self.cache);
                    }
                    NavTab::Settings => {
                        SettingsView::ui(ui);
                    }
                }
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.session.save_session();
    }
}
