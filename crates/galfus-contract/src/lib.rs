//! Host integration contracts for Galfus execution.

#[cfg(test)]
mod tests;
pub mod thread;

pub use thread::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostValue {
    Null,
    Int32(i32),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<HostValue>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostResponse {
    Success(HostValue),
    Error(String),
}

pub trait MessageInjector: Send + Sync {
    fn inject_system_response(&self, thread_id: usize, response: HostResponse);
}

pub trait HostProvider: Send {
    fn dispatch(
        &mut self,
        thread_id: usize,
        name: &str,
        args: &[HostValue],
        injector: std::sync::Arc<dyn MessageInjector>,
    );
}

/// Optional host capabilities supplied for one execution.
#[derive(Default)]
pub struct Providers {
    host: Option<Box<dyn HostProvider>>,
}

impl Providers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_host(host: Box<dyn HostProvider>) -> Self {
        Self { host: Some(host) }
    }

    pub fn host_mut(&mut self) -> Option<&mut (dyn HostProvider + 'static)> {
        self.host.as_deref_mut()
    }
}
