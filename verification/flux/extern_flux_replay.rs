//! Extern companion for Flux replay-bound refinement annotations.
//!
//! ============================================================================
//! PRODUCTION BINDING — SCOPED-ONLY
//! ============================================================================
//!
//! This companion file serves as the production anchor for the Flux
//! refinement models in `verification/flux/vb-p0vpw-sequence-replay.rs`.
//! It documents the production types that the replay model functions
//! mirror.
//!
//! Production sources:
//!   `crates/vb_storage/src/types.rs` line 73
//!     `pub struct EventSeq(u64);` — sequence bounds for journal events.
//!   `crates/vb_storage/src/recovery/types.rs`
//!     `pub struct RecoveryRuntimeSummary` — summary with first_seq / last_seq.
//!     `pub enum RecoveryTerminalState` — terminal state enum.
//!     `pub struct RecoveryCannotResumeState` — resumability witness.
//!   `crates/vb_storage/src/journal/replay.rs`
//!     Journal replay entry points used by recovery.
//!   `crates/vb_storage/src/recovery/replay/core.rs`
//!     Core replay logic with step ordering checks.
//!
//! The replay model functions in the spec file mirror the recovery
//! invariants:
//!   - `contiguous_sequence_check` → journal replay contiguity verification
//!   - `step_started_diverges` / `step_started_valid` → StepIdx monotonicity
//!   - `replay_tail_valid` / `replay_tail_diverges` → snapshot-plus-tail bounds
//!   - `action_already_resolved` / `non_idempotent_action_blocked` →
//!     non-idempotent action blocking policy
//!   - `is_terminal_event` / `terminal_event_from_latest_attempt` →
//!     terminal state extraction
//!
//! The Flux refinements are SCOPED-ONLY: they model the same invariants
//! that the production recovery types enforce by construction, but the
//! production types do not carry `#[refined_by]` annotations in this
//! crate version.
//!
//! See verification/flux/WIRING_STATUS.md for the full artifact inventory.

#![forbid(unsafe_code)]
#![allow(dead_code)]
