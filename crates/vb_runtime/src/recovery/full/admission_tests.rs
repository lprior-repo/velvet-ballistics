use super::*;
use crate::admission::RunAdmission;
use crate::journal::StorageRuntimeJournal;
use crate::runtime::Runtime;
use crate::shard::ShardConfig;
use std::num::NonZeroUsize;
use std::sync::Arc;
use vb_core::{
    AggregateResourceBudget, CompiledNodeKind, ResourceContract, SlotIdx, StepIdx, WorkflowParts,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn descriptor(value: u8) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: value.into(),
        max_action_tickets: value.into(),
        max_parallel_in_flight: value.into(),
        max_retries_per_action: value.into(),
        max_gather_pages: value.into(),
        max_gather_items: value.into(),
        max_for_each_iterations: value.into(),
        max_together_branches: value.into(),
        max_repeat_attempts: value.into(),
        max_run_time_seconds: value.into(),
        max_result_bytes: value.into(),
        max_total_slots_written: value.into(),
        max_timer_entries: value.into(),
        max_trace_events: value.into(),
        max_queue_depth: value.into(),
        max_journal_batch_bytes: value.into(),
        max_ipc_payload_bytes: value.into(),
        max_blob_bytes: value.into(),
        max_input_bytes: value.into(),
        max_step_budget_per_tick: value.into(),
        max_transitions_per_tick: value.into(),
        max_open_gates: value.into(),
        max_signal_subscriptions: value.into(),
        max_incident_emissions: value.into(),
        max_transitions: value.into(),
        max_journal_bytes_total: value.into(),
        max_value_store_entries: value.into(),
        max_cost_units: value.into(),
    }
}

#[test]
fn every_recorded_budget_dimension_is_enforced_without_mutation() -> TestResult {
    let required = descriptor(2);
    let enough = descriptor(3);
    assert_eq!(
        validate_recorded_budget_limits(&required, &required),
        Ok(())
    );
    assert_eq!(validate_recorded_budget_limits(&enough, &required), Ok(()));
    let deficiencies: [(&str, fn(&mut AggregateResourceBudget)); 28] = [
        ("max_steps_executable", |budget| {
            budget.max_steps_executable = 1
        }),
        ("max_action_tickets", |budget| budget.max_action_tickets = 1),
        ("max_parallel_in_flight", |budget| {
            budget.max_parallel_in_flight = 1
        }),
        ("max_retries_per_action", |budget| {
            budget.max_retries_per_action = 1
        }),
        ("max_gather_pages", |budget| budget.max_gather_pages = 1),
        ("max_gather_items", |budget| budget.max_gather_items = 1),
        ("max_for_each_iterations", |budget| {
            budget.max_for_each_iterations = 1
        }),
        ("max_together_branches", |budget| {
            budget.max_together_branches = 1
        }),
        ("max_repeat_attempts", |budget| {
            budget.max_repeat_attempts = 1
        }),
        ("max_run_time_seconds", |budget| {
            budget.max_run_time_seconds = 1
        }),
        ("max_result_bytes", |budget| budget.max_result_bytes = 1),
        ("max_total_slots_written", |budget| {
            budget.max_total_slots_written = 1
        }),
        ("max_timer_entries", |budget| budget.max_timer_entries = 1),
        ("max_trace_events", |budget| budget.max_trace_events = 1),
        ("max_queue_depth", |budget| budget.max_queue_depth = 1),
        ("max_journal_batch_bytes", |budget| {
            budget.max_journal_batch_bytes = 1
        }),
        ("max_ipc_payload_bytes", |budget| {
            budget.max_ipc_payload_bytes = 1
        }),
        ("max_blob_bytes", |budget| budget.max_blob_bytes = 1),
        ("max_input_bytes", |budget| budget.max_input_bytes = 1),
        ("max_step_budget_per_tick", |budget| {
            budget.max_step_budget_per_tick = 1
        }),
        ("max_transitions_per_tick", |budget| {
            budget.max_transitions_per_tick = 1
        }),
        ("max_open_gates", |budget| budget.max_open_gates = 1),
        ("max_signal_subscriptions", |budget| {
            budget.max_signal_subscriptions = 1
        }),
        ("max_incident_emissions", |budget| {
            budget.max_incident_emissions = 1
        }),
        ("max_transitions", |budget| budget.max_transitions = 1),
        ("max_journal_bytes_total", |budget| {
            budget.max_journal_bytes_total = 1
        }),
        ("max_value_store_entries", |budget| {
            budget.max_value_store_entries = 1
        }),
        ("max_cost_units", |budget| budget.max_cost_units = 1),
    ];
    for (name, lower) in deficiencies {
        let mut recorded = enough;
        lower(&mut recorded);
        let before = postcard::to_allocvec(&recorded)?;
        assert_eq!(
            validate_recorded_budget_limits(&recorded, &required),
            Err(cannot_resume_error("admission_budget_insufficient")),
            "{name}"
        );
        assert_eq!(postcard::to_allocvec(&recorded)?, before, "{name}");
    }
    Ok(())
}

