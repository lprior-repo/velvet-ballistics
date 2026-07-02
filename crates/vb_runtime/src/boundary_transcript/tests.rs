#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
//! Unit tests for the boundary transcript module.
//!
//! These tests are split out of `boundary_transcript.rs` so the production
//! source can satisfy the per-file line-limit gate. The test surface is
//! unchanged from the previous monolithic `boundary_transcript.rs`:
//! it covers the FIFO capacity policy, the journal projection, the
//! authority-bearing `record_*` direct capture API, and parity across
//! two independent transcript instances.

use super::*;
use crate::journal::RuntimeJournalEvent;
use crate::shard::PendingTimerKind;
use vb_core::action::ActionFailureCode;
use vb_core::action::ActionTicket;
use vb_core::action::RetryPolicy;
use vb_core::ids::ActionId;
use vb_core::ids::RunId;
use vb_core::ids::SeqNo;
use vb_core::ids::SlotIdx;
use vb_core::ids::StepIdx;
use vb_core::value::SlotValue;
use vb_core::value::Taint;

/// Builds a canonical test ticket for parity / sequencing tests.
fn test_ticket(run: RunId, seq: u64) -> ActionTicket {
    ActionTicket {
        run,
        step: StepIdx::new(1),
        seq: SeqNo::new(seq),
        action: ActionId::new(7),
        attempt: 1,
        idempotency_key: 0xDEAD_BEEF_u128,
        capacity: 3,
    }
}

#[test]
fn empty_transcript_reports_no_entries() {
    let t = BoundaryTranscript::new();
    assert_eq!(t.len(), 0);
    assert!(t.is_empty());
    assert_eq!(t.dropped(), 0);
    assert_eq!(t.capacity(), BoundaryTranscript::DEFAULT_CAPACITY);
    let snap = t.snapshot();
    assert!(snap.is_empty());
}

#[test]
fn push_assigns_monotonic_sequences() {
    let mut t = BoundaryTranscript::with_capacity(4);
    let s0 = t
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(1),
            step: StepIdx::new(0),
        })
        .expect("push");
    let s1 = t
        .push(BoundaryEvent::WaitResolved {
            run: RunId::new(1),
            step: StepIdx::new(0),
        })
        .expect("push");
    let s2 = t
        .push(BoundaryEvent::AskScheduled {
            run: RunId::new(2),
            step: StepIdx::new(3),
        })
        .expect("push");
    assert_eq!(s0, Some(0));
    assert_eq!(s1, Some(1));
    assert_eq!(s2, Some(2));
    assert_eq!(t.len(), 3);
    assert_eq!(t.dropped(), 0);
}

/// **Blocker 4 (capacity rollback, FIFO)** — when the buffer exceeds
/// capacity, the **oldest** entry is dropped and `dropped()` increments.
/// The choice of FIFO is documented in the module-level docs.
#[test]
fn capacity_rollback_drops_oldest_entries_fifo() {
    let mut t = BoundaryTranscript::with_capacity(3);
    let s0 = t
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(1),
            step: StepIdx::new(0),
        })
        .expect("push");
    let s1 = t
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(2),
            step: StepIdx::new(0),
        })
        .expect("push");
    let s2 = t
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(3),
            step: StepIdx::new(0),
        })
        .expect("push");
    // First overflow.
    let s3 = t
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(4),
            step: StepIdx::new(0),
        })
        .expect("push");
    // Second overflow.
    let s4 = t
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(5),
            step: StepIdx::new(0),
        })
        .expect("push");

    assert_eq!(s0, Some(0));
    assert_eq!(s1, Some(1));
    assert_eq!(s2, Some(2));
    assert_eq!(s3, Some(3));
    assert_eq!(s4, Some(4));
    // Buffer is still bounded at capacity.
    assert_eq!(t.len(), 3);
    // Two entries were dropped on overflow.
    assert_eq!(t.dropped(), 2);
    // Surviving snapshot must start with the entries that survived the
    // overflow — run 3, 4, 5 — and the seq numbers must be the ones
    // originally assigned, proving that sequence numbers continue to
    // advance monotonically even when entries are dropped.
    let snap = t.snapshot();
    assert_eq!(snap.len(), 3);
    assert_eq!(snap[0].seq, 2);
    assert_eq!(snap[1].seq, 3);
    assert_eq!(snap[2].seq, 4);
    match &snap[0].event {
        BoundaryEvent::WaitScheduled { run, .. } => assert_eq!(*run, RunId::new(3)),
        other => panic!("expected WaitScheduled run=3, got {other:?}"),
    }
    match &snap[2].event {
        BoundaryEvent::WaitScheduled { run, .. } => assert_eq!(*run, RunId::new(5)),
        other => panic!("expected WaitScheduled run=5, got {other:?}"),
    }
}

