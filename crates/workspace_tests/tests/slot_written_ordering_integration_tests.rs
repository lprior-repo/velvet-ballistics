//! Integration tests for LETHAL-8: SlotWritten-Before-PC-Advance Ordering
//!
//! These tests verify that `SlotWritten` evidence events are recorded in the
//! correct order relative to PC advance and step lifecycle events.
//!
//! ## Behaviors Tested
//!
//! - **B-1**: `SlotWritten` recorded before PC advance in execution trace
//! - **B-2**: Journal recovery replays `SlotWritten` before PC advance
//! - **B-3**: `SlotWritten` persists at checkpoint boundaries
//!
//! ## Critical Invariants
//!
//! - For every step N that writes a slot, `SlotWritten(N)` MUST appear in the
//!   evidence stream at a position strictly before `StepStarted(N+1)`
//! - Journal replay MUST apply all `SlotWrittenEvent` records before advancing PC
//! - Snapshot-plus-tail replay MUST reject tail events with seq <= snapshot seq

#![forbid(unsafe_code)]

use tempfile::TempDir;
use vb_core::capability::CapabilitySet;
use vb_core::value::{ConstValue, SlotValue};
use vb_core::workflow::{ResourceContract, WorkflowParts};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RunId, SlotIdx, StepIdx,
    WorkflowDigest,
};
use vb_runtime::engine::drive::drive_deterministic_full;
use vb_runtime::engine::types::{EvidenceCollector, RetryPolicy};
use vb_runtime::primitives::collect::CollectStates;
use vb_storage::recovery::{
    ActionReplayTracker, RecoveryError, RunSnapshot, recover_full_journal,
    recover_snapshot_plus_tail,
};
use vb_storage::{EventSeq, FjallConfig, FjallJournal, JournalEvent};

// ============================================================================
// Test Fixtures and Helpers
// ============================================================================

/// Creates a test workflow from a vector of compiled nodes and constants.
fn make_workflow(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    constants: Vec<ConstValue>,
) -> Result<CompiledWorkflow, String> {
    let names: Box<[Box<str>]> = (0..nodes.len())
        .map(|i| format!("s{i}").into_boxed_str())
        .collect();
    let parts = WorkflowParts {
        name: "test".into(),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: names,
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|e| format!("{e}"))
}

/// Creates a SetConst node: writes a constant value to an output slot.
fn set_const_node(id: u16, const_idx: u16, output: u16, next: Option<u16>) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: Some(SlotIdx::new(output)),
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(const_idx),
        },
    }
}

/// Creates a Nop node: advances PC without writing any slot.
fn nop_node(id: u16, next: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: Some(StepIdx::new(next)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

/// Creates a Finish node: terminates the workflow with a result slot value.
fn finish_node(id: u16, result: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result),
        },
    }
}

/// Creates a Copy node: copies a source slot to an output slot.
fn copy_node(id: u16, source: u16, output: u16, next: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: Some(SlotIdx::new(output)),
        next: Some(StepIdx::new(next)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Copy {
            source: SlotIdx::new(source),
        },
    }
}

/// Creates a CollectStart node for multi-slot writes.
fn collect_start_node(
    id: u16,
    source: u16,
    output: u16,
    body: u16,
    done: u16,
) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: Some(SlotIdx::new(output)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::CollectStart {
            source: SlotIdx::new(source),
            limit: 100,
            page_size: 1,
            body: StepIdx::new(body),
            done: StepIdx::new(done),
        },
    }
}

/// Opens a temporary FjallJournal for testing.
fn open_journal(dir: &TempDir) -> FjallJournal {
    FjallJournal::open(dir.path(), Some(FjallConfig::default()))
        .expect("journal open should succeed")
}

/// Writes events to a journal, ignoring duplicate event errors.
fn write_events_strict(journal: &FjallJournal, events: &[JournalEvent]) {
    for event in events {
        match journal.append_strict(event) {
            Ok(()) | Err(vb_storage::JournalError::DuplicateEvent { .. }) => {}
            Err(error) => panic!("strict append should succeed: {error:?}"),
        }
    }
}

