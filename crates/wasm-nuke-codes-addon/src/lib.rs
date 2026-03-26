use robcos_hosted_addon_contract::{
    HostedAddonFrame, HostedAddonInitRequest, HostedAddonResponse, HostedAddonSize,
    HostedAddonUpdateRequest, HostedColor, HostedDrawCommand, HostedTextAlign,
};
use robcos_wasm_addon_sdk::{export_wasm_addon, WasmAddon};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NukeCodesData {
    alpha: String,
    bravo: String,
    charlie: String,
    source: String,
    fetched_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
enum NukeCodesView {
    #[default]
    Unloaded,
    Data(NukeCodesData),
    Error(String),
}

#[derive(Default)]
struct NukeCodesAddon {
    title: String,
    surface_label: String,
    size: HostedAddonSize,
    view: NukeCodesView,
}

impl WasmAddon for NukeCodesAddon {
    fn initialize(&mut self, init: HostedAddonInitRequest) -> HostedAddonResponse {
        self.title = "Nuke Codes".to_string();
        self.surface_label = match init.surface {
            robcos_hosted_addon_contract::HostedAddonSurface::Desktop => "DESKTOP".to_string(),
            robcos_hosted_addon_contract::HostedAddonSurface::Terminal => "TERMINAL".to_string(),
        };
        self.size = init.size;
        self.view = decode_view(init.host_context);
        HostedAddonResponse::Ready {
            title: self.title.clone(),
            frame: self.render_frame("R refresh   ESC back"),
        }
    }

    fn update(&mut self, update: HostedAddonUpdateRequest) -> HostedAddonResponse {
        self.size = update.size;
        if let Some(context) = update.host_context {
            self.view = decode_view(Some(context));
        }
        HostedAddonResponse::Frame {
            frame: self.render_frame("R refresh   ESC back"),
        }
    }
}

impl NukeCodesAddon {
    fn render_frame(&self, status: &str) -> HostedAddonFrame {
        let mut commands = vec![
                HostedDrawCommand::Text {
                    x: 20.0,
                    y: 28.0,
                    text: self.title.clone(),
                    color: color(120, 255, 120),
                    size: 22.0,
                    align: HostedTextAlign::LeftTop,
                },
                HostedDrawCommand::Text {
                    x: 20.0,
                    y: 58.0,
                    text: format!("SURFACE: {}", self.surface_label),
                    color: color(96, 208, 96),
                    size: 14.0,
                    align: HostedTextAlign::LeftTop,
                },
            HostedDrawCommand::Rect {
                x: 20.0,
                y: 74.0,
                width: (self.size.width - 40.0).max(80.0),
                height: 1.0,
                fill: color(64, 160, 64),
            },
        ];

        match &self.view {
            NukeCodesView::Unloaded => {
                commands.push(HostedDrawCommand::Text {
                    x: 20.0,
                    y: 104.0,
                    text: "No host data loaded yet.".to_string(),
                    color: color(120, 255, 120),
                    size: 16.0,
                    align: HostedTextAlign::LeftTop,
                });
            }
            NukeCodesView::Data(data) => {
                commands.extend([
                    HostedDrawCommand::Text {
                        x: 20.0,
                        y: 106.0,
                        text: format!("ALPHA   {}", data.alpha),
                        color: color(120, 255, 120),
                        size: 18.0,
                        align: HostedTextAlign::LeftTop,
                    },
                    HostedDrawCommand::Text {
                        x: 20.0,
                        y: 138.0,
                        text: format!("BRAVO   {}", data.bravo),
                        color: color(120, 255, 120),
                        size: 18.0,
                        align: HostedTextAlign::LeftTop,
                    },
                    HostedDrawCommand::Text {
                        x: 20.0,
                        y: 170.0,
                        text: format!("CHARLIE {}", data.charlie),
                        color: color(120, 255, 120),
                        size: 18.0,
                        align: HostedTextAlign::LeftTop,
                    },
                    HostedDrawCommand::Text {
                        x: 20.0,
                        y: 220.0,
                        text: format!("SOURCE: {}", data.source),
                        color: color(96, 208, 96),
                        size: 14.0,
                        align: HostedTextAlign::LeftTop,
                    },
                    HostedDrawCommand::Text {
                        x: 20.0,
                        y: 246.0,
                        text: format!("FETCHED: {}", data.fetched_at),
                        color: color(96, 208, 96),
                        size: 14.0,
                        align: HostedTextAlign::LeftTop,
                    },
                ]);
            }
            NukeCodesView::Error(message) => {
                commands.push(HostedDrawCommand::Text {
                    x: 20.0,
                    y: 104.0,
                    text: "Failed to fetch launch codes.".to_string(),
                    color: color(120, 255, 120),
                    size: 16.0,
                    align: HostedTextAlign::LeftTop,
                });
                commands.push(HostedDrawCommand::Text {
                    x: 20.0,
                    y: 136.0,
                    text: message.clone(),
                    color: color(255, 160, 120),
                    size: 14.0,
                    align: HostedTextAlign::LeftTop,
                });
            }
        }

        HostedAddonFrame {
            size: self.size,
            clear: Some(color(8, 16, 8)),
            commands,
            status_line: Some(status.to_string()),
        }
    }
}

fn decode_view(context: Option<Value>) -> NukeCodesView {
    context
        .and_then(|value| serde_json::from_value::<NukeCodesView>(value).ok())
        .unwrap_or_default()
}

fn color(r: u8, g: u8, b: u8) -> HostedColor {
    HostedColor { r, g, b, a: 255 }
}

export_wasm_addon!(NukeCodesAddon);
