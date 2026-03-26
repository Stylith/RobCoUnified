use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostedAddonProtocol {
    ShellSurfaceV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostedAddonSurface {
    Desktop,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct HostedAddonSize {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostedPointerButton {
    Primary,
    Secondary,
    Middle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum HostedInputEvent {
    Key {
        key: String,
        pressed: bool,
    },
    Text {
        text: String,
    },
    PointerMove {
        x: f32,
        y: f32,
    },
    PointerButton {
        button: HostedPointerButton,
        pressed: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostedAddonInitRequest {
    pub addon_id: String,
    pub surface: HostedAddonSurface,
    pub size: HostedAddonSize,
    pub scale_factor: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_context: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostedAddonUpdateRequest {
    pub size: HostedAddonSize,
    pub delta_seconds: f32,
    #[serde(default)]
    pub input: Vec<HostedInputEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_context: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum HostedAddonRequest {
    Initialize(HostedAddonInitRequest),
    Update(HostedAddonUpdateRequest),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HostedTextAlign {
    #[default]
    LeftTop,
    LeftCenter,
    CenterTop,
    CenterCenter,
    CenterBottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HostedDrawCommand {
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        fill: HostedColor,
    },
    Text {
        x: f32,
        y: f32,
        text: String,
        color: HostedColor,
        size: f32,
        #[serde(default)]
        align: HostedTextAlign,
    },
    Image {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        asset_path: String,
        #[serde(default)]
        tint: Option<HostedColor>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostedAddonFrame {
    pub size: HostedAddonSize,
    #[serde(default)]
    pub clear: Option<HostedColor>,
    #[serde(default)]
    pub commands: Vec<HostedDrawCommand>,
    #[serde(default)]
    pub status_line: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum HostedAddonResponse {
    Ready {
        title: String,
        frame: HostedAddonFrame,
    },
    Frame {
        frame: HostedAddonFrame,
    },
    Exit {
        reason: String,
    },
    Error {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{
        HostedAddonFrame, HostedAddonInitRequest, HostedAddonProtocol, HostedAddonRequest,
        HostedAddonResponse, HostedAddonSize, HostedAddonSurface, HostedColor, HostedDrawCommand,
        HostedTextAlign,
    };

    #[test]
    fn hosted_addon_protocol_round_trips() {
        let encoded = serde_json::to_string(&HostedAddonProtocol::ShellSurfaceV1).unwrap();
        let decoded: HostedAddonProtocol = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, HostedAddonProtocol::ShellSurfaceV1);
    }

    #[test]
    fn hosted_addon_request_round_trips() {
        let request = HostedAddonRequest::Initialize(HostedAddonInitRequest {
            addon_id: "games.example-addon".to_string(),
            surface: HostedAddonSurface::Desktop,
            size: HostedAddonSize {
                width: 826.0,
                height: 700.0,
            },
            scale_factor: 1.25,
            host_context: None,
        });

        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: HostedAddonRequest = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, request);
    }

    #[test]
    fn hosted_addon_response_round_trips() {
        let response = HostedAddonResponse::Ready {
            title: "Red Menace".to_string(),
            frame: HostedAddonFrame {
                size: HostedAddonSize {
                    width: 826.0,
                    height: 700.0,
                },
                clear: Some(HostedColor {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                }),
                commands: vec![HostedDrawCommand::Text {
                    x: 24.0,
                    y: 40.0,
                    text: "READY".to_string(),
                    color: HostedColor {
                        r: 56,
                        g: 255,
                        b: 120,
                        a: 255,
                    },
                    size: 18.0,
                    align: HostedTextAlign::LeftTop,
                }],
                status_line: Some("Hosted addon ready.".to_string()),
            },
        };

        let encoded = serde_json::to_string(&response).unwrap();
        let decoded: HostedAddonResponse = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, response);
    }
}
