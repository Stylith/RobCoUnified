use super::RobcoNativeApp;
use eframe::egui::{self, Color32, Context};
use std::path::PathBuf;

const STANDALONE_EDITOR_DEFAULT_SIZE: [f32; 2] = [820.0, 560.0];
const STANDALONE_EDITOR_MIN_SIZE: [f32; 2] = [400.0, 300.0];

pub struct RobcoNativeEditorApp {
    inner: RobcoNativeApp,
}

impl RobcoNativeEditorApp {
    pub fn new(session_username: Option<String>, start_path: Option<PathBuf>) -> Self {
        let mut inner = RobcoNativeApp::default();
        inner.prepare_standalone_editor_window(session_username, start_path);
        Self { inner }
    }

    pub fn default_window_size() -> [f32; 2] {
        STANDALONE_EDITOR_DEFAULT_SIZE
    }

    pub fn min_window_size() -> [f32; 2] {
        STANDALONE_EDITOR_MIN_SIZE
    }
}

impl eframe::App for RobcoNativeEditorApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from_rgb(0, 0, 0).to_normalized_gamma_f32()
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.inner.update_standalone_editor_window(ctx);
    }
}