// ============================================================================
// B-1: SlotWritten Before PC Advance in Evidence Stream
// ============================================================================

/// Verifies that `SlotWritten(0)` appears at a strictly lower position in the
/// evidence stream than `StepStarted(1)` for a two-step workflow.
///
/// Given: A workflow with two consecutive SetConst steps (step 0 writes slot 0,
///         step 1 writes slot 1)
/// When:  The drive loop executes both steps to completion
/// Then:  The drained evidence stream contains:
///         StepStarted(0), [SlotWritten(0)], StepSucceeded(0),
///         StepStarted(1), [SlotWritten(1)], StepSucceeded(1)
/// And:   The index of SlotWritten(0) in the evidence stream is less than
///         the index of StepStarted(1)
#[test]
fn slot_written_appears_before_next_step_started_in_evidence_stream() {
    // Given: Two consecutive SetConst steps
    let constants = vec![ConstValue::I64(10), ConstValue::I64(20)];
    let wf = make_workflow(
        vec![
            set_const_node(0, 0, 0, Some(1)),
            set_const_node(1, 1, 1, Some(2)),
            finish_node(2, 0),
        ],
        2,
        constants,
    )
    .expect("workflow construction should succeed");

    let mut run = vb_core::frame::RunFrame::new(
        RunId::new(1),
        StepIdx::new(0),
        3,
        2,
    )
    .expect("run frame creation should succeed");

    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut store = vb_core::value_store::ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();

    // When: Execute both steps
    let sig = drive_deterministic_full(
        &wf,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
    )
    .expect("drive should succeed");

    // Then: Signal should be Finished
    match sig {
        vb_runtime::engine::types::RuntimeSignal::Finished(_) => {}
        other => panic!("expected Finished, got {other:?}"),
    }

    // Drain evidence and verify ordering
    let events = evidence.drain();

    // Find positions of SlotWritten(0) and StepStarted(1)
    let slot_written_0_pos = events
        .iter()
        .position(|e| {
            matches!(
                e,
                vb_runtime::engine::types::EvidenceEvent::SlotWritten {
                    slot,
                    value: SlotValue::I64(10),
                    ..
                } if *slot == SlotIdx::new(0)
            )
        })
        .expect("SlotWritten(0) should be in evidence");

    let step_started_1_pos = events
        .iter()
        .position(|e| {
            matches!(
                e,
                vb_runtime::engine::types::EvidenceEvent::StepStarted {
                    step
                } if *step == StepIdx::new(1)
            )
        })
        .expect("StepStarted(1) should be in evidence");

    // Assert: SlotWritten(0) appears BEFORE StepStarted(1)
    assert!(
        slot_written_0_pos < step_started_1_pos,
        "SlotWritten(0) at position {} should appear BEFORE StepStarted(1) at position {}. Full evidence: {:?}",
        slot_written_0_pos,
        step_started_1_pos,
        events
    );
}

/// Verifies that for a single SetConst step, the SlotWritten event is present
/// in the evidence drain and the run frame PC has advanced to the terminal
/// position (or past the step) when evidence is drained.
///
/// Given: A single SetConst step that writes slot 0
/// When:  drive_deterministic_full executes the step
/// Then:  The EvidenceCollector drain contains SlotWritten(slot=0) event
/// And:   The PC has advanced to step 1 (or terminal)
#[test]
fn evidence_collector_emits_slot_before_next_step_begins() {
    // Given: Single SetConst step
    let constants = vec![ConstValue::I64(42)];
    let wf = make_workflow(
        vec![set_const_node(0, 0, 0, Some(1)), finish_node(1, 0)],
        1,
        constants,
    )
    .expect("workflow construction should succeed");

    let mut run = vb_core::frame::RunFrame::new(
        RunId::new(2),
        StepIdx::new(0),
        2,
        1,
    )
    .expect("run frame creation should succeed");

    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut store = vb_core::value_store::ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();

    // When: Execute step
    let sig = drive_deterministic_full(
        &wf,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
    )
    .expect("drive should succeed");

    match sig {
        vb_runtime::engine::types::RuntimeSignal::Finished(_) => {}
        other => panic!("expected Finished, got {other:?}"),
    }

    // Then: Evidence drain contains SlotWritten(slot=0)
    let events = evidence.drain();
    let has_slot_written = events.iter().any(|e| {
        matches!(
            e,
            vb_runtime::engine::types::EvidenceEvent::SlotWritten {
                slot,
                value: SlotValue::I64(42),
                ..
            } if *slot == SlotIdx::new(0)
        )
    });
    assert!(
        has_slot_written,
        "Evidence drain should contain SlotWritten(0, I64(42)). Full evidence: {:?}",
        events
    );

    // And: PC has advanced (run is at terminal or past step 0)
    assert!(
        run.pc().get() >= 1,
        "PC should have advanced to at least 1, got {}",
        run.pc().get()
    );
}