fn workflow() -> TestResult<CompiledWorkflow> {
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("recovered-budget-limits"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            vb_core::node(
                StepIdx::ZERO,
                CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::ZERO,
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(1)),
            ),
            vb_core::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    parts.digest =
        WorkflowDigest::from_bytes(*blake3::hash(&postcard::to_allocvec(&parts)?).as_bytes());
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: vb_core::action::ActionName::new("recovered-budget-action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1000,
        idempotency: vb_core::Idempotency::DeterministicPure,
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        required_capabilities: Box::from([]),
    })
}

#[test]
fn current_requirements_validate_without_replacing_admission_identity_or_limits() -> TestResult {
    let workflow = workflow()?;
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        RuntimePolicy::Strict,
        &[contract()?],
    )?;
    let recorded = AggregateResourceBudget::from_workflow(&workflow)?;
    let run = RunId::new(201);
    let admission = RunAdmission::with_budget(
        artifact.digest,
        run,
        CapabilitySet::empty(),
        RuntimePolicy::Strict,
        recorded,
    )
    .with_policy_pin(vb_storage::RunPolicyPin::Pinned(artifact.policy_digest));
    let before = postcard::to_allocvec(&admission)?;
    validate_recovered_admission_evidence(RecoveredAdmissionEvidence {
        context: RecoveredAdmissionContext {
            run,
            expected_workflow: workflow.digest(),
            runtime_policy: RuntimePolicy::Strict,
        },
        admission: &admission,
        artifact: &artifact,
        workflow: &workflow,
    })?;
    assert_eq!(admission.budget(), Some(recorded));
    assert_eq!(postcard::to_allocvec(&admission)?, before);
    Ok(())
}

#[test]
fn missing_recorded_budget_is_not_inferred_as_zero_or_replaced() -> TestResult {
    let workflow = workflow()?;
    let admission = RunAdmission::new(
        WorkflowDigest::from_bytes([8; 32]),
        RunId::new(202),
        CapabilitySet::empty(),
        RuntimePolicy::Strict,
    );
    assert_eq!(validate_recovered_budget(&admission, &workflow), Ok(()));
    assert_eq!(admission.budget(), None);
    Ok(())
}

struct DurableFixture {
    runtime: Runtime,
    journal: Arc<vb_storage::FjallJournal>,
    _dir: tempfile::TempDir,
    run: RunId,
    recorded: AggregateResourceBudget,
}

