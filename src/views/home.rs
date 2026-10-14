use egui::{vec2, Align, Color32, Layout, Rect, RichText, Ui, UiBuilder};

pub struct HomeView;

impl HomeView {
    /// The welcome screen only navigates to scan setup; it never starts disk I/O.
    pub fn ui(ui: &mut Ui, logo: Option<egui::TextureId>) -> bool {
        let area = ui.available_rect_before_wrap();
        let width = area.width().min(400.0);
        let rect = Rect::from_center_size(area.center(), vec2(width, 230.0));
        let mut content = ui.new_child(
            UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::top_down(Align::Center)),
        );
        if let Some(texture) = logo {
            let logo_width = width.min(300.0);
            content.image(egui::load::SizedTexture::new(
                texture,
                vec2(logo_width, logo_width * 88.0 / 300.0),
            ));
        } else {
            content.label(RichText::new("zWipeRoll").size(32.0).strong());
        }
        content.add_space(14.0);
        content.label(
            RichText::new("Swipe through. Wipe what you don’t need.")
                .size(14.0)
                .color(crate::theme::muted(ui.visuals().dark_mode)),
        );
        content.add_space(28.0);
        content
            .add(
                egui::Button::new(RichText::new("Get Started →").color(Color32::WHITE))
                    .fill(Color32::from_rgb(10, 117, 246))
                    .corner_radius(22)
                    .min_size(vec2(190.0, 44.0)),
            )
            .clicked()
    }
}
