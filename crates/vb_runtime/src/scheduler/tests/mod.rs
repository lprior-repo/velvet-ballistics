#![forbid(unsafe_code)]
//! Test module aggregator for the seeded scheduler facade.
//!
//! Each sub-module owns a focused concern (determinism, policy,
//! budget, transcript, counters, RNG). Splitting the file keeps each
//! unit ≤ 200 lines so the 300-line source-length ceiling remains
//! satisfied per module and reviewers can navigate a single concern
//! at a time.

mod budget_tests;
mod counters_tests;
mod determinism_tests;
mod fixtures;
mod policy_tests;
mod rng_tests;
mod transcript_tests;

// Verification harness parity: a trivial test exists in this module
// so `cargo test -p vb_runtime scheduler` reports the full set.
#[cfg(test)]
#[test]
fn scheduler_test_module_aggregator_compiles() {
    // Aggregator present; sub-modules own the actual tests.
}