fn durable_fixture(deficient_cost: bool) -> TestResult<DurableFixture> {
    let dir = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(dir.path(), None)?);
    let workflow = workflow()?;
    let contract = contract()?;
    let artifact = vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        std::slice::from_ref(&contract),
    )?;
    let run = RunId::new(203);
    let mut recorded = AggregateResourceBudget::from_workflow(&workflow)?;
    if deficient_cost {
        recorded.max_cost_units = recorded
            .max_cost_units
            .checked_sub(1)
            .ok_or("positive action cost required")?;
    } else {
        recorded.max_cost_units = recorded
            .max_cost_units
            .checked_add(1)
            .ok_or("larger recorded cost must remain bounded")?;
        validate_descriptor(&recorded)?;
    }
    let seq = vb_core::SeqNo::new(2);
    let ticket = vb_core::ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq,
        action: ActionId::new(0),
        run_attempt: 1,
        attempt: 1,
        capacity: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(
            run,
            1,
            seq,
            ActionId::new(0),
        ),
        fencing_token: 0,
    };
    let events = [
        vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::ZERO,
            workflow: workflow.digest(),
        },
        vb_storage::JournalEvent::RunAdmissionBudget {
            run,
            seq: vb_storage::EventSeq::new(1),
            artifact_digest: artifact.digest,
            input_digest: vb_storage::RunInputBinding::LegacyUnbound,
            policy_pin: vb_storage::RunPolicyPin::Pinned(artifact.policy_digest),
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
            budget: recorded,
        },
        vb_storage::JournalEvent::StepStarted {
            run,
            seq: vb_storage::EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        vb_storage::JournalEvent::ActionScheduledTicket {
            run,
            seq: vb_storage::EventSeq::new(3),
            ticket,
            input: SlotIdx::ZERO,
            output: SlotIdx::new(1),
            action_abi_digest: action_contract_abi_digest(&contract)?,
        },
    ];
    for event in events {
        journal.append_strict(&event)?;
    }
    let runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
    );
    Ok(DurableFixture {
        runtime,
        journal,
        _dir: dir,
        run,
        recorded,
    })
}

#[test]
fn sufficient_budget_recovery_query_is_a_nonmutating_artifact_control() -> TestResult {
    let fixture = durable_fixture(false)?;
    let before = fixture.journal.events_for_run_full(fixture.run)?;
    let state = fixture.runtime.pending_boundary_snapshot(4);
    let required = AggregateResourceBudget::from_workflow(&workflow()?)?;
    assert!(fixture.recorded.max_cost_units > required.max_cost_units);
    let original = crate::recovery::hydrate_run_admission_from_events(&before)
        .ok_or("original durable admission missing")?;
    assert_eq!(original.budget(), Some(fixture.recorded));
    let original_bytes = postcard::to_allocvec(&original)?;
    let product = match fixture.runtime.recover_product(fixture.run)? {
        crate::recovery::RuntimeRecoveryProduct::Resumable(product) => product,
        other => return Err(format!("expected artifact recovery control, got {other:?}").into()),
    };
    let command = product.into_recover_command();
    let carried = match &command.admission {
        crate::shard::types::RecoveredAdmissionKind::Historical(admission) => admission,
        _ => return Err("recovery command lost historical admission".into()),
    };
    assert_eq!(carried.as_ref(), &original);
    assert_eq!(carried.budget(), Some(fixture.recorded));
    assert_ne!(carried.budget(), Some(required));
    assert_eq!(postcard::to_allocvec(carried.as_ref())?, original_bytes);
    assert_eq!(command.run, fixture.run);
    assert_eq!(command.artifact_digest, original.artifact_digest());
    assert_eq!(fixture.runtime.pending_boundary_snapshot(4), state);
    assert_eq!(fixture.journal.events_for_run_full(fixture.run)?, before);
    Ok(())
}

#[test]
fn underfunded_cost_refuses_public_recovery_before_enqueue_or_state_mutation() -> TestResult {
    let fixture = durable_fixture(true)?;
    let before = fixture.journal.events_for_run_full(fixture.run)?;
    let state = fixture.runtime.pending_boundary_snapshot(4);
    let error = cannot_resume_error("admission_budget_insufficient");
    assert_eq!(
        fixture.runtime.recover_product(fixture.run),
        Err(error.clone())
    );
    assert_eq!(fixture.runtime.recover_and_resume(fixture.run), Err(error));
    assert_eq!(fixture.runtime.pending_boundary_snapshot(4), state);
    assert_eq!(fixture.journal.events_for_run_full(fixture.run)?, before);
    assert!(matches!(
        fixture.runtime.snapshot_run(fixture.run, 1)?,
        crate::shard::InspectResponse::NotFound { .. }
    ));
    assert!(fixture.journal.events_for_run_full(fixture.run)?.iter().any(|event| matches!(event,
        vb_storage::JournalEvent::RunAdmissionBudget { budget, .. } if *budget == fixture.recorded)));
    Ok(())
}

