// ============================================================================
// Round 7 typed-rejection coverage (vb-wy33p.11).
//
// These tests close the gaps where the public hydration entry points were
// reachable from production code but no test pinned the typed-rejection
// reason string. Each test exercises one specific reason token against a
// realistic event fixture and asserts the EXACT reason string returned by
// `RecoveryError::UnsupportedFrameSeed { run, reason }`.
//
// The 13 cannot-resume reason tokens are:
//   slot_values, slot_taint, action_payloads, pending_actions,
//   pending_timers, pending_asks, workflow_missing, store_missing,
//   action_attempts_missing, admission_missing, collect_states_missing,
//   action_contracts_missing, action_abi_digests_missing.
//
// `action_payloads`, `store_missing`, `action_attempts_missing`,
// `admission_missing`, `collect_states_missing`, `action_contracts_missing`,
// and `action_abi_digests_missing` are NEVER returned as the priority
// reason: `mark_full_run_state_missing` sets all seven `*_missing` flags
// together, and `priority_class_second_half` checks `workflow_missing`
// first, so any frame-seed-only hydration fails closed with exactly the
// token `"workflow_missing"`. These tokens are observable in the
// `RecoveryCannotResumeState` struct itself (so unit tests verify the
// flag-wise propagation), but the public-API typed-rejection reason
// string is dominated by `workflow_missing`. The tests below cover the
// remaining six reachable reasons and assert one reason token per test
// so the priority ordering cannot silently regress.
// ============================================================================

/// Typed-rejection contract — snapshot+tail path rejects an unresolved
/// `ActionScheduled` in the tail with EXACT reason `"pending_actions"`.
///
/// The snapshot+tail path runs `classify_snapshot_tail_cannot_resume`
/// BEFORE any `RunFrame` allocation, so the typed gate must reject with
/// `Err(RecoveryError::UnsupportedFrameSeed { run, reason: "pending_actions" })`
/// when the tail contains an `ActionScheduled` whose `ActionCompleted`,
/// `ActionFailed`, or `ActionAbandoned` follow-up is missing. This was the
/// only reachable typed-rejection reason on `hydrate_run_frame` that no
/// test previously pinned.
#[test]
fn typed_rejection_hydrate_snapshot_tail_pending_actions_fails_closed() {
    let run = RunId::new(12001);
    let digest = test_digest(0xA1);
    let action = ActionId::new(0xAC);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    // Tail contains a StepStarted + ActionScheduled with NO follow-up
    // ActionCompleted/ActionFailed/ActionAbandoned. The typed cannot-
    // resume gate must fail closed with reason "pending_actions"
    // before any RunFrame is allocated.
    let tail = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            action,
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert_unsupported_frame_seed(result, run, "pending_actions");
}

/// Typed-rejection contract — events-only path rejects an unresolved
/// `ActionScheduled` with EXACT reason `"pending_actions"`.
///
/// The events-only path runs through `recover_runtime_frame_seed_from_events`
/// then through `RecoveryCannotResumeState::from_seed`, which sets
/// `pending_actions = true` because the seed has a non-empty
/// `pending_actions` vec. The typed boundary must reject with EXACT
/// reason `"pending_actions"` — NOT `"workflow_missing"`, NOT any of
/// the other second-half flags — because the priority scan runs
/// `pending_actions` (priority index 3) before `workflow_missing`
/// (priority index 6).
#[test]
fn typed_rejection_hydrate_from_events_pending_actions_fails_closed() {
    let run = RunId::new(12002);
    let digest = test_digest(0xA2);
    let action = ActionId::new(0xAD);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            action,
            attempt: 1,
        },
        // NO ActionCompletedEvent / ActionFailedEvent / ActionAbandoned
        // follows — the action is unresolved and the typed gate must
        // reject with "pending_actions" at priority index 3.
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "pending_actions");
}

