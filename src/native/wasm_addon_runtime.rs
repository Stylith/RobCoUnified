use crate::native::InstalledWasmAddonModule;
use crate::platform::{HostedAddonRequest, HostedAddonResponse};
use wasmi::{Engine, Linker, Memory, Module, Store, TypedFunc};

#[allow(dead_code)]
pub(crate) struct WasmAddonModuleSession {
    store: Store<()>,
    memory: Memory,
    alloc: TypedFunc<i32, i32>,
    handle_json: TypedFunc<(i32, i32), i64>,
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
            .map_err(|error| {
                format!(
                    "Failed to instantiate WASM addon '{}' from '{}': {error}",
                    module.addon_id,
                    module.module_path.display()
                )
            })?;
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
        AddonId, HostedAddonFrame, HostedAddonInitRequest, HostedAddonProtocol,
        HostedAddonRequest, HostedAddonResponse, HostedAddonSize, HostedAddonSurface,
        HostedAddonUpdateRequest,
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
        let dir = std::env::temp_dir().join(format!("robcos-wasm-addon-{label}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
