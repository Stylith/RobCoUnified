use anyhow::Result;
use eframe::egui::{IconData, ViewportBuilder};
use robcos::config::reload_settings;
use robcos::core::auth::ensure_default_admin;
use robcos::native::{configure_native_context, RobcoNativeApp};

const APP_ICON_BYTES: &[u8] = include_bytes!("../../../icon.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeStartupWindowMode {
    Maximized,
    Windowed,
    Fullscreen,
}

fn load_icon() -> Option<IconData> {
    let image = image::load_from_memory(APP_ICON_BYTES).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn parse_native_startup_window_mode(value: Option<&str>) -> NativeStartupWindowMode {
    match value.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "fullscreen" => NativeStartupWindowMode::Fullscreen,
        Some(value) if matches!(value.as_str(), "windowed" | "safe") => {
            NativeStartupWindowMode::Windowed
        }
        Some(value) if matches!(value.as_str(), "maximized" | "desktop") => {
            NativeStartupWindowMode::Maximized
        }
        Some(_) | None => NativeStartupWindowMode::Maximized,
    }
}

fn build_startup_viewport() -> ViewportBuilder {
    let mode = parse_native_startup_window_mode(
        std::env::var("ROBCOS_NATIVE_WINDOW_MODE").ok().as_deref(),
    );
    let viewport = ViewportBuilder::default()
        .with_inner_size([1360.0, 840.0])
        .with_min_inner_size([960.0, 600.0])
        .with_title("RobCoOS Native");
    match mode {
        // Default path: keeps the desktop feel without forcing exclusive fullscreen.
        NativeStartupWindowMode::Maximized => viewport
            .with_decorations(false)
            .with_fullscreen(false)
            .with_maximized(true),
        NativeStartupWindowMode::Windowed => viewport
            .with_decorations(true)
            .with_fullscreen(false)
            .with_maximized(false),
        NativeStartupWindowMode::Fullscreen => viewport
            .with_decorations(false)
            .with_fullscreen(true)
            .with_maximized(false),
    }
}

fn main() -> Result<()> {
    ensure_default_admin();
    reload_settings();
    let mut viewport = build_startup_viewport();
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "RobCoOS Native",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_zoom_factor(1.0);
            configure_native_context(&cc.egui_ctx);
            Ok(Box::new(RobcoNativeApp::default()))
        }),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{parse_native_startup_window_mode, NativeStartupWindowMode};

    #[test]
    fn parse_native_startup_window_mode_defaults_to_maximized() {
        assert_eq!(
            parse_native_startup_window_mode(None),
            NativeStartupWindowMode::Maximized
        );
        assert_eq!(
            parse_native_startup_window_mode(Some("unknown")),
            NativeStartupWindowMode::Maximized
        );
    }

    #[test]
    fn parse_native_startup_window_mode_supports_safe_and_fullscreen() {
        assert_eq!(
            parse_native_startup_window_mode(Some("windowed")),
            NativeStartupWindowMode::Windowed
        );
        assert_eq!(
            parse_native_startup_window_mode(Some("safe")),
            NativeStartupWindowMode::Windowed
        );
        assert_eq!(
            parse_native_startup_window_mode(Some("fullscreen")),
            NativeStartupWindowMode::Fullscreen
        );
    }
}
