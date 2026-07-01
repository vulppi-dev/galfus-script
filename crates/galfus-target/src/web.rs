use crate::{TargetCall, TargetCapabilityProvider, TargetResult};
use std::sync::{Arc, Mutex};

/// Web/playground target that captures output in memory and reports stdin EOF.
#[derive(Clone, Default)]
pub struct WebTarget {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl WebTarget {
    pub fn new() -> Self {
        Self::default()
    }

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
