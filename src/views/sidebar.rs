use crate::theme::ThemeMode;
use egui::{vec2, Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, Ui, UiBuilder};

#[derive(Default)]
pub struct SidebarAction {
    pub tab: Option<NavTab>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavTab {
    Home,
    Scan,
    Folders,
    SingleFolder,
    Review,
    Trash,
    Settings,
}

pub struct SidebarView;

impl SidebarView {
    pub fn ui(
        ui: &mut Ui,
        current: NavTab,
        cleaned: u64,
        logo: Option<egui::TextureId>,
        theme: ThemeMode,
    ) -> SidebarAction {
        let mut action = SidebarAction::default();
        let dark = theme.is_dark();
        let available = ui.available_rect_before_wrap();
        let width = (available.width() * 0.216).clamp(176.0, 260.0);
        let rect = Rect::from_min_size(available.min, vec2(width, available.height()));
        ui.painter().rect_filled(
            rect,
            CornerRadius {
                nw: 12,
                sw: 12,
                ne: 0,
                se: 0,
            },
            crate::theme::sidebar(dark),
        );
        ui.painter().line_segment(
            [rect.right_top(), rect.right_bottom()],
            Stroke::new(1.0, crate::theme::border(dark)),
        );

        let mut side = ui.new_child(
            UiBuilder::new()
                .max_rect(rect.shrink2(vec2(12.0, 12.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        Self::window_controls(&mut side);
        side.add_space(18.0);

        let title = side
            .horizontal(|ui| {
                if let Some(texture) = logo {
                    let logo_width = ui.available_width().min(width * 0.78);
                    ui.image(egui::load::SizedTexture::new(
                        texture,
                        vec2(logo_width, logo_width * 34.0 / 116.0),
                    ));
                }
            })
            .response;
        crate::window_chrome::title_bar(&mut side, title.rect, egui::Id::new("sidebar_title_drag"));

        side.add_space(22.0);
        for (tab, label) in [
            (NavTab::Home, "Home"),
            (NavTab::Scan, "Global Scan"),
            (NavTab::Folders, "Library"),
            (NavTab::SingleFolder, "Single Folder"),
            (NavTab::Review, "Review"),
            (NavTab::Trash, "Trash"),
        ] {
            if Self::nav_item(&mut side, tab, label, current == tab, dark).clicked() {
                action.tab = Some(tab);
            }
            side.add_space(3.0);
        }

        side.add_space((side.available_height() - 82.0).max(0.0));
        side.painter().line_segment(
            [
                Pos2::new(side.min_rect().left(), side.cursor().top()),
                Pos2::new(side.max_rect().right(), side.cursor().top()),
            ],
            Stroke::new(1.0, crate::theme::border(dark)),
        );
        side.add_space(10.0);
        if Self::nav_item(
            &mut side,
            NavTab::Settings,
            "Settings",
            current == NavTab::Settings,
            dark,
        )
        .clicked()
        {
            action.tab = Some(NavTab::Settings);
        }
        side.add_space(7.0);
        side.label(
            egui::RichText::new(format!(
                "{} cleaned",
                crate::model::media::format_bytes(cleaned)
            ))
            .size(10.0)
            .color(crate::theme::muted(dark)),
        );

        ui.advance_cursor_after_rect(rect);
        action
    }

    fn window_controls(ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let close = Self::traffic_light(ui, Color32::from_rgb(255, 95, 86), "Close");
            let minimize = Self::traffic_light(ui, Color32::from_rgb(255, 189, 46), "Minimize");
            let maximize =
                Self::traffic_light(ui, Color32::from_rgb(39, 201, 63), "Maximize / Restore");
            if close.clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if minimize.clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            if maximize.clicked() {
                crate::window_chrome::toggle_maximize(ui.ctx());
            }
        });
    }

    fn traffic_light(ui: &mut Ui, color: Color32, tooltip: &str) -> Response {
        let (rect, response) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::click());
        ui.painter().circle_filled(rect.center(), 5.2, color);
        if response.hovered() {
            ui.painter().circle_stroke(
                rect.center(),
                5.2,
                Stroke::new(1.0, Color32::from_black_alpha(90)),
            );
        }
        response.on_hover_text(tooltip)
    }

    fn nav_item(ui: &mut Ui, tab: NavTab, label: &str, active: bool, dark: bool) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
        let fill = if active {
            crate::theme::selection(dark)
        } else if response.hovered() {
            crate::theme::surface_raised(dark)
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, CornerRadius::same(7), fill);
        let color = if active {
            crate::theme::selection_text(dark)
        } else if response.hovered() {
            crate::theme::text(dark)
        } else {
            crate::theme::muted(dark)
        };
        Self::paint_icon(
            ui,
            tab,
            Pos2::new(rect.left() + 17.0, rect.center().y),
            color,
        );
        ui.painter().text(
            Pos2::new(rect.left() + 36.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(14.0),
            color,
        );
        response
    }

    fn paint_icon(ui: &Ui, tab: NavTab, center: Pos2, color: Color32) {
        let p = ui.painter();
        let s = Stroke::new(1.35, color);
        match tab {
            NavTab::Home => {
                p.line(
                    vec![
                        center + vec2(-6.0, 0.0),
                        center + vec2(0.0, -5.0),
                        center + vec2(6.0, 0.0),
                    ],
                    s,
                );
                p.line(
                    vec![
                        center + vec2(-4.0, -1.0),
                        center + vec2(-4.0, 5.0),
                        center + vec2(4.0, 5.0),
                        center + vec2(4.0, -1.0),
                    ],
                    s,
                );
            }
            NavTab::Scan => {
                p.circle_stroke(center - vec2(1.5, 1.5), 4.0, s);
                p.line_segment([center + vec2(1.5, 1.5), center + vec2(5.0, 5.0)], s);
            }
            NavTab::Folders | NavTab::SingleFolder => {
                let r = Rect::from_center_size(center + vec2(0.0, 1.0), vec2(12.0, 8.0));
                p.rect_stroke(r, CornerRadius::same(1), s, egui::StrokeKind::Inside);
                p.line(
                    vec![
                        r.left_top() + vec2(1.0, 0.0),
                        r.left_top() + vec2(4.0, -2.5),
                        r.left_top() + vec2(7.0, -2.5),
                    ],
                    s,
                );
            }
            NavTab::Review => {
                let r = Rect::from_center_size(center, vec2(11.0, 10.0));
                p.rect_stroke(r, CornerRadius::same(1), s, egui::StrokeKind::Inside);
                p.circle_filled(r.right_top() + vec2(-2.5, 2.5), 1.2, color);
                p.line(
                    vec![
                        r.left_bottom() + vec2(1.5, -1.5),
                        center,
                        r.right_bottom() + vec2(-1.0, -1.5),
                    ],
                    s,
                );
            }
            NavTab::Trash => {
                p.rect_stroke(
                    Rect::from_center_size(center + vec2(0.0, 1.5), vec2(8.0, 9.0)),
                    CornerRadius::same(1),
                    s,
                    egui::StrokeKind::Inside,
                );
                p.line_segment([center + vec2(-5.0, -4.5), center + vec2(5.0, -4.5)], s);
                p.line_segment([center + vec2(-2.0, -6.0), center + vec2(2.0, -6.0)], s);
            }
            NavTab::Settings => {
                p.circle_stroke(center, 5.0, s);
                p.circle_stroke(center, 1.8, s);
                for angle in [
                    0.0_f32,
                    std::f32::consts::FRAC_PI_2,
                    std::f32::consts::PI,
                    3.0 * std::f32::consts::FRAC_PI_2,
                ] {
                    let v = vec2(angle.cos(), angle.sin());
                    p.line_segment([center + v * 5.0, center + v * 6.5], s);
                }
            }
        }
    }
}