#[test]
fn capacity_clamped_to_minimum_one() {
    let t = BoundaryTranscript::with_capacity(0);
    assert_eq!(t.capacity(), 1);
    assert!(t.is_empty());
}

#[test]
fn snapshot_from_filters_by_sequence() {
    let mut t = BoundaryTranscript::with_capacity(8);
    let _ = t
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(1),
            step: StepIdx::new(0),
        })
        .expect("push");
    let _ = t
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(2),
            step: StepIdx::new(0),
        })
        .expect("push");
    let _ = t
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(3),
            step: StepIdx::new(0),
        })
        .expect("push");
    let snap = t.snapshot_from(1);
    assert_eq!(snap.len(), 2);
    assert_eq!(snap[0].seq, 1);
    assert_eq!(snap[1].seq, 2);
}

#[test]
fn snapshot_range_is_half_open() {
    let mut t = BoundaryTranscript::with_capacity(8);
    for i in 0..5u16 {
        let _ = t
            .push(BoundaryEvent::WaitScheduled {
                run: RunId::new(u64::from(i)),
                step: StepIdx::new(i),
            })
            .expect("push");
    }
    let snap = t.snapshot_range(1, 4);
    assert_eq!(snap.len(), 3);
    assert_eq!(snap[0].seq, 1);
    assert_eq!(snap[2].seq, 3);
}

#[test]
fn shared_transcript_clone_shares_storage() {
    let shared = SharedBoundaryTranscript::with_capacity(2);
    let clone = shared.clone();
    shared
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(1),
            step: StepIdx::new(0),
        })
        .expect("push must succeed");
    // Both handles observe the same retained entries.
    assert_eq!(shared.len().expect("len"), 1);
    assert_eq!(clone.len().expect("len"), 1);
    // Push beyond capacity on either handle rolls off on both.
    clone
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(2),
            step: StepIdx::new(0),
        })
        .expect("push must succeed");
    clone
        .push(BoundaryEvent::WaitScheduled {
            run: RunId::new(3),
            step: StepIdx::new(0),
        })
        .expect("push must succeed");
    assert_eq!(shared.dropped().expect("dropped"), 1);
    assert_eq!(clone.dropped().expect("dropped"), 1);
    assert_eq!(shared.len().expect("len"), 2);
}

/// **Blocker 1 (timer authority)** — the transcript must capture the
/// full authority fields required to replay a timer firing.
#[test]
fn timer_captured_carries_full_authority() {
    let shared = SharedBoundaryTranscript::with_capacity(8);
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let run = RunId::new(42);
    let step = StepIdx::new(3);
    let deadline = std::time::Instant::now();
    let authority = TimerAuthority::new(
        run,
        step,
        PendingTimerKind::Ask,
        /* generation */ 7,
        deadline,
        /* logical_deadline */ 100,
    );
    let seq = proj
        .record_timer_captured(&authority)
        .expect("record must succeed");
    assert_eq!(seq, Some(0));
    let snap = shared.snapshot().expect("snapshot");
    assert_eq!(snap.len(), 1);
    match &snap[0].event {
        BoundaryEvent::TimerCaptured {
            run: r,
            step: s,
            kind,
            generation,
            deadline: dl,
            logical_deadline: ldl,
        } => {
            assert_eq!(*r, run);
            assert_eq!(*s, step);
            assert_eq!(*kind, PendingTimerKind::Ask);
            assert_eq!(*generation, 7);
            assert_eq!(*dl, deadline);
            assert_eq!(*ldl, 100);
        }
        other => panic!("expected TimerCaptured, got {other:?}"),
    }
}

