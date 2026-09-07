use std::path::{Path, PathBuf};
use egui::{ColorImage, Context};
use image::GenericImageView;

pub fn load_and_downscale(path: &Path, max_w: u32, max_h: u32) -> Result<ColorImage, String> {
    let img = image::ImageReader::open(path)
        .map_err(|e| format!("Impossibile aprire {}: {}", path.display(), e))?
        .with_guessed_format()
        .map_err(|e| format!("Formato non riconosciuto per {}: {}", path.display(), e))?
        .decode()
        .map_err(|e| format!("Errore di decodifica per {}: {}", path.display(), e))?;

    let (orig_w, orig_h) = img.dimensions();

    // Downscale if larger than target viewport bounds (keeps high quality aspect ratio)
    let final_img = if orig_w > max_w || orig_h > max_h {
        img.thumbnail(max_w, max_h)
    } else {
        img
    };

    let size = [final_img.width() as usize, final_img.height() as usize];
    let rgba = final_img.to_rgba8();
    Ok(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

pub struct ImageLoader {
    request_tx: crossbeam_channel::Sender<(PathBuf, u32, u32)>,
    result_rx: crossbeam_channel::Receiver<(PathBuf, Result<ColorImage, String>)>,
}

impl ImageLoader {
    pub fn new(worker_count: usize, ctx: Option<Context>) -> Self {
        let (req_tx, req_rx) = crossbeam_channel::unbounded::<(PathBuf, u32, u32)>();
        let (res_tx, res_rx) = crossbeam_channel::unbounded::<(PathBuf, Result<ColorImage, String>)>();

        for _ in 0..worker_count {
            let rx = req_rx.clone();
            let tx = res_tx.clone();
            let ctx_clone = ctx.clone();
            std::thread::spawn(move || {
                while let Ok((path, max_w, max_h)) = rx.recv() {
                    let res = load_and_downscale(&path, max_w, max_h);
                    let _ = tx.send((path, res));
                    // Request GUI repaint immediately so texture is uploaded at 0ms latency
                    if let Some(ctx) = &ctx_clone {
                        ctx.request_repaint();
                    }
                }
            });
        }

        Self {
            request_tx: req_tx,
            result_rx: res_rx,
        }
    }

    pub fn request(&self, path: PathBuf, max_w: u32, max_h: u32) {
        let _ = self.request_tx.send((path, max_w, max_h));
    }

    pub fn try_recv(&self) -> Option<(PathBuf, Result<ColorImage, String>)> {
        self.result_rx.try_recv().ok()
    }
}
