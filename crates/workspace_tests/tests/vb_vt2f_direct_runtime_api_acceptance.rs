#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket,
    Idempotency, RetryPolicy, RetrySafety, SideEffect,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::RuntimeError;
use vb_runtime::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskAnswer, AskTicket, InspectResponse, ShardConfig};
use vb_runtime::trace::TraceEvent;

const DIRECT_API_TARGET: &str =
    "crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs";

fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("expected non-zero shard count, got {value}"))
}

fn relaxed_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        policy: RuntimePolicy::Relaxed,
    }
}

fn strict_config() -> ShardConfig {
    ShardConfig {
        policy: RuntimePolicy::Strict,
        ..relaxed_config()
    }
}

fn trace_starved_config() -> ShardConfig {
    ShardConfig {
        trace_capacity: 1,
        ..relaxed_config()
    }
}

fn node(id: u16, output: Option<u16>, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: output.map(SlotIdx::new),
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind,
    }
}

fn workflow_from_parts(
    name: &str,
    digest_byte: u8,
    nodes: Box<[CompiledNode]>,
    constants: Box<[ConstValue]>,
    slot_count: u16,
) -> Result<CompiledWorkflow, String> {
    let parts = WorkflowParts {
        input_slots: Box::new([]),
        name: Box::from(name),
        digest: WorkflowDigest::from_bytes([digest_byte; 32]),
        nodes,
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants,
        slot_count,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|err| format!("workflow fixture {name} invalid: {err:?}"))
}

fn finished_workflow() -> Result<CompiledWorkflow, String> {
    workflow_from_parts(
        "vt2f_finished",
        0x21,
        Box::from([
            node(
                0,
                Some(0),
                Some(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            ),
            node(
                1,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
            ),
        ]),
        Box::from([ConstValue::Bool(true)]),
        1,
    )
}

fn action_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    workflow_from_parts(
        "vt2f_action_then_finish",
        0x22,
        Box::from([
            node(
                0,
                Some(1),
                Some(1),
                CompiledNodeKind::Do {
                    action: ActionId::new(7),
                    input: SlotIdx::ZERO,
                },
            ),
            node(
                1,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            ),
        ]),
        Box::from([]),
        2,
    )
}

fn ask_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    workflow_from_parts(
        "vt2f_ask_then_finish",
        0x23,
        Box::from([
            node(
                0,
                Some(0),
                Some(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            ),
            node(
                1,
                Some(1),
                Some(2),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
            ),
            node(
                2,
                None,
                Some(3),
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::ZERO,
                    timeout_slot: Some(SlotIdx::new(1)),
                },
            ),
            node(
                3,
                None,
                Some(4),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(2),
                },
            ),
            node(
                4,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            ),
        ]),
        Box::from([ConstValue::I64(11), ConstValue::I64(30)]),
        3,
    )
}

fn required_capability(action: ActionId) -> Capability {
    Capability::new(Box::from("vt2f.contract.required"), action)
}

fn action_contract(
    action: ActionId,
    output_slots: u16,
    requires_capability: bool,
) -> Result<ActionContract, String> {
    Ok(ActionContract {
        id: action,
        name: ActionName::new("test-action")
            .map_err(|error| format!("action name fixture invalid: {error:?}"))?,
        input_slot_count: 1,
        output_slot_count: output_slots,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: if requires_capability {
            Box::from([required_capability(action)])
        } else {
            Box::new([])
        },
    })
}

