use anyhow::Result;
use eframe::egui::{IconData, ViewportBuilder};
use nucleon::config::{reload_settings, set_current_user};
use nucleon::core::auth::ensure_default_admin;
use nucleon::native::{
    configure_native_context, desktop_session_service::restore_current_user_from_last_session,
    standalone_env_value, NucleonNativeEditorApp,
};
use std::path::PathBuf;

const APP_ICON_BYTES: &[u8] = include_bytes!("../../../icon.png");
const APP_TITLE: &str = "Nucleon Text Editor";

fn load_icon() -> Option<IconData> {
    let image = image::load_from_memory(APP_ICON_BYTES).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn bind_launch_user() -> Option<String> {
    let session_username = standalone_env_value();
    if let Some(username) = session_username.as_deref() {
        set_current_user(Some(username));
    } else {
        restore_current_user_from_last_session();
    }
    session_username
}

fn start_path_from_args() -> Option<PathBuf> {
    std::env::args_os().nth(1).map(PathBuf::from)
}

fn main() -> Result<()> {
    ensure_default_admin();
    let session_username = bind_launch_user();
    reload_settings();

    let mut viewport = ViewportBuilder::default()
        .with_title(APP_TITLE)
        .with_inner_size(NucleonNativeEditorApp::default_window_size())
        .with_min_inner_size(NucleonNativeEditorApp::min_window_size());
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
            Ok(Box::new(NucleonNativeEditorApp::new(
                session_username.clone(),
                start_path.clone(),
            )))
        }),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))
}
