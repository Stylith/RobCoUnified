use crate::native::InstalledWasmAddonModule;
use crate::platform::{
    HostedAddonFrame, HostedAddonInitRequest, HostedAddonRequest, HostedAddonResponse,
    HostedAddonSize, HostedAddonSurface, HostedAddonUpdateRequest, HostedColor, HostedDrawCommand,
    HostedInputEvent, HostedTextAlign,
};
use chrono::Local;
use eframe::egui::{self, Align2, Context, FontFamily, FontId, Key, Sense, TextureHandle, Ui};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use wasmi::{Engine, Linker, Memory, Module, Store, TypedFunc};

#[allow(dead_code)]
pub(crate) struct WasmAddonModuleSession {
    store: Store<()>,
    memory: Memory,
    alloc: TypedFunc<i32, i32>,
    handle_json: TypedFunc<(i32, i32), i64>,
}

pub(crate) struct WasmHostedAddonState {
    session: WasmAddonModuleSession,
    title: String,
    frame: HostedAddonFrame,
    bundle_dir: PathBuf,
    host_context: Option<Value>,
    textures: HashMap<String, TextureHandle>,
}

#[allow(dead_code)]
impl WasmAddonModuleSession {
    pub(crate) fn spawn(
        module: &InstalledWasmAddonModule,
        init: &HostedAddonRequest,
    ) -> Result<(Self, HostedAddonResponse), String> {
        let engine = Engine::default();
        let wasm_bytes = std::fs::read(&module.module_path).map_err(|error| {
            format!(
                "Failed to read WASM addon '{}' from '{}': {error}",
                module.addon_id,
                module.module_path.display()
            )
        })?;
        let compiled = Module::new(&engine, &wasm_bytes).map_err(|error| {
            format!(
                "Failed to compile WASM addon '{}' from '{}': {error}",
                module.addon_id,
                module.module_path.display()
            )
        })?;
        let linker = Linker::<()>::new(&engine);
        let mut store = Store::new(&engine, ());
        let instance = linker
            .instantiate(&mut store, &compiled)
            .and_then(|pre| pre.start(&mut store))
            .map_err(|error| format_wasm_instantiate_error(module, &error.to_string()))?;
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| "WASM addon did not export memory.".to_string())?;
        let alloc = instance
            .get_typed_func::<i32, i32>(&store, "nd_alloc")
            .map_err(|error| format!("WASM addon is missing nd_alloc: {error}"))?;
        let handle_json = instance
            .get_typed_func::<(i32, i32), i64>(&store, "nd_handle_json")
            .map_err(|error| format!("WASM addon is missing nd_handle_json: {error}"))?;

        let mut session = Self {
            store,
            memory,
            alloc,
            handle_json,
        };
        let response = session.request(init)?;
        Ok((session, response))
    }

    pub(crate) fn request(
        &mut self,
        request: &HostedAddonRequest,
    ) -> Result<HostedAddonResponse, String> {
        let encoded = serde_json::to_vec(request)
            .map_err(|error| format!("Failed to encode WASM addon request: {error}"))?;
        let ptr = self
            .alloc
            .call(
                &mut self.store,
                i32::try_from(encoded.len())
                    .map_err(|_| "WASM addon request exceeded i32 length.".to_string())?,
            )
            .map_err(|error| format!("WASM addon allocation failed: {error}"))?;
        let offset = usize::try_from(ptr)
            .map_err(|_| "WASM addon returned an invalid negative pointer.".to_string())?;
        self.memory
            .write(&mut self.store, offset, &encoded)
            .map_err(|error| format!("Failed to write WASM addon request: {error}"))?;

        let packed = self
            .handle_json
            .call(
                &mut self.store,
                (
                    ptr,
                    i32::try_from(encoded.len())
                        .map_err(|_| "WASM addon request exceeded i32 length.".to_string())?,
                ),
            )
            .map_err(|error| format!("WASM addon request handler failed: {error}"))?;
        let (response_ptr, response_len) = unpack_ptr_len(packed)?;
        let mut response_bytes = vec![0_u8; response_len];
        self.memory
            .read(&self.store, response_ptr, &mut response_bytes)
            .map_err(|error| format!("Failed to read WASM addon response: {error}"))?;
        serde_json::from_slice(&response_bytes)
            .map_err(|error| format!("Failed to decode WASM addon response: {error}"))
    }
}