fn action_contracts_through(
    action: ActionId,
    output_slots: u16,
) -> Result<Box<[ActionContract]>, String> {
    let target = action.get();
    (0..=target)
        .map(|id| {
            let current = ActionId::new(id);
            let outputs = if id == target { output_slots } else { 0 };
            action_contract(current, outputs, id == target)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn action_grants(action: ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([required_capability(action)]))
}

fn submit_action_workflow(
    runtime: &Runtime,
    run: RunId,
    workflow: CompiledWorkflow,
) -> Result<(), String> {
    let action = ActionId::new(7);
    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow,
            Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            action_grants(action),
            action_contracts_through(action, 1)?,
        )
        .map_err(|error| format!("submit action workflow for {run:?} failed: {error:?}"))
}

fn issued_action_ticket(runtime: &Runtime, run: RunId) -> Result<ActionTicket, String> {
    runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or_else(|| format!("runtime did not issue a pending action ticket for {run:?}"))
}

fn action_output(value: SlotValue, taint: Taint) -> Result<ActionOutputReady, String> {
    let encoded_len = u32::try_from(encoded(&value)?.len())
        .map_err(|error| format!("action output fixture length does not fit u32: {error}"))?;
    Ok(ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint,
        encoded_len,
    })
}

fn observed_ask_ticket_from_pending_state(
    runtime: &Runtime,
    run: RunId,
    workflow: &CompiledWorkflow,
) -> Result<AskTicket, String> {
    // Runtime currently exposes pending run/ask_step but no issued AskTicket.
    // This pairs that observation with the fixture's compiled next edge; it cannot prove
    // runtime-issued resume-step or answer-slot authority.
    let boundary = runtime.pending_boundary_snapshot(16);
    let pending = boundary
        .shards()
        .iter()
        .flat_map(|shard| shard.pending_asks().iter())
        .find(|ask| ask.run_id() == run)
        .ok_or_else(|| format!("runtime did not report a pending ask for {run:?}"))?;
    let ask_step = pending.ask_step();
    let resume_step = workflow
        .node(ask_step)
        .and_then(|node| node.next)
        .ok_or_else(|| format!("ask fixture has no resume step at {ask_step:?}"))?;
    Ok(AskTicket {
        run,
        ask_step,
        resume_step,
    })
}

fn ask_answer(ticket: AskTicket, value: SlotValue, taint: Taint) -> Result<AskAnswer, String> {
    let encoded_len = u32::try_from(encoded(&value)?.len())
        .map_err(|error| format!("ask answer fixture length does not fit u32: {error}"))?;
    Ok(AskAnswer {
        ticket,
        answer_slot: SlotIdx::new(2),
        value,
        taint,
        encoded_len,
    })
}

fn run_one_tick(runtime: &mut Runtime) -> Result<(), String> {
    assert_eq!(runtime.tick_all(), Ok(true));
    Ok(())
}

fn encoded(value: &SlotValue) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(value).map_err(|err| format!("postcard encode failed: {err:?}"))
}

fn journal_events(journal: &VolatileRuntimeJournal) -> Result<Vec<RuntimeJournalEvent>, String> {
    journal
        .snapshot()
        .map_err(|err| format!("journal snapshot failed: {err:?}"))
}

fn trace_events(runtime: &Runtime, run: RunId) -> Result<Vec<TraceEvent>, String> {
    runtime
        .list_events(run)
        .map_err(|err| format!("list_events({run:?}) failed: {err:?}"))
}

fn count_events_for_run(events: &[TraceEvent], run: RunId) -> usize {
    events.iter().filter(|event| event.run_id() == run).count()
}

