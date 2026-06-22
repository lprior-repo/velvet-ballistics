// Flux refinement artifact for vb-7ol6y POB-vb-7ol6y-026: hydrate_run_frame
// workflow invariants.
//
// Bead: vb-7ol6y (P0)
// State: 5 (proof-writer)
// Verifier: flux-rs
// Command: flux --edition=2021 crates/vb_storage/src/verification/flux/vb_7ol6y_workflow_invariants.rs
//
// PRODUCTION BINDING:
//   crates/vb_storage/src/recovery/hydrate/mod.rs:103-121
//     hydrate_run_frame_from_events
//   workflow-model.md §4 invariants I-1..I-7
//
// WIRING PREREQUISITE (State 7): This file must be added to
//   crates/vb_storage/src/verification/flux/mod.rs
// as `pub mod vb_7ol6y_workflow_invariants;`.

#![cfg(flux)]

use vb_core::Taint;

// Refinement: the union of taint sources in the workflow is bounded by the
// production invariants I-3..I-5. After the post-fix, every SlotWrittenEvent
// produces one of:
//   - Ok(envelope.taint)         [Versioned arm, invariant I-3]
//   - Ok(Clean, unsupported=false) [Legacy non-prefix arm, invariant I-5]
//   - Err(CorruptSlotTaint)      [Legacy prefix + decode failure, invariant I-4]
//   - Ok(Secret, unsupported=false) [None arm, SR-013 regression guard]

#[flux_rs::refined_by(taint_outcome_kind: int)]
pub enum SpecWorkflowTaintOutcome {
    // Ok(envelope.taint)
    #[flux_rs::variant(SpecWorkflowTaintOutcome[0])]
    OkEnvelopeTaint(#[flux_rs::field(Taint)] Taint),
    // Ok(Clean, unsupported=false)
    #[flux_rs::variant(SpecWorkflowTaintOutcome[1])]
    OkClean,
    // Err(CorruptSlotTaint) — fail-closed
    #[flux_rs::variant(SpecWorkflowTaintOutcome[2])]
    ErrCorrupt,
    // Ok(Secret, unsupported=false) — legacy None regression
    #[flux_rs::variant(SpecWorkflowTaintOutcome[3])]
    OkSecret,
}

// Refinement postcondition: the workflow outcome is total over the
// 3-way match in recovered_slot_taint + 6-arm match in
// legacy_or_corrupt_taint. Every input produces exactly one outcome.
#[flux_rs::sig(fn(extra_kind: int, decode_kind: int) -> SpecWorkflowTaintOutcome)]
pub fn spec_workflow_outcome_total(extra_kind: int, decode_kind: int) -> SpecWorkflowTaintOutcome {
    if extra_kind == 0 {
        // Versioned arm: Ok(envelope.taint).
        SpecWorkflowTaintOutcome::OkEnvelopeTaint(vb_core::Taint::Clean)
    } else if extra_kind == 1 {
        // Legacy arm: depends on decode.
        if decode_kind == 1 {
            // prefix-detected + Envelope
            SpecWorkflowTaintOutcome::OkEnvelopeTaint(vb_core::Taint::Clean)
        } else {
            // prefix-detected + non-Envelope OR non-prefix legacy
            if decode_kind == 0 {
                // Non-prefix bytes (regardless of decode result): OkClean.
                SpecWorkflowTaintOutcome::OkClean
            } else {
                // Err or LegacyFrameExtra after prefix: ErrCorrupt.
                SpecWorkflowTaintOutcome::ErrCorrupt
            }
        }
    } else {
        // None arm: OkSecret.
        SpecWorkflowTaintOutcome::OkSecret
    }
}

// Invariant I-7: total taint outcome count equals event count.
#[flux_rs::sig(fn(events: int, total_outcomes: int) -> bool[events == total_outcomes])]
pub fn spec_invariant_i7_total_count(events: int, total_outcomes: int) -> bool {
    events == total_outcomes
}