fn format_wasm_instantiate_error(module: &InstalledWasmAddonModule, error: &str) -> String {
    if error.contains("__wbindgen_placeholder__") {
        return format!(
            "WASM addon '{}' from '{}' expects wasm-bindgen/web imports and is not compatible with the shell addon host.",
            module.addon_id,
            module.module_path.display()
        );
    }
    format!(
        "Failed to instantiate WASM addon '{}' from '{}': {error}",
        module.addon_id,
        module.module_path.display()
    )
}

impl WasmHostedAddonState {
    pub(crate) fn spawn(
        module: &InstalledWasmAddonModule,
        surface: HostedAddonSurface,
        size: HostedAddonSize,
    ) -> Result<Self, String> {
        let init = HostedAddonRequest::Initialize(HostedAddonInitRequest {
            addon_id: module.addon_id.to_string(),
            surface,
            size,
            scale_factor: 1.0,
            host_context: initial_host_context(&module.bundle_dir),
        });
        let (session, response) = WasmAddonModuleSession::spawn(module, &init)?;
        match response {
            HostedAddonResponse::Ready { title, frame } => Ok(Self {
                session,
                title,
                frame,
                bundle_dir: module.bundle_dir.clone(),
                host_context: hosted_request_context(&init),
                textures: HashMap::new(),
            }),
            HostedAddonResponse::Frame { frame } => Ok(Self {
                session,
                title: module.addon_id.to_string(),
                frame,
                bundle_dir: module.bundle_dir.clone(),
                host_context: hosted_request_context(&init),
                textures: HashMap::new(),
            }),
            HostedAddonResponse::Exit { reason } => Err(format!(
                "WASM addon '{}' exited during initialization: {reason}",
                module.addon_id
            )),
            HostedAddonResponse::Error { message } => Err(format!(
                "WASM addon '{}' initialization failed: {message}",
                module.addon_id
            )),
        }
    }

