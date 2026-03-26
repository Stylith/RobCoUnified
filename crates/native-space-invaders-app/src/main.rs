use anyhow::Result;
use eframe::{
    egui::{self, Color32, Context, Frame, IconData, ViewportBuilder},
    App,
};
use robcos_native_zeta_invaders_app::{
    input_from_ctx, AtlasTextures, SpaceInvadersConfig, SpaceInvadersGame,
};
use std::time::Instant;

const APP_ICON_BYTES: &[u8] = include_bytes!("../../../icon.png");
const APP_TITLE: &str = "Zeta Invaders";
const DEFAULT_WINDOW_SIZE: [f32; 2] = [820.0, 920.0];
const MIN_WINDOW_SIZE: [f32; 2] = [480.0, 560.0];

struct SpaceInvadersStandaloneApp {
    game: SpaceInvadersGame,
    atlas: Option<AtlasTextures>,
    last_frame: Instant,
}

impl SpaceInvadersStandaloneApp {
    fn new() -> Self {
        Self {
            game: SpaceInvadersGame::new(SpaceInvadersConfig::default()),
            atlas: None,
            last_frame: Instant::now(),
        }
    }
}

impl App for SpaceInvadersStandaloneApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::BLACK.to_normalized_gamma_f32()
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        if self.atlas.is_none() {
            self.atlas = Some(AtlasTextures::new(ctx));
        }

        let input = input_from_ctx(ctx);
        self.game.update(&input, dt);

        egui::CentralPanel::default()
            .frame(Frame::default().fill(Color32::BLACK))
            .show(ctx, |ui| {
                if let Some(atlas) = &self.atlas {
                    self.game.draw(ui, atlas);
                }
            });
    }
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

fn main() -> Result<()> {
    let mut viewport = ViewportBuilder::default()
        .with_title(APP_TITLE)
        .with_inner_size(DEFAULT_WINDOW_SIZE)
        .with_min_inner_size(MIN_WINDOW_SIZE);
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_zoom_factor(1.0);
            Ok(Box::new(SpaceInvadersStandaloneApp::new()))
        }),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))
}
