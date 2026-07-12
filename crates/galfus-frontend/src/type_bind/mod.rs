mod type_bind;

#[cfg(test)]
mod tests;

pub use type_bind::{TypeBindResult, TypeLoweringResult, bind_types, lower_types};
