//! Extern companion for Flux sequence-bound refinement annotations.
//!
//! ============================================================================
//! PRODUCTION BINDING — SCOPED-ONLY
//! ============================================================================
//!
//! This companion file serves as the production anchor for the Flux
//! refinement model in `verification/flux/vb-p0vpw-sequence-replay.rs`.
//! It documents the production type that the refined model mirrors.
//!
//! Production source:
//!   `crates/vb_storage/src/types.rs` line 73
//!   `pub struct EventSeq(u64);` — a u64 newtype with values in [0, u64::MAX].
//!
//! The refined model (`EventSeqRefined`) in the spec file defines the same
//! invariant (`0 <= raw && raw <= u64::MAX`) that `EventSeq` enforces by
//! construction via its `u64` inner value.
//!
//! Contiguity models in the spec file correspond to the replay contiguity
//! checks in:
//!   `crates/vb_storage/src/journal/replay.rs`
//!   `crates/vb_storage/src/recovery/replay/core.rs`
//!
//! Step ordering models correspond to `StepIdx` usage in:
//!   `crates/vb_storage/src/recovery/types.rs`
//!
//! The Flux refinements are SCOPED-ONLY: they model the same invariants
//! that the production types enforce by construction, but the production
//! types do not carry `#[refined_by]` annotations in this crate version.
//! Future production binding requires annotating `EventSeq` with
//! `#[flux_rs::refined_by(raw: u64)]` and `#[flux_rs::invariant(...)]`.
//!
//! See verification/flux/WIRING_STATUS.md for the full artifact inventory.

#![forbid(unsafe_code)]
#![allow(dead_code)]