#[test]
fn test_direct_api_submit_to_finish_returns_result_and_taint() -> Result<(), String> {
    // Given: SCN-VT2F-001 fresh relaxed runtime and deterministic public workflow fixture.
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config(), journal.clone());
    let run = RunId::new(1001);
    let expected_value = SlotValue::Bool(true);
    let expected_bytes = encoded(&expected_value)?;

    // When: submitted via Runtime::submit_direct and driven by explicit public tick_all.
    assert_eq!(runtime.submit_direct(run, finished_workflow()?), Ok(()));
    run_one_tick(&mut runtime)?;

    // Then: exact result value, taint, event class, counters, and active-list state are observable.
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.list_active_runs(8, None), Vec::new());
    assert_eq!(
        runtime.list_events(run),
        Ok(vec![
            TraceEvent::RunSubmitted { run },
            TraceEvent::StepStarted {
                run,
                step: StepIdx::ZERO
            },
            TraceEvent::SlotWritten {
                run,
                slot: SlotIdx::ZERO,
                value: expected_bytes.clone()
            },
            TraceEvent::StepStarted {
                run,
                step: StepIdx::new(1)
            },
            TraceEvent::RunFinished { run },
        ])
    );
    let events = journal_events(&journal)?;
    let expected_slot_event = RuntimeJournalEvent::SlotWritten {
        run,
        slot: SlotIdx::ZERO,
        value: expected_bytes,
        taint: Taint::Clean,
        extra: None,
    };
    assert_eq!(events.contains(&expected_slot_event), true);
    assert_eq!(
        events.contains(&RuntimeJournalEvent::RunFinished {
            run,
            result: SlotIdx::ZERO
        }),
        true
    );
    Ok(())
}