    pub(crate) fn update(
        &mut self,
        size: HostedAddonSize,
        delta_seconds: f32,
        input: Vec<HostedInputEvent>,
    ) -> Result<(), String> {
        if should_refresh_host_context(&input) || self.host_context.is_none() {
            self.host_context = initial_host_context(&self.bundle_dir);
        }
        match self
            .session
            .request(&HostedAddonRequest::Update(HostedAddonUpdateRequest {
                size,
                delta_seconds,
                input,
                host_context: self.host_context.clone(),
            }))? {
            HostedAddonResponse::Ready { title, frame } => {
                self.title = title;
                self.frame = frame;
                Ok(())
            }
            HostedAddonResponse::Frame { frame } => {
                self.frame = frame;
                Ok(())
            }
            HostedAddonResponse::Exit { reason } => Err(format!("WASM addon exited: {reason}")),
            HostedAddonResponse::Error { message } => Err(message),
        }
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn draw(&mut self, ui: &mut Ui) {
        let frame = self.frame.clone();
        let available = ui.available_size_before_wrap();
        let desired = egui::vec2(
            available.x.max(frame.size.width.max(1.0)),
            available.y.max(frame.size.height.max(1.0)),
        );
        let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
        let painter = ui.painter_at(rect);
        if let Some(clear) = &frame.clear {
            painter.rect_filled(rect, 0.0, hosted_color(clear));
        }

        let scale_x = if frame.size.width > 0.0 {
            rect.width() / frame.size.width
        } else {
            1.0
        };
        let scale_y = if frame.size.height > 0.0 {
            rect.height() / frame.size.height
        } else {
            1.0
        };
        let scale = scale_x.min(scale_y).max(0.0001);
        let content_size = egui::vec2(frame.size.width * scale, frame.size.height * scale);
        let content_rect = egui::Rect::from_center_size(rect.center(), content_size);

        for command in &frame.commands {
            match command {
                HostedDrawCommand::Rect {
                    x,
                    y,
                    width,
                    height,
                    fill,
                } => {
                    let min = egui::pos2(
                        content_rect.left() + (*x * scale),
                        content_rect.top() + (*y * scale),
                    );
                    let size = egui::vec2(width * scale, height * scale);
                    painter.rect_filled(
                        egui::Rect::from_min_size(min, size),
                        0.0,
                        hosted_color(fill),
                    );
                }
                HostedDrawCommand::Text {
                    x,
                    y,
                    text,
                    color,
                    size,
                    align,
                } => {
                    let pos = egui::pos2(
                        content_rect.left() + (*x * scale),
                        content_rect.top() + (*y * scale),
                    );
                    painter.text(
                        pos,
                        hosted_align(*align),
                        text,
                        FontId::new((size * scale).max(10.0), FontFamily::Monospace),
                        hosted_color(color),
                    );
                }
                HostedDrawCommand::Image {
                    x,
                    y,
                    width,
                    height,
                    asset_path,
                    tint,
                } => {
                    let min = egui::pos2(
                        content_rect.left() + (*x * scale),
                        content_rect.top() + (*y * scale),
                    );
                    let size = egui::vec2(width * scale, height * scale);
                    let image_rect = egui::Rect::from_min_size(min, size);
                    if let Some(texture) = self.load_texture(ui.ctx(), asset_path) {
                        let uv =
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                        let mut mesh = egui::epaint::Mesh::with_texture(texture.id());
                        mesh.add_rect_with_uv(
                            image_rect,
                            uv,
                            tint.as_ref()
                                .map(hosted_color)
                                .unwrap_or(egui::Color32::WHITE),
                        );
                        painter.add(egui::Shape::mesh(mesh));
                    } else {
                        painter.rect_stroke(
                            image_rect,
                            0.0,
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(64, 160, 64)),
                        );
                        painter.text(
                            image_rect.left_top() + egui::vec2(6.0, 6.0),
                            Align2::LEFT_TOP,
                            format!("IMAGE {}", asset_path),
                            FontId::new(10.0, FontFamily::Monospace),
                            egui::Color32::from_rgb(96, 208, 96),
                        );
                    }
                }
            }
        }
    }

