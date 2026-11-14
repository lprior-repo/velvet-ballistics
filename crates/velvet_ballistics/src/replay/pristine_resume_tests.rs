use super::select_pristine_resume;
use vb_core::errors::CoreError;
use vb_core::{RunId, RuntimePolicy, WorkflowDigest};
use vb_storage::{EventSeq, JournalEvent, RunInputBinding, RunPolicyPin};

fn budget() -> vb_core::AggregateResourceBudget {
    vb_core::AggregateResourceBudget {
        max_steps_executable: 1,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 1,
        max_result_bytes: 1,
        max_total_slots_written: 1,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_open_gates: 1,
        max_signal_subscriptions: 1,
        max_incident_emissions: 1,
        max_transitions: 1,
        max_journal_bytes_total: 1,
        max_value_store_entries: 1,
        max_cost_units: 1,
    }
}

fn prefix(run: RunId, policy: RuntimePolicy) -> Vec<JournalEvent> {
    let artifact_version = vb_storage::compat::CURRENT_IR_VERSION;
    vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: WorkflowDigest::from_bytes([1; 32]),
        },
        JournalEvent::RunAdmissionBudget {
            run,
            seq: EventSeq::new(1),
            artifact_digest: WorkflowDigest::from_bytes([2; 32]),
            input_digest: RunInputBinding::Digest(WorkflowDigest::from_bytes([3; 32])),
            policy_pin: RunPolicyPin::Pinned(WorkflowDigest::from_bytes([4; 32])),
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy,
            budget: budget(),
        },
        JournalEvent::CompatibilityDecisionEvent {
            run,
            seq: EventSeq::new(2),
            decision: vb_storage::compat::decide_boundary(
                vb_storage::compat::artifact_ir_version_inputs(artifact_version),
                vb_storage::compat::schema_version_digest(artifact_version),
                vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION),
            ),
        },
        JournalEvent::RunBudgetOriginEvent {
            run,
            seq: EventSeq::new(3),
            started_millis: 456,
        },
    ]
}

#[test]
fn rejected_pristine_routes_keep_the_exact_original_error() -> Result<(), String> {
    let run = RunId::new(73);
    let timestamp = chrono::DateTime::from_timestamp(123, 456).ok_or("invalid timestamp")?;
    let originals = [
        CoreError::LifecycleDuplicateRequest {
            code: CoreError::LIFECYCLE_DUPLICATE_REQUEST_CODE,
            context: "original active context".into(),
            timestamp,
            bead_id: Some(run),
            command: Some("resume"),
        },
        CoreError::LifecycleInvalidTransition {
            code: CoreError::LIFECYCLE_INVALID_TRANSITION_CODE,
            context: "original invalid context".into(),
            timestamp,
            bead_id: Some(run),
            command: Some("resume"),
        },
    ];
    for original in originals {
        let read_failure = Err(vb_storage::JournalError::TooManyEvents {
            run,
            limit: 65_536,
            observed: 65_537,
        });
        assert_eq!(
            select_pristine_resume(run, original.clone(), read_failure),
            Err(original.clone())
        );
        assert_eq!(
            select_pristine_resume(run, original.clone(), Ok(Vec::new())),
            Err(original.clone())
        );
        assert_eq!(
            select_pristine_resume(
                run,
                original.clone(),
                Ok(prefix(run, RuntimePolicy::Relaxed)),
            ),
            Err(original.clone())
        );
        let mut trailing = prefix(run, RuntimePolicy::Strict);
        trailing.push(JournalEvent::RunResumed {
            run,
            seq: EventSeq::new(4),
            timestamp,
        });
        assert_eq!(
            select_pristine_resume(run, original.clone(), Ok(trailing)),
            Err(original.clone())
        );
        assert_eq!(
            select_pristine_resume(run, original, Ok(prefix(run, RuntimePolicy::Strict))),
            Ok(())
        );
    }
    Ok(())
}

#[test]
fn pristine_selector_rejects_each_run_and_sequence_mismatch() -> Result<(), String> {
    let run = RunId::new(73);
    let other = RunId::new(74);
    let timestamp = chrono::DateTime::from_timestamp(123, 456).ok_or("invalid timestamp")?;
    let original = CoreError::LifecycleDuplicateRequest {
        code: CoreError::LIFECYCLE_DUPLICATE_REQUEST_CODE,
        context: "run and sequence rejection sentinel".into(),
        timestamp,
        bead_id: Some(run),
        command: Some("resume"),
    };
    assert_eq!(
        select_pristine_resume(
            run,
            original.clone(),
            Ok(prefix(run, RuntimePolicy::Strict)),
        ),
        Ok(())
    );
    for field in 0..4 {
        let mut events = prefix(run, RuntimePolicy::Strict);
        let [
            JournalEvent::RunAccepted {
                run: accepted,
                seq: first,
                ..
            },
            JournalEvent::RunAdmissionBudget {
                run: admitted,
                seq: second,
                ..
            },
            ..,
        ] = events.as_mut_slice()
        else {
            return Err("unexpected pristine fixture shape".into());
        };
        match field {
            0 => *accepted = other,
            1 => *admitted = other,
            2 => *first = EventSeq::new(1),
            3 => *second = EventSeq::ZERO,
            _ => return Err("unexpected bounded case".into()),
        }
        assert_eq!(
            select_pristine_resume(run, original.clone(), Ok(events)),
            Err(original.clone()),
            "isolated pristine field mismatch {field}"
        );
    }
    Ok(())
}

