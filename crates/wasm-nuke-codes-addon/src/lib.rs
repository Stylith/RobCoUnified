use robcos_hosted_addon_contract::{
    HostedAddonFrame, HostedAddonInitRequest, HostedAddonResponse, HostedAddonSize,
    HostedAddonUpdateRequest, HostedColor, HostedDrawCommand,
};
use robcos_wasm_addon_sdk::{export_wasm_addon, WasmAddon};

#[derive(Default)]
struct NukeCodesAddon {
    title: String,
    surface_label: String,
    size: HostedAddonSize,
}

impl WasmAddon for NukeCodesAddon {
    fn initialize(&mut self, init: HostedAddonInitRequest) -> HostedAddonResponse {
        self.title = "Nuke Codes".to_string();
        self.surface_label = match init.surface {
            robcos_hosted_addon_contract::HostedAddonSurface::Desktop => "DESKTOP".to_string(),
            robcos_hosted_addon_contract::HostedAddonSurface::Terminal => "TERMINAL".to_string(),
        };
        self.size = init.size;
        HostedAddonResponse::Ready {
            title: self.title.clone(),
            frame: self.render_frame("WASM addon loaded."),
        }
    }

    fn update(&mut self, update: HostedAddonUpdateRequest) -> HostedAddonResponse {
        self.size = update.size;
        HostedAddonResponse::Frame {
            frame: self.render_frame("Awaiting provider host bridge."),
        }
    }
}

impl NukeCodesAddon {
    fn render_frame(&self, status: &str) -> HostedAddonFrame {
        HostedAddonFrame {
            size: self.size,
            clear: Some(color(8, 16, 8)),
            commands: vec![
                HostedDrawCommand::Text {
                    x: 20.0,
                    y: 28.0,
                    text: self.title.clone(),
                    color: color(120, 255, 120),
                    size: 22.0,
                },
                HostedDrawCommand::Text {
                    x: 20.0,
                    y: 58.0,
                    text: format!("SURFACE: {}", self.surface_label),
                    color: color(96, 208, 96),
                    size: 14.0,
                },
                HostedDrawCommand::Rect {
                    x: 20.0,
                    y: 74.0,
                    width: (self.size.width - 40.0).max(80.0),
                    height: 1.0,
                    fill: color(64, 160, 64),
                },
                HostedDrawCommand::Text {
                    x: 20.0,
                    y: 104.0,
                    text: "This addon is now hosted from an external WASM bundle.".to_string(),
                    color: color(120, 255, 120),
                    size: 16.0,
                },
                HostedDrawCommand::Text {
                    x: 20.0,
                    y: 132.0,
                    text: "Next step: expose provider/data access through the shell host.".to_string(),
                    color: color(96, 208, 96),
                    size: 14.0,
                },
            ],
            status_line: Some(status.to_string()),
        }
    }
}

fn color(r: u8, g: u8, b: u8) -> HostedColor {
    HostedColor { r, g, b, a: 255 }
}

export_wasm_addon!(NukeCodesAddon);
