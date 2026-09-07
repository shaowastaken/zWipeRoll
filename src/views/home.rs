use egui::{
    vec2, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, Sense, Ui, UiBuilder,
};

pub enum HomeAction {
    None,
    GetStarted,
    ScanDisks,
}

pub struct HomeView;

impl HomeView {
    pub fn ui(
        ui: &mut Ui,
        logo_texture_id: Option<egui::TextureId>,
        is_scanning: bool,
        scanned_count: usize,
    ) -> HomeAction {
        let mut action = HomeAction::None;
        let area = ui.available_rect_before_wrap();

        if is_scanning {
            // Panel 2: Global Scan in progress
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.heading(
                    RichText::new("Global Scan")
                        .size(24.0)
                        .color(Color32::WHITE)
                        .strong(),
                );
                ui.label(
                    RichText::new("Scanning all supported images on your drives...")
                        .color(Color32::from_rgb(142, 171, 192))
                        .size(13.0),
                );
                ui.add_space(35.0);

                let card_w = 480.0;
                let card_h = 70.0;
                let (rect, _) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(12),
                    Color32::from_rgb(24, 41, 54),
                );

                // Drive icon + Name + Count
                let icon_pos = Pos2::new(rect.left() + 18.0, rect.center().y);
                ui.painter().text(
                    icon_pos,
                    Align2::LEFT_CENTER,
                    "💽",
                    FontId::proportional(26.0),
                    Color32::WHITE,
                );

                let label_pos = Pos2::new(rect.left() + 55.0, rect.top() + 20.0);
                ui.painter().text(
                    label_pos,
                    Align2::LEFT_TOP,
                    "Local Drives (C:, OneDrive, Pictures)",
                    FontId::proportional(14.0),
                    Color32::WHITE,
                );

                let files_pos = Pos2::new(rect.right() - 20.0, rect.top() + 20.0);
                ui.painter().text(
                    files_pos,
                    Align2::RIGHT_TOP,
                    format!("{} files", scanned_count),
                    FontId::proportional(13.0),
                    Color32::from_rgb(187, 209, 219),
                );

                ui.add_space(30.0);
                if ui
                    .add(
                        egui::Button::new(RichText::new("Cancel Scan").color(Color32::WHITE))
                            .fill(Color32::from_rgb(45, 68, 85))
                            .min_size(vec2(130.0, 36.0)),
                    )
                    .clicked()
                {
                    action = HomeAction::None;
                }
            });
            return action;
        }

        // Panel 1: App Launch / Home Centrato Assoluto
        let center = area.center();
        let content_w = 400.0;
        let content_h = 360.0;
        let content_rect = Rect::from_center_size(center, vec2(content_w, content_h));

        let mut child_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(content_rect)
                .layout(Layout::top_down(egui::Align::Center)),
        );

        // Render Centered Logo
        if let Some(tex_id) = logo_texture_id {
            let logo_size = vec2(170.0, 170.0);
            child_ui.image(egui::load::SizedTexture::new(tex_id, logo_size));
        } else {
            child_ui.label(RichText::new("📷").size(90.0));
        }

        child_ui.add_space(16.0);

        // Wordmark Title
        child_ui.label(
            RichText::new("zWipeRoll")
                .size(32.0)
                .color(Color32::WHITE)
                .strong(),
        );

        // Tagline
        child_ui.label(
            RichText::new("Swipe through. Wipe what you don’t need.")
                .size(14.0)
                .color(Color32::from_rgb(142, 171, 192)),
        );

        child_ui.add_space(28.0);

        // Pill Button: "Get Started →"
        let btn_size = vec2(165.0, 44.0);
        let (btn_rect, btn_resp) = child_ui.allocate_exact_size(btn_size, Sense::click());
        let is_hover = btn_resp.hovered();

        let btn_color = if is_hover {
            Color32::from_rgb(14, 182, 238)
        } else {
            Color32::from_rgb(12, 165, 214) // Azure #0CA5D6
        };

        child_ui.painter().rect_filled(btn_rect, CornerRadius::same(22), btn_color);
        child_ui.painter().text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            "Get Started →",
            FontId::proportional(15.0),
            Color32::WHITE,
        );

        if btn_resp.clicked() {
            action = HomeAction::GetStarted;
        }

        action
    }
}
