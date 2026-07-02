#![forbid(unsafe_code)]

//! Deterministic runtime and journal fault injection engine (vb-wy33p.12).
//!
//! This module models the runtime/storage boundary surface as a set of
//! [`NamedBoundary`] passages and overlays a [`FaultEvent`] schedule on
//! top. The engine is pure (no IO, no threads, no clock) and bounded by
//! `max_faults` and `max_runtime_steps` so it can be invoked safely from
//! tests and from scripts.
//!
//! Determinism contract: for a fixed `(seed, fault_schedule, boundaries)`
//! triple the engine produces byte-identical [`FaultReport`]s. Different
//! seeds produce different schedules of outcomes via a SplitMix64 PRNG
//! that disambiguates unspecified transient/retry decisions.
//!
//! Module layout:
//! - [`types`] — public boundary, fault, outcome, journal, and config
//!   data types.
//! - [`prng`] — SplitMix64 PRNG used to disambiguate unspecified
//!   transient/retry decisions.
//! - [`engine`] — deterministic simulator + schedule hash.
//!
//! See `.beads/vb-wy33p.12/evidence/design.md` for the design write-up
//! and `crates/workspace_tests/tests/fault_injection_tests.rs` for the
//! executable contract.

pub mod display;
pub mod engine;
pub mod prng;
pub mod report;
pub mod types;

pub use engine::{run_fault_injection, validate_config};
pub use prng::compute_schedule_hash;
pub use report::{FaultOutcome, FaultReport, JournalOutcome, MissingReason};
pub use types::{
    BoundarySlot, BudgetKind, CheckpointSeq, CrashSeverity, FailureCode, FaultConfig, FaultError,
    FaultEvent, NamedBoundary,
};
