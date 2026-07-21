use galfus_contract::{HostProvider, HostResponse, HostValue, MessageInjector};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[cfg(feature = "wasm")]
use js_sys::{Function, Uint8Array};
#[cfg(feature = "wasm")]
use wasm_bindgen::JsValue;

#[cfg(test)]
mod tests;

/// In-memory synchronous I/O for playground hosts and tests.
#[derive(Clone, Default)]
pub struct BufferIoProvider {
    state: Arc<Mutex<BufferIoState>>,
}

#[derive(Default)]
struct BufferIoState {
    input: VecDeque<u8>,
    output: Vec<u8>,
    #[cfg(feature = "wasm")]
    write_callback: Option<WriteCallback>,
}

#[cfg(feature = "wasm")]
#[derive(Clone)]
struct WriteCallback(Function);

#[cfg(feature = "wasm")]
// The playground executes synchronously on the browser's single thread.
unsafe impl Send for WriteCallback {}

impl BufferIoProvider {
    pub fn new(input: impl Into<Vec<u8>>) -> Self {
        Self {
            state: Arc::new(Mutex::new(BufferIoState {
                input: input.into().into(),
                output: Vec::new(),
                #[cfg(feature = "wasm")]
                write_callback: None,
            })),
        }
    }

    pub fn take_output(&self) -> Vec<u8> {
        std::mem::take(&mut self.state.lock().expect("buffer I/O state").output)
    }

    pub fn send_read_data(&self, bytes: &[u8]) {
        self.state
            .lock()
            .expect("buffer I/O state")
            .input
            .extend(bytes);
    }

    #[cfg(feature = "wasm")]
    pub fn set_write_callback(&self, callback: Function) {
        self.state.lock().expect("buffer I/O state").write_callback = Some(WriteCallback(callback));
    }
}

impl HostProvider for BufferIoProvider {
    fn dispatch(
        &mut self,
        thread_id: usize,
        method: &str,
        args: &[HostValue],
        injector: Arc<dyn MessageInjector>,
    ) {
        match method {
            "write" => {
                if let Some(HostValue::Bytes(bytes)) = args.first() {
                    #[cfg(feature = "wasm")]
                    let callback = {
                        let mut state = self.state.lock().expect("buffer I/O state");
                        state.output.extend_from_slice(bytes);
                        state.write_callback.clone()
                    };

                    #[cfg(not(feature = "wasm"))]
                    self.state
                        .lock()
                        .expect("buffer I/O state")
                        .output
                        .extend_from_slice(bytes);

                    #[cfg(feature = "wasm")]
                    if let Some(WriteCallback(callback)) = callback {
                        let value = Uint8Array::from(bytes.as_slice());
                        if let Err(e) = callback.call1(&JsValue::UNDEFINED, &value.into()) {
                            injector.inject_system_response(
                                thread_id,
                                HostResponse::Error(format!("{:?}", e)),
                            );
                            return;
                        }
                    }
                    injector
                        .inject_system_response(thread_id, HostResponse::Success(HostValue::Null));
                } else {
                    injector.inject_system_response(
                        thread_id,
                        HostResponse::Error("Invalid arguments for write".to_string()),
                    );
                }
            }
            "read" => {
                let terminator = if let Some(HostValue::Bytes(b)) = args.first() {
                    b.clone()
                } else {
                    injector.inject_system_response(
                        thread_id,
                        HostResponse::Error("Invalid arguments for read".to_string()),
                    );
                    return;
                };

                if terminator.is_empty() {
                    injector.inject_system_response(
                        thread_id,
                        HostResponse::Error("input terminator must not be empty".to_string()),
                    );
                    return;
                }

                let mut state = self.state.lock().expect("buffer I/O state");
                if state.input.is_empty() {
                    injector.inject_system_response(
                        thread_id,
                        HostResponse::Success(HostValue::Bytes(Vec::new())),
                    );
                    return;
                }

                let mut input = Vec::new();
                while let Some(byte) = state.input.pop_front() {
                    input.push(byte);
                    if input.ends_with(&terminator) {
                        input.truncate(input.len() - terminator.len());
                        break;
                    }
                }

                injector.inject_system_response(
                    thread_id,
                    HostResponse::Success(HostValue::Bytes(input)),
                );
            }
            _ => {
                injector.inject_system_response(
                    thread_id,
                    HostResponse::Error(format!("Method {} not found", method)),
                );
            }
        }
    }
}
