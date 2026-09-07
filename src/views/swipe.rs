use egui::{
    vec2, Align2, Color32, CornerRadius, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};
use crate::imaging::TextureCache;
use crate::model::{Decision, MediaEntry, Session};

pub enum SwipeAction {
    None,
    OpenTrash,
}

pub struct SwipeView {
    drag_offset: Vec2,
    is_dragging: bool,
}

impl Default for SwipeView {
    fn default() -> Self {
        Self {
            drag_offset: Vec2::ZERO,
            is_dragging: false,
        }
    }
}

impl SwipeView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        session: &mut Session,
        cache: &mut TextureCache,
    ) -> SwipeAction {
        let mut action = SwipeAction::None;

        // Key shortcuts: A -> Trash, D -> Keep, Arrows -> Scroll only, Ctrl+Z -> Undo
        self.handle_input(ui, session);

        if session.items.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.heading("Nessuna immagine in revisione.");
                ui.add_space(10.0);
                ui.label("Seleziona una cartella dalla sezione Folders o avvia una scansione.");
            });
            return action;
        }

        ui.add_space(10.0);

        // 1. Top Bar: Counter "3,421 / 8,742" centered, Fullscreen and Options at right (Panel 5)
        let total = session.items.len();
        let curr = session.current_index + 1;
        let counter_str = format!("{} / {}", curr, total);

        ui.horizontal(|ui| {
            let avail_w = ui.available_width();
            let center_x = ui.cursor().left() + avail_w / 2.0;

            ui.painter().text(
                Pos2::new(center_x, ui.cursor().center().y),
                Align2::CENTER_CENTER,
                counter_str,
                FontId::proportional(14.0),
                Color32::from_rgb(186, 206, 218),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if ui.button("•••").on_hover_text("Opzioni").clicked() {
                    action = SwipeAction::OpenTrash;
                }
                if ui.button("⛶").on_hover_text("Schermo Intero").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                }
            });
        });

        ui.add_space(8.0);

        // 2. Center Card Area (Occupies most vertical space)
        let card_area_h = (ui.available_height() - 110.0).max(280.0);
        let card_area_w = ui.available_width();
        let (deck_rect, _) =
            ui.allocate_exact_size(vec2(card_area_w, card_area_h), Sense::hover());

        // Preload adjacent images
        let max_w = (card_area_w * 1.5).max(1280.0) as u32;
        let max_h = (card_area_h * 1.5).max(800.0) as u32;
        let curr_idx = session.current_index;

        if let Some(item) = session.items.get(curr_idx) {
            cache.request_if_needed(item.path.clone(), max_w, max_h);
        }
        if let Some(item) = session.items.get(curr_idx + 1) {
            cache.request_if_needed(item.path.clone(), max_w, max_h);
        }
        if let Some(item) = session.items.get(curr_idx + 2) {
            cache.request_if_needed(item.path.clone(), max_w, max_h);
        }
        if curr_idx > 0 {
            if let Some(item) = session.items.get(curr_idx - 1) {
                cache.request_if_needed(item.path.clone(), max_w, max_h);
            }
        }

        // Render card
        if let Some(item) = session.current_item().cloned() {
            self.render_card(ui, deck_rect, &item, cache, session);
        }

        // 3. Side Navigation Arrows ‹ and ›
        let arrow_radius = 20.0;
        let arrow_y = deck_rect.center().y;

        // Left arrow ‹
        let left_center = Pos2::new(deck_rect.left() + 24.0, arrow_y);
        let left_rect = Rect::from_center_size(left_center, vec2(arrow_radius * 2.0, arrow_radius * 2.0));
        let left_resp = ui.allocate_rect(left_rect, Sense::click());
        let left_hover = left_resp.hovered();

        ui.painter().circle_filled(
            left_center,
            arrow_radius,
            if left_hover {
                Color32::from_rgba_unmultiplied(32, 52, 70, 220)
            } else {
                Color32::from_rgba_unmultiplied(20, 36, 52, 140)
            },
        );
        ui.painter().text(
            left_center,
            Align2::CENTER_CENTER,
            "‹",
            FontId::proportional(26.0),
            Color32::WHITE,
        );
        if left_resp.clicked() {
            session.prev();
            self.drag_offset = Vec2::ZERO;
        }

        // Right arrow ›
        let right_center = Pos2::new(deck_rect.right() - 24.0, arrow_y);
        let right_rect = Rect::from_center_size(right_center, vec2(arrow_radius * 2.0, arrow_radius * 2.0));
        let right_resp = ui.allocate_rect(right_rect, Sense::click());
        let right_hover = right_resp.hovered();

        ui.painter().circle_filled(
            right_center,
            arrow_radius,
            if right_hover {
                Color32::from_rgba_unmultiplied(32, 52, 70, 220)
            } else {
                Color32::from_rgba_unmultiplied(20, 36, 52, 140)
            },
        );
        ui.painter().text(
            right_center,
            Align2::CENTER_CENTER,
            "›",
            FontId::proportional(26.0),
            Color32::WHITE,
        );
        if right_resp.clicked() {
            session.next();
            self.drag_offset = Vec2::ZERO;
        }

        // 4. Bottom Dock: Exact Round Circles 58px (#EF4444 and #22C55E)
        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 45.0;

                // Red Circle Trash (58px)
                let circle_size = vec2(58.0, 58.0);
                let (trash_rect, trash_resp) = ui.allocate_exact_size(circle_size, Sense::click());
                let trash_hover = trash_resp.hovered();

                let trash_color = if trash_hover {
                    Color32::from_rgb(255, 80, 80)
                } else {
                    Color32::from_rgb(239, 68, 68) // #EF4444
                };

                ui.painter().circle_filled(trash_rect.center(), 28.0, trash_color);
                ui.painter().text(
                    trash_rect.center(),
                    Align2::CENTER_CENTER,
                    "🗑",
                    FontId::proportional(24.0),
                    Color32::WHITE,
                );

                if trash_resp.clicked() {
                    session.apply_decision(Decision::Trash);
                    self.drag_offset = Vec2::ZERO;
                }

                // Green Circle Keep (58px)
                let (keep_rect, keep_resp) = ui.allocate_exact_size(circle_size, Sense::click());
                let keep_hover = keep_resp.hovered();

                let keep_color = if keep_hover {
                    Color32::from_rgb(46, 215, 108)
                } else {
                    Color32::from_rgb(34, 197, 94) // #22C55E
                };

                ui.painter().circle_filled(keep_rect.center(), 28.0, keep_color);
                ui.painter().text(
                    keep_rect.center(),
                    Align2::CENTER_CENTER,
                    "✓",
                    FontId::proportional(26.0),
                    Color32::WHITE,
                );

                if keep_resp.clicked() {
                    session.apply_decision(Decision::Keep);
                    self.drag_offset = Vec2::ZERO;
                }
            });

            ui.add_space(4.0);

            // Subtitle label: "Swipe or use ← →"
            ui.label(
                egui::RichText::new("Swipe or use A / D • ← →")
                    .size(12.0)
                    .color(Color32::from_rgb(110, 139, 158)),
            );
        });

        action
    }

    fn render_card(
        &mut self,
        ui: &mut Ui,
        deck_rect: Rect,
        item: &MediaEntry,
        cache: &mut TextureCache,
        session: &mut Session,
    ) {
        let card_size = deck_rect.size() * vec2(0.85, 0.95);
        let center = deck_rect.center() + self.drag_offset;
        let card_rect = Rect::from_center_size(center, card_size);

        let response = ui.allocate_rect(card_rect, Sense::drag());

        if response.dragged() {
            self.is_dragging = true;
            self.drag_offset += response.drag_delta();
        }

        if response.drag_stopped() {
            self.is_dragging = false;
            let threshold = card_size.x * 0.25;

            if self.drag_offset.x > threshold {
                session.apply_decision(Decision::Keep);
            } else if self.drag_offset.x < -threshold {
                session.apply_decision(Decision::Trash);
            }
            self.drag_offset = Vec2::ZERO;
        }

        let painter = ui.painter_at(deck_rect);

        // Card Surface Background with 20px Corner Radius
        painter.rect_filled(
            card_rect,
            CornerRadius::same(20),
            Color32::from_rgb(18, 30, 42),
        );

        // Swipe tint/glow
        let threshold = card_size.x * 0.25;
        if self.drag_offset.x > 25.0 {
            let p = (self.drag_offset.x / threshold).clamp(0.0, 1.0);
            let color = Color32::from_rgba_unmultiplied(34, 197, 94, (p * 220.0) as u8);
            painter.rect_stroke(
                card_rect,
                CornerRadius::same(20),
                Stroke::new(3.0, color),
                StrokeKind::Inside,
            );
        } else if self.drag_offset.x < -25.0 {
            let p = (-self.drag_offset.x / threshold).clamp(0.0, 1.0);
            let color = Color32::from_rgba_unmultiplied(239, 68, 68, (p * 220.0) as u8);
            painter.rect_stroke(
                card_rect,
                CornerRadius::same(20),
                Stroke::new(3.0, color),
                StrokeKind::Inside,
            );
        } else {
            painter.rect_stroke(
                card_rect,
                CornerRadius::same(20),
                Stroke::new(1.0, Color32::from_rgb(38, 58, 76)),
                StrokeKind::Inside,
            );
        }

        // Draw Image
        if let Some(texture) = cache.get(&item.path) {
            let tex_size = texture.size_vec2();
            let max_display = card_size - vec2(20.0, 20.0);

            let scale = (max_display.x / tex_size.x)
                .min(max_display.y / tex_size.y)
                .min(1.0);
            let display_size = tex_size * scale;
            let img_rect = Rect::from_center_size(card_rect.center(), display_size);

            painter.image(
                texture.id(),
                img_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            painter.text(
                card_rect.center(),
                Align2::CENTER_CENTER,
                "Loading...",
                FontId::proportional(16.0),
                Color32::from_rgb(110, 139, 158),
            );
        }
    }

    fn handle_input(&mut self, ui: &Ui, session: &mut Session) {
        ui.input(|i| {
            if i.key_pressed(Key::D) {
                session.apply_decision(Decision::Keep);
                self.drag_offset = Vec2::ZERO;
            } else if i.key_pressed(Key::A) {
                session.apply_decision(Decision::Trash);
                self.drag_offset = Vec2::ZERO;
            } else if i.key_pressed(Key::ArrowRight) {
                session.next();
                self.drag_offset = Vec2::ZERO;
            } else if i.key_pressed(Key::ArrowLeft) {
                session.prev();
                self.drag_offset = Vec2::ZERO;
            } else if (i.modifiers.ctrl && i.key_pressed(Key::Z)) || i.key_pressed(Key::U) {
                session.undo();
                self.drag_offset = Vec2::ZERO;
            }
        });
    }
}
