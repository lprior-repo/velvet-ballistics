//! Compound step primitive lowering module.
//!
//! Splits the old 608-line part_04.rs into three focused submodules:
//! - `compound`: aggregate, repeat, wait, ask lowerers
//! - `body_dispatch`: emit_single_body_set, emit_single_body_together, body_constant_index
//! - `reduce_chain`: emit_reduce_body_steps

pub(crate) mod compound;
pub(crate) mod body_dispatch;
pub(crate) mod reduce_chain;

// Re-export the public API. These were `pub(crate)` or `pub(super)` in the
// original single-file part_04.rs and are re-exported at crate level via
// `mod_compile_lowering.rs` → `pub(crate) use part_04::*;`.
#[allow(unused_imports)]
pub(crate) use compound::{
    lower_canonical_aggregate, lower_canonical_ask, lower_canonical_repeat, lower_canonical_wait,
};
#[allow(unused_imports)]
pub(crate) use body_dispatch::{body_constant_index, emit_single_body_set, emit_single_body_together};
#[allow(unused_imports)]
pub(crate) use reduce_chain::emit_reduce_body_steps;
