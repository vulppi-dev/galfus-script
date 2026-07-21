//! Host integration contracts for Galfus execution.

pub mod thread;
pub use thread::*;

#[cfg(test)]
mod tests;

/// The I/O operation that failed in a host provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOperation {
    Read,
    Write,
}

/// An error returned by a host I/O provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoProviderError {
    operation: IoOperation,
    message: String,
}

impl IoProviderError {
    pub fn new(operation: IoOperation, message: impl Into<String>) -> Self {
        Self {
            operation,
            message: message.into(),
        }
    }

    pub fn operation(&self) -> IoOperation {
        self.operation
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

/// The result of reading from a host I/O provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IoRead {
    Bytes(Vec<u8>),
    EndOfInput,
}

/// Synchronous host I/O available to a Galfus execution.
pub trait IoProvider: Send {
    fn read(&mut self, terminator: &[u8]) -> Result<IoRead, IoProviderError>;

    fn write(&mut self, bytes: &[u8]) -> Result<(), IoProviderError>;
}

/// Optional host capabilities supplied for one execution.
#[derive(Default)]
pub struct Providers {
    io: Option<Box<dyn IoProvider>>,
}

impl Providers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_io(io: Box<dyn IoProvider>) -> Self {
        Self { io: Some(io) }
    }

    pub fn io_mut(&mut self) -> Option<&mut (dyn IoProvider + 'static)> {
        self.io.as_deref_mut()
    }
}
