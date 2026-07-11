#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Retired Flux-rs sketch for the vb-y9d3v action-ticket fence.
//!
//! This module is deliberately free of `flux_rs` attributes because `vb_runtime`
//! does not carry a `flux-rs` dependency or package metadata that makes the old
//! extern-spec sketch executable. Keeping non-compiling Flux annotations behind
//! `#[cfg(flux)]` let a stale artifact contradict production while ordinary
//! `cargo flux -p vb_runtime` silently skipped it.
//!
//! Current executable production contract for runtime-a3 / vb-4969v:
//! - `Shard::handle_action_completion` and `Shard::handle_action_failure` call
//!   the aggregate pending-action ownership fence in
//!   `shard/lifecycle/chunk_004.rs` before journal, frame, counter, or trace
//!   mutation.
//! - Legacy completion now gates the `(run, step)` pair against the same
//!   aggregate pending-action map before journaling `StepSucceeded`.
//! - `Shard::handle_cancel` and `Shard::handle_kill` append `ActionAbandoned`
//!   and the terminal marker as one same-run journal batch before active-state
//!   or pending-boundary mutation.
//! - If the terminal batch append fails, the pending action, timer, active run
//!   state, runtime state, counters, trace ring, and per-run journal sequence
//!   remain retryable.
//!
//! Flux proof status: no Flux proof is claimed for this retired module. Reinstating
//! a Flux lane requires adding explicit Flux package metadata/dependencies,
//! binding the specs to production functions, and recording passing `cargo flux`
//! evidence plus trusted-boundary scan output.

/// Human-readable status surfaced by cfg-flux smoke checks.
pub(crate) const VB_Y9D3V_FLUX_REFINEMENTS_STATUS: &str = "retired-no-flux-proof";
