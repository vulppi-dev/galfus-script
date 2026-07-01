use crate::{TargetCall, TargetCapabilityProvider, TargetResult};
use std::io::{Read, Write};

/// Native desktop/server target backed by standard input and output.
pub struct NativeTarget;

pub type DefaultTargetCapabilityProvider = NativeTarget;

impl TargetCapabilityProvider for NativeTarget {
    fn invoke(&mut self, call: TargetCall<'_>) -> Result<TargetResult, String> {
        match call {
            TargetCall::Write(data) => {
                std::io::stdout()
                    .write_all(data)
                    .map_err(|error| error.to_string())?;
                std::io::stdout()
                    .flush()
                    .map_err(|error| error.to_string())?;
                Ok(TargetResult::Success)
            }
            TargetCall::Read => {
                let mut buf = [0u8; 1];
                match std::io::stdin().read_exact(&mut buf) {
                    Ok(_) => Ok(TargetResult::ReadByte(Some(buf[0]))),
                    Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                        Ok(TargetResult::ReadByte(None))
                    }
                    Err(error) => Err(error.to_string()),
                }
            }
        }
    }
}
