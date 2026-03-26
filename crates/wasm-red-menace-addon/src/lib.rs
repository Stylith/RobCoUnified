use robcos_hosted_addon_contract::{
    HostedAddonInitRequest, HostedAddonResponse, HostedAddonSize, HostedAddonSurface,
    HostedAddonUpdateRequest,
};
use robcos_native_red_menace_app::{
    input_from_hosted_events, RedMenaceConfig, RedMenaceGame,
};
use robcos_wasm_addon_sdk::{export_wasm_addon, WasmAddon};

struct RedMenaceAddon {
    game: RedMenaceGame,
    title: String,
    surface_label: String,
    size: HostedAddonSize,
}

impl Default for RedMenaceAddon {
    fn default() -> Self {
        Self {
            game: RedMenaceGame::new(RedMenaceConfig::default()),
            title: "Red Menace".to_string(),
            surface_label: "DESKTOP".to_string(),
            size: HostedAddonSize {
                width: 826.0,
                height: 700.0,
            },
        }
    }
}

impl WasmAddon for RedMenaceAddon {
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
                "TERMINAL" => "TAB BACK   ARROWS/WASD MOVE   SPACE ACTION   ENTER START",
                _ => "ARROWS/WASD MOVE   SPACE ACTION   ENTER START",
            }),
        }
    }
}

impl RedMenaceAddon {
    fn frame(&self, status: &str) -> robcos_hosted_addon_contract::HostedAddonFrame {
        let mut frame = self.game.hosted_frame();
        frame.status_line = Some(format!("{}   {}", self.surface_label, status));
        frame
    }
}

export_wasm_addon!(RedMenaceAddon);
