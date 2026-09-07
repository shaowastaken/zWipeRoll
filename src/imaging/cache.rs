use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use egui::{Context, TextureHandle, TextureOptions};
use super::decoder::ImageLoader;

pub struct TextureCache {
    loader: ImageLoader,
    textures: HashMap<PathBuf, TextureHandle>,
    access_order: VecDeque<PathBuf>,
    pending_requests: HashMap<PathBuf, bool>,
    capacity: usize,
}

impl TextureCache {
    pub fn new(capacity: usize, ctx: Option<Context>) -> Self {
        Self {
            loader: ImageLoader::new(3, ctx),
            textures: HashMap::new(),
            access_order: VecDeque::new(),
            pending_requests: HashMap::new(),
            capacity,
        }
    }

    pub fn poll_results(&mut self, ctx: &Context) {
        while let Some((path, result)) = self.loader.try_recv() {
            self.pending_requests.remove(&path);
            if let Ok(color_image) = result {
                let name = path.to_string_lossy().to_string();
                let texture = ctx.load_texture(&name, color_image, TextureOptions::LINEAR);
                self.insert(path, texture);
            }
        }
    }

    pub fn get(&mut self, path: &PathBuf) -> Option<&TextureHandle> {
        if self.textures.contains_key(path) {
            if let Some(pos) = self.access_order.iter().position(|p| p == path) {
                self.access_order.remove(pos);
            }
            self.access_order.push_back(path.clone());
            self.textures.get(path)
        } else {
            None
        }
    }

    pub fn request_if_needed(&mut self, path: PathBuf, max_w: u32, max_h: u32) {
        if !self.textures.contains_key(&path) && !self.pending_requests.contains_key(&path) {
            self.pending_requests.insert(path.clone(), true);
            self.loader.request(path, max_w, max_h);
        }
    }

    fn insert(&mut self, path: PathBuf, texture: TextureHandle) {
        if self.textures.len() >= self.capacity {
            if let Some(oldest) = self.access_order.pop_front() {
                self.textures.remove(&oldest);
            }
        }
        self.access_order.push_back(path.clone());
        self.textures.insert(path, texture);
    }

    pub fn clear(&mut self) {
        self.textures.clear();
        self.access_order.clear();
        self.pending_requests.clear();
    }
}