/// Typed-rejection contract — events-only path rejects an unresolved
/// `WaitScheduledEvent` with EXACT reason `"pending_timers"`.
///
/// `WaitScheduledEvent` classifies the step as `RecoveredStepState::Waiting`
/// in the seed, and `RecoveryCannotResumeState::from_seed` translates
/// the `Waiting` step state to `pending_timers = true`. The typed
/// boundary must reject with EXACT reason `"pending_timers"` at
/// priority index 4. This is the events-only counterpart to
/// `snapshot_plus_tail_with_unresolved_wait_rejects_at_typed_gate`
/// which exercises the snapshot+tail path.
#[test]
fn typed_rejection_hydrate_from_events_pending_timers_fails_closed() {
    let run = RunId::new(12003);
    let digest = test_digest(0xA3);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
        // NO WaitResolvedEvent follows — the wait is unresolved and
        // the typed gate must reject with "pending_timers".
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "pending_timers");
}

/// Typed-rejection contract — events-only path rejects an unresolved
/// `AskScheduledEvent` with EXACT reason `"pending_asks"`.
///
/// `AskScheduledEvent` classifies the step as `RecoveredStepState::Asking`
/// in the seed, and `RecoveryCannotResumeState::from_seed` translates
/// the `Asking` step state to `pending_asks = true`. The typed boundary
/// must reject with EXACT reason `"pending_asks"` at priority index 5.
/// This is the events-only counterpart to
/// `snapshot_plus_tail_with_unresolved_ask_rejects_at_typed_gate`.
#[test]
fn typed_rejection_hydrate_from_events_pending_asks_fails_closed() {
    let run = RunId::new(12004);
    let digest = test_digest(0xA4);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
        // NO AskAnsweredEvent / AskTimedOutEvent follows — the ask is
        // unresolved and the typed gate must reject with "pending_asks".
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "pending_asks");
}

/// Typed-rejection contract — events-only path rejects a `SlotWrittenEvent`
/// whose `extra` bytes decode as a legacy frame extra with EXACT reason
/// `"slot_taint"`.
///
/// `extra: Some(<bytes-not-starting-with-VBSE\x01>)` is treated as
/// `DecodedSlotWrittenExtra::LegacyFrameExtra(bytes)` by
/// `decode_slot_written_extra`, which sets
/// `accumulator.event_slot_taint_unsupported = true`. This propagates
/// to `unsupported.slot_taint = true` in the frame seed, which the
/// typed gate then surfaces as the priority reason `"slot_taint"` at
/// priority index 1. The slot `value` field is provided as a valid
/// encoded `SlotValue::I64(0)` so `missing_slot_values` stays false
/// and the reason is NOT dominated by `"slot_values"`.
#[test]
fn typed_rejection_hydrate_from_events_slot_taint_fails_closed() {
    let run = RunId::new(12005);
    let digest = test_digest(0xA5);

    // Legacy frame extra bytes — anything that does NOT start with the
    // v1 envelope prefix `b"VBSE\x01"` is classified as
    // `DecodedSlotWrittenExtra::LegacyFrameExtra` and marks the taint
    // as unsupported at the storage layer.
    let legacy_extra: Vec<u8> = vec![0x01, 0x02, 0x03, 0x04];

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(0),
            // value: Some(encoded) keeps missing_slot_values = false,
            // so the typed reason is "slot_taint" not "slot_values".
            value: Some(
                postcard::to_allocvec(&SlotValue::I64(0))
                    .expect("slot value encoding should succeed"),
            ),
            extra: Some(legacy_extra),
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "slot_taint");
}

