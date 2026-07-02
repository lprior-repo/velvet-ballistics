#![forbid(unsafe_code)]
//! Seeded autonomous scheduler facade (P1 BEAD vb-wy33p.7).
//!
//! Drives a [`crate::Runtime`] in deterministic seed-driven steps.
//! The same seed + workflow always produces the same journal byte stream
//! and transcript; different seeds explore different boundary decisions.
//!
//! All control flow is statically bounded by the configured
//! [`SchedulerConfig::max_steps`] and [`SchedulerConfig::max_ticks`].
//!
//! # Architecture
//!
//! - [`SeededScheduler`] owns the [`crate::Runtime`] and a splitmix64 PRNG.
//! - [`decide_boundary`](SeededScheduler::decide_boundary) is the seeded
//!   decision point: the policy + seed select one
//!   [`BoundaryDecision`] variant for each [`BoundaryChoice`].
//! - [`BoundaryTranscript`] records every decision in order so callers
//!   can replay the boundary exploration without re-running the runtime.
//! - [`SeededScheduler::run_to_completion`] drives the runtime until
//!   natural completion (all shards shut down), a fail decision, or a
//!   configured budget is exhausted.
//!
//! This module is **pure facade**: it does not mutate runtime semantics
//! or relax runtime safety constraints. It only sequences runtime calls
//! and selects boundary outcomes.

mod config;
mod decision;
mod decision_select;
mod error;
mod rng;
mod transcript;
mod types;

pub use config::{SchedulerConfig, SeededScheduler};
pub use error::SchedulerError;
pub use rng::RngState;
pub use transcript::{BoundaryTranscript, BoundaryTranscriptEntry};
pub use types::{BoundaryChoice, BoundaryDecision, BoundaryPolicy, RunEndReason, RunResult};

// Verification harnesses gated internally (kani/test/verus/flux via cfg)
#[cfg(test)]
mod tests;
