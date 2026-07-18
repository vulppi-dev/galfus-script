use super::*;

#[test]
fn reads_until_terminator_and_keeps_remaining_input() {
    let mut provider = BufferIoProvider::new(b"first\r\nsecond".to_vec());

    assert_eq!(
        provider.read(b"\r\n").expect("reads first input"),
        IoRead::Bytes(b"first".to_vec())
    );
    assert_eq!(
        provider.read(b"\r\n").expect("reads remaining input"),
        IoRead::Bytes(b"second".to_vec())
    );
    assert_eq!(
        provider.read(b"\r\n").expect("reads EOF"),
        IoRead::EndOfInput
    );
}

#[test]
fn captures_written_output() {
    let mut provider = BufferIoProvider::default();

    provider.write(b"hello").expect("writes first output");
    provider.write(b" world").expect("writes second output");

    assert_eq!(provider.take_output(), b"hello world");
    assert_eq!(provider.take_output(), b"");
}

#[test]
fn rejects_an_empty_terminator() {
    let mut provider = BufferIoProvider::default();
    let error = provider.read(b"").expect_err("rejects empty terminator");

    assert_eq!(error.operation(), IoOperation::Read);
}

#[test]
fn receives_read_data_after_creation() {
    let mut provider = BufferIoProvider::default();
    provider.send_read_data(b"input\n");

    assert_eq!(
        provider.read(b"\n").expect("reads supplied input"),
        IoRead::Bytes(b"input".to_vec())
    );
}
