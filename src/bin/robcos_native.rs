use anyhow::Result;
use eframe::egui::{IconData, ViewportBuilder};
use robcos::native::{configure_native_context, RobcoNativeApp};

fn load_icon() -> Option<IconData> {
    let bytes = std::fs::read("icon.png").ok()?;
    let image = image::load_from_memory(&bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn main() -> Result<()> {
    let mut viewport = ViewportBuilder::default()
        .with_inner_size([1360.0, 840.0])
        .with_min_inner_size([960.0, 600.0])
        .with_decorations(false)
        .with_fullscreen(true)
        .with_title("RobCoOS Native");
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