#[test]
fn test_direct_api_inspect_known_and_unknown_run_returns_exact_state() -> Result<(), String> {
    // Given: SCN-VT2F-002 one active suspended run and one absent run id.
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count(1)?, relaxed_config());
    let run = RunId::new(1002);
    let absent = RunId::new(9002);
    assert_eq!(
        submit_action_workflow(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    run_one_tick(&mut runtime)?;

    // When: snapshot and queued inspect are requested through public APIs.
    assert_eq!(
        runtime.snapshot_run(absent, 777),
        Ok(InspectResponse::NotFound {
            run: absent,
            correlation: 777
        })
    );
    assert_eq!(
        runtime.snapshot_run(run, 778),
        Ok(InspectResponse::Found(vb_runtime::shard::InspectSnapshot {
            run,
            correlation: 778,
            pc: StepIdx::ZERO,
            executed: 0,
        }))
    );
    assert_eq!(runtime.inspect_run(run, 779), Ok(()));
    run_one_tick(&mut runtime)?;

    // Then: active list and inspect response preserve exact run/correlation/pc/executed state.
    assert_eq!(
        runtime
            .list_active_runs(8, None)
            .iter()
            .map(|summary| summary.run_id)
            .collect::<Vec<_>>(),
        vec![run]
    );
    assert_eq!(
        runtime.take_inspect_response(run),
        Ok(Some(InspectResponse::Found(
            vb_runtime::shard::InspectSnapshot {
                run,
                correlation: 779,
                pc: StepIdx::ZERO,
                executed: 0,
            }
        )))
    );
    Ok(())
}

#[test]
fn test_direct_api_cancel_known_run_records_cancellation() -> Result<(), String> {
    // Given: SCN-VT2F-003 a known suspended run.
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config(), journal.clone());
    let run = RunId::new(1003);
    assert_eq!(
        submit_action_workflow(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    run_one_tick(&mut runtime)?;

    // When: cancel_run is called and the runtime is deterministically drained.
    assert_eq!(runtime.cancel_run(run), Ok(()));
    run_one_tick(&mut runtime)?;

    // Then: cancellation evidence is exact and the run is not active.
    assert_eq!(runtime.counters_snapshot().runs_failed, 1);
    assert_eq!(runtime.list_active_runs(8, None), Vec::new());
    assert_eq!(
        runtime.snapshot_run(run, 55),
        Ok(InspectResponse::NotFound {
            run,
            correlation: 55
        })
    );
    assert_eq!(
        trace_events(&runtime, run)?.contains(&TraceEvent::RunCancelled { run }),
        true
    );
    assert_eq!(
        journal_events(&journal)?
            .contains(&RuntimeJournalEvent::RunCancelled { run, reason: None }),
        true
    );
    Ok(())
}

#[test]
fn test_direct_api_action_completion_resumes_correct_run() -> Result<(), String> {
    // Given: SCN-VT2F-004 two suspended runs and a public action ticket for one run.
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config(), journal.clone());
    let run = RunId::new(1004);
    let unrelated = RunId::new(2004);
    assert_eq!(
        submit_action_workflow(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    assert_eq!(
        submit_action_workflow(&runtime, unrelated, action_then_finish_workflow()?),
        Ok(())
    );
    run_one_tick(&mut runtime)?;
    run_one_tick(&mut runtime)?;
    let ticket = issued_action_ticket(&runtime, run)?;
    let unrelated_before = runtime.snapshot_run(unrelated, 44);
    let unrelated_events_before = trace_events(&runtime, unrelated)?;
    let expected_value = SlotValue::I64(4242);
    let expected_taint = Taint::DerivedFromSecret;
    let expected_bytes = encoded(&expected_value)?;

    // When: the matching ticket completes with exact output value and taint.
    assert_eq!(
        runtime
            .complete_action_with_output(ticket, action_output(expected_value, expected_taint)?,),
        Ok(())
    );
    run_one_tick(&mut runtime)?;

    // Then: only the matching run completes and writes the exact output.
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.snapshot_run(unrelated, 44), unrelated_before);
    assert_eq!(trace_events(&runtime, unrelated)?, unrelated_events_before);
    assert_eq!(
        trace_events(&runtime, run)?.contains(&TraceEvent::ActionCompleted {
            run,
            step: StepIdx::ZERO
        }),
        true
    );
    let expected_completion_event = RuntimeJournalEvent::ActionCompletedEnvelope {
        ticket,
        output: SlotIdx::new(1),
        value: expected_bytes.clone(),
        encoded_len: u32::try_from(expected_bytes.len())
            .map_err(|error| format!("encoded output length does not fit u32: {error}"))?,
        taint: expected_taint,
        value_digest: *blake3::hash(&expected_bytes).as_bytes(),
        action_abi_digest: WorkflowDigest::from_bytes([0; 32]),
    };
    assert_eq!(
        journal_events(&journal)?.contains(&expected_completion_event),
        true
    );

    // And: tickets altered to target another run or action fail exactly without changing journal or run state.
    let journal_before_rejections = journal_events(&journal)?;
    let counters_before_rejections = runtime.counters_snapshot();
    assert_eq!(
        runtime.complete_action_with_output(
            ActionTicket {
                run: unrelated,
                ..ticket
            },
            action_output(SlotValue::I64(9), Taint::Clean)?
        ),
        Err(RuntimeError::InvalidActionCompletion)
    );
    assert_eq!(
        runtime.complete_action_with_output(
            ActionTicket {
                run: unrelated,
                action: ActionId::new(99),
                ..issued_action_ticket(&runtime, unrelated)?
            },
            action_output(SlotValue::I64(9), Taint::Clean)?
        ),
        Err(RuntimeError::InvalidActionCompletion)
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(journal_events(&journal)?, journal_before_rejections);
    assert_eq!(runtime.counters_snapshot(), counters_before_rejections);
    assert_eq!(runtime.snapshot_run(unrelated, 44), unrelated_before);
    assert_eq!(trace_events(&runtime, unrelated)?, unrelated_events_before);
    Ok(())
}

#[test]
fn test_direct_api_action_failure_records_typed_failure() -> Result<(), String> {
    // Given: SCN-VT2F-005 a suspended action workflow.
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config(), journal.clone());
    let run = RunId::new(1005);
    assert_eq!(
        submit_action_workflow(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    run_one_tick(&mut runtime)?;
    let ticket = issued_action_ticket(&runtime, run)?;
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };

    // When: fail_action is called with a typed non-retryable failure.
    assert_eq!(runtime.fail_action(ticket, failure), Ok(()));
    run_one_tick(&mut runtime)?;

    // Then: the failure type is visible and the run does not complete successfully.
    assert_eq!(runtime.counters_snapshot().runs_failed, 1);
    assert_eq!(runtime.counters_snapshot().runs_completed, 0);
    assert_eq!(
        trace_events(&runtime, run)?.contains(&TraceEvent::ActionFailed {
            run,
            step: StepIdx::ZERO,
            code: ActionFailureCode::Timeout,
        }),
        true
    );
    assert_eq!(
        journal_events(&journal)?.contains(&RuntimeJournalEvent::ActionFailed {
            run,
            step: StepIdx::ZERO,
            action: ActionId::new(7),
            attempt: 1,
        }),
        true
    );
    assert_eq!(
        journal_events(&journal)?.contains(&RuntimeJournalEvent::RunFailed { run }),
        true
    );
    Ok(())
}

#[test]
fn test_direct_api_action_failure_rejects_wrong_run_ticket_without_mutating_unrelated_run()
-> Result<(), String> {
    // Given: SCN-VT2F-005 / ERR-003 one unrelated suspended action run and an absent-run ticket.
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config(), journal.clone());
    let unrelated = RunId::new(2505);
    let absent = RunId::new(9505);
    assert_eq!(
        submit_action_workflow(&runtime, unrelated, action_then_finish_workflow()?),
        Ok(())
    );
    run_one_tick(&mut runtime)?;
    let issued = issued_action_ticket(&runtime, unrelated)?;
    let unrelated_before = runtime.snapshot_run(unrelated, 505);
    let unrelated_events_before = trace_events(&runtime, unrelated)?;
    let counters_before = runtime.counters_snapshot();
    let journal_before = journal_events(&journal)?;
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };

    // When: fail_action is enqueued for a wrong/absent run id.
    assert_eq!(
        runtime.fail_action(
            ActionTicket {
                run: absent,
                ..issued
            },
            failure,
        ),
        Ok(())
    );

    let tick_result = runtime.tick_all();

    // Then: the unrelated run is unchanged and the deterministic tick returns the exact typed error.
    assert_eq!(runtime.snapshot_run(unrelated, 505), unrelated_before);
    assert_eq!(trace_events(&runtime, unrelated)?, unrelated_events_before);
    assert_eq!(runtime.counters_snapshot(), counters_before);
    assert_eq!(
        runtime
            .list_active_runs(8, None)
            .iter()
            .map(|summary| summary.run_id)
            .collect::<Vec<_>>(),
        vec![unrelated]
    );
    assert_eq!(tick_result, Err(RuntimeError::InvalidActionCompletion));
    assert_eq!(journal_events(&journal)?, journal_before);
    Ok(())
}

#[test]
fn test_direct_api_answer_ask_resumes_suspended_run() -> Result<(), String> {
    // Given: SCN-VT2F-006 a run suspended on Ask with its observed pending ask state.
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config(), journal.clone());
    let run = RunId::new(1006);
    let workflow = ask_then_finish_workflow()?;
    assert_eq!(runtime.submit_direct(run, workflow.clone()), Ok(()));
    run_one_tick(&mut runtime)?;
    let ticket = observed_ask_ticket_from_pending_state(&runtime, run, &workflow)?;
    let answer_value = SlotValue::I64(6060);
    let answer_taint = Taint::DerivedFromSecret;
    let answer_bytes = encoded(&answer_value)?;
    let answer = ask_answer(ticket, answer_value, answer_taint)?;

    // When: answer_ask resumes the run.
    assert_eq!(runtime.answer_ask(answer), Ok(()));
    run_one_tick(&mut runtime)?;

    // Then: answer value/taint and terminal state are exact.
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(
        trace_events(&runtime, run)?.contains(&TraceEvent::AskAnswered {
            run,
            step: StepIdx::new(2),
            slot: SlotIdx::new(2)
        }),
        true
    );
    let expected_slot_event = RuntimeJournalEvent::SlotWritten {
        run,
        slot: SlotIdx::new(2),
        value: answer_bytes,
        taint: answer_taint,
        extra: None,
    };
    assert_eq!(
        journal_events(&journal)?.contains(&expected_slot_event),
        true
    );
    assert_eq!(
        runtime.snapshot_run(run, 66),
        Ok(InspectResponse::NotFound {
            run,
            correlation: 66
        })
    );
    Ok(())
}