/// **Blocker 1 (timer authority)** — the journal projection does NOT
/// emit `TimerCaptured` because the runtime journal does not carry
/// the timer authority fields (generation/deadline/kind/logical). The
/// runtime must push those events directly. This test asserts that
/// the projection leaves `WaitScheduled` / `AskScheduled` / `WaitResolved`
/// / `AskTimedOut` events as the recoverable surrogate events.
#[test]
fn journal_projection_does_not_synthesize_timer_authority() {
    let shared = SharedBoundaryTranscript::with_capacity(8);
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let run = RunId::new(1);
    let step = StepIdx::new(2);
    // These events have no authority in the journal; the projection
    // emits a recoverable surrogate (WaitScheduled/AskScheduled/
    // WaitResolved/AskTimedOut) instead of synthesizing TimerCaptured
    // or TimerFired entries the journal cannot authoritatively support.
    proj.record(&RuntimeJournalEvent::WaitScheduled { run, step })
        .expect("record");
    proj.record(&RuntimeJournalEvent::AskScheduled { run, step })
        .expect("record");
    proj.record(&RuntimeJournalEvent::WaitResolved { run, step })
        .expect("record");
    proj.record(&RuntimeJournalEvent::AskTimedOut { run, step })
        .expect("record");
    let snap = shared.snapshot().expect("snapshot");
    assert_eq!(snap.len(), 4);
    for entry in &snap {
        assert!(
            !matches!(
                entry.event,
                BoundaryEvent::TimerCaptured { .. } | BoundaryEvent::TimerFired { .. }
            ),
            "journal projection must not synthesize timer authority events"
        );
    }
}

/// **Blocker 2 (legacy completion)** — the projection must handle
/// both the modern `ActionCompletedEnvelope` path AND the legacy
/// `ActionCompleted` path so the boundary transcript reflects both.
#[test]
fn journal_projection_handles_modern_and_legacy_completion_paths() {
    let shared = SharedBoundaryTranscript::with_capacity(8);
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let run = RunId::new(1);
    let modern_ticket = test_ticket(run, 1);
    let modern = RuntimeJournalEvent::ActionCompletedEnvelope {
        ticket: modern_ticket,
        output: SlotIdx::new(4),
        value: vec![0xAB, 0xCD],
        encoded_len: 2,
        taint: Taint::Clean,
        value_digest: [0x42; 32],
        action_abi_digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
    };
    proj.record(&modern).expect("record");
    let legacy = RuntimeJournalEvent::ActionCompleted {
        run,
        step: StepIdx::new(5),
        action: ActionId::new(7),
    };
    proj.record(&legacy).expect("record");
    let snap = shared.snapshot().expect("snapshot");
    assert_eq!(snap.len(), 2);
    match &snap[0].event {
        BoundaryEvent::ActionCompletedModern {
            run: r,
            ticket,
            output_slot,
            value_digest,
            ..
        } => {
            assert_eq!(*r, run);
            assert_eq!(*ticket, modern_ticket);
            assert_eq!(*output_slot, SlotIdx::new(4));
            assert_eq!(*value_digest, [0x42; 32]);
        }
        other => panic!("expected ActionCompletedModern, got {other:?}"),
    }
    match &snap[1].event {
        BoundaryEvent::ActionCompletedLegacy {
            run: r,
            step,
            action,
        } => {
            assert_eq!(*r, run);
            assert_eq!(*step, StepIdx::new(5));
            assert_eq!(*action, ActionId::new(7));
        }
        other => panic!("expected ActionCompletedLegacy, got {other:?}"),
    }
}

