use super::*;
use std::sync::{Arc, Mutex};

struct BufferIo {
    input: Option<IoRead>,
    state: Arc<Mutex<BufferIoState>>,
}

#[derive(Default)]
struct BufferIoState {
    output: Vec<u8>,
    terminator: Vec<u8>,
}

impl IoProvider for BufferIo {
    fn read(&mut self, terminator: &[u8]) -> Result<IoRead, IoProviderError> {
        self.state.lock().expect("buffer state").terminator = terminator.to_vec();
        Ok(self.input.take().unwrap_or(IoRead::EndOfInput))
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), IoProviderError> {
        self.state
            .lock()
            .expect("buffer state")
            .output
            .extend_from_slice(bytes);
        Ok(())
    }
}

#[test]
fn providers_allow_execution_without_io() {
    assert!(Providers::new().io_mut().is_none());
}

#[test]
fn io_provider_receives_reads_and_writes() {
    let state = Arc::new(Mutex::new(BufferIoState::default()));
    let io = BufferIo {
        input: Some(IoRead::Bytes(b"input".to_vec())),
        state: Arc::clone(&state),
    };
    let mut providers = Providers::with_io(Box::new(io));
    let io = providers.io_mut().expect("I/O provider is present");

    assert_eq!(
        io.read(b"\r\n").expect("reads input"),
        IoRead::Bytes(b"input".to_vec())
    );
    io.write(b"output").expect("writes output");

    let state = state.lock().expect("buffer state");
    assert_eq!(state.terminator, b"\r\n");
    assert_eq!(state.output, b"output");
}

#[test]
fn provider_errors_keep_operation_context() {
    let error = IoProviderError::new(IoOperation::Write, "write failed");

    assert_eq!(error.operation(), IoOperation::Write);
    assert_eq!(error.message(), "write failed");
}