#[test]
fn test_direct_api_answer_ask_rejects_stale_ticket_without_mutating_unrelated_run()
-> Result<(), String> {
    // Given: SCN-VT2F-006 / ERR-004 one formerly valid ask ticket and an unrelated suspended run.
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config(), journal.clone());
    let stale = RunId::new(1606);
    let unrelated = RunId::new(2606);
    let workflow = ask_then_finish_workflow()?;
    assert_eq!(runtime.submit_direct(stale, workflow.clone()), Ok(()));
    assert_eq!(
        submit_action_workflow(&runtime, unrelated, action_then_finish_workflow()?),
        Ok(())
    );
    run_one_tick(&mut runtime)?;
    run_one_tick(&mut runtime)?;
    let ticket = observed_ask_ticket_from_pending_state(&runtime, stale, &workflow)?;
    assert_eq!(
        runtime.answer_ask(ask_answer(ticket, SlotValue::I64(6060), Taint::Clean)?),
        Ok(())
    );
    run_one_tick(&mut runtime)?;
    assert_eq!(
        runtime.snapshot_run(stale, 606),
        Ok(InspectResponse::NotFound {
            run: stale,
            correlation: 606
        })
    );
    let unrelated_before = runtime.snapshot_run(unrelated, 607);
    let unrelated_events_before = trace_events(&runtime, unrelated)?;
    let active_before = runtime
        .list_active_runs(8, None)
        .iter()
        .map(|summary| summary.run_id)
        .collect::<Vec<_>>();
    let counters_before = runtime.counters_snapshot();

    // When: the previously issued ask ticket is replayed after its run is terminal.
    let journal_before_stale_replay = journal_events(&journal)?;
    assert_eq!(
        runtime.answer_ask(ask_answer(ticket, SlotValue::I64(6061), Taint::Clean)?),
        Ok(())
    );
    assert_eq!(journal_events(&journal)?, journal_before_stale_replay);
    assert_eq!(runtime.tick_all(), Err(RuntimeError::RunNotFound));
    assert_eq!(journal_events(&journal)?, journal_before_stale_replay);

    // Then: terminal replay fails exactly and unrelated observable state is unchanged.
    assert_eq!(runtime.snapshot_run(unrelated, 607), unrelated_before);
    assert_eq!(trace_events(&runtime, unrelated)?, unrelated_events_before);
    assert_eq!(
        runtime
            .list_active_runs(8, None)
            .iter()
            .map(|summary| summary.run_id)
            .collect::<Vec<_>>(),
        active_before
    );
    assert_eq!(runtime.counters_snapshot(), counters_before);
    Ok(())
}

