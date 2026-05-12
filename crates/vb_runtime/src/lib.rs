#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::comparison_chain)]
#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::assertions_on_constants,
        clippy::bool_assert_comparison,
        clippy::clone_on_copy,
        clippy::get_first,
        clippy::manual_contains,
        clippy::map_clone,
        clippy::panic,
        clippy::redundant_locals
    )
)]

//! Hot-path runtime engine for velvet-ballastics.
//!
//! Owns shard scheduling, frame pools, action dispatch, timer wheels,
//! bounded queues, and deterministic step execution.

pub mod action;
pub mod admission;
pub mod counters;
pub mod durability_matrix;
pub mod engine;
mod error;
pub mod frame_pool;
pub mod idempotency;
pub mod journal;
pub mod primitives;
pub mod recovery;
pub mod runtime;
pub mod shard;
pub mod taint;
pub mod trace;

pub use error::{RuntimeError, RuntimeResult};
pub use shard::{AskAnswer, AskTicket};

#[cfg(test)]
mod test_harness;
