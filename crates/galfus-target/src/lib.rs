#[cfg(test)]
mod tests;

use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetCall<'a> {
    Write(&'a [u8]),
    Read,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetResult {
    Success,
    ReadByte(Option<u8>),
}

pub trait TargetCapabilityProvider: Send + Sync {
    fn invoke(&mut self, call: TargetCall<'_>) -> Result<TargetResult, String>;
}

/// Native (desktop/server) target — writes to stdout, reads from stdin.
pub struct NativeTarget;

/// Backwards-compatibility alias.
pub type DefaultTargetCapabilityProvider = NativeTarget;

impl TargetCapabilityProvider for NativeTarget {
    fn invoke(&mut self, call: TargetCall<'_>) -> Result<TargetResult, String> {
        match call {
            TargetCall::Write(data) => {
                std::io::stdout()
                    .write_all(data)
                    .map_err(|e| e.to_string())?;
                std::io::stdout().flush().map_err(|e| e.to_string())?;
                Ok(TargetResult::Success)
            }
            TargetCall::Read => {
                let mut buf = [0u8; 1];
                match std::io::stdin().read_exact(&mut buf) {
                    Ok(_) => Ok(TargetResult::ReadByte(Some(buf[0]))),
                    Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        Ok(TargetResult::ReadByte(None))
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
        }
    }
}

/// Web/playground target: captures all `Write` output into a shared buffer.
/// Use [`WebTarget::take_output`] to retrieve the accumulated bytes.
#[derive(Clone, Default)]
pub struct WebTarget {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl WebTarget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drain and return all bytes written so far.
    pub fn take_output(&self) -> Vec<u8> {
        std::mem::take(&mut *self.buffer.lock().unwrap())
    }
}

impl TargetCapabilityProvider for WebTarget {
    fn invoke(&mut self, call: TargetCall<'_>) -> Result<TargetResult, String> {
        match call {
            TargetCall::Write(data) => {
                self.buffer.lock().unwrap().extend_from_slice(data);
                Ok(TargetResult::Success)
            }
            TargetCall::Read => Ok(TargetResult::ReadByte(None)),
        }
    }
}
