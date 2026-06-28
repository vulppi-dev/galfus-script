pub mod builder;
pub mod lower;
pub mod mir;
pub mod validator;

pub use builder::*;
pub use lower::lower_module;
pub use mir::*;
pub use validator::*;

#[cfg(test)]
mod tests;
