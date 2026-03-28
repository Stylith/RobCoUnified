use super::NucleonNativeApp;
use eframe::egui::{self, Color32, Context};

const STANDALONE_APPLICATIONS_DEFAULT_SIZE: [f32; 2] = [700.0, 480.0];
const STANDALONE_APPLICATIONS_MIN_SIZE: [f32; 2] = [320.0, 250.0];

pub struct NucleonNativeApplicationsApp {
    inner: NucleonNativeApp,
}

impl NucleonNativeApplicationsApp {
    pub fn new(session_username: Option<String>) -> Self {
        let mut inner = NucleonNativeApp::default();
        inner.prepare_standalone_applications_window(session_username);
        Self { inner }
    }

    pub fn default_window_size() -> [f32; 2] {
        STANDALONE_APPLICATIONS_DEFAULT_SIZE
    }

    pub fn min_window_size() -> [f32; 2] {
        STANDALONE_APPLICATIONS_MIN_SIZE
    }
}

impl eframe::App for NucleonNativeApplicationsApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from_rgb(0, 0, 0).to_normalized_gamma_f32()
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.inner.update_standalone_applications_window(ctx);
    }
}
