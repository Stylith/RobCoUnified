use anyhow::Result;
use eframe::egui::{IconData, ViewportBuilder};
use robcos::config::{get_settings, reload_settings, NativeStartupWindowMode};
use robcos::core::auth::ensure_default_admin;
use robcos::native::{configure_native_context, RobcoNativeApp};

const APP_ICON_BYTES: &[u8] = include_bytes!("../../../icon.png");

fn load_icon() -> Option<IconData> {
    let image = image::load_from_memory(APP_ICON_BYTES).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn parse_native_startup_window_mode_override(
    value: Option<&str>,
) -> Option<NativeStartupWindowMode> {
    match value.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "fullscreen" => Some(NativeStartupWindowMode::Fullscreen),
        Some(value) if matches!(value.as_str(), "windowed" | "safe") => {
            Some(NativeStartupWindowMode::Windowed)
        }
        Some(value)
            if matches!(
                value.as_str(),
                "borderless" | "borderless-fullscreen" | "borderless_fullscreen" | "desktop"
            ) =>
        {
            Some(NativeStartupWindowMode::BorderlessFullscreen)
        }
        Some(value)
            if matches!(
                value.as_str(),
                "maximized" | "maximized-window" | "maximized_window"
            ) =>
        {
            Some(NativeStartupWindowMode::Maximized)
        }
        Some(_) | None => None,
    }
}

fn resolve_native_startup_window_mode(
    configured_mode: NativeStartupWindowMode,
    override_value: Option<&str>,
) -> NativeStartupWindowMode {
    parse_native_startup_window_mode_override(override_value).unwrap_or(configured_mode)
}

fn build_startup_viewport(mode: NativeStartupWindowMode) -> ViewportBuilder {
    let viewport = ViewportBuilder::default()
        .with_inner_size([1360.0, 840.0])
        .with_min_inner_size([960.0, 600.0])
        .with_title("RobCoOS Native");
    match mode {
        NativeStartupWindowMode::Maximized => viewport
            .with_decorations(true)
            .with_fullscreen(false)
            .with_maximized(true),
        // Borderless fullscreen keeps the desktop feel without forcing exclusive fullscreen.
        NativeStartupWindowMode::BorderlessFullscreen => viewport
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
    let settings = get_settings();
    let mode = resolve_native_startup_window_mode(
        settings.native_startup_window_mode,
        std::env::var("ROBCOS_NATIVE_WINDOW_MODE").ok().as_deref(),
    );
    let mut viewport = build_startup_viewport(mode);
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
    use super::{parse_native_startup_window_mode_override, resolve_native_startup_window_mode};
    use robcos::config::NativeStartupWindowMode;

    #[test]
    fn parse_native_startup_window_mode_override_returns_none_for_missing_or_unknown() {
        assert_eq!(parse_native_startup_window_mode_override(None), None);
        assert_eq!(
            parse_native_startup_window_mode_override(Some("unknown")),
            None
        );
    }

    #[test]
    fn parse_native_startup_window_mode_override_supports_safe_borderless_and_fullscreen() {
        assert_eq!(
            parse_native_startup_window_mode_override(Some("windowed")),
            Some(NativeStartupWindowMode::Windowed)
        );
        assert_eq!(
            parse_native_startup_window_mode_override(Some("safe")),
            Some(NativeStartupWindowMode::Windowed)
        );
        assert_eq!(
            parse_native_startup_window_mode_override(Some("borderless")),
            Some(NativeStartupWindowMode::BorderlessFullscreen)
        );
        assert_eq!(
            parse_native_startup_window_mode_override(Some("desktop")),
            Some(NativeStartupWindowMode::BorderlessFullscreen)
        );
        assert_eq!(
            parse_native_startup_window_mode_override(Some("fullscreen")),
            Some(NativeStartupWindowMode::Fullscreen)
        );
    }

    #[test]
    fn resolve_native_startup_window_mode_prefers_env_override() {
        assert_eq!(
            resolve_native_startup_window_mode(
                NativeStartupWindowMode::Maximized,
                Some("windowed"),
            ),
            NativeStartupWindowMode::Windowed
        );
        assert_eq!(
            resolve_native_startup_window_mode(
                NativeStartupWindowMode::Windowed,
                Some("borderless_fullscreen"),
            ),
            NativeStartupWindowMode::BorderlessFullscreen
        );
        assert_eq!(
            resolve_native_startup_window_mode(
                NativeStartupWindowMode::Windowed,
                Some("fullscreen"),
            ),
            NativeStartupWindowMode::Fullscreen
        );
        assert_eq!(
            resolve_native_startup_window_mode(NativeStartupWindowMode::Windowed, None,),
            NativeStartupWindowMode::Windowed
        );
    }
}
