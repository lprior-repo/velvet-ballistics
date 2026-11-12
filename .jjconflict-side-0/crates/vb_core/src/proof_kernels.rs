#![forbid(unsafe_code)]
#![allow(unused_macros)]
//! Proof kernels for velvet-ballistics.
//!
//! Tiny, pure, sequential Rust kernels suitable for formal verification.
//! These are extracted from vb_core for Verus/Aeneas extraction.
//!
//! Rules:
//! - safe Rust only
//! - sequential only
//! - no IO
//! - no Fjall
//! - no IPC sockets
//! - no Makepad
//! - no threads
//! - no HashMap
//! - small functions
//!
//! These kernels are extracted and verified with Verus/Aeneas.
//! The full runtime is verified through other means (Kani, fuzz, TLA+).

pub mod envelope_header;
pub mod resource_budget;
pub mod step_state;
pub mod taint;
pub mod vb_kyyf_normalization;