#[test]
fn test_direct_api_answer_ask_rejects_stale_ticket_when_terminal_trace_was_evicted()
-> Result<(), String> {
    // Given: SCN-VT2F-006 / ERR-004 a previously valid ask ticket whose run is terminal and
    // whose trace evidence has been evicted, plus one unrelated suspended run.
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count(1)?, trace_starved_config(), journal.clone());
    let stale = RunId::new(1616);
    let unrelated = RunId::new(2616);
    let workflow = ask_then_finish_workflow()?;
    assert_eq!(runtime.submit_direct(stale, workflow.clone()), Ok(()));
    assert_eq!(
        submit_action_workflow(&runtime, unrelated, action_then_finish_workflow()?),
        Ok(())
    );
    run_one_tick(&mut runtime)?;
    run_one_tick(&mut runtime)?;
    let ticket = observed_ask_ticket_from_pending_state(&runtime, stale, &workflow)?;
    assert_eq!(
        runtime.answer_ask(ask_answer(ticket, SlotValue::I64(6160), Taint::Clean)?),
        Ok(())
    );
    run_one_tick(&mut runtime)?;
    assert_eq!(
        runtime.snapshot_run(stale, 616),
        Ok(InspectResponse::NotFound {
            run: stale,
            correlation: 616
        })
    );
    assert_eq!(trace_events(&runtime, stale)?.len(), 1);
    let unrelated_before = runtime.snapshot_run(unrelated, 617);
    let unrelated_events_before = trace_events(&runtime, unrelated)?;
    let active_before = runtime
        .list_active_runs(8, None)
        .iter()
        .map(|summary| summary.run_id)
        .collect::<Vec<_>>();
    let counters_before = runtime.counters_snapshot();

    // When: the previously issued ask ticket is replayed after terminal trace evidence was evicted.
    let journal_before_stale_replay = journal_events(&journal)?;
    let answer_result = runtime.answer_ask(ask_answer(ticket, SlotValue::I64(6161), Taint::Clean)?);

    // Then: the ticket is rejected at processing time with an exact typed error and no journal write.
    assert_eq!(answer_result, Ok(()));
    assert_eq!(journal_events(&journal)?, journal_before_stale_replay);
    assert_eq!(runtime.tick_all(), Err(RuntimeError::RunNotFound));
    assert_eq!(journal_events(&journal)?, journal_before_stale_replay);
    assert_eq!(runtime.snapshot_run(unrelated, 617), unrelated_before);
    assert_eq!(trace_events(&runtime, unrelated)?, unrelated_events_before);
    assert_eq!(
        runtime
            .list_active_runs(8, None)
            .iter()
            .map(|summary| summary.run_id)
            .collect::<Vec<_>>(),
        active_before
    );
    assert_eq!(runtime.counters_snapshot(), counters_before);
    Ok(())
}