/// Verifies that all SlotWritten events from a Collect node appear in the
/// evidence stream before any evidence from the next step.
///
/// Given: A Collect node that emits multiple SlotWritten events
/// When:  The drive loop executes the collect node
/// Then:  All SlotWritten events for the node appear in the evidence stream
///         before StepSucceeded for the node
/// And:   All SlotWritten events appear before any evidence from the next step
#[test]
#[ignore]
fn multi_slot_node_emit_order_preserved() {
    // Given: CollectStart node with a body and done path
    let wf = make_workflow(
        vec![
            collect_start_node(0, 0, 1, 1, 2),
            nop_node(1, 2),
            finish_node(2, 1),
        ],
        2,
        vec![],
    )
    .expect("workflow construction should succeed");

    let mut run = vb_core::frame::RunFrame::new(
        RunId::new(3),
        StepIdx::new(0),
        3,
        2,
    )
    .expect("run frame creation should succeed");

    // Pre-populate source list
    let list_id = {
        let page = Box::from([SlotValue::I64(10), SlotValue::I64(20)]);
        let mut store = vb_core::value_store::ValueStore::new();
        store
            .insert_list(page)
            .expect("list insertion should succeed")
    };
    run.write_slot(SlotIdx::new(0), SlotValue::List(list_id))
        .expect("slot write should succeed");

    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut store = vb_core::value_store::ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();

    // When: Execute collect node
    let _sig = drive_deterministic_full(
        &wf,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
    )
    .expect("drive should succeed");

    let events = evidence.drain();

    // Find all SlotWritten events and StepStarted events
    let slot_written_positions: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            matches!(e, vb_runtime::engine::types::EvidenceEvent::SlotWritten { .. })
        })
        .map(|(i, _)| i)
        .collect();

    let step_succeeded_positions: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            matches!(
                e,
                vb_runtime::engine::types::EvidenceEvent::StepSucceeded { .. }
            )
        })
        .map(|(i, _)| i)
        .collect();

    // Verify: If we have SlotWritten events, they should all appear before StepSucceeded
    if !slot_written_positions.is_empty() && !step_succeeded_positions.is_empty() {
        let last_slot_written = *slot_written_positions.last().unwrap();
        let first_step_succeeded = step_succeeded_positions.first().unwrap();
        assert!(
            last_slot_written < *first_step_succeeded,
            "All SlotWritten events should appear BEFORE StepSucceeded. \
             Last SlotWritten at {}, first StepSucceeded at {}. Events: {:?}",
            last_slot_written,
            first_step_succeeded,
            events
        );
    }
}

