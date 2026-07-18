use galfus_host::{IoOperation, IoProvider, IoProviderError, IoRead};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

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
}

impl BufferIoProvider {
    pub fn new(input: impl Into<Vec<u8>>) -> Self {
        Self {
            state: Arc::new(Mutex::new(BufferIoState {
                input: input.into().into(),
                output: Vec::new(),
            })),
        }
    }

    pub fn take_output(&self) -> Vec<u8> {
        std::mem::take(&mut self.state.lock().expect("buffer I/O state").output)
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
        self.state
            .lock()
            .expect("buffer I/O state")
            .output
            .extend_from_slice(bytes);
        Ok(())
    }
}
