//!
//! Kani harness group for shard lifecycle verification — TLA bridge vb-282my.
//!
//! Bead: vb-282my | State: 5 (proof-writer)
//! Feature gate: kani-shard-lifecycle
//!
//! Harness files:
//!   - kani_shard_lifecycle_harnesses.rs (existing, extended with RetryFSM proofs)
//!   - kani_ask_answer_lifecycle.rs  (AskAnswer lifecycle ordering)
//!   - kani_resume_state_machine.rs (Resume FSM consistency)
//!   - kani_admission_ordering.rs   (Admission header ordering)
//!
//! All harnesses use #[cfg(kani)] internally and are gated here
//! behind the kani-shard-lifecycle feature.

#![forbid(unsafe_code)]
#![cfg(kani)]

// Existing harnesses (po-001 through po-011 from vb-8mdp.5, extended with RetryFSM)
#[path = "verification/kani/kani_shard_lifecycle_harnesses.rs"]
mod kani_shard_lifecycle_harnesses;

// New vb-282my harnesses
#[path = "verification/kani/kani_ask_answer_lifecycle.rs"]
mod kani_ask_answer_lifecycle;

#[path = "verification/kani/kani_resume_state_machine.rs"]
mod kani_resume_state_machine;

#[path = "verification/kani/kani_admission_ordering.rs"]
mod kani_admission_ordering;