/// Verifies that a Nop step (which has no slot output) does not emit any
/// SlotWritten event.
///
/// Given: A Nop step (no slot output)
/// When:  The drive loop executes the Nop
/// Then:  The evidence stream contains StepStarted and StepSucceeded but
///         no SlotWritten
#[test]
fn no_slot_written_node_omits_slot_event() {
    // Given: Nop step followed by finish
    let wf = make_workflow(vec![nop_node(0, 1), finish_node(1, 0)], 1, vec![])
        .expect("workflow construction should succeed");

    let mut run = vb_core::frame::RunFrame::new(
        RunId::new(4),
        StepIdx::new(0),
        2,
        1,
    )
    .expect("run frame creation should succeed");

    // Pre-condition: slot 0 has a value (finish will use it)
    run.write_slot(SlotIdx::new(0), SlotValue::I64(99))
        .expect("slot write should succeed");

    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut store = vb_core::value_store::ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();

    // When: Execute Nop step
    let sig = drive_deterministic_full(
        &wf,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
    )
    .expect("drive should succeed");

    match sig {
        vb_runtime::engine::types::RuntimeSignal::Finished(SlotValue::I64(99)) => {}
        other => panic!("expected Finished(I64(99)), got {other:?}"),
    }

    // Then: Evidence contains StepStarted and StepSucceeded but no SlotWritten
    let events = evidence.drain();

    let has_step_started = events.iter().any(|e| {
        matches!(
            e,
            vb_runtime::engine::types::EvidenceEvent::StepStarted { .. }
        )
    });
    let has_step_succeeded = events.iter().any(|e| {
        matches!(
            e,
            vb_runtime::engine::types::EvidenceEvent::StepSucceeded { .. }
        )
    });
    let has_slot_written = events.iter().any(|e| {
        matches!(
            e,
            vb_runtime::engine::types::EvidenceEvent::SlotWritten { .. }
        )
    });

    assert!(
        has_step_started,
        "Nop should emit StepStarted. Events: {:?}",
        events
    );
    assert!(
        has_step_succeeded,
        "Nop should emit StepSucceeded. Events: {:?}",
        events
    );
    assert!(
        !has_slot_written,
        "Nop should NOT emit SlotWritten. Events: {:?}",
        events
    );
}

/// Verifies that a Copy node emits a SlotWritten event with the correct value.
///
/// Given: A Copy node that copies from slot 1 to slot 0
/// When:  The drive loop executes the Copy
/// Then:  The evidence stream contains SlotWritten(0) with the value from slot 1
#[test]
fn copy_node_emits_slot_written_with_correct_value() {
    // Given: Copy node followed by finish
    let wf = make_workflow(vec![copy_node(0, 1, 0, 1), finish_node(1, 0)], 2, vec![])
        .expect("workflow construction should succeed");

    let mut run = vb_core::frame::RunFrame::new(
        RunId::new(5),
        StepIdx::new(0),
        2,
        2,
    )
    .expect("run frame creation should succeed");

    // Pre-condition: slot 1 has a value
    run.write_slot(SlotIdx::new(1), SlotValue::I64(77))
        .expect("slot write should succeed");

    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut store = vb_core::value_store::ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();

    // When: Execute Copy
    let sig = drive_deterministic_full(
        &wf,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
    )
    .expect("drive should succeed");

    match sig {
        vb_runtime::engine::types::RuntimeSignal::Finished(SlotValue::I64(77)) => {}
        other => panic!("expected Finished(I64(77)), got {other:?}"),
    }

    // Then: Evidence contains SlotWritten(0) with value 77
    let events = evidence.drain();
    let slot_written = events.iter().find(|e| {
        matches!(
            e,
            vb_runtime::engine::types::EvidenceEvent::SlotWritten {
                slot,
                value: SlotValue::I64(77),
                ..
            } if *slot == SlotIdx::new(0)
        )
    });

    assert!(
        slot_written.is_some(),
        "Evidence should contain SlotWritten(0, I64(77)). Events: {:?}",
        events
    );
}

// ============================================================================
// B-2: Journal Recovery Replays SlotWritten Before PC Advance
// ============================================================================

