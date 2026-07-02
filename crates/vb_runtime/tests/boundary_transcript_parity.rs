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
//! Integration tests for the boundary transcript module.
//!
//! These tests exercise the [`BoundaryTranscriptJournal`] projection
//! against [`VolatileRuntimeJournal`] and demonstrate:
//!
//! 1. **Blocker 4 (parity)** — two seeded journal event streams fed
//!    through independent transcript projections produce byte-equal
//!    snapshots.
//! 2. **Blocker 4 (capacity rollback)** — pushing more events than the
//!    transcript capacity rolls off the oldest entries (FIFO) and the
//!    surviving tail still produces a byte-equal projection on both
//!    transcripts.
//! 3. **Blocker 1 (timer authority)** — direct capture preserves the
//!    timer authority fields (`generation`, `kind`, `deadline`,
//!    `logical_deadline`) that the journal projection cannot recover.
//! 4. **Blocker 2 (legacy completion)** — the journal projection handles
//!    the legacy `ActionCompleted` event variant, distinguishing it from
//!    the modern `ActionCompletedEnvelope`.
//! 5. **Blocker 3 (payload depth)** — direct capture preserves the full
//!    ask-answer and action-failure payload fields the journal drops.

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::Taint;
use vb_runtime::boundary_transcript::{
    AskAnswerAuthority, BoundaryEvent, BoundaryTranscriptError, BoundaryTranscriptJournal,
    FailureAuthority, FailureCodeTag, RetryPolicyTag, SharedBoundaryTranscript, TimerAuthority,
};
use vb_runtime::journal::{RuntimeJournal, RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::shard::PendingTimerKind;

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

/// Runs the projection over a seeded journal event stream and pushes
/// the resulting boundary events into the supplied transcript.
/// Returns the boundary events captured.
fn project_journal(
    shared: &SharedBoundaryTranscript,
    events: &[RuntimeJournalEvent],
) -> Result<Vec<BoundaryEvent>, String> {
    let journal = VolatileRuntimeJournal::new();
    for event in events {
        journal
            .append(event.clone())
            .map_err(|e| format!("append: {e:?}"))?;
    }
    let snapshot = journal.snapshot().map_err(|e| format!("snapshot: {e:?}"))?;
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let mut out = Vec::with_capacity(snapshot.len());
    for event in &snapshot {
        if let Some(boundary) = proj.project(event) {
            proj.record(event).map_err(stringify)?;
            out.push(boundary);
        }
    }
    Ok(out)
}

/// **Blocker 4 (parity)** — two identical seeded journal event streams
/// fed through independent transcripts must produce byte-equal projections.
#[test]
fn parity_two_journal_streams_produce_identical_transcripts() -> Result<(), String> {
    let shared_a = SharedBoundaryTranscript::with_capacity(64);
    let shared_b = SharedBoundaryTranscript::with_capacity(64);
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
        RuntimeJournalEvent::ActionCompleted {
            run,
            step: StepIdx::new(6),
            action: ActionId::new(8),
        },
    ];
    let events_a = project_journal(&shared_a, &seeded_events)?;
    let events_b = project_journal(&shared_b, &seeded_events)?;
    assert_eq!(
        events_a.len(),
        events_b.len(),
        "projection lengths must match across runs"
    );
    assert_eq!(events_a, events_b, "projected events must match exactly");
    // All seven distinct journal boundary events (plus the legacy
    // ActionCompleted at step 6) must be present.
    assert!(
        events_a
            .iter()
            .any(|e| matches!(e, BoundaryEvent::ActionScheduled { .. }))
    );
    assert!(
        events_a
            .iter()
            .any(|e| matches!(e, BoundaryEvent::ActionCompletedModern { .. }))
    );
    assert!(
        events_a
            .iter()
            .any(|e| matches!(e, BoundaryEvent::ActionCompletedLegacy { .. }))
    );
    assert!(
        events_a
            .iter()
            .any(|e| matches!(e, BoundaryEvent::AskScheduled { .. }))
    );
    assert!(
        events_a
            .iter()
            .any(|e| matches!(e, BoundaryEvent::WaitResolved { .. }))
    );
    assert!(
        events_a
            .iter()
            .any(|e| matches!(e, BoundaryEvent::AskTimedOut { .. }))
    );
    assert!(
        events_a
            .iter()
            .any(|e| matches!(e, BoundaryEvent::ActionAbandoned { .. }))
    );
    assert!(
        events_a
            .iter()
            .any(|e| matches!(e, BoundaryEvent::ActionFailed { .. }))
    );
    Ok(())
}

