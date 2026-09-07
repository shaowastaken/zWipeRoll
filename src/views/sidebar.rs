use egui::{
    vec2, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, Response, RichText, Sense,
    Stroke, Ui, UiBuilder,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavTab {
    Scan,
    Folders,
    Review,
    Trash,
    Settings,
}

pub struct SidebarView;

impl SidebarView {
    pub fn ui(
        ui: &mut Ui,
        current_tab: NavTab,
        cleaned_bytes: u64,
        logo_texture_id: Option<egui::TextureId>,
    ) -> Option<NavTab> {
        let mut selected_tab = None;

        let available_rect = ui.available_rect_before_wrap();
        let sidebar_width = 210.0;
        let sidebar_rect = Rect::from_min_size(
            available_rect.min,
            vec2(sidebar_width, available_rect.height()),
        );

        // Sidebar Background: macOS dark window sidebar #0F1A24
        let painter = ui.painter();
        painter.rect_filled(
            sidebar_rect,
            CornerRadius {
                nw: 14,
                sw: 14,
                ne: 0,
                se: 0,
            },
            Color32::from_rgb(15, 26, 36),
        );
        // Vertical divider line: #1D2D3D
        painter.line_segment(
            [
                Pos2::new(sidebar_rect.right(), sidebar_rect.top()),
                Pos2::new(sidebar_rect.right(), sidebar_rect.bottom()),
            ],
            Stroke::new(1.0, Color32::from_rgb(29, 45, 61)),
        );

        // Content within sidebar using new_child
        let mut sidebar_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(sidebar_rect.shrink2(vec2(14.0, 14.0)))
                .layout(Layout::top_down(egui::Align::Min)),
        );

        // 1. macOS Traffic Lights (Close / Minimize / Fullscreen)
        sidebar_ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let dot_radius = 6.0;

            // Red #FF5F56
            let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::click());
            ui.painter().circle_filled(r.center(), dot_radius, Color32::from_rgb(255, 95, 86));

            // Yellow #FFBD2E
            let (y, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::click());
            ui.painter().circle_filled(y.center(), dot_radius, Color32::from_rgb(255, 189, 46));

            // Green #27C93F
            let (g, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::click());
            ui.painter().circle_filled(g.center(), dot_radius, Color32::from_rgb(39, 201, 63));
        });

        sidebar_ui.add_space(20.0);

        // 2. Logo & Wordmark Header: "zWipeRoll"
        sidebar_ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if let Some(tex_id) = logo_texture_id {
                let logo_size = vec2(24.0, 24.0);
                ui.image(egui::load::SizedTexture::new(tex_id, logo_size));
            } else {
                ui.label(RichText::new("📷").size(18.0));
            }

            ui.label(
                RichText::new("zWipeRoll")
                    .size(16.0)
                    .color(Color32::WHITE)
                    .strong(),
            );
        });

        sidebar_ui.add_space(22.0);

        // 3. Navigation Pills
        let tabs = [
            (NavTab::Scan, "🔍", "Scan"),
            (NavTab::Folders, "📁", "Folders"),
            (NavTab::Review, "🖼", "Review"),
            (NavTab::Trash, "🗑", "Trash"),
        ];

        for (tab, icon, label) in tabs {
            let is_active = current_tab == tab;
            if Self::render_nav_pill(&mut sidebar_ui, is_active, icon, label).clicked() {
                selected_tab = Some(tab);
            }
            sidebar_ui.add_space(4.0);
        }

        // 4. Bottom Footer (Settings & Cleaned Bytes Badge)
        let bottom_space = sidebar_ui.available_height() - 75.0;
        if bottom_space > 0.0 {
            sidebar_ui.add_space(bottom_space);
        }

        let is_settings_active = current_tab == NavTab::Settings;
        if Self::render_nav_pill(&mut sidebar_ui, is_settings_active, "⚙", "Settings").clicked() {
            selected_tab = Some(NavTab::Settings);
        }

        sidebar_ui.add_space(10.0);

        // Cleaned stats badge
        sidebar_ui.label(
            RichText::new(format!(
                "{} cleaned",
                crate::model::media::format_bytes(cleaned_bytes)
            ))
            .size(11.0)
            .color(Color32::from_rgb(110, 139, 158)),
        );

        // Advance the parent ui beyond the sidebar width
        ui.advance_cursor_after_rect(Rect::from_min_size(
            available_rect.min,
            vec2(sidebar_width, available_rect.height()),
        ));

        selected_tab
    }

    fn render_nav_pill(
        ui: &mut Ui,
        is_active: bool,
        icon: &str,
        label: &str,
    ) -> Response {
        let desired_size = vec2(182.0, 36.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        let bg_color = if is_active {
            Color32::from_rgb(12, 118, 184) // macOS Active Blue #0C76B8
        } else if response.hovered() {
            Color32::from_rgba_unmultiplied(24, 42, 58, 180)
        } else {
            Color32::TRANSPARENT
        };

        ui.painter().rect_filled(rect, CornerRadius::same(10), bg_color);

        let text_color = if is_active {
            Color32::WHITE
        } else if response.hovered() {
            Color32::from_rgb(224, 238, 247)
        } else {
            Color32::from_rgb(142, 171, 192)
        };

        let icon_pos = Pos2::new(rect.left() + 14.0, rect.center().y);
        ui.painter().text(
            icon_pos,
            Align2::LEFT_CENTER,
            icon,
            FontId::proportional(15.0),
            text_color,
        );

        let label_pos = Pos2::new(rect.left() + 38.0, rect.center().y);
        ui.painter().text(
            label_pos,
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(14.0),
            text_color,
        );

        response
    }
}