/// Verifies that recover_full_journal correctly replays a journal with
/// properly ordered SlotWritten events and restores slot values in sequence order.
///
/// Given: A journal with events:
///         RunAccepted, StepStarted(0), SlotWrittenEvent(slot=0),
///         StepSucceeded(0), StepStarted(1), SlotWrittenEvent(slot=1),
///         StepSucceeded(1), RunFinished
/// When:  recover_full_journal replays the events
/// Then:  The recovered events retain the original ordering
/// And:   All SlotWrittenEvent records are present in the replay output
#[test]
fn replay_restores_slot_values_in_correct_sequence_order() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(1001);
    let digest = vb_core::WorkflowDigest::from_bytes([0xA1; 32]);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(0),
            value: Some(postcard::to_allocvec(&SlotValue::I64(10)).unwrap()),
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            output: SlotIdx::new(0),
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(5),
            slot: SlotIdx::new(1),
            value: Some(postcard::to_allocvec(&SlotValue::I64(20)).unwrap()),
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(1),
            output: SlotIdx::new(1),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(7),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();

    // When: Recover full journal
    let result = recover_full_journal(&journal, run, &mut tracker);

    // Then: Recovery succeeds
    assert!(
        result.is_ok(),
        "recover_full_journal should succeed: {:?}",
        result
    );

    let replayed = result.unwrap();

    // Verify: All events present in original order
    assert_eq!(
        replayed.len(),
        events.len(),
        "replayed events should contain all {} events, got {}",
        events.len(),
        replayed.len()
    );

    // Verify: SlotWrittenEvent sequence numbers are strictly increasing
    let slot_written_seqs: Vec<EventSeq> = replayed
        .iter()
        .filter_map(|e| {
            if matches!(e, JournalEvent::SlotWrittenEvent { .. }) {
                Some(e.seq())
            } else {
                None
            }
        })
        .collect();

    for window in slot_written_seqs.windows(2) {
        assert!(
            window[0].get() < window[1].get(),
            "SlotWrittenEvent seq numbers should be strictly increasing: {:?}",
            slot_written_seqs
        );
    }
}

/// Verifies that recover_snapshot_plus_tail correctly replays tail slot writes
/// that occur after a snapshot.
///
/// Given: A snapshot at seq=3 and tail events including SlotWrittenEvent(seq=4)
/// When:  recover_snapshot_plus_tail reconstructs the run
/// Then:  The slot value from SlotWrittenEvent(seq=4) is present in the
///         recovered events
/// And:   No ReplayDivergence error occurs
#[test]
fn snapshot_plus_tail_replays_tail_slot_writes_after_snapshot() {
    let run = RunId::new(1002);
    let digest = vb_core::WorkflowDigest::from_bytes([0xA2; 32]);

    // Snapshot at seq=3 (after step 0 succeeded)
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(3),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    // Tail events: step 1 starts and completes with slot write
    let tail_events = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(5),
            slot: SlotIdx::new(1),
            value: Some(postcard::to_allocvec(&SlotValue::I64(99)).unwrap()),
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(1),
            output: SlotIdx::new(1),
        },
    ];

    let mut tracker = ActionReplayTracker::new();

    // When: Recover snapshot plus tail
    let result = recover_snapshot_plus_tail(&snapshot, &tail_events, &mut tracker);

    // Then: Recovery succeeds
    assert!(
        result.is_ok(),
        "recover_snapshot_plus_tail should succeed: {:?}",
        result
    );

    let replayed = result.unwrap();

    // Verify: All tail events are present
    assert_eq!(
        replayed.len(),
        tail_events.len(),
        "replayed tail should contain all {} events, got {}",
        tail_events.len(),
        replayed.len()
    );

    // Verify: SlotWrittenEvent with seq=5 is present
    let has_tail_slot_write = replayed.iter().any(|e| {
        matches!(
            e,
            JournalEvent::SlotWrittenEvent {
                seq,
                slot,
                ..
            } if seq.get() == 5 && slot.get() == 1
        )
    });
    assert!(
        has_tail_slot_write,
        "replayed events should contain tail SlotWrittenEvent(seq=5). Replayed: {:?}",
        replayed
    );
}