    fn load_texture(&mut self, ctx: &Context, asset_path: &str) -> Option<&TextureHandle> {
        if self.textures.contains_key(asset_path) {
            return self.textures.get(asset_path);
        }

        let absolute = resolve_bundle_asset_path(&self.bundle_dir, asset_path)?;
        let bytes = std::fs::read(&absolute).ok()?;
        let image = image::load_from_memory(&bytes).ok()?.into_rgba8();
        let (width, height) = image.dimensions();
        let texture = ctx.load_texture(
            format!(
                "hosted_addon_asset_{}_{}",
                self.title.replace(' ', "_"),
                asset_path.replace('/', "_")
            ),
            egui::ColorImage::from_rgba_unmultiplied(
                [width as usize, height as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::NEAREST,
        );
        self.textures.insert(asset_path.to_string(), texture);
        self.textures.get(asset_path)
    }
}

pub(crate) fn draw_hosted_addon_frame(ui: &mut Ui, state: &mut WasmHostedAddonState) {
    state.draw(ui);
}

pub(crate) fn collect_hosted_keyboard_input(ctx: &Context, active: bool) -> Vec<HostedInputEvent> {
    if !active {
        return Vec::new();
    }
    let held_keys = [
        (Key::ArrowLeft, "arrow-left"),
        (Key::ArrowRight, "arrow-right"),
        (Key::ArrowUp, "arrow-up"),
        (Key::ArrowDown, "arrow-down"),
        (Key::A, "a"),
        (Key::D, "d"),
        (Key::W, "w"),
        (Key::S, "s"),
        (Key::Space, "space"),
    ];
    let pressed_keys = [
        (Key::Enter, "enter"),
        (Key::Escape, "escape"),
        (Key::P, "p"),
        (Key::R, "r"),
    ];
    let mut events = Vec::new();
    for (key, label) in held_keys {
        if ctx.input(|i| i.key_down(key)) {
            events.push(HostedInputEvent::Key {
                key: label.to_string(),
                pressed: true,
            });
        }
    }
    for (key, label) in pressed_keys {
        if ctx.input(|i| i.key_pressed(key)) {
            events.push(HostedInputEvent::Key {
                key: label.to_string(),
                pressed: true,
            });
        }
    }
    events
}

fn hosted_color(color: &HostedColor) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(color.r, color.g, color.b, color.a)
}

fn hosted_align(align: HostedTextAlign) -> Align2 {
    match align {
        HostedTextAlign::LeftTop => Align2::LEFT_TOP,
        HostedTextAlign::LeftCenter => Align2::LEFT_CENTER,
        HostedTextAlign::CenterTop => Align2::CENTER_TOP,
        HostedTextAlign::CenterCenter => Align2::CENTER_CENTER,
        HostedTextAlign::CenterBottom => Align2::CENTER_BOTTOM,
    }
}

fn resolve_bundle_asset_path(bundle_dir: &Path, asset_path: &str) -> Option<PathBuf> {
    let relative = Path::new(asset_path);
    if relative.is_absolute() {
        return None;
    }
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }
    Some(bundle_dir.join(relative))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HostUrlProvider {
    source: String,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeReferenceData {
    alpha: String,
    bravo: String,
    charlie: String,
    source: String,
    fetched_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
enum CodeReferenceView {
    #[default]
    Unloaded,
    Data(CodeReferenceData),
    Error(String),
}

fn initial_host_context(bundle_dir: &Path) -> Option<Value> {
    if let Some(spec_path) = resolve_bundle_asset_path(bundle_dir, "host-context.json") {
        if let Ok(raw) = std::fs::read_to_string(spec_path) {
            if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                return Some(value);
            }
        }
    }

    let providers_path = resolve_bundle_asset_path(bundle_dir, "providers.json")?;
    let providers = load_host_url_providers(&providers_path).ok()?;
    serde_json::to_value(fetch_code_reference_with_providers(&providers)).ok()
}

fn hosted_request_context(request: &HostedAddonRequest) -> Option<Value> {
    match request {
        HostedAddonRequest::Initialize(init) => init.host_context.clone(),
        HostedAddonRequest::Update(update) => update.host_context.clone(),
        HostedAddonRequest::Shutdown => None,
    }
}

fn should_refresh_host_context(input: &[HostedInputEvent]) -> bool {
    input.iter().any(|event| {
        matches!(
            event,
            HostedInputEvent::Key { key, pressed } if *pressed && key == "r"
        )
    })
}

fn load_host_url_providers(path: &Path) -> Result<Vec<HostUrlProvider>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read host provider file: {error}"))?;
    serde_json::from_str::<Vec<HostUrlProvider>>(&raw)
        .map_err(|error| format!("failed to parse host provider file: {error}"))
}

fn fetch_code_reference_with_providers(providers: &[HostUrlProvider]) -> CodeReferenceView {
    let mut last_error = "no provider attempts".to_string();
    for provider in providers {
        match fetch_html(&provider.url)
            .and_then(|html| extract_codes(&html).map(|(a, b, c)| (a, b, c)))
        {
            Ok((alpha, bravo, charlie)) => {
                return CodeReferenceView::Data(CodeReferenceData {
                    alpha,
                    bravo,
                    charlie,
                    source: provider.source.clone(),
                    fetched_at: Local::now().format("%Y-%m-%d %I:%M %p").to_string(),
                });
            }
            Err(err) => {
                last_error = format!("{}: {err}", provider.source);
            }
        }
    }
    CodeReferenceView::Error(last_error)
}

fn fetch_html(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-fsSL", "--connect-timeout", "8", "--max-time", "16", url])
        .output()
        .map_err(|error| format!("curl spawn failed: {error}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl failed: {}", err.trim()));
    }