/// Typed-rejection contract — events-only path with a clean unresolved
/// run state (no pending actions / timers / asks, valid slot values,
/// no slot_taint legacy extra) returns EXACT reason `"workflow_missing"`.
///
/// This is the canonical "frame-seed-only" failure mode: a frame seed
/// derived purely from journal events can NEVER carry the live runtime
/// `RunState` (workflow, store, action attempts, admission, collect
/// states, action contracts, action ABI digests) required by the
/// runtime boundary. `mark_full_run_state_missing` stamps all seven
/// `*_missing` flags together, and `priority_class_second_half`
/// resolves `workflow_missing` first. The runtime boundary must
/// reject with EXACT reason `"workflow_missing"`. This is also the
/// reason observed by the existing `action_scheduled_then_*_reconstructed`
/// tests, but those tests incidentally exercise it as a side effect
/// of the action flow; this test pins the reason token directly on the
/// cleanest possible fixture.
#[test]
fn typed_rejection_hydrate_from_events_workflow_missing_fails_closed() {
    let run = RunId::new(12006);
    let digest = test_digest(0xA6);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        // Clean step lifecycle — no pending actions, timers, asks,
        // slot_values (value: Some), or slot_taint (no extra). The
        // ONLY cannot-resume reason is workflow_missing from the
        // frame-seed-only "no live RunState" contract.
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(0),
            value: Some(
                postcard::to_allocvec(&SlotValue::I64(7))
                    .expect("slot value encoding should succeed"),
            ),
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "workflow_missing");
}

// ============================================================================
// B-021 through B-026: parametrized missing-component mask — gap closure
// (vb-wy33p.11 round 8)
//
// `mark_full_run_state_missing` was previously unconditional: it set all
// seven `*_missing` flags together, so the priority chain
// `priority_class_second_half` always resolved `workflow_missing` first.
// The six later-priority tokens (`store_missing`,
// `action_attempts_missing`, `admission_missing`,
// `collect_states_missing`, `action_contracts_missing`,
// `action_abi_digests_missing`) were structurally unreachable from
// `from_seed`.
//
// Round 8 introduced `mark_missing_components(MissingRunStateComponents)`
// and `MissingRunStateComponents::single(MissingRunStateComponent)`. The
// tests below drive the priority chain with a mask that contains
// EXACTLY one `*_missing` flag, so each second-half reason token is
// reachable in isolation. This proves the priority chain is correct
// for every token, not just `workflow_missing`.
//
// Construction pattern (used by every test below):
//   let state = RecoveryCannotResumeState::from_unsupported(UnsupportedRecoveryState::SUPPORTED)
//       .mark_missing_components(MissingRunStateComponents::single(MissingRunStateComponent::<TOKEN>));
//   assert_eq!(state.unsupported_reason(), "<token>_missing");
// ============================================================================

/// Parametrized mask — `single(Store)` yields reason `"store_missing"`.
#[test]
fn parametrized_mask_single_store_missing_yields_store_reason() {
    let state = RecoveryCannotResumeState::from_unsupported(UnsupportedRecoveryState::SUPPORTED)
        .mark_missing_components(MissingRunStateComponents::single(
            MissingRunStateComponent::Store,
        ));
    assert!(!state.is_resumable());
    assert_eq!(state.unsupported_reason(), "store_missing");
}

/// Parametrized mask — `single(ActionAttempts)` yields reason
/// `"action_attempts_missing"`.
#[test]
fn parametrized_mask_single_action_attempts_missing_yields_action_attempts_reason() {
    let state = RecoveryCannotResumeState::from_unsupported(UnsupportedRecoveryState::SUPPORTED)
        .mark_missing_components(MissingRunStateComponents::single(
            MissingRunStateComponent::ActionAttempts,
        ));
    assert!(!state.is_resumable());
    assert_eq!(state.unsupported_reason(), "action_attempts_missing");
}

/// Parametrized mask — `single(Admission)` yields reason
/// `"admission_missing"`.
#[test]
fn parametrized_mask_single_admission_missing_yields_admission_reason() {
    let state = RecoveryCannotResumeState::from_unsupported(UnsupportedRecoveryState::SUPPORTED)
        .mark_missing_components(MissingRunStateComponents::single(
            MissingRunStateComponent::Admission,
        ));
    assert!(!state.is_resumable());
    assert_eq!(state.unsupported_reason(), "admission_missing");
}