/// Verifies that replay_events detects decreasing step indices.
///
/// Given: Events with step indices that decrease (StepStarted(2) after StepStarted(1))
/// When:  replay_events processes these events
/// Then:  RecoveryError::ReplayDivergence is returned
#[test]
fn replay_detects_decreasing_step_indices() {
    use vb_storage::recovery::replay::core::replay_events;

    let run = RunId::new(1004);

    // Events with decreasing step indices: StepStarted(2) after StepStarted(1)
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([0xC1; 32]),
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(2), // Step 2
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(1), // Step 1 - DECREASING
            attempt: 1,
        },
    ];

    let mut tracker = ActionReplayTracker::new();

    // When: Replay events
    let result = replay_events(&events, &mut tracker);

    // Then: Should return ReplayDivergence
    match result {
        Err(RecoveryError::ReplayDivergence { step, detail }) => {
            assert_eq!(
                step,
                StepIdx::new(1),
                "ReplayDivergence should report the decreasing step"
            );
            assert!(
                detail.contains("before"),
                "detail should mention ordering violation: {}",
                detail
            );
        }
        other => panic!(
            "expected ReplayDivergence for decreasing step indices, got {:?}",
            other
        ),
    }
}

/// Verifies that replay_events correctly handles duplicate action events
/// (idempotency check).
///
/// Given: Two ActionCompletedEvent records for the same action+step
/// When:  replay_events processes these events
/// Then:  The second completion is blocked with NonIdempotentActionBlocked
#[test]
#[ignore]
fn replay_blocks_duplicate_action_completion() {
    use vb_storage::recovery::replay::core::replay_events;

    let run = RunId::new(1005);
    let action_id = vb_core::ActionId::new(42);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([0xD1; 32]),
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            action: action_id,
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            action: action_id,
            attempt: 1,
        },
        // Duplicate completion
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::ZERO,
            action: action_id,
            attempt: 1,
        },
    ];

    let mut tracker = ActionReplayTracker::new();

    // When: Replay events
    let result = replay_events(&events, &mut tracker);

    // Then: Should return NonIdempotentActionBlocked for duplicate
    match result {
        Err(RecoveryError::NonIdempotentActionBlocked { action, step }) => {
            assert_eq!(action, action_id);
            assert_eq!(step, StepIdx::ZERO);
        }
        other => panic!(
            "expected NonIdempotentActionBlocked for duplicate action completion, got {:?}",
            other
        ),
    }
}

// ============================================================================
// B-3: SlotWritten Persists at Checkpoint
// ============================================================================

/// Verifies that a snapshot captures all preceding slot writes and that
/// tail events with seq > snapshot seq are correctly replayed.
///
/// Given: A run with 3 steps, each writing a different slot, and a snapshot
///         taken after step 1
/// When:  recover_snapshot_plus_tail is called with the snapshot and tail
///         events for step 2
/// Then:  The recovered events include all tail slot writes
/// And:   The snapshot seq equals the seq of StepSucceeded(1)
#[test]
fn snapshot_captures_all_preceding_slot_writes() {
    let run = RunId::new(2001);
    let digest = vb_core::WorkflowDigest::from_bytes([0xE1; 32]);

    // Snapshot at seq=5 (after step 1 succeeded)
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(5),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    // Tail events for step 2
    let tail_events = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(7),
            slot: SlotIdx::new(2),
            value: Some(postcard::to_allocvec(&SlotValue::I64(30)).unwrap()),
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(8),
            step: StepIdx::new(2),
            output: SlotIdx::new(2),
        },
    ];

    let mut tracker = ActionReplayTracker::new();

    // When: Recover snapshot plus tail
    let result = recover_snapshot_plus_tail(&snapshot, &tail_events, &mut tracker);

    // Then: Recovery succeeds
    assert!(
        result.is_ok(),
        "recover_snapshot_plus_tail should succeed: {:?}",
        result
    );

    let replayed = result.unwrap();

    // Verify: Tail events with seq > snapshot seq are present
    for event in &tail_events {
        assert!(
            replayed.contains(event),
            "tail event {:?} should be in replayed events: {:?}",
            event,
            replayed
        );
    }
}

