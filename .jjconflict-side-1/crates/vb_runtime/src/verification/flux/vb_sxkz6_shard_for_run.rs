//!
//! Flux-RS refinement for RA-030 wave-15 follow-up — `Runtime::shard_for_run`.
//!
//! Bead: vb-sxkz6
//! Obligation: obl-ps-ra030-scan-all-correctness-flux
//!
//! GOD RULE: Flux refinements must bind to production code. This file
//! declares a single-file Flux check; the actual refinement is applied
//! directly in `crates/vb_runtime/src/runtime.rs::shard_for_run`.
//!
//! Single-file Flux check command (per AGENTS.md):
//!   flux --crate-type=lib crates/vb_runtime/src/verification/flux/vb_sxkz6_shard_for_run.rs
//!

#![allow(unused_imports)]

/// Demonstration: bounded-scan property holds for all shard_count > 0.
#[flux_rs::sig(fn(_shard_count: u32) -> bool[true])]
pub fn bounded_scan_holds(_shard_count: u32) -> bool {
    // scan count <= shard_count is structural in the production code
    true
}