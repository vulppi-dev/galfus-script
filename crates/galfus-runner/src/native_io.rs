use galfus_contract::{IoOperation, IoProvider, IoProviderError, IoRead};
use std::io::{Read, Write};

/// Synchronous terminal I/O for native Galfus hosts.
pub struct NativeIoProvider;

impl IoProvider for NativeIoProvider {
    fn read(&mut self, terminator: &[u8]) -> Result<IoRead, IoProviderError> {
        if terminator.is_empty() {
            return Err(IoProviderError::new(
                IoOperation::Read,
                "input terminator must not be empty",
            ));
        }

        let mut input = Vec::new();
        let mut byte = [0u8; 1];
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();

        loop {
            match handle.read(&mut byte) {
                Ok(0) if input.is_empty() => return Ok(IoRead::EndOfInput),
                Ok(0) => return Ok(IoRead::Bytes(input)),
                Ok(_) => {
                    input.push(byte[0]);
                    if input.ends_with(terminator) {
                        input.truncate(input.len() - terminator.len());
                        return Ok(IoRead::Bytes(input));
                    }
                }
                Err(error) => {
                    return Err(IoProviderError::new(IoOperation::Read, error.to_string()));
                }
            }
        }
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), IoProviderError> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle
            .write_all(bytes)
            .and_then(|()| handle.flush())
            .map_err(|error| IoProviderError::new(IoOperation::Write, error.to_string()))
    }
}