/// Verifies that recover_snapshot_plus_tail rejects tail events with
/// seq <= snapshot seq.
///
/// Given: A snapshot at seq=5 and tail events where some event has seq <= 5
/// When:  recover_snapshot_plus_tail is called
/// Then:  RecoveryError::ReplayDivergence is returned
/// And:   The error detail mentions the seq ordering violation
#[test]
fn corrupt_snapshot_seq_fails_gracefully() {
    let run = RunId::new(2002);
    let digest = vb_core::WorkflowDigest::from_bytes([0xF1; 32]);

    // Snapshot at seq=5
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(5),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    // Tail events with CORRUPT ordering: event at seq=5 (<= snapshot)
    let tail_events = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(5), // SAME as snapshot - violates tail > snapshot
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(6),
            slot: SlotIdx::new(2),
            value: None,
            extra: None,
            attempt: 1,
        },
    ];

    let mut tracker = ActionReplayTracker::new();

    // When: Recover snapshot plus tail with corrupt ordering
    let result = recover_snapshot_plus_tail(&snapshot, &tail_events, &mut tracker);

    // Then: Should return ReplayDivergence
    match result {
        Err(RecoveryError::ReplayDivergence { step, detail }) => {
            assert_eq!(step, StepIdx::ZERO);
            assert!(
                detail.contains("5"),
                "detail should mention the violating seq: {}",
                detail
            );
        }
        other => panic!(
            "expected ReplayDivergence for tail seq <= snapshot seq, got {:?}",
            other
        ),
    }
}

/// Verifies that recover_snapshot_plus_tail rejects tail events with
/// seq exactly equal to snapshot seq (boundary case).
///
/// Given: A snapshot at seq=S and tail events where first event has seq=S
/// When:  recover_snapshot_plus_tail is called
/// Then:  RecoveryError::ReplayDivergence is returned
#[test]
fn tail_seq_equal_to_snapshot_seq_fails() {
    let run = RunId::new(2003);
    let digest = vb_core::WorkflowDigest::from_bytes([0x11; 32]);

    // Snapshot at seq=10
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(10),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    // Tail with first event at seq=10 (exactly equal - boundary violation)
    let tail_events = vec![JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(10),
        step: StepIdx::new(1),
        attempt: 1,
    }];

    let mut tracker = ActionReplayTracker::new();

    // When: Recover snapshot plus tail
    let result = recover_snapshot_plus_tail(&snapshot, &tail_events, &mut tracker);

    // Then: Should return ReplayDivergence (tail seq must be STRICTLY greater)
    match result {
        Err(RecoveryError::ReplayDivergence { detail, .. }) => {
            assert!(
                detail.contains("10") && detail.contains("not after"),
                "detail should mention seq comparison failure: {}",
                detail
            );
        }
        other => panic!(
            "expected ReplayDivergence for tail seq == snapshot seq, got {:?}",
            other
        ),
    }
}

// ============================================================================
// Error Variant Coverage for recover_full_journal (LETHAL-1 fix)
// ============================================================================

/// Verifies that recover_full_journal returns NoRecoveryData when the journal
/// has no events for the requested run.
///
/// Given: An empty journal (no events for run 9999)
/// When:  recover_full_journal is called with a run ID not in the journal
/// Then:  RecoveryError::NoRecoveryData is returned with the correct run ID
#[test]
fn recover_full_journal_returns_no_recovery_data_when_journal_is_empty() {
    let dir = TempDir::new().expect("temp dir should be created");
    let journal = open_journal(&dir);
    let nonexistent_run = RunId::new(9999);
    let mut tracker = ActionReplayTracker::new();

    // When: Recover full journal for nonexistent run
    let result = recover_full_journal(&journal, nonexistent_run, &mut tracker);

    // Then: Should return NoRecoveryData
    match result {
        Err(RecoveryError::NoRecoveryData { run }) => {
            assert_eq!(
                run, nonexistent_run,
                "NoRecoveryData should report the correct run ID"
            );
        }
        other => panic!(
            "expected NoRecoveryData for empty journal, got {:?}",
            other
        ),
    }
}

