#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod filesystem;
mod imaging;
mod model;
mod views;

use app::PcSpongeApp;
use eframe::egui::{vec2, IconData, ViewportBuilder};

fn load_app_icon() -> Option<IconData> {
    // Load embedded or local icon
    let icon_bytes = include_bytes!("../assets/icon.png");
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = img.dimensions();
        return Some(IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        });
    }
    None
}

use image::GenericImageView;

fn main() -> eframe::Result {
    let mut viewport = ViewportBuilder::default()
        .with_title("zWipeRoll - Gallery & Photo Cleaner")
        .with_inner_size(vec2(1150.0, 780.0))
        .with_min_inner_size(vec2(850.0, 580.0))
        .with_drag_and_drop(true);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "zWipeRoll",
        native_options,
        Box::new(|cc| Ok(Box::new(PcSpongeApp::new(cc)))),
    )
}