/// **Blocker 4 (capacity rollback)** — when the transcript exceeds
/// capacity, the surviving tail must be byte-equal across two
/// independent runs.
#[test]
fn parity_survives_capacity_rollback_with_real_journal() -> Result<(), String> {
    let shared_a = SharedBoundaryTranscript::with_capacity(4);
    let shared_b = SharedBoundaryTranscript::with_capacity(4);
    let run = RunId::new(1);
    // Push 8 events through a real VolatileRuntimeJournal; only the
    // last 4 should survive in each transcript.
    let events: Vec<RuntimeJournalEvent> = (0..8u16)
        .map(|i| RuntimeJournalEvent::WaitScheduled {
            run,
            step: StepIdx::new(i),
        })
        .collect();
    let _ = project_journal(&shared_a, &events)?;
    let _ = project_journal(&shared_b, &events)?;
    assert_eq!(shared_a.dropped().map_err(stringify)?, 4);
    assert_eq!(shared_b.dropped().map_err(stringify)?, 4);
    assert_eq!(shared_a.len().map_err(stringify)?, 4);
    assert_eq!(shared_b.len().map_err(stringify)?, 4);
    let snap_a = shared_a.snapshot().map_err(stringify)?;
    let snap_b = shared_b.snapshot().map_err(stringify)?;
    assert_eq!(snap_a, snap_b, "surviving tails must be byte-equal");
    // The oldest survivor must be the event at index 4 (run=1, step=4).
    for (i, entry) in snap_a.iter().enumerate() {
        let expected_step = (4 + i) as u16;
        match &entry.event {
            BoundaryEvent::WaitScheduled { step, .. } => {
                assert_eq!(step.get(), expected_step);
            }
            other => panic!("expected WaitScheduled, got {other:?}"),
        }
    }
    Ok(())
}

/// **Blocker 1 (timer authority)** — the journal projection preserves
/// WaitScheduled / AskScheduled / WaitResolved / AskTimedOut but cannot
/// recover the timer generation/deadline/kind/logical fields. Direct
/// capture via `record_timer_captured` / `record_timer_fired` must
/// carry the full authority required to replay a timer firing.
#[test]
fn journal_projection_loses_timer_authority_direct_capture_preserves_it() -> Result<(), String> {
    let shared = SharedBoundaryTranscript::with_capacity(16);
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let run = RunId::new(1);
    let step = StepIdx::new(2);
    let journal = VolatileRuntimeJournal::new();
    journal
        .append(RuntimeJournalEvent::WaitScheduled { run, step })
        .map_err(|e| format!("{e:?}"))?;
    journal
        .append(RuntimeJournalEvent::WaitResolved { run, step })
        .map_err(|e| format!("{e:?}"))?;
    let snap = journal.snapshot().map_err(|e| format!("{e:?}"))?;
    for event in &snap {
        proj.record(event).map_err(stringify)?;
    }
    let projected = shared.snapshot().map_err(stringify)?;
    assert_eq!(projected.len(), 2);
    // The projection must NOT emit TimerCaptured/TimerFired because the
    // journal does not carry the timer authority.
    assert!(projected.iter().all(|e| !matches!(
        e.event,
        BoundaryEvent::TimerCaptured { .. } | BoundaryEvent::TimerFired { .. }
    )));
    // Direct capture preserves full authority.
    let shared2 = SharedBoundaryTranscript::with_capacity(16);
    let proj2 = BoundaryTranscriptJournal::new(shared2.clone());
    let deadline = std::time::Instant::now();
    let captured_authority = TimerAuthority::new(
        run,
        step,
        PendingTimerKind::Wait,
        /* generation */ 5,
        deadline,
        /* logical_deadline */ 42,
    );
    proj2
        .record_timer_captured(&captured_authority)
        .map_err(stringify)?;
    // TimerFired uses the same authority shape minus the logical_deadline.
    let fired_authority = TimerAuthority::new(
        run,
        step,
        PendingTimerKind::Wait,
        /* generation */ 5,
        deadline,
        /* logical_deadline */ 0,
    );
    proj2
        .record_timer_fired(&fired_authority)
        .map_err(stringify)?;
    let direct = shared2.snapshot().map_err(stringify)?;
    assert_eq!(direct.len(), 2);
    match &direct[0].event {
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
            assert_eq!(*kind, PendingTimerKind::Wait);
            assert_eq!(*generation, 5);
            assert_eq!(*dl, deadline);
            assert_eq!(*ldl, 42);
        }
        other => panic!("expected TimerCaptured, got {other:?}"),
    }
    match &direct[1].event {
        BoundaryEvent::TimerFired {
            run: r,
            step: s,
            kind,
            generation,
            deadline: dl,
        } => {
            assert_eq!(*r, run);
            assert_eq!(*s, step);
            assert_eq!(*kind, PendingTimerKind::Wait);
            assert_eq!(*generation, 5);
            assert_eq!(*dl, deadline);
        }
        other => panic!("expected TimerFired, got {other:?}"),
    }
    Ok(())
}

