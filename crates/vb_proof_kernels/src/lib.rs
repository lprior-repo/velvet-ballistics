//! Proof kernels for velvet-ballastics.
//!
//! Tiny, pure, sequential Rust kernels suitable for formal verification.
//! These are extracted from vb_core for Verus/Aeneas extraction.

pub mod taint;
pub mod step_state;
pub mod resource_budget;
pub mod envelope_header;
