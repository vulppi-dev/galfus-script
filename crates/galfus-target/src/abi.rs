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
