use robcos_hosted_addon_contract::{
    HostedAddonInitRequest, HostedAddonRequest, HostedAddonResponse, HostedAddonUpdateRequest,
};
use std::sync::{Mutex, OnceLock};

pub trait WasmAddon: Default + Send + 'static {
    fn initialize(&mut self, init: HostedAddonInitRequest) -> HostedAddonResponse;
    fn update(&mut self, update: HostedAddonUpdateRequest) -> HostedAddonResponse;

    fn shutdown(&mut self) {}
}

pub fn dispatch_request<T: WasmAddon>(
    instance: &OnceLock<Mutex<T>>,
    request: HostedAddonRequest,
) -> HostedAddonResponse {
    let mut addon = instance
        .get_or_init(|| Mutex::new(T::default()))
        .lock()
        .expect("wasm addon mutex poisoned");
    match request {
        HostedAddonRequest::Initialize(init) => addon.initialize(init),
        HostedAddonRequest::Update(update) => addon.update(update),
        HostedAddonRequest::Shutdown => {
            addon.shutdown();
            HostedAddonResponse::Exit {
                reason: "shutdown".to_string(),
            }
        }
    }
}

pub fn alloc_guest_buffer(len: i32) -> i32 {
    if len <= 0 {
        return 0;
    }
    let mut bytes = Vec::<u8>::with_capacity(len as usize);
    let ptr = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    ptr as i32
}

pub fn handle_json_request<T: WasmAddon>(
    instance: &OnceLock<Mutex<T>>,
    response_buffer: &OnceLock<Mutex<Vec<u8>>>,
    ptr: i32,
    len: i32,
) -> i64 {
    let encoded = unsafe {
        let bytes = std::slice::from_raw_parts(ptr as *const u8, len.max(0) as usize);
        handle_json_bytes(instance, bytes)
    };

    let mut buffer = response_buffer
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("wasm addon response mutex poisoned");
    *buffer = encoded;
    let ptr = buffer.as_ptr() as u32 as u64;
    let len = buffer.len() as u32 as u64;
    ((ptr << 32) | len) as i64
}

pub fn handle_json_bytes<T: WasmAddon>(
    instance: &OnceLock<Mutex<T>>,
    bytes: &[u8],
) -> Vec<u8> {
    let response = match serde_json::from_slice::<HostedAddonRequest>(bytes) {
        Ok(request) => dispatch_request(instance, request),
        Err(error) => HostedAddonResponse::Error {
            message: format!("Failed to parse addon request: {error}"),
        },
    };
    serde_json::to_vec(&response).unwrap_or_else(|error| {
        serde_json::to_vec(&HostedAddonResponse::Error {
            message: format!("Failed to encode addon response: {error}"),
        })
        .expect("error response should serialize")
    })
}

#[macro_export]
macro_rules! export_wasm_addon {
    ($addon_ty:ty) => {
        static ND_INSTANCE: ::std::sync::OnceLock<::std::sync::Mutex<$addon_ty>> =
            ::std::sync::OnceLock::new();
        static ND_RESPONSE_BUFFER: ::std::sync::OnceLock<::std::sync::Mutex<::std::vec::Vec<u8>>> =
            ::std::sync::OnceLock::new();

        #[no_mangle]
        pub extern "C" fn nd_alloc(len: i32) -> i32 {
            $crate::alloc_guest_buffer(len)
        }

        #[no_mangle]
        pub extern "C" fn nd_handle_json(ptr: i32, len: i32) -> i64 {
            $crate::handle_json_request::<$addon_ty>(
                &ND_INSTANCE,
                &ND_RESPONSE_BUFFER,
                ptr,
                len,
            )
        }
    };
}

#[cfg(test)]
mod tests {
    use super::{dispatch_request, handle_json_bytes, WasmAddon};
    use robcos_hosted_addon_contract::{
        HostedAddonFrame, HostedAddonInitRequest, HostedAddonRequest, HostedAddonResponse,
        HostedAddonSize, HostedAddonSurface, HostedAddonUpdateRequest,
    };
    use std::sync::OnceLock;

    #[derive(Default)]
    struct MockAddon;

    impl WasmAddon for MockAddon {
        fn initialize(&mut self, init: HostedAddonInitRequest) -> HostedAddonResponse {
            HostedAddonResponse::Ready {
                title: init.addon_id,
                frame: HostedAddonFrame {
                    size: init.size,
                    clear: None,
                    commands: Vec::new(),
                    status_line: Some("ready".to_string()),
                },
            }
        }

        fn update(&mut self, update: HostedAddonUpdateRequest) -> HostedAddonResponse {
            HostedAddonResponse::Frame {
                frame: HostedAddonFrame {
                    size: update.size,
                    clear: None,
                    commands: Vec::new(),
                    status_line: Some("updated".to_string()),
                },
            }
        }
    }

    #[test]
    fn dispatch_request_routes_initialize_and_update() {
        let instance = OnceLock::new();
        let init = HostedAddonRequest::Initialize(HostedAddonInitRequest {
            addon_id: "tools.mock".to_string(),
            surface: HostedAddonSurface::Terminal,
            size: HostedAddonSize {
                width: 80.0,
                height: 25.0,
            },
            scale_factor: 1.0,
        });
        let ready = dispatch_request::<MockAddon>(&instance, init);
        assert!(matches!(ready, HostedAddonResponse::Ready { .. }));

        let frame = dispatch_request::<MockAddon>(
            &instance,
            HostedAddonRequest::Update(HostedAddonUpdateRequest {
                size: HostedAddonSize {
                    width: 80.0,
                    height: 25.0,
                },
                delta_seconds: 1.0 / 60.0,
                input: Vec::new(),
            }),
        );
        assert!(matches!(frame, HostedAddonResponse::Frame { .. }));
    }

    #[test]
    fn handle_json_bytes_round_trips_json_buffers() {
        let instance = OnceLock::new();
        let request = HostedAddonRequest::Initialize(HostedAddonInitRequest {
            addon_id: "tools.mock".to_string(),
            surface: HostedAddonSurface::Desktop,
            size: HostedAddonSize {
                width: 320.0,
                height: 200.0,
            },
            scale_factor: 1.0,
        });
        let encoded = serde_json::to_vec(&request).unwrap();
        let response_bytes = handle_json_bytes::<MockAddon>(&instance, &encoded);
        let response: HostedAddonResponse = serde_json::from_slice(&response_bytes).unwrap();
        assert!(matches!(response, HostedAddonResponse::Ready { .. }));
    }
}