fn validate_descriptor(budget: &AggregateResourceBudget) -> TestResult {
    vb_core::budget::validate_aggregate_budget(budget, &vb_core::BoundednessPolicy::DEFAULT)?;
    vb_core::budget::validate_step_ceilings(budget)?;
    Ok(())
}

#[test]
fn one_hot_required_dimensions_bind_their_own_recorded_limits() -> TestResult {
    let raise: [(&str, fn(&mut AggregateResourceBudget)); 28] = [
        ("max_steps_executable", |required| {
            required.max_steps_executable = 3
        }),
        ("max_action_tickets", |required| {
            required.max_action_tickets = 3
        }),
        ("max_parallel_in_flight", |required| {
            required.max_parallel_in_flight = 3
        }),
        ("max_retries_per_action", |required| {
            required.max_retries_per_action = 3
        }),
        ("max_gather_pages", |required| required.max_gather_pages = 3),
        ("max_gather_items", |required| required.max_gather_items = 3),
        ("max_for_each_iterations", |required| {
            required.max_for_each_iterations = 3
        }),
        ("max_together_branches", |required| {
            required.max_together_branches = 3
        }),
        ("max_repeat_attempts", |required| {
            required.max_repeat_attempts = 3
        }),
        ("max_run_time_seconds", |required| {
            required.max_run_time_seconds = 3
        }),
        ("max_result_bytes", |required| required.max_result_bytes = 3),
        ("max_total_slots_written", |required| {
            required.max_total_slots_written = 3
        }),
        ("max_timer_entries", |required| {
            required.max_timer_entries = 3
        }),
        ("max_trace_events", |required| required.max_trace_events = 3),
        ("max_queue_depth", |required| required.max_queue_depth = 3),
        ("max_journal_batch_bytes", |required| {
            required.max_journal_batch_bytes = 3
        }),
        ("max_ipc_payload_bytes", |required| {
            required.max_ipc_payload_bytes = 3
        }),
        ("max_blob_bytes", |required| required.max_blob_bytes = 3),
        ("max_input_bytes", |required| required.max_input_bytes = 3),
        ("max_step_budget_per_tick", |required| {
            required.max_step_budget_per_tick = 3
        }),
        ("max_transitions_per_tick", |required| {
            required.max_transitions_per_tick = 3
        }),
        ("max_open_gates", |required| required.max_open_gates = 3),
        ("max_signal_subscriptions", |required| {
            required.max_signal_subscriptions = 3
        }),
        ("max_incident_emissions", |required| {
            required.max_incident_emissions = 3
        }),
        ("max_transitions", |required| required.max_transitions = 3),
        ("max_journal_bytes_total", |required| {
            required.max_journal_bytes_total = 3
        }),
        ("max_value_store_entries", |required| {
            required.max_value_store_entries = 3
        }),
        ("max_cost_units", |required| required.max_cost_units = 3),
    ];
    let uniform_two = descriptor(2);
    let larger = descriptor(3);
    validate_descriptor(&uniform_two)?;
    validate_descriptor(&larger)?;
    for (name, raise) in raise {
        let mut required = descriptor(1);
        raise(&mut required);
        validate_descriptor(&required)?;
        let exact = required;
        assert_eq!(
            validate_recorded_budget_limits(&exact, &required),
            Ok(()),
            "{name}: sufficient exact control"
        );
        assert_eq!(
            validate_recorded_budget_limits(&larger, &required),
            Ok(()),
            "{name}: sufficient larger control"
        );
        let before = postcard::to_allocvec(&uniform_two)?;
        assert_eq!(
            validate_recorded_budget_limits(&uniform_two, &required),
            Err(cannot_resume_error("admission_budget_insufficient")),
            "{name}: only the elevated required dimension exceeds recorded limits"
        );
        assert_eq!(postcard::to_allocvec(&uniform_two)?, before, "{name}");
    }
    Ok(())
}
