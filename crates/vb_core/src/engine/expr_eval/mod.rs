#![forbid(unsafe_code)]
//! Expression evaluation engine.

mod accessors;
mod core;
#[cfg(test)]
mod core_tests;
mod ops;
#[cfg(test)]
mod ops_tests;
mod ops_text_list;
#[cfg(test)]
mod ops_text_list_tests;
mod stack;
#[cfg(test)]
mod stack_tests;

pub use accessors::{eval_accessor, eval_accessor_with_store};
pub use core::{eval_expr, eval_expr_with_store};

// HVR-PO-CORE-004: exclude legacy expression Kani modules from vb-god2f resource lane discovery.
#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
mod kani_stack;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
mod kani_div_zero;

#[cfg(test)]
mod tests;