/// **Blocker 3 (payload depth)** — the journal projection of an
/// `AskAnswered` event must carry `slot`, and the direct capture API
/// must allow the runtime to push the full payload (`taint`,
/// `encoded_len`, `resume_step`).
#[test]
fn ask_answered_direct_capture_carries_full_payload() {
    let shared = SharedBoundaryTranscript::with_capacity(8);
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let authority = AskAnswerAuthority::new(
        RunId::new(1),
        StepIdx::new(2),
        StepIdx::new(3),
        SlotIdx::new(7),
        Taint::Secret,
        /* encoded_len */ 64,
    );
    let seq = proj.record_ask_answered(&authority).expect("record");
    assert_eq!(seq, Some(0));
    let snap = shared.snapshot().expect("snapshot");
    assert_eq!(snap.len(), 1);
    match &snap[0].event {
        BoundaryEvent::AskAnswered {
            run,
            ask_step,
            resume_step,
            slot,
            taint,
            encoded_len,
        } => {
            assert_eq!(*run, RunId::new(1));
            assert_eq!(*ask_step, StepIdx::new(2));
            assert_eq!(*resume_step, StepIdx::new(3));
            assert_eq!(*slot, SlotIdx::new(7));
            assert_eq!(*taint, Taint::Secret);
            assert_eq!(*encoded_len, 64);
        }
        other => panic!("expected AskAnswered, got {other:?}"),
    }
}

#[test]
fn action_failed_direct_capture_carries_full_failure_payload() {
    let shared = SharedBoundaryTranscript::with_capacity(8);
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let authority = FailureAuthority::new(
        RunId::new(1),
        StepIdx::new(5),
        ActionId::new(9),
        /* attempt */ 2,
        FailureCodeTag::from(ActionFailureCode::Timeout),
        RetryPolicyTag::from(RetryPolicy::Retryable),
        Taint::DerivedFromSecret,
    );
    let seq = proj.record_action_failed(&authority).expect("record");
    assert_eq!(seq, Some(0));
    let snap = shared.snapshot().expect("snapshot");
    assert_eq!(snap.len(), 1);
    match &snap[0].event {
        BoundaryEvent::ActionFailed {
            run,
            step,
            action,
            attempt,
            failure_code,
            retry_policy_tag,
            taint,
        } => {
            assert_eq!(*run, RunId::new(1));
            assert_eq!(*step, StepIdx::new(5));
            assert_eq!(*action, ActionId::new(9));
            assert_eq!(*attempt, 2);
            assert_eq!(*failure_code, ActionFailureCode::Timeout as u8);
            assert_eq!(*retry_policy_tag, 1);
            assert_eq!(*taint, Taint::DerivedFromSecret);
        }
        other => panic!("expected ActionFailed, got {other:?}"),
    }
}

/// **Blocker 4 (parity)** — two identical seeded journal event
/// streams, projected through two independent transcript instances,
/// must produce byte-equal transcripts (same sequence numbers, same
/// variant kinds, same payload fields).
#[test]
fn parity_two_transcripts_produce_identical_projections() {
    let shared_a = SharedBoundaryTranscript::with_capacity(64);
    let shared_b = SharedBoundaryTranscript::with_capacity(64);
    let proj_a = BoundaryTranscriptJournal::new(shared_a.clone());
    let proj_b = BoundaryTranscriptJournal::new(shared_b.clone());
    let run = RunId::new(1);
    let ticket = test_ticket(run, 1);
    let seeded_events: Vec<RuntimeJournalEvent> = vec![
        RuntimeJournalEvent::ActionScheduledTicket {
            ticket,
            input: SlotIdx::new(0),
            output: SlotIdx::new(1),
            action_abi_digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        },
        RuntimeJournalEvent::ActionCompletedEnvelope {
            ticket,
            output: SlotIdx::new(1),
            value: vec![0x01, 0x02, 0x03],
            encoded_len: 3,
            taint: Taint::Clean,
            value_digest: [0xAA; 32],
            action_abi_digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        },
        RuntimeJournalEvent::AskScheduled {
            run,
            step: StepIdx::new(2),
        },
        RuntimeJournalEvent::WaitResolved {
            run,
            step: StepIdx::new(3),
        },
        RuntimeJournalEvent::AskTimedOut {
            run,
            step: StepIdx::new(4),
        },
        RuntimeJournalEvent::ActionAbandoned { ticket },
        RuntimeJournalEvent::ActionFailed {
            run,
            step: StepIdx::new(5),
            action: ActionId::new(7),
            attempt: 1,
        },
    ];
    for event in &seeded_events {
        proj_a.record(event).expect("record a");
        proj_b.record(event).expect("record b");
    }
    let snap_a = shared_a.snapshot().expect("snapshot a");
    let snap_b = shared_b.snapshot().expect("snapshot b");
    assert_eq!(snap_a.len(), snap_b.len());
    assert_eq!(snap_a.len(), seeded_events.len());
    for (a, b) in snap_a.iter().zip(snap_b.iter()) {
        assert_eq!(a.seq, b.seq, "sequence numbers must match");
        assert_eq!(a.event, b.event, "events must match exactly");
    }
}

