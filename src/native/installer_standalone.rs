use super::RobcoNativeApp;
use eframe::egui::{self, Color32, Context};

const STANDALONE_INSTALLER_DEFAULT_SIZE: [f32; 2] = [800.0, 600.0];
const STANDALONE_INSTALLER_MIN_SIZE: [f32; 2] = [500.0, 400.0];

pub struct RobcoNativeInstallerApp {
    inner: RobcoNativeApp,
}

impl RobcoNativeInstallerApp {
    pub fn new(session_username: Option<String>) -> Self {
        let mut inner = RobcoNativeApp::default();
        inner.prepare_standalone_installer_window(session_username);
        Self { inner }
    }

    pub fn default_window_size() -> [f32; 2] {
        STANDALONE_INSTALLER_DEFAULT_SIZE
    }

    pub fn min_window_size() -> [f32; 2] {
        STANDALONE_INSTALLER_MIN_SIZE
    }
}

impl eframe::App for RobcoNativeInstallerApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from_rgb(0, 0, 0).to_normalized_gamma_f32()
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.inner.update_standalone_installer_window(ctx);
    }
}
