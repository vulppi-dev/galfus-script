#[cfg(test)]
mod tests;

pub mod error;
pub mod runtime;

pub use error::{StackFrameInfo, VmError, VmPanic};
pub use runtime::{CallFrame, HeapObject, IoHandler, VirtualMachine};