#[test]
fn pristine_selector_requires_resume_command_for_both_candidate_errors() -> Result<(), String> {
    let run = RunId::new(73);
    let timestamp = chrono::DateTime::from_timestamp(123, 456).ok_or("invalid timestamp")?;
    for command in [None, Some("start"), Some("Resume")] {
        let originals = [
            CoreError::LifecycleDuplicateRequest {
                code: CoreError::LIFECYCLE_DUPLICATE_REQUEST_CODE,
                context: "duplicate noncandidate sentinel".into(),
                timestamp,
                bead_id: Some(run),
                command,
            },
            CoreError::LifecycleInvalidTransition {
                code: CoreError::LIFECYCLE_INVALID_TRANSITION_CODE,
                context: "transition noncandidate sentinel".into(),
                timestamp,
                bead_id: Some(run),
                command,
            },
        ];
        for original in originals {
            assert_eq!(
                select_pristine_resume(
                    run,
                    original.clone(),
                    Ok(prefix(run, RuntimePolicy::Strict)),
                ),
                Err(original),
                "non-resume candidate {command:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn pristine_selector_rejects_reordered_and_single_headers() -> Result<(), String> {
    let run = RunId::new(73);
    let timestamp = chrono::DateTime::from_timestamp(123, 456).ok_or("invalid timestamp")?;
    let original = CoreError::LifecycleInvalidTransition {
        code: CoreError::LIFECYCLE_INVALID_TRANSITION_CODE,
        context: "header shape rejection sentinel".into(),
        timestamp,
        bead_id: Some(run),
        command: Some("resume"),
    };
    for shape in 0..3 {
        let mut events = prefix(run, RuntimePolicy::Strict);
        match shape {
            0 => events.reverse(),
            1 => events.truncate(1),
            2 => events = events.into_iter().skip(1).collect(),
            _ => return Err("unexpected bounded header case".into()),
        }
        assert_eq!(
            select_pristine_resume(run, original.clone(), Ok(events)),
            Err(original.clone()),
            "invalid pristine header shape {shape}"
        );
    }
    Ok(())
}

#[test]
fn pristine_selector_rejects_missing_zero_and_duplicate_origins() -> Result<(), String> {
    let run = RunId::new(73);
    let timestamp = chrono::DateTime::from_timestamp(123, 456).ok_or("invalid timestamp")?;
    let original = CoreError::LifecycleInvalidTransition {
        code: CoreError::LIFECYCLE_INVALID_TRANSITION_CODE,
        context: "origin rejection sentinel".into(),
        timestamp,
        bead_id: Some(run),
        command: Some("resume"),
    };
    let mut missing = prefix(run, RuntimePolicy::Strict);
    let _ = missing.pop();
    assert_eq!(
        select_pristine_resume(run, original.clone(), Ok(missing)),
        Err(original.clone())
    );

    let mut zero = prefix(run, RuntimePolicy::Strict);
    let Some(JournalEvent::RunBudgetOriginEvent { started_millis, .. }) = zero.last_mut() else {
        return Err("current origin fixture missing".into());
    };
    *started_millis = 0;
    assert_eq!(
        select_pristine_resume(run, original.clone(), Ok(zero)),
        Err(original.clone())
    );

    let mut duplicate = prefix(run, RuntimePolicy::Strict);
    duplicate.push(JournalEvent::RunBudgetOriginEvent {
        run,
        seq: EventSeq::new(4),
        started_millis: 456,
    });
    assert_eq!(
        select_pristine_resume(run, original.clone(), Ok(duplicate)),
        Err(original)
    );
    Ok(())
}

#[test]
fn pristine_selector_rejects_unbound_input_and_unverified_policy() -> Result<(), String> {
    let run = RunId::new(73);
    let timestamp = chrono::DateTime::from_timestamp(123, 456).ok_or("invalid timestamp")?;
    let original = CoreError::LifecycleInvalidTransition {
        code: CoreError::LIFECYCLE_INVALID_TRANSITION_CODE,
        context: "admission binding rejection sentinel".into(),
        timestamp,
        bead_id: Some(run),
        command: Some("resume"),
    };
    assert_eq!(
        select_pristine_resume(
            run,
            original.clone(),
            Ok(prefix(run, RuntimePolicy::Strict)),
        ),
        Ok(())
    );
    for unbound_input in [true, false] {
        let mut events = prefix(run, RuntimePolicy::Strict);
        let [
            _,
            JournalEvent::RunAdmissionBudget {
                input_digest,
                policy_pin,
                ..
            },
            ..,
        ] = events.as_mut_slice()
        else {
            return Err("unexpected pristine admission fixture".into());
        };
        if unbound_input {
            *input_digest = RunInputBinding::LegacyUnbound;
        } else {
            *policy_pin = RunPolicyPin::Unverified;
        }
        assert_eq!(
            select_pristine_resume(run, original.clone(), Ok(events)),
            Err(original.clone()),
            "isolated admission binding mismatch: unbound_input={unbound_input}"
        );
    }
    let obsolete = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: WorkflowDigest::from_bytes([1; 32]),
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: WorkflowDigest::from_bytes([2; 32]),
            input_digest: RunInputBinding::Digest(WorkflowDigest::from_bytes([3; 32])),
            policy_pin: RunPolicyPin::Pinned(WorkflowDigest::from_bytes([4; 32])),
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
        },
    ];
    assert_eq!(
        select_pristine_resume(run, original.clone(), Ok(obsolete)),
        Err(original)
    );
    Ok(())
}
