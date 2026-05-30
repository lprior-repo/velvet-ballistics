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
        clippy::boxed_local,
        clippy::clone_on_copy,
        clippy::cloned_ref_to_slice_refs,
        clippy::expect_used,
        clippy::get_first,
        clippy::indexing_slicing,
        clippy::let_underscore_must_use,
        clippy::len_zero,
        clippy::manual_contains,
        clippy::manual_repeat_n,
        clippy::map_clone,
        clippy::map_flatten,
        clippy::needless_return,
        clippy::ok_expect,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::redundant_closure,
        clippy::redundant_locals,
        clippy::type_complexity,
        clippy::unnecessary_fallible_conversions,
        clippy::unwrap_used,
        clippy::approx_constant
    )
)]

//! Hot-path runtime engine for velvet-ballistics.
//!
//! Owns shard scheduling, frame pools, action dispatch, timer wheels,
//! bounded queues, and deterministic step execution.

pub mod action;
pub mod action_queue;
pub mod admission;
pub mod counters;

pub mod durability_matrix;
pub mod engine;
mod error;
pub mod frame_pool;
pub mod idempotency;
pub mod ipc_refinement;
pub mod journal;
#[cfg(all(kani, feature = "kani-capability-harnesses"))]
pub mod kani_capability_harnesses;
#[cfg(all(kani, feature = "kani-engine-yaml-admission"))]
pub mod kani_engine_yaml_admission;
#[cfg(all(kani, feature = "kani-shard-command-queue"))]
pub mod kani_shard_command_queue;
#[cfg(all(kani, feature = "kani-trace-ring"))]
pub mod kani_trace_ring;
#[cfg(all(kani, feature = "kani-vt2f-runtime-facade"))]
pub mod kani_vt2f_runtime_facade;
#[cfg(all(kani, feature = "kani-vt2f-shard-lower-semantics"))]
pub mod kani_vt2f_shard_lower_semantics;

#[cfg(all(kani, feature = "kani-admission-store"))]
pub mod kani_admission_store;

#[cfg(all(kani, feature = "kani-shard-lifecycle"))]
pub mod kani_shard_lifecycle;

#[cfg(loom)]
pub mod models;
pub mod primitives;
pub mod recovery;
pub mod runtime;
pub mod shard;
pub mod taint;
pub mod trace;
#[cfg(all(kani, feature = "kani-yaml-e2e-admission-matrix"))]
pub mod yaml_e2e_admission_matrix;

pub use error::{RuntimeError, RuntimeResult};
pub use shard::{AskAnswer, AskTicket, ResumeError, ResumeResult, ResumeStatus};

#[cfg(test)]
mod test_harness;

#[cfg(test)]
mod verification {
    pub(crate) mod proptest;
}

#[cfg(kani)]
mod verification {
    pub(crate) mod kani;
}
