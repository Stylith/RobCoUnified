use anyhow::Result;
use eframe::egui::{IconData, ViewportBuilder};
use robcos::config::reload_settings;
use robcos::core::auth::ensure_default_admin;
use robcos::native::{configure_native_context, RobcoNativeFileManagerApp};
use std::path::PathBuf;

const APP_ICON_BYTES: &[u8] = include_bytes!("../../../icon.png");
const APP_TITLE: &str = "My Computer";

fn load_icon() -> Option<IconData> {
    let image = image::load_from_memory(APP_ICON_BYTES).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn start_path_from_args() -> Option<PathBuf> {
    std::env::args_os().nth(1).map(PathBuf::from)
}

fn main() -> Result<()> {
    ensure_default_admin();
    reload_settings();

    let mut viewport = ViewportBuilder::default()
        .with_title(APP_TITLE)
        .with_inner_size(RobcoNativeFileManagerApp::default_window_size())
        .with_min_inner_size(RobcoNativeFileManagerApp::min_window_size());
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let start_path = start_path_from_args();

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_zoom_factor(1.0);
            configure_native_context(&cc.egui_ctx);
            Ok(Box::new(RobcoNativeFileManagerApp::new(start_path.clone())))
        }),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))
}
