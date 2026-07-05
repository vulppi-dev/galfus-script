pub use error::{StackFrameInfo, VmError, VmPanic};
pub use runtime::{CallFrame, HeapObject, VirtualMachine, VmContext, VmObjectRef, VmValue};

#[cfg(test)]
mod tests;

pub mod error;
pub mod runtime;