    String::from_utf8(output.stdout).map_err(|error| format!("invalid utf8: {error}"))
}

fn extract_codes(html: &str) -> Result<(String, String, String), String> {
    let alpha = extract_code_for(html, &["alpha", "site alpha", "silo alpha"]);
    let bravo = extract_code_for(html, &["bravo", "site bravo", "silo bravo"]);
    let charlie = extract_code_for(html, &["charlie", "site charlie", "silo charlie"]);

    match (alpha, bravo, charlie) {
        (Some(a), Some(b), Some(c)) => Ok((a, b, c)),
        _ => Err("could not parse alpha/bravo/charlie codes".to_string()),
    }
}

fn extract_code_for(html: &str, labels: &[&str]) -> Option<String> {
    let lower = html.to_lowercase();
    labels
        .iter()
        .find_map(|label| {
            let mut start = 0usize;
            while let Some(pos) = lower[start..].find(label) {
                let abs = start + pos;
                let left = abs.saturating_sub(120);
                let right = (abs + 220).min(html.len());
                if let Some(code) = first_eight_digit_code(&html[left..right]) {
                    return Some(code);
                }
                start = abs + label.len();
            }
            None
        })
        .or_else(|| first_eight_digit_code(html))
}

fn first_eight_digit_code(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    if bytes.len() < 8 {
        return None;
    }
    for i in 0..=(bytes.len() - 8) {
        let window = &bytes[i..i + 8];
        if !window.iter().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let prev_ok = i == 0 || !bytes[i - 1].is_ascii_digit();
        let next_ok = i + 8 == bytes.len() || !bytes[i + 8].is_ascii_digit();
        if prev_ok && next_ok {
            return Some(String::from_utf8_lossy(window).to_string());
        }
    }
    None
}

fn unpack_ptr_len(packed: i64) -> Result<(usize, usize), String> {
    let ptr = usize::try_from((packed >> 32) as u32)
        .map_err(|_| "WASM addon returned an invalid response pointer.".to_string())?;
    let len = usize::try_from((packed & 0xffff_ffff) as u32)
        .map_err(|_| "WASM addon returned an invalid response length.".to_string())?;
    Ok((ptr, len))
}

