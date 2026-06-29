#[cfg(test)]
mod tests;

use std::io::{Read, Write};

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

pub struct DefaultTargetCapabilityProvider;

impl TargetCapabilityProvider for DefaultTargetCapabilityProvider {
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