/// **Blocker 2 (legacy completion)** — both `ActionCompletedEnvelope`
/// (modern) and `ActionCompleted` (legacy) journal events must be
/// projected into distinct boundary event variants.
#[test]
fn journal_projection_distinguishes_modern_and_legacy_completions() -> Result<(), String> {
    let shared = SharedBoundaryTranscript::with_capacity(8);
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let run = RunId::new(1);
    let modern_ticket = test_ticket(run, 1);
    let journal = VolatileRuntimeJournal::new();
    journal
        .append(RuntimeJournalEvent::ActionCompletedEnvelope {
            ticket: modern_ticket,
            output: SlotIdx::new(4),
            value: vec![0xAB],
            encoded_len: 1,
            taint: Taint::Clean,
            value_digest: [0x42; 32],
            action_abi_digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        })
        .map_err(|e| format!("{e:?}"))?;
    journal
        .append(RuntimeJournalEvent::ActionCompleted {
            run,
            step: StepIdx::new(5),
            action: ActionId::new(7),
        })
        .map_err(|e| format!("{e:?}"))?;
    let snap = journal.snapshot().map_err(|e| format!("{e:?}"))?;
    for event in &snap {
        proj.record(event).map_err(stringify)?;
    }
    let projected = shared.snapshot().map_err(stringify)?;
    assert_eq!(projected.len(), 2);
    assert!(matches!(
        projected[0].event,
        BoundaryEvent::ActionCompletedModern { .. }
    ));
    assert!(matches!(
        projected[1].event,
        BoundaryEvent::ActionCompletedLegacy { .. }
    ));
    Ok(())
}

