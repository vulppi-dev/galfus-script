#[cfg(test)]
mod tests;

mod abi;
mod native;
mod web;

pub use abi::{TargetCall, TargetCapabilityProvider, TargetResult};
pub use native::{DefaultTargetCapabilityProvider, NativeTarget};
pub use web::WebTarget;
