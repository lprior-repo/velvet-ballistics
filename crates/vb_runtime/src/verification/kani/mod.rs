#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani proof harnesses for vb_runtime verification.
//!
//! Every submodule in this directory MUST be gated by `#[cfg(kani)]`
//! (or `#[cfg(feature = "...")]` when the harness is feature-scoped)
//! so that the files only compile under the Kani verifier, never under
//! a plain `cargo build`. This satisfies the harness-isolation rule in
//! AGENTS.md ("Bulky or stale harness groups must be behind package features")
//! by ensuring unrelated package lanes never compile kani-only code.

// Engine / primitive ordering harnesses.
pub(crate) mod kani_engine_signals;
pub(crate) mod kani_for_each_ordering;
pub(crate) mod kani_retry_math;
pub(crate) mod kani_together_ordering;

// Shard lifecycle harnesses (action ticket fence + RA-030 follow-up).
pub(crate) mod kani_attempt_fence_harnesses;
#[cfg(feature = "kani-sxkz6-shard-for-run")]
pub(crate) mod kani_sxkz6_shard_for_run;

// vb-p5pfb: Runtime::shard_index boundary group (proof-writer
// execution of vb-puvkn / vb-xm7j7).
// Declared in lib.rs as a top-level kani module.

// vb-282my TLA bridge harnesses (admission / ask-answer / resume FSM).
pub(crate) mod kani_admission_ordering;
pub(crate) mod kani_ask_answer_lifecycle;
pub(crate) mod kani_resume_state_machine;