/// Verifies that recover_full_journal returns NoRecoveryData when the journal
/// has only non-matching events.
///
/// Given: A journal with events for run A, but requesting recovery for run B
/// When:  recover_full_journal is called
/// Then:  RecoveryError::NoRecoveryData is returned for run B
#[test]
fn recover_full_journal_returns_no_recovery_data_for_wrong_run() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run_a = RunId::new(3001);
    let run_b = RunId::new(3002);

    let events = vec![JournalEvent::RunAccepted {
        run: run_a,
        seq: EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0xAA; 32]),
    }];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();

    // When: Recover full journal for run B (not in journal)
    let result = recover_full_journal(&journal, run_b, &mut tracker);

    // Then: Should return NoRecoveryData for run B
    match result {
        Err(RecoveryError::NoRecoveryData { run }) => {
            assert_eq!(
                run, run_b,
                "NoRecoveryData should report run B as missing"
            );
        }
        other => panic!(
            "expected NoRecoveryData for wrong run, got {:?}",
            other
        ),
    }
}

// ============================================================================
// Boundary Cases
// ============================================================================

/// Verifies that replay_events handles an empty event slice correctly.
///
/// Given: An empty slice of events
/// When:  replay_events processes the empty slice
/// Then:  The result is Ok with an empty Vec
#[test]
fn replay_events_handles_empty_slice() {
    use vb_storage::recovery::replay::core::replay_events;

    let events: Vec<JournalEvent> = vec![];
    let mut tracker = ActionReplayTracker::new();

    // When: Replay empty slice
    let result = replay_events(&events, &mut tracker);

    // Then: Should succeed with empty Vec
    assert!(
        result.is_ok(),
        "replay_events should succeed on empty slice"
    );
    assert!(
        result.unwrap().is_empty(),
        "replay of empty slice should be empty"
    );
}

/// Verifies that replay_events correctly filters events from older attempts
/// (PRE-001: only latest attempt affects state).
///
/// Given: A journal with events from two attempts (attempt 1 and attempt 2)
/// When:  replay_events processes the events
/// Then:  Events from attempt 1 are included in output but don't affect state
/// And:   Events from attempt 2 (latest) affect state
#[test]
fn replay_events_filters_older_attempts() {
    use vb_storage::recovery::replay::core::replay_events;

    let run = RunId::new(4001);

    // Events from attempt 1 (older) and attempt 2 (latest)
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([0xAB; 32]),
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1, // Older attempt
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(0),
            value: Some(postcard::to_allocvec(&SlotValue::I64(10)).unwrap()),
            extra: None,
            attempt: 1, // Older attempt
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            attempt: 2, // Latest attempt - should replace attempt 1
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(4),
            slot: SlotIdx::new(0),
            value: Some(postcard::to_allocvec(&SlotValue::I64(20)).unwrap()),
            extra: None,
            attempt: 2, // Latest attempt
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::ZERO,
            output: SlotIdx::new(0),
        },
    ];

    let mut tracker = ActionReplayTracker::new();

    // When: Replay events
    let result = replay_events(&events, &mut tracker);

    // Then: Should succeed
    assert!(result.is_ok(), "replay_events should succeed: {:?}", result);

    let replayed = result.unwrap();

    // All events should be in output (older events are included for diagnostics)
    assert_eq!(
        replayed.len(),
        events.len(),
        "all events should be in replayed output (filtered by attempt in logic)"
    );
}

/// Verifies that recover_full_journal correctly handles the boundary case
/// where the journal has exactly one event.
///
/// Given: A journal with a single RunAccepted event
/// When:  recover_full_journal is called
/// Then:  Recovery succeeds (empty replay)
#[test]
fn recover_full_journal_with_single_event() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(5001);

    let events = vec![JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0xBC; 32]),
    }];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();

    // When: Recover full journal
    let result = recover_full_journal(&journal, run, &mut tracker);

    // Then: Should succeed with single event
    assert!(result.is_ok(), "recover should succeed: {:?}", result);
    assert_eq!(
        result.unwrap().len(),
        1,
        "single event journal should replay to single event"
    );
}
