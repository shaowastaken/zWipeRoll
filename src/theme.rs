use egui::{Color32, Context, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }

    pub fn apply(self, ctx: &Context) {
        ctx.set_theme(if self.is_dark() {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        let mut visuals = if self.is_dark() {
            Visuals::dark()
        } else {
            Visuals::light()
        };

        visuals.panel_fill = background(self.is_dark());
        visuals.window_fill = surface(self.is_dark());
        visuals.extreme_bg_color = surface_raised(self.is_dark());
        visuals.faint_bg_color = surface(self.is_dark());
        visuals.widgets.noninteractive.bg_fill = surface(self.is_dark());
        visuals.widgets.noninteractive.fg_stroke.color = text(self.is_dark());
        visuals.widgets.inactive.bg_fill = surface_raised(self.is_dark());
        visuals.widgets.inactive.fg_stroke.color = text(self.is_dark());
        for widget in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
        ] {
            widget.corner_radius = egui::CornerRadius::same(8);
            widget.bg_stroke = egui::Stroke::new(1.0, border(self.is_dark()));
        }
        visuals.widgets.inactive.weak_bg_fill = surface_raised(self.is_dark());
        visuals.widgets.hovered.bg_fill = if self.is_dark() {
            Color32::from_rgb(25, 56, 82)
        } else {
            Color32::from_rgb(220, 237, 252)
        };
        visuals.widgets.active.bg_fill = Color32::from_rgb(10, 117, 246);
        visuals.widgets.hovered.weak_bg_fill = visuals.widgets.hovered.bg_fill;
        visuals.widgets.active.weak_bg_fill = selection(self.is_dark());
        visuals.selection.bg_fill = Color32::from_rgb(10, 117, 246);
        visuals.window_corner_radius = egui::CornerRadius::same(12);
        ctx.set_visuals(visuals);
        ctx.style_mut_of(
            if self.is_dark() {
                egui::Theme::Dark
            } else {
                egui::Theme::Light
            },
            |style| {
                style.spacing.button_padding = egui::vec2(14.0, 7.0);
                style.spacing.interact_size.y = 30.0;
                style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            },
        );
    }
}

pub fn background(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(19, 23, 29)
    } else {
        Color32::from_rgb(245, 248, 251)
    }
}

pub fn sidebar(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(23, 28, 34)
    } else {
        Color32::from_rgb(234, 238, 243)
    }
}

pub fn surface(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(26, 31, 37)
    } else {
        Color32::from_rgb(249, 251, 253)
    }
}

pub fn surface_raised(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(34, 41, 48)
    } else {
        Color32::from_rgb(242, 246, 250)
    }
}

pub fn border(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(49, 57, 66)
    } else {
        Color32::from_rgb(210, 220, 230)
    }
}

pub fn selection(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(14, 49, 99)
    } else {
        Color32::from_rgb(207, 228, 249)
    }
}

pub fn selection_text(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(244, 247, 250)
    } else {
        Color32::from_rgb(0, 77, 221)
    }
}

pub fn text(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(244, 247, 250)
    } else {
        Color32::from_rgb(16, 29, 45)
    }
}

pub fn muted(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(155, 171, 184)
    } else {
        Color32::from_rgb(84, 103, 121)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn theme_switch_applies_controls_to_the_active_style() {
        let context = Context::default();
        for mode in [ThemeMode::Dark, ThemeMode::Light, ThemeMode::Dark] {
            mode.apply(&context);
            let style = context.style_of(context.theme());
            assert_eq!(style.visuals.dark_mode, mode.is_dark());
            assert_eq!(style.visuals.panel_fill, background(mode.is_dark()));
            assert_eq!(
                style.visuals.widgets.inactive.weak_bg_fill,
                surface_raised(mode.is_dark())
            );
            assert_eq!(
                style.visuals.widgets.inactive.corner_radius,
                egui::CornerRadius::same(8)
            );
            assert_eq!(style.spacing.button_padding, egui::vec2(14.0, 7.0));
        }
    }
}
