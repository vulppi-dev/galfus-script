use galfus_host::{IoOperation, IoProvider, IoProviderError, IoRead};
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

impl IoProvider for BufferIoProvider {
    fn read(&mut self, terminator: &[u8]) -> Result<IoRead, IoProviderError> {
        if terminator.is_empty() {
            return Err(IoProviderError::new(
                IoOperation::Read,
                "input terminator must not be empty",
            ));
        }

        let mut state = self.state.lock().expect("buffer I/O state");
        if state.input.is_empty() {
            return Ok(IoRead::EndOfInput);
        }

        let mut input = Vec::new();
        while let Some(byte) = state.input.pop_front() {
            input.push(byte);
            if input.ends_with(terminator) {
                input.truncate(input.len() - terminator.len());
                break;
            }
        }

        Ok(IoRead::Bytes(input))
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), IoProviderError> {
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
            let value = Uint8Array::from(bytes);
            callback
                .call1(&JsValue::UNDEFINED, &value.into())
                .map_err(|error| IoProviderError::new(IoOperation::Write, format!("{error:?}")))?;
        }
        Ok(())
    }
}