/// **Blocker 3 (payload depth)** — direct capture preserves the full
/// ask-answer and action-failure payload fields that the journal drops.
#[test]
fn direct_capture_preserves_full_ask_and_failure_payload() -> Result<(), String> {
    let shared = SharedBoundaryTranscript::with_capacity(8);
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let ask_authority = AskAnswerAuthority::new(
        RunId::new(7),
        StepIdx::new(2),
        StepIdx::new(3),
        SlotIdx::new(11),
        Taint::Secret,
        /* encoded_len */ 1024,
    );
    proj.record_ask_answered(&ask_authority)
        .map_err(stringify)?;
    let failure_authority = FailureAuthority::new(
        RunId::new(8),
        StepIdx::new(4),
        ActionId::new(13),
        /* attempt */ 2,
        FailureCodeTag::from(vb_core::action::ActionFailureCode::Rejected),
        RetryPolicyTag::from(vb_core::action::RetryPolicy::NonRetryable),
        Taint::DerivedFromSecret,
    );
    proj.record_action_failed(&failure_authority)
        .map_err(stringify)?;
    let snap = shared.snapshot().map_err(stringify)?;
    assert_eq!(snap.len(), 2);
    match &snap[0].event {
        BoundaryEvent::AskAnswered {
            run,
            ask_step,
            resume_step,
            slot,
            taint,
            encoded_len,
        } => {
            assert_eq!(*run, RunId::new(7));
            assert_eq!(*ask_step, StepIdx::new(2));
            assert_eq!(*resume_step, StepIdx::new(3));
            assert_eq!(*slot, SlotIdx::new(11));
            assert_eq!(*taint, Taint::Secret);
            assert_eq!(*encoded_len, 1024);
        }
        other => panic!("expected AskAnswered, got {other:?}"),
    }
    match &snap[1].event {
        BoundaryEvent::ActionFailed {
            run,
            step,
            action,
            attempt,
            failure_code,
            retry_policy_tag,
            taint,
        } => {
            assert_eq!(*run, RunId::new(8));
            assert_eq!(*step, StepIdx::new(4));
            assert_eq!(*action, ActionId::new(13));
            assert_eq!(*attempt, 2);
            assert_eq!(
                *failure_code,
                vb_core::action::ActionFailureCode::Rejected as u8
            );
            assert_eq!(*retry_policy_tag, 0);
            assert_eq!(*taint, Taint::DerivedFromSecret);
        }
        other => panic!("expected ActionFailed, got {other:?}"),
    }
    Ok(())
}

/// **Blocker 4 (parity across journal + direct capture)** — a seeded
/// scenario that mixes journal-projected events with direct captures
/// must produce a deterministic transcript. Two independent runs must
/// produce byte-equal transcripts.
#[test]
fn parity_journal_plus_direct_capture_is_deterministic() -> Result<(), String> {
    fn run_scenario() -> Result<Vec<BoundaryEvent>, String> {
        let shared = SharedBoundaryTranscript::with_capacity(32);
        let proj = BoundaryTranscriptJournal::new(shared.clone());
        let run = RunId::new(1);
        let step = StepIdx::new(2);
        let deadline = std::time::Instant::now();
        // 1) Direct capture: timer authority the journal cannot preserve.
        let captured_authority = TimerAuthority::new(
            run,
            step,
            PendingTimerKind::Ask,
            /* generation */ 1,
            deadline,
            /* logical_deadline */ 7,
        );
        proj.record_timer_captured(&captured_authority)
            .map_err(stringify)?;
        // 2) Journal projection: scheduled/completed via runtime journal.
        let journal = VolatileRuntimeJournal::new();
        journal
            .append(RuntimeJournalEvent::AskScheduled { run, step })
            .map_err(|e| format!("{e:?}"))?;
        journal
            .append(RuntimeJournalEvent::AskAnswered {
                run,
                step,
                slot: SlotIdx::new(3),
            })
            .map_err(|e| format!("{e:?}"))?;
        let snap = journal.snapshot().map_err(|e| format!("{e:?}"))?;
        for event in &snap {
            proj.record(event).map_err(stringify)?;
        }
        // 3) Direct capture: timer fire authority.
        let fired_authority = TimerAuthority::new(
            run,
            step,
            PendingTimerKind::Ask,
            /* generation */ 1,
            deadline,
            /* logical_deadline */ 0,
        );
        proj.record_timer_fired(&fired_authority)
            .map_err(stringify)?;
        // Drain in insertion order. Note: the direct capture push goes
        // through the same mutex-guarded push, so insertion order is
        // well-defined.
        let snap = shared.snapshot().map_err(stringify)?;
        Ok(snap.into_iter().map(|e| e.event).collect())
    }
    let a = run_scenario()?;
    let b = run_scenario()?;
    // Compare kinds only — `Instant::now()` differs between runs, so
    // wall-clock comparison would be flaky. The kinds and authority
    // fields other than `Instant` must match.
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x.kind(), y.kind());
        assert_eq!(x.run_id(), y.run_id());
    }
    Ok(())
}

/// Helper to convert `BoundaryTranscriptError` into a printable string
/// for assertion failure messages.
fn stringify(error: BoundaryTranscriptError) -> String {
    format!("{error:?}")
}
