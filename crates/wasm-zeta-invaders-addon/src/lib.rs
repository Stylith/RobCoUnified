use robcos_hosted_addon_contract::{
    HostedAddonFrame, HostedAddonInitRequest, HostedAddonResponse, HostedAddonSize,
    HostedAddonSurface, HostedAddonUpdateRequest,
};
use robcos_native_zeta_invaders_app::{
    input_from_hosted_events, SpaceInvadersConfig, SpaceInvadersGame,
};
use robcos_wasm_addon_sdk::{export_wasm_addon, WasmAddon};

struct ZetaInvadersAddon {
    game: SpaceInvadersGame,
    title: String,
    surface_label: String,
    size: HostedAddonSize,
}

impl Default for ZetaInvadersAddon {
    fn default() -> Self {
        Self {
            game: SpaceInvadersGame::new(SpaceInvadersConfig::default()),
            title: "Zeta Invaders".to_string(),
            surface_label: "DESKTOP".to_string(),
            size: HostedAddonSize {
                width: 224.0,
                height: 256.0,
            },
        }
    }
}

impl WasmAddon for ZetaInvadersAddon {
    fn initialize(&mut self, init: HostedAddonInitRequest) -> HostedAddonResponse {
        self.game.reset();
        self.surface_label = match init.surface {
            HostedAddonSurface::Desktop => "DESKTOP".to_string(),
            HostedAddonSurface::Terminal => "TERMINAL".to_string(),
        };
        self.size = init.size;
        HostedAddonResponse::Ready {
            title: self.title.clone(),
            frame: self.frame("WASM addon initialized."),
        }
    }

    fn update(&mut self, update: HostedAddonUpdateRequest) -> HostedAddonResponse {
        self.size = update.size;
        let input = input_from_hosted_events(&update.input);
        self.game.update(&input, update.delta_seconds);
        HostedAddonResponse::Frame {
            frame: self.frame(match self.surface_label.as_str() {
                "TERMINAL" => "TAB BACK   ARROWS/A D MOVE   SPACE FIRE   ENTER START",
                _ => "ARROWS/A D MOVE   SPACE FIRE   ENTER START   ESC/P PAUSE",
            }),
        }
    }
}

impl ZetaInvadersAddon {
    fn frame(&self, status: &str) -> HostedAddonFrame {
        let mut frame = self.game.hosted_frame();
        frame.status_line = Some(format!("{}   {}", self.surface_label, status));
        frame
    }
}

export_wasm_addon!(ZetaInvadersAddon);