/// **Blocker 4 (parity + capacity rollback)** — when a transcript
/// reaches capacity, the surviving tail must still produce a
/// byte-equal projection on two independent runs (parity is
/// preserved across capacity overflow).
#[test]
fn parity_survives_capacity_rollback() {
    let shared_a = SharedBoundaryTranscript::with_capacity(4);
    let shared_b = SharedBoundaryTranscript::with_capacity(4);
    let proj_a = BoundaryTranscriptJournal::new(shared_a.clone());
    let proj_b = BoundaryTranscriptJournal::new(shared_b.clone());
    let run = RunId::new(1);
    // Push 8 events; only the last 4 should survive.
    let events: Vec<RuntimeJournalEvent> = (0..8u16)
        .map(|i| RuntimeJournalEvent::WaitScheduled {
            run,
            step: StepIdx::new(i),
        })
        .collect();
    for event in &events {
        proj_a.record(event).expect("record a");
        proj_b.record(event).expect("record b");
    }
    // Capacity overflow dropped 4 entries.
    assert_eq!(shared_a.dropped().expect("dropped a"), 4);
    assert_eq!(shared_b.dropped().expect("dropped b"), 4);
    assert_eq!(shared_a.len().expect("len a"), 4);
    // The surviving tail is identical on both transcripts.
    let snap_a = shared_a.snapshot().expect("snapshot a");
    let snap_b = shared_b.snapshot().expect("snapshot b");
    assert_eq!(snap_a, snap_b);
    // Sequence numbers prove monotonicity survived the overflow.
    assert_eq!(snap_a[0].seq, 4);
    assert_eq!(snap_a[3].seq, 7);
    // Confirm the oldest survivor is event index 4 (run=1, step=4),
    // not event index 0 (which was the first to be dropped).
    for (i, entry) in snap_a.iter().enumerate() {
        let expected_step = (4 + i) as u16;
        match &entry.event {
            BoundaryEvent::WaitScheduled { step, .. } => {
                assert_eq!(step.get(), expected_step);
            }
            other => panic!("expected WaitScheduled, got {other:?}"),
        }
    }
}

#[test]
fn run_id_extractor_returns_correct_run_for_all_variants() {
    let run = RunId::new(99);
    let ticket = test_ticket(run, 1);
    let cases: Vec<BoundaryEvent> = vec![
        BoundaryEvent::ActionScheduled { run, ticket },
        BoundaryEvent::ActionScheduledLegacy {
            run,
            step: StepIdx::new(0),
            action: ActionId::new(1),
        },
        BoundaryEvent::ActionCompletedModern {
            run,
            ticket,
            output_slot: SlotIdx::new(0),
            encoded_len: 0,
            taint: Taint::Clean,
            value_digest: [0; 32],
        },
        BoundaryEvent::ActionCompletedLegacy {
            run,
            step: StepIdx::new(0),
            action: ActionId::new(1),
        },
        BoundaryEvent::ActionFailed {
            run,
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
            failure_code: 0,
            retry_policy_tag: 0,
            taint: Taint::Clean,
        },
        BoundaryEvent::ActionAbandoned { run, ticket },
        BoundaryEvent::AskScheduled {
            run,
            step: StepIdx::new(0),
        },
        BoundaryEvent::AskAnswered {
            run,
            ask_step: StepIdx::new(0),
            resume_step: StepIdx::new(0),
            slot: SlotIdx::new(0),
            taint: Taint::Clean,
            encoded_len: 0,
        },
        BoundaryEvent::AskTimedOut {
            run,
            step: StepIdx::new(0),
        },
        BoundaryEvent::WaitScheduled {
            run,
            step: StepIdx::new(0),
        },
        BoundaryEvent::WaitResolved {
            run,
            step: StepIdx::new(0),
        },
        BoundaryEvent::TimerCaptured {
            run,
            step: StepIdx::new(0),
            kind: PendingTimerKind::Wait,
            generation: 0,
            deadline: std::time::Instant::now(),
            logical_deadline: 0,
        },
        BoundaryEvent::TimerFired {
            run,
            step: StepIdx::new(0),
            kind: PendingTimerKind::Wait,
            generation: 0,
            deadline: std::time::Instant::now(),
        },
    ];
    assert_eq!(cases.len(), 13);
    for event in &cases {
        assert_eq!(event.run_id(), run);
        assert!(!event.kind().is_empty());
    }
}

