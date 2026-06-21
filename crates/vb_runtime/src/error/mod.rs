mod conversions;
mod diagnostics;
mod display;
mod equality;
mod from_impls;
mod input_mapping;
mod types;

pub use input_mapping::InputMappingFailureKind;
pub use types::RuntimeError;

// Re-export RuntimeState for use as a field type in RuntimeError variants.
pub(crate) use crate::shard::run_state::RuntimeState;

/// Result alias for runtime operations.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[cfg(test)]
mod tests_basic;
#[cfg(test)]
mod tests_conversion_refinement;
#[cfg(test)]
mod tests_diagnostics;
