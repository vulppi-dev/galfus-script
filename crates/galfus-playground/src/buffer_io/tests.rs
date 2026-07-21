use super::*;
use std::sync::{Arc, Mutex};

struct MockInjector {
    response: Arc<Mutex<Option<HostResponse>>>,
}

impl MessageInjector for MockInjector {
    fn inject_system_response(&self, _thread_id: usize, response: HostResponse) {
        *self.response.lock().unwrap() = Some(response);
    }
}

fn call_dispatch(
    provider: &mut BufferIoProvider,
    method: &str,
    args: &[HostValue],
) -> HostResponse {
    let response = Arc::new(Mutex::new(None));
    let injector = Arc::new(MockInjector {
        response: Arc::clone(&response),
    });
    provider.dispatch(0, method, args, injector);
    response.lock().unwrap().take().unwrap()
}

#[test]
fn reads_until_terminator_and_keeps_remaining_input() {
    let mut provider = BufferIoProvider::new(b"first\r\nsecond".to_vec());

    assert_eq!(
        call_dispatch(&mut provider, "read", &[HostValue::Bytes(b"\r\n".to_vec())]),
        HostResponse::Success(HostValue::Bytes(b"first".to_vec()))
    );
    assert_eq!(
        call_dispatch(&mut provider, "read", &[HostValue::Bytes(b"\r\n".to_vec())]),
        HostResponse::Success(HostValue::Bytes(b"second".to_vec()))
    );
    assert_eq!(
        call_dispatch(&mut provider, "read", &[HostValue::Bytes(b"\r\n".to_vec())]),
        HostResponse::Success(HostValue::Bytes(Vec::new()))
    );
}

#[test]
fn captures_written_output() {
    let mut provider = BufferIoProvider::default();

    call_dispatch(
        &mut provider,
        "write",
        &[HostValue::Bytes(b"hello".to_vec())],
    );
    call_dispatch(
        &mut provider,
        "write",
        &[HostValue::Bytes(b" world".to_vec())],
    );

    assert_eq!(provider.take_output(), b"hello world");
    assert_eq!(provider.take_output(), b"");
}

#[test]
fn rejects_an_empty_terminator() {
    let mut provider = BufferIoProvider::default();
    let error = call_dispatch(&mut provider, "read", &[HostValue::Bytes(b"".to_vec())]);

    assert!(
        matches!(error, HostResponse::Error(msg) if msg == "input terminator must not be empty")
    );
}

#[test]
fn receives_read_data_after_creation() {
    let mut provider = BufferIoProvider::default();
    provider.send_read_data(b"input\n");

    assert_eq!(
        call_dispatch(&mut provider, "read", &[HostValue::Bytes(b"\n".to_vec())]),
        HostResponse::Success(HostValue::Bytes(b"input".to_vec()))
    );
}