#[test]
fn test_direct_api_answer_ask_rejects_wrong_run_ticket_without_mutating_unrelated_run()
-> Result<(), String> {
    // Given: SCN-VT2F-006 / ERR-004 an authentic pending ask ticket and a different active run.
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config(), journal.clone());
    let wrong = RunId::new(9606);
    let ask_owner = RunId::new(1607);
    let unrelated = RunId::new(2607);
    let workflow = ask_then_finish_workflow()?;
    assert_eq!(runtime.submit_direct(ask_owner, workflow.clone()), Ok(()));
    assert_eq!(
        submit_action_workflow(&runtime, unrelated, action_then_finish_workflow()?),
        Ok(())
    );
    run_one_tick(&mut runtime)?;
    run_one_tick(&mut runtime)?;
    let ticket = observed_ask_ticket_from_pending_state(&runtime, ask_owner, &workflow)?;
    let unrelated_before = runtime.snapshot_run(unrelated, 608);
    let unrelated_events_before = trace_events(&runtime, unrelated)?;
    let active_before = runtime
        .list_active_runs(8, None)
        .iter()
        .map(|summary| summary.run_id)
        .collect::<Vec<_>>();
    let counters_before = runtime.counters_snapshot();
    let journal_before = journal_events(&journal)?;

    // When: a valid ticket's run id is changed to an absent run while another run remains active.
    assert_eq!(
        runtime.answer_ask(ask_answer(
            AskTicket {
                run: wrong,
                ..ticket
            },
            SlotValue::I64(6061),
            Taint::Clean
        )?),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(journal_events(&journal)?, journal_before);

    // Then: tick is not affected and unrelated observable state is unchanged.
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.snapshot_run(unrelated, 608), unrelated_before);
    assert_eq!(trace_events(&runtime, unrelated)?, unrelated_events_before);
    assert_eq!(
        runtime
            .list_active_runs(8, None)
            .iter()
            .map(|summary| summary.run_id)
            .collect::<Vec<_>>(),
        active_before
    );
    assert_eq!(runtime.counters_snapshot(), counters_before);
    assert_eq!(journal_events(&journal)?, journal_before);
    Ok(())
}