#[test]
fn event_kind_strings_are_distinct() {
    let run = RunId::new(1);
    let ticket = test_ticket(run, 1);
    let variants = [
        BoundaryEvent::ActionScheduled { run, ticket }.kind(),
        BoundaryEvent::ActionScheduledLegacy {
            run,
            step: StepIdx::new(0),
            action: ActionId::new(1),
        }
        .kind(),
        BoundaryEvent::ActionCompletedModern {
            run,
            ticket,
            output_slot: SlotIdx::new(0),
            encoded_len: 0,
            taint: Taint::Clean,
            value_digest: [0; 32],
        }
        .kind(),
        BoundaryEvent::ActionCompletedLegacy {
            run,
            step: StepIdx::new(0),
            action: ActionId::new(1),
        }
        .kind(),
        BoundaryEvent::ActionFailed {
            run,
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
            failure_code: 0,
            retry_policy_tag: 0,
            taint: Taint::Clean,
        }
        .kind(),
        BoundaryEvent::ActionAbandoned { run, ticket }.kind(),
        BoundaryEvent::AskScheduled {
            run,
            step: StepIdx::new(0),
        }
        .kind(),
        BoundaryEvent::AskAnswered {
            run,
            ask_step: StepIdx::new(0),
            resume_step: StepIdx::new(0),
            slot: SlotIdx::new(0),
            taint: Taint::Clean,
            encoded_len: 0,
        }
        .kind(),
        BoundaryEvent::AskTimedOut {
            run,
            step: StepIdx::new(0),
        }
        .kind(),
        BoundaryEvent::WaitScheduled {
            run,
            step: StepIdx::new(0),
        }
        .kind(),
        BoundaryEvent::WaitResolved {
            run,
            step: StepIdx::new(0),
        }
        .kind(),
        BoundaryEvent::TimerCaptured {
            run,
            step: StepIdx::new(0),
            kind: PendingTimerKind::Wait,
            generation: 0,
            deadline: std::time::Instant::now(),
            logical_deadline: 0,
        }
        .kind(),
        BoundaryEvent::TimerFired {
            run,
            step: StepIdx::new(0),
            kind: PendingTimerKind::Wait,
            generation: 0,
            deadline: std::time::Instant::now(),
        }
        .kind(),
    ];
    // The Set type would tell us they're unique; we just compare
    // pairwise because allocating a HashSet in tests is overkill.
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(
                variants[i], variants[j],
                "variant kinds must be distinct (i={i}, j={j})"
            );
        }
    }
}

/// Helper: exercises a round-trip of the `PushEvent` -> Snapshot
/// path so the test runner sees real `SlotValue`/non-trivial types.
#[test]
fn snapshot_round_trip_preserves_full_payload() {
    let mut t = BoundaryTranscript::with_capacity(2);
    let ticket = test_ticket(RunId::new(1), 1);
    let _ = t
        .push(BoundaryEvent::ActionCompletedModern {
            run: ticket.run,
            ticket,
            output_slot: SlotIdx::new(9),
            encoded_len: 16,
            taint: Taint::Random,
            value_digest: [0xCC; 32],
        })
        .expect("push");
    let snap = t.snapshot();
    assert_eq!(snap.len(), 1);
    match &snap[0].event {
        BoundaryEvent::ActionCompletedModern {
            output_slot,
            encoded_len,
            taint,
            value_digest,
            ..
        } => {
            assert_eq!(*output_slot, SlotIdx::new(9));
            assert_eq!(*encoded_len, 16);
            assert_eq!(*taint, Taint::Random);
            assert_eq!(*value_digest, [0xCC; 32]);
        }
        other => panic!("expected ActionCompletedModern, got {other:?}"),
    }
    // Use SlotValue so the import isn't dead-code.
    let _value = SlotValue::Bool(true);
}