#[cfg(test)]
mod tests {
    use super::WasmAddonModuleSession;
    use crate::native::InstalledWasmAddonModule;
    use crate::platform::{
        AddonId, HostedAddonFrame, HostedAddonInitRequest, HostedAddonProtocol, HostedAddonRequest,
        HostedAddonResponse, HostedAddonSize, HostedAddonSurface, HostedAddonUpdateRequest,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn wasm_addon_module_session_round_trips_json_protocol() {
        let dir = temp_dir("wasm_addon_module_session_round_trips_json_protocol");
        let module_path = dir.join("mock-addon.wasm");
        fs::write(&module_path, build_mock_wasm_module()).unwrap();

        let module = InstalledWasmAddonModule {
            addon_id: AddonId::from("games.mock-addon"),
            protocol: HostedAddonProtocol::ShellSurfaceV1,
            module_path,
            bundle_dir: dir.clone(),
        };
        let init = HostedAddonRequest::Initialize(HostedAddonInitRequest {
            addon_id: "games.mock-addon".to_string(),
            surface: HostedAddonSurface::Desktop,
            size: HostedAddonSize {
                width: 320.0,
                height: 200.0,
            },
            scale_factor: 1.0,
            host_context: None,
        });

        let (mut session, ready) = WasmAddonModuleSession::spawn(&module, &init).unwrap();
        assert_eq!(
            ready,
            HostedAddonResponse::Ready {
                title: "Mock WASM Addon".to_string(),
                frame: HostedAddonFrame {
                    size: HostedAddonSize {
                        width: 320.0,
                        height: 200.0,
                    },
                    clear: None,
                    commands: Vec::new(),
                    status_line: Some("ready".to_string()),
                },
            }
        );

        let frame = session
            .request(&HostedAddonRequest::Update(HostedAddonUpdateRequest {
                size: HostedAddonSize {
                    width: 320.0,
                    height: 200.0,
                },
                delta_seconds: 1.0 / 60.0,
                input: Vec::new(),
                host_context: None,
            }))
            .unwrap();
        assert_eq!(
            frame,
            HostedAddonResponse::Frame {
                frame: HostedAddonFrame {
                    size: HostedAddonSize {
                        width: 320.0,
                        height: 200.0,
                    },
                    clear: None,
                    commands: Vec::new(),
                    status_line: Some("updated".to_string()),
                },
            }
        );
    }

    fn build_mock_wasm_module() -> Vec<u8> {
        let ready_json = serde_json::to_string(&HostedAddonResponse::Ready {
            title: "Mock WASM Addon".to_string(),
            frame: HostedAddonFrame {
                size: HostedAddonSize {
                    width: 320.0,
                    height: 200.0,
                },
                clear: None,
                commands: Vec::new(),
                status_line: Some("ready".to_string()),
            },
        })
        .unwrap();
        let frame_json = serde_json::to_string(&HostedAddonResponse::Frame {
            frame: HostedAddonFrame {
                size: HostedAddonSize {
                    width: 320.0,
                    height: 200.0,
                },
                clear: None,
                commands: Vec::new(),
                status_line: Some("updated".to_string()),
            },
        })
        .unwrap();
        let ready_len = ready_json.len();
        let frame_len = frame_json.len();

        let wat = format!(
            r#"(module
                (memory (export "memory") 1)
                (global $heap (mut i32) (i32.const 1024))
                (global $call_count (mut i32) (i32.const 0))
                (data (i32.const 0) "{ready}")
                (data (i32.const 256) "{frame}")
                (func (export "nd_alloc") (param $len i32) (result i32)
                    (local $ptr i32)
                    global.get $heap
                    local.set $ptr
                    global.get $heap
                    local.get $len
                    i32.add
                    global.set $heap
                    local.get $ptr)
                (func (export "nd_handle_json") (param $ptr i32) (param $len i32) (result i64)
                    (local $response_ptr i32)
                    (local $response_len i32)
                    global.get $call_count
                    i32.eqz
                    if
                        i32.const 0
                        local.set $response_ptr
                        i32.const {ready_len}
                        local.set $response_len
                    else
                        i32.const 256
                        local.set $response_ptr
                        i32.const {frame_len}
                        local.set $response_len
                    end
                    global.get $call_count
                    i32.const 1
                    i32.add
                    global.set $call_count
                    local.get $response_ptr
                    i64.extend_i32_u
                    i64.const 32
                    i64.shl
                    local.get $response_len
                    i64.extend_i32_u
                    i64.or))
            "#,
            ready = wat_bytes(&ready_json),
            frame = wat_bytes(&frame_json),
            ready_len = ready_len,
            frame_len = frame_len,
        );
        wat::parse_str(wat).unwrap()
    }

    fn wat_bytes(value: &str) -> String {
        value
            .as_bytes()
            .iter()
            .map(|byte| format!("\\{:02x}", byte))
            .collect()
    }

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("nucleon-wasm-addon-{label}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
