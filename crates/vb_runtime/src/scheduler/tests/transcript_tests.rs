#![forbid(unsafe_code)]
//! Transcript tests for the seeded autonomous scheduler facade.
//!
//! `BoundaryTranscript` records every decision the scheduler emits in
//! order so callers can replay the boundary exploration without
//! re-executing the runtime. The tests below pin the append-only
//! record, the empty-transcript baseline, and the compact
//! decision-variant tag projection.

use vb_core::ids::StepIdx;

use crate::scheduler::tests::fixtures::{FIXTURE_SEED_A, make_scheduler};
use crate::scheduler::transcript::BoundaryTranscript;
use crate::scheduler::types::{BoundaryChoice, BoundaryDecision, BoundaryPolicy};

#[test]
fn transcript_records_decisions_in_order() {
    let mut scheduler = make_scheduler(FIXTURE_SEED_A, BoundaryPolicy::First);
    let _ = scheduler.decide_boundary(BoundaryChoice::Free);
    let _ = scheduler.decide_boundary(BoundaryChoice::Free);
    let _ = scheduler.decide_boundary(BoundaryChoice::Free);
    assert_eq!(scheduler.transcript_len(), 3);
    let entries = scheduler.transcript().as_slice();
    assert_eq!(entries[0].step_index, 0);
    assert_eq!(entries[1].step_index, 1);
    assert_eq!(entries[2].step_index, 2);
    for entry in entries {
        assert_eq!(entry.decision, BoundaryDecision::Advance);
    }
}

#[test]
fn transcript_default_is_empty() {
    let transcript = BoundaryTranscript::new();
    assert_eq!(transcript.len(), 0);
    assert!(transcript.is_empty());
}

#[test]
fn transcript_decision_variants_compact_form() {
    let mut transcript = BoundaryTranscript::new();
    transcript.record(0, BoundaryChoice::Free, BoundaryDecision::Advance, 0, None);
    transcript.record(
        1,
        BoundaryChoice::Free,
        BoundaryDecision::Yield {
            to_step: StepIdx::new(2),
        },
        0,
        None,
    );
    transcript.record(
        2,
        BoundaryChoice::Free,
        BoundaryDecision::Fail {
            variant: crate::RuntimeError::ShutdownInProgress,
        },
        0,
        None,
    );
    transcript.record(
        3,
        BoundaryChoice::Free,
        BoundaryDecision::Retry { delay_ticks: 4 },
        0,
        None,
    );
    let tags = transcript.decision_variants();
    assert_eq!(tags, vec![0u8, 1u8, 2u8, 3u8]);
}

#[test]
fn transcript_clear_resets_state() {
    // The `clear` method is documented as a test-fixture-only API.
    // Here we exercise it to pin the documented contract.
    let mut transcript = BoundaryTranscript::new();
    transcript.record(0, BoundaryChoice::Free, BoundaryDecision::Advance, 0, None);
    assert_eq!(transcript.len(), 1);
    transcript.clear();
    assert_eq!(transcript.len(), 0);
    assert!(transcript.is_empty());
}

#[test]
fn transcript_get_returns_entry_by_index() {
    let mut transcript = BoundaryTranscript::new();
    transcript.record(
        0,
        BoundaryChoice::Free,
        BoundaryDecision::Advance,
        0xDEAD_BEEF_CAFE_F00D,
        None,
    );
    let entry = transcript.get(0).unwrap_or_else(|| {
        // structurally unreachable: we just recorded index 0.
        panic!("entry 0 must exist")
    });
    assert_eq!(entry.step_index, 0);
    assert_eq!(entry.post_rng_state, 0xDEAD_BEEF_CAFE_F00D);
    assert_eq!(transcript.get(1), None);
}

#[test]
fn transcript_last_returns_most_recent_entry() {
    let mut transcript = BoundaryTranscript::new();
    assert_eq!(transcript.last(), None);
    transcript.record(0, BoundaryChoice::Free, BoundaryDecision::Advance, 0, None);
    transcript.record(1, BoundaryChoice::Free, BoundaryDecision::Advance, 0, None);
    let last = transcript
        .last()
        .unwrap_or_else(|| panic!("last must exist"));
    assert_eq!(last.step_index, 1);
}
