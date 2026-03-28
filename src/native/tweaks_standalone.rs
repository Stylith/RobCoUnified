use super::NucleonNativeApp;
use eframe::egui::{self, Color32, Context};

const STANDALONE_TWEAKS_DEFAULT_SIZE: [f32; 2] = [820.0, 560.0];

pub struct NucleonNativeTweaksApp {
    inner: NucleonNativeApp,
}

impl NucleonNativeTweaksApp {
    pub fn new(session_username: Option<String>) -> Self {
        let mut inner = NucleonNativeApp::default();
        inner.prepare_standalone_tweaks_window(session_username);
        Self { inner }
    }

    pub fn default_window_size() -> [f32; 2] {
        STANDALONE_TWEAKS_DEFAULT_SIZE
    }

    pub fn min_window_size() -> [f32; 2] {
        STANDALONE_TWEAKS_DEFAULT_SIZE
    }
}

impl eframe::App for NucleonNativeTweaksApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from_rgb(0, 0, 0).to_normalized_gamma_f32()
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.inner.update_standalone_tweaks_window(ctx);
    }
}
