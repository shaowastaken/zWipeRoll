use egui::viewport::ResizeDirection;
use egui::{Context, CursorIcon, Id, Pos2, Rect, Sense, Ui, ViewportCommand};

pub fn toggle_maximize(ctx: &Context) {
    let maximized = ctx
        .input(|input| input.viewport().maximized)
        .unwrap_or(false);
    // Let the native window manager retain and restore the normal window bounds.
    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
}

pub fn title_bar(ui: &mut Ui, rect: Rect, id: Id) {
    let response = ui.interact(rect, id, Sense::click_and_drag());
    if response.double_clicked() {
        toggle_maximize(ui.ctx());
    } else if response.is_pointer_button_down_on()
        && ui.input(|input| input.pointer.primary_pressed())
    {
        // Native caption dragging owns maximized drag-to-restore and pointer anchoring.
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

fn resize_direction(bounds: Rect, point: Pos2) -> Option<ResizeDirection> {
    if !bounds.contains(point) {
        return None;
    }
    let left = point.x < bounds.left() + 7.0;
    let right = point.x > bounds.right() - 7.0;
    let top = point.y < bounds.top() + 7.0;
    let bottom = point.y > bounds.bottom() - 7.0;
    let near_left = point.x < bounds.left() + 14.0;
    let near_right = point.x > bounds.right() - 14.0;
    let near_top = point.y < bounds.top() + 14.0;
    let near_bottom = point.y > bounds.bottom() - 14.0;
    match (near_left, near_right, near_top, near_bottom) {
        (true, _, true, _) => Some(ResizeDirection::NorthWest),
        (_, true, true, _) => Some(ResizeDirection::NorthEast),
        (true, _, _, true) => Some(ResizeDirection::SouthWest),
        (_, true, _, true) => Some(ResizeDirection::SouthEast),
        _ if left => Some(ResizeDirection::West),
        _ if right => Some(ResizeDirection::East),
        _ if top => Some(ResizeDirection::North),
        _ if bottom => Some(ResizeDirection::South),
        _ => None,
    }
}

pub fn resize_edges(ctx: &Context) {
    let (point, pressed, maximized) = ctx.input(|input| {
        (
            input.pointer.hover_pos(),
            input.pointer.primary_pressed(),
            input.viewport().maximized.unwrap_or(false),
        )
    });
    if maximized {
        return;
    }
    let Some(direction) = point.and_then(|point| resize_direction(ctx.content_rect(), point))
    else {
        return;
    };
    ctx.set_cursor_icon(match direction {
        ResizeDirection::North | ResizeDirection::South => CursorIcon::ResizeVertical,
        ResizeDirection::East | ResizeDirection::West => CursorIcon::ResizeHorizontal,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::ResizeNeSw,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::ResizeNwSe,
    });
    // Press-time hit testing avoids child widgets stealing the perimeter gesture.
    if pressed {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        ctx: &Context,
        point: Option<Pos2>,
        pressed: Option<bool>,
        maximized: bool,
        time: f64,
        draw: impl FnMut(&mut Ui),
    ) -> Vec<ViewportCommand> {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1100.0, 700.0))),
            time: Some(time),
            ..Default::default()
        };
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .maximized = Some(maximized);
        if let Some(point) = point {
            input.events.push(egui::Event::PointerMoved(point));
            if let Some(pressed) = pressed {
                input.events.push(egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                });
            }
        }
        let mut output = ctx.run_ui(input, draw);
        output.textures_delta.clear();
        output
            .viewport_output
            .remove(&egui::ViewportId::ROOT)
            .unwrap()
            .commands
    }

    #[test]
    fn all_eight_edges_start_native_resize_on_primary_press() {
        for (point, direction) in [
            (egui::pos2(1.0, 1.0), ResizeDirection::NorthWest),
            (egui::pos2(1099.0, 1.0), ResizeDirection::NorthEast),
            (egui::pos2(1.0, 699.0), ResizeDirection::SouthWest),
            (egui::pos2(1099.0, 699.0), ResizeDirection::SouthEast),
            (egui::pos2(550.0, 1.0), ResizeDirection::North),
            (egui::pos2(550.0, 699.0), ResizeDirection::South),
            (egui::pos2(1.0, 350.0), ResizeDirection::West),
            (egui::pos2(1099.0, 350.0), ResizeDirection::East),
        ] {
            let commands = frame(
                &Context::default(),
                Some(point),
                Some(true),
                false,
                0.0,
                |ui| resize_edges(ui.ctx()),
            );
            assert!(commands.iter().any(|command| matches!(command, ViewportCommand::BeginResize(actual) if *actual == direction)), "Missing native resize at {point:?}: {commands:?}");
        }
    }

    #[test]
    fn resize_ignores_interior_hover_and_maximized_window() {
        for (point, press, maximized) in [
            (egui::pos2(550.0, 350.0), Some(true), false),
            (egui::pos2(1.0, 350.0), None, false),
            (egui::pos2(1.0, 350.0), Some(true), true),
        ] {
            let commands = frame(
                &Context::default(),
                Some(point),
                press,
                maximized,
                0.0,
                |ui| resize_edges(ui.ctx()),
            );
            assert!(
                !commands
                    .iter()
                    .any(|command| matches!(command, ViewportCommand::BeginResize(_))),
                "Unexpected native resize: {commands:?}"
            );
        }
    }

    #[test]
    fn title_press_starts_native_drag_without_manual_restore() {
        for maximized in [false, true] {
            let ctx = Context::default();
            let rect = Rect::from_min_max(egui::pos2(100.0, 8.0), egui::pos2(1090.0, 26.0));
            let draw = |ui: &mut Ui| title_bar(ui, rect, Id::new("test_title"));
            frame(&ctx, None, None, maximized, 0.0, draw);
            let commands = frame(&ctx, Some(rect.center()), Some(true), maximized, 0.1, draw);
            assert!(
                commands
                    .iter()
                    .any(|command| matches!(command, ViewportCommand::StartDrag)),
                "Native drag must start on press: {commands:?}"
            );
            assert!(
                !commands.iter().any(|command| matches!(
                    command,
                    ViewportCommand::Maximized(_)
                        | ViewportCommand::InnerSize(_)
                        | ViewportCommand::OuterPosition(_)
                )),
                "Native caption drag must own restore and anchoring: {commands:?}"
            );
        }
    }

    #[test]
    fn title_double_click_toggles_native_maximize_without_geometry_commands() {
        for maximized in [false, true] {
            let ctx = Context::default();
            let rect = Rect::from_min_max(egui::pos2(100.0, 8.0), egui::pos2(1090.0, 26.0));
            let draw = |ui: &mut Ui| title_bar(ui, rect, Id::new("test_title"));
            frame(&ctx, None, None, maximized, 0.0, draw);
            frame(&ctx, Some(rect.center()), Some(true), maximized, 0.1, draw);
            frame(
                &ctx,
                Some(rect.center()),
                Some(false),
                maximized,
                0.15,
                draw,
            );
            frame(&ctx, Some(rect.center()), Some(true), maximized, 0.2, draw);
            let commands = frame(
                &ctx,
                Some(rect.center()),
                Some(false),
                maximized,
                0.25,
                draw,
            );
            assert!(commands.iter().any(|command| matches!(command, ViewportCommand::Maximized(actual) if *actual != maximized)), "Double click must toggle native maximize: {commands:?}");
            assert!(
                !commands.iter().any(|command| matches!(
                    command,
                    ViewportCommand::InnerSize(_) | ViewportCommand::OuterPosition(_)
                )),
                "Native maximize must retain restoration geometry: {commands:?}"
            );
        }
    }

    #[test]
    fn green_sidebar_control_toggles_observed_native_state() {
        use crate::views::{NavTab, SidebarView};
        let ctx = Context::default();
        let draw = |ui: &mut Ui| {
            SidebarView::ui(ui, NavTab::Home, 0, None, crate::theme::ThemeMode::Dark);
        };
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1100.0, 700.0))),
                time: Some(0.0),
                ..Default::default()
            },
            draw,
        );
        output.textures_delta.clear();
        // Click the rendered green control rather than pinning its layout coordinates.
        let point = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Circle(circle)
                    if circle.fill == egui::Color32::from_rgb(39, 201, 63) =>
                {
                    Some(circle.center)
                }
                _ => None,
            })
            .expect("Sidebar must render its green native window control");
        for (index, maximized) in [false, true].into_iter().enumerate() {
            let time = 1.0 + index as f64;
            frame(&ctx, Some(point), Some(true), maximized, time, draw);
            let commands = frame(&ctx, Some(point), Some(false), maximized, time + 0.05, draw);
            assert!(commands.iter().any(|command| matches!(command, ViewportCommand::Maximized(actual) if *actual != maximized)), "Green control must toggle observed native state: {commands:?}");
            assert!(
                !commands.iter().any(|command| matches!(
                    command,
                    ViewportCommand::InnerSize(_) | ViewportCommand::OuterPosition(_)
                )),
                "Native restore geometry must remain untouched: {commands:?}"
            );
        }
    }
}