#[test]
fn test_direct_api_list_events_and_drain_trace_have_exact_semantics() -> Result<(), String> {
    // Given: SCN-VT2F-007 two runs that each emitted distinguishable trace events.
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count(1)?, relaxed_config());
    let run_a = RunId::new(1007);
    let run_b = RunId::new(2007);
    assert_eq!(runtime.submit_direct(run_a, finished_workflow()?), Ok(()));
    assert_eq!(runtime.submit_direct(run_b, finished_workflow()?), Ok(()));
    // tick_all processes one command per shard per tick; two ticks are
    // required to drain both submits on a 1-shard runtime so that
    // list_events can locate both runs on the owning shard.
    run_one_tick(&mut runtime)?;
    run_one_tick(&mut runtime)?;

    // When: list_events is repeated, then drain_trace is called twice.
    let first_a = trace_events(&runtime, run_a)?;
    let second_a = trace_events(&runtime, run_a)?;
    let first_b = trace_events(&runtime, run_b)?;
    let drained = runtime.drain_trace();
    let post_drain = runtime.drain_trace();

    // Then: list is non-destructive and per-run, while drain is aggregate and destructive.
    assert_eq!(first_a, second_a);
    assert_eq!(first_a.iter().all(|event| event.run_id() == run_a), true);
    assert_eq!(first_b.iter().all(|event| event.run_id() == run_b), true);
    assert_eq!(count_events_for_run(&drained, run_a), first_a.len());
    assert_eq!(count_events_for_run(&drained, run_b), first_b.len());
    assert_eq!(post_drain, Vec::new());
    Ok(())
}

#[test]
fn test_direct_api_health_and_shutdown_equivalent_behavior() -> Result<(), String> {
    // Given: SCN-VT2F-008 an active runtime with queued work.
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count(1)?, relaxed_config());
    let run = RunId::new(1008);
    assert_eq!(runtime.submit_direct(run, finished_workflow()?), Ok(()));
    let pre_metrics = runtime.collect_metrics();

    // When: graceful shutdown is requested and post-shutdown operations are probed.
    assert_eq!(pre_metrics.runs_active, 0);
    assert_eq!(
        pre_metrics
            .shards
            .first()
            .map(|shard| shard.command_queue_depth),
        Some(1)
    );
    assert_eq!(runtime.shutdown_graceful(), Ok(()));

    // Then: queued work drains exactly once and further deterministic progress is inactive.
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.tick_all(), Ok(false));
    assert_eq!(runtime.tick_all(), Ok(false));
    // Post-shutdown submissions are rejected at the shard boundary with a
    // typed `ShutdownInProgress` error so callers see the runtime is no
    // longer accepting work instead of a phantom enqueue.
    assert_eq!(
        runtime.submit_direct(RunId::new(9008), finished_workflow()?),
        Err(RuntimeError::ShutdownInProgress)
    );
    assert_eq!(runtime.tick_all(), Ok(false));
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    Ok(())
}

#[test]
fn test_direct_api_rejects_submission_when_accepted_artifact_required() -> Result<(), String> {
    // Given: SCN-VT2F-009 strict/admission-required policy with no accepted artifact record.
    let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count(1)?, strict_config());
    let workflow = finished_workflow()?;
    let digest = workflow.digest();
    let run = RunId::new(1009);

    // When: raw direct submission is attempted without an accepted artifact.
    let submission = runtime.submit_direct(run, workflow);

    // Then: failing-first drift evidence expects a typed admission rejection at the submit boundary.
    assert_eq!(
        submission,
        Err(RuntimeError::AdmissionArtifactNotFound { digest }),
        "SCN-VT2F-009 master/code drift: master lines 3310-3345 require AdmissionRequired-equivalent rejection before raw direct execution; current public submit_direct must not accept run {run:?} without accepted artifact"
    );
    assert_eq!(runtime.list_active_runs(8, None), Vec::new());
    Ok(())
}

#[test]
fn test_direct_api_evidence_target_constant_matches_catalog_contract() {
    // Given/When/Then: SCN-VT2F-010 direct API target path is stable for catalog closure.
    assert_eq!(
        DIRECT_API_TARGET,
        "crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs"
    );
}