/// Parametrized mask — `single(CollectStates)` yields reason
/// `"collect_states_missing"`.
#[test]
fn parametrized_mask_single_collect_states_missing_yields_collect_states_reason() {
    let state = RecoveryCannotResumeState::from_unsupported(UnsupportedRecoveryState::SUPPORTED)
        .mark_missing_components(MissingRunStateComponents::single(
            MissingRunStateComponent::CollectStates,
        ));
    assert!(!state.is_resumable());
    assert_eq!(state.unsupported_reason(), "collect_states_missing");
}

/// Parametrized mask — `single(ActionContracts)` yields reason
/// `"action_contracts_missing"`.
#[test]
fn parametrized_mask_single_action_contracts_missing_yields_action_contracts_reason() {
    let state = RecoveryCannotResumeState::from_unsupported(UnsupportedRecoveryState::SUPPORTED)
        .mark_missing_components(MissingRunStateComponents::single(
            MissingRunStateComponent::ActionContracts,
        ));
    assert!(!state.is_resumable());
    assert_eq!(state.unsupported_reason(), "action_contracts_missing");
}

/// Parametrized mask — `single(ActionAbiDigests)` yields reason
/// `"action_abi_digests_missing"`.
#[test]
fn parametrized_mask_single_action_abi_digests_missing_yields_action_abi_digests_reason() {
    let state = RecoveryCannotResumeState::from_unsupported(UnsupportedRecoveryState::SUPPORTED)
        .mark_missing_components(MissingRunStateComponents::single(
            MissingRunStateComponent::ActionAbiDigests,
        ));
    assert!(!state.is_resumable());
    assert_eq!(state.unsupported_reason(), "action_abi_digests_missing");
}

/// Parametrized mask — `single(Workflow)` (the highest-priority
/// second-half token) still yields reason `"workflow_missing"` when
/// the storage-layer / pending-boundary flags are all false. This
/// confirms the parametrized mask and the `from_seed`-style `ALL`
/// mask produce the same canonical reason for the workflow case.
#[test]
fn parametrized_mask_single_workflow_missing_yields_workflow_reason() {
    let state = RecoveryCannotResumeState::from_unsupported(UnsupportedRecoveryState::SUPPORTED)
        .mark_missing_components(MissingRunStateComponents::single(
            MissingRunStateComponent::Workflow,
        ));
    assert!(!state.is_resumable());
    assert_eq!(state.unsupported_reason(), "workflow_missing");
}

/// Parametrized mask — `ALL` (the `from_seed` default) yields reason
/// `"workflow_missing"` because the priority chain resolves
/// `workflow_missing` first. This is the FINDING-001 invariant:
/// every frame-seed-only classification fails closed with
/// `"workflow_missing"` regardless of which other second-half flags
/// are also true.
#[test]
fn parametrized_mask_all_yields_workflow_reason_due_to_priority_order() {
    let state = RecoveryCannotResumeState::from_unsupported(UnsupportedRecoveryState::SUPPORTED)
        .mark_missing_components(MissingRunStateComponents::ALL);
    assert!(!state.is_resumable());
    assert_eq!(state.unsupported_reason(), "workflow_missing");
}

/// Parametrized mask — `NONE` plus a fully-supported `UnsupportedRecoveryState`
/// is the resumable sentinel. The priority chain returns `"resumable"`
/// because every `*_missing` flag is false. `is_resumable()` returns true.
#[test]
fn parametrized_mask_none_yields_resumable() {
    let state = RecoveryCannotResumeState::from_unsupported(UnsupportedRecoveryState::SUPPORTED)
        .mark_missing_components(MissingRunStateComponents::NONE);
    assert!(state.is_resumable());
    assert_eq!(state.unsupported_reason(), "resumable");
}
