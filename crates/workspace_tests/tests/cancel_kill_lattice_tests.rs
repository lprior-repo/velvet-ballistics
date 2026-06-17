#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::enum_variant_names,
    clippy::manual_contains,
    clippy::if_same_then_else,
    clippy::multiple_bound_locations,
    clippy::identity_op,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

#![cfg(test)]
#![forbid(unsafe_code)]
//! cancel_kill_lattice_tests: Cancel/Kill State Machine Lattice Tests
//!
//! Integration tests for Cancel and Kill behavior against the step-state lattice.
//! Tests verify state transition invariants defined in the canonical step-state model.
//!
//! Behaviors covered:
//! - HP-1: cancel running run transitions to terminal cancelled state
//! - HP-3: cancel action-suspended run removes pending action
//! - HP-4: action after cancel returns error
//! - EC-1: terminal states don't regress
//! - INV-1: terminal never regresses
//!
//! Reference spec: `verification/verus/step_state_machine.rs`

use std::num::NonZeroUsize;
use std::sync::Arc;

use postcard;
use vb_core::action::{
    ActionFailure, ActionFailureCode, ActionOutputReady, ActionTicket, Idempotency, RetryPolicy,
    RetrySafety, SideEffect, compute_action_idempotency_key,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{InspectResponse, ShardConfig, TerminalOutcome};
use vb_runtime::trace::TraceEvent;
use vb_runtime::RuntimeError;

fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("expected non-zero shard count, got {value}"))
}

fn test_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        policy: RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
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
        "finished",
        0xA1,
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
        "action_then_finish",
        0xA3,
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

fn required_capability(action: ActionId) -> Capability {
    Capability::new(Box::from("test.contract.required"), action)
}

fn action_contract(
    action: ActionId,
    input_slots: u16,
    output_slots: u16,
) -> vb_core::action::ActionContract {
    vb_core::action::ActionContract {
        id: action,
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: input_slots,
        output_slot_count: output_slots,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::from([required_capability(action)]),
    }
}

fn action_contracts_through(
    action: ActionId,
    input_slots: u16,
    output_slots: u16,
) -> Box<[vb_core::action::ActionContract]> {
    let target = action.get();
    let mut contracts = Vec::new();
    let mut id = 0u16;
    loop {
        let current = ActionId::new(id);
        if id == target {
            contracts.push(action_contract(current, input_slots, output_slots));
            break;
        }
        contracts.push(action_contract(current, 0, 0));
        id = id.saturating_add(1);
    }
    contracts.into_boxed_slice()
}

fn action_grants(action: ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([required_capability(action)]))
}

fn submit_action_then_finish(
    runtime: &Runtime,
    run: RunId,
    workflow: CompiledWorkflow,
) -> vb_runtime::RuntimeResult<()> {
    let action = ActionId::new(7);
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
        action_grants(action),
        action_contracts_through(action, 1, 1),
    )
}

fn action_ticket(run: RunId, action: ActionId) -> ActionTicket {
    ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, SeqNo::ZERO, action),
        capacity: 1,
        ..Default::default()
    }
}

fn action_output(value: SlotValue) -> ActionOutputReady {
    let encoded = postcard::to_allocvec(&value).unwrap();
    ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint: Taint::Clean,
        encoded_len: encoded.len() as u32,
    }
}

fn tick_and_drain(runtime: &mut Runtime) -> Result<Vec<TraceEvent>, String> {
    assert_eq!(
        runtime.tick_all(),
        Ok(true),
        "tick_all should return true when shards alive"
    );
    Ok(Vec::new())
}

fn tick_count(runtime: &mut Runtime, count: usize) -> Result<(), String> {
    for _ in 0..count {
        assert_eq!(
            runtime.tick_all(),
            Ok(true),
            "tick_all should return true while draining queued commands"
        );
    }
    Ok(())
}

// =============================================================================
// HP-1: cancel running run transitions to terminal cancelled state
// =============================================================================

/// HP-1: Cancel transitions a running run to terminal Cancelled state.
///
/// Given an active run in Running state, when cancel_run is called,
/// then the run transitions to Cancelled terminal state and all resources
/// are released.
#[test]
fn hp1_cancel_running_run_transitions_to_cancelled() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20001);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    let counters_before = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_completed, 0,
        "run should be suspended waiting for action"
    );

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters = runtime.counters_snapshot();
    assert_eq!(counters.runs_failed, 1, "cancelled run counts as failed");
    assert_eq!(counters.runs_completed, 0, "cancelled run is not completed");

    let events = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    assert!(
        events.iter().any(|e| matches!(
            e,
            RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run
        )),
        "journal must contain RunCancelled event"
    );

    Ok(())
}

// =============================================================================
// HP-3: cancel action-suspended run removes pending action
// =============================================================================

/// HP-3: Cancel removes pending action for action-suspended run.
///
/// Given a run suspended waiting for an action (Resumable state),
/// when cancel_run is called, then the pending action is removed
/// and subsequent action completion returns error.
// HP-3: Cancel removes pending action for action-suspended run.
#[test]
fn hp3_cancel_action_suspended_run_removes_pending_action() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20003);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let failure = ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let result = runtime.fail_action(action_ticket(run, ActionId::new(7)), failure);
    assert!(
        result.is_err(),
        "action completion after cancel should return error"
    );

    Ok(())
}

// =============================================================================
// HP-4: action after cancel returns error
// =============================================================================

/// HP-4: Action completion after cancel returns error.
#[test]
fn hp4_action_after_cancel_returns_error() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20004);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let result = runtime.complete_action_with_output(
        action_ticket(run, ActionId::new(7)),
        action_output(SlotValue::I64(42)),
    );
    assert!(
        result.is_err(),
        "action completion after cancel should return error"
    );

    Ok(())
}

// =============================================================================
// EC-1: terminal states don't regress
// =============================================================================

/// EC-1: Terminal states don't regress (idempotent self-transition only).
///
/// Given a run in terminal Cancelled state, when cancel is called again,
/// then the state remains Cancelled (no regression to non-terminal state).
#[test]
fn ec1_terminal_cancelled_state_does_not_regress() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20005);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_before = runtime.counters_snapshot();

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter should not change on second cancel"
    );
    assert_eq!(
        counters_before.runs_completed, counters_after.runs_completed,
        "completed counter should not change on second cancel"
    );

    Ok(())
}

// =============================================================================
// INV-1: terminal never regresses
// =============================================================================

/// INV-1: Terminal state never regresses to non-terminal state.
///
/// Given a run in terminal Cancelled state, when tick_all is called multiple times,
/// then the run remains in terminal state and counters do not change.
#[test]
fn inv1_terminal_never_regresses_after_cancel() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20006);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_before = runtime.counters_snapshot();

    for _ in 0..5 {
        runtime
            .tick_all()
            .map_err(|e| format!("tick_all failed: {e:?}"))?;
    }

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter should not change after cancel"
    );
    assert_eq!(
        counters_before.runs_completed, counters_after.runs_completed,
        "completed counter should not change after cancel"
    );

    assert_eq!(
        runtime.snapshot_run(run, 1),
        Ok(InspectResponse::Terminal {
            run,
            correlation: 1,
            outcome: TerminalOutcome::Cancelled,
        }),
        "cancelled run should be inspectable as Terminal::Cancelled (vb-wxl5r)"
    );

    Ok(())
}

/// INV-1: Terminal state never regresses - completed run stays terminal.
///
/// Given a run that completed successfully, when cancel is called,
/// then the completed state is preserved (cancel on completed run is idempotent).
#[test]
fn inv1_completed_run_terminal_never_regresses() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20007);

    assert_eq!(runtime.submit_direct(run, finished_workflow()?), Ok(()));
    tick_count(&mut runtime, 2)?;

    let counters_before = runtime.counters_snapshot();
    assert_eq!(counters_before.runs_completed, 1, "run should be completed");

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_completed, counters_after.runs_completed,
        "completed counter should not change after cancel on completed run"
    );

    Ok(())
}

// =============================================================================
// C2: Cancel/Kill Missing and Already-Terminal — Cancel-Based Tests (vb-b8i8f)
// TDD RED: handle_cancel always returns Ok(()). After State 10, it should
// return typed errors. Current behavior IS side-effect-free for missing/
// terminal runs, so these tests verify side-effect-free contract.
// =============================================================================

/// B07/B11/B15: cancel on never-submitted run returns typed error during tick.
#[test]
fn cancel_missing_run_returns_run_not_found_error() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(40001);

    let counters_before = runtime.counters_snapshot();
    let journal_before = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;

    // C2: enqueue succeeds; error returned during tick processing.
    assert_eq!(runtime.cancel_run(run), Ok(()));

    let tick_result = runtime.tick_all();
    assert!(
        tick_result.is_err(),
        "tick must return error for missing run cancel, got {:?}",
        tick_result
    );

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter unchanged for missing run cancel"
    );

    let journal_after = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    assert_eq!(
        journal_after.len(),
        journal_before.len(),
        "journal unchanged"
    );

    Ok(())
}

/// B08/B12/B16: cancel on already-terminal run produces no side effects.
#[test]
fn cancel_terminal_run_produces_no_side_effects() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(40002);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_before = runtime.counters_snapshot();
    let journal_before = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    let event_count_before = journal_before.len();

    // Second cancel on already-cancelled run
    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter unchanged on second cancel"
    );

    let journal_after = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    let cancelled_count = journal_after
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run))
        .count();
    assert_eq!(cancelled_count, 1, "exactly one RunCancelled event");
    assert_eq!(journal_after.len(), event_count_before, "journal unchanged");

    Ok(())
}

// =============================================================================
// C3: Single Terminal Journal Event — Cancel Strengthened Tests (vb-b8i8f)
// =============================================================================

/// B25: second cancel after first cancel retains exactly one journal event.
#[test]
fn second_cancel_after_first_cancel_retains_one_event() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(50007);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let events_after_first = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    let cancel_count_first = events_after_first
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run))
        .count();
    assert_eq!(cancel_count_first, 1);

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let events_after_second = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    let cancel_count_second = events_after_second
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run))
        .count();
    assert_eq!(
        cancel_count_second, 1,
        "still exactly one RunCancelled event"
    );

    Ok(())
}

// =============================================================================
// C4: Stale Action/Timer Cleanup — Cancel-Based Tests (vb-b8i8f)
// =============================================================================

/// B33/B34: Action completion after cancel returns error during tick.
#[test]
fn action_completion_after_cancel_returns_error() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(60001);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    // C4: enqueue rejects immediately for cancelled runs.
    assert_eq!(
        runtime.complete_action_with_output(
            action_ticket(run, ActionId::new(7)),
            action_output(SlotValue::I64(42)),
        ),
        Err(RuntimeError::InvalidActionCompletion)
    );

    Ok(())
}

/// B34: Action failure after cancel returns error during tick.
#[test]
fn action_failure_after_cancel_returns_error() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(60002);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let failure = ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    // C4: enqueue rejects immediately for cancelled runs.
    assert_eq!(
        runtime.fail_action(action_ticket(run, ActionId::new(7)), failure),
        Err(RuntimeError::InvalidActionCompletion)
    );

    Ok(())
}

/// B41: Stale action does not mutate state (counters, journal unchanged).
#[test]
fn stale_action_after_cancel_does_not_mutate_state() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(60005);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_before = runtime.counters_snapshot();
    let journal_before = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    let event_count_before = journal_before.len();

    // Stale action completion — enqueue succeeds; tick returns error.
    let _ = runtime.complete_action_with_output(
        action_ticket(run, ActionId::new(7)),
        action_output(SlotValue::I64(99)),
    );
    let _ = runtime.tick_all();

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter unchanged after stale action"
    );
    assert_eq!(
        counters_before.runs_completed, counters_after.runs_completed,
        "completed counter unchanged after stale action"
    );

    let journal_after = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    assert_eq!(
        journal_after.len(),
        event_count_before,
        "journal unchanged after stale action attempt"
    );

    Ok(())
}

// =============================================================================
// K1: Kill live run — kill_run public API (vb-b8i8f State 11)
// =============================================================================

/// K1: kill_run enqueues ShardCommand::Kill and after tick produces RunKilled journal event.
#[test]
fn kill_live_run_produces_runkilled_journal_event() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(70001);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    let counters_before = runtime.counters_snapshot();
    assert_eq!(counters_before.runs_completed, 0);

    assert_eq!(runtime.kill_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters = runtime.counters_snapshot();
    assert_eq!(counters.runs_failed, 1, "killed run counts as failed");

    let events = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    assert!(
        events.iter().any(|e| matches!(
            e,
            RuntimeJournalEvent::RunKilled { run: r } if *r == run
        )),
        "journal must contain RunKilled event after kill"
    );

    Ok(())
}

/// K2: kill on never-submitted run returns typed error during tick.
#[test]
fn kill_missing_run_returns_run_not_found_error() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(70002);

    let counters_before = runtime.counters_snapshot();

    // C2: enqueue succeeds; error returned during tick processing.
    assert_eq!(runtime.kill_run(run), Ok(()));

    let tick_result = runtime.tick_all();
    assert!(
        tick_result.is_err(),
        "tick must return error for missing run kill, got {:?}",
        tick_result
    );

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter unchanged for missing run kill"
    );

    Ok(())
}

/// K3: kill on already-cancelled run is idempotent (no side effects).
#[test]
fn kill_on_cancelled_run_is_idempotent() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(70003);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_before = runtime.counters_snapshot();

    // Kill on already-cancelled run: idempotent, no side effects.
    assert_eq!(runtime.kill_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter unchanged after kill on cancelled run"
    );

    let events = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    let killed_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunKilled { run: r } if *r == run))
        .count();
    assert_eq!(
        killed_count, 0,
        "no RunKilled event for already-cancelled run"
    );

    Ok(())
}

/// K4: second kill after first kill has no effect (single journal event).
#[test]
fn second_kill_after_first_kill_produces_no_extra_event() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(70004);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.kill_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_before = runtime.counters_snapshot();

    assert_eq!(runtime.kill_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter unchanged on second kill"
    );

    let events = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    let killed_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunKilled { run: r } if *r == run))
        .count();
    assert_eq!(
        killed_count, 1,
        "exactly one RunKilled event after double kill"
    );

    Ok(())
}

/// K5: Action completion after kill returns error during tick.
#[test]
fn action_completion_after_kill_returns_error() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(70005);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.kill_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    // C4: enqueue rejects immediately for killed runs.
    assert_eq!(
        runtime.complete_action_with_output(
            action_ticket(run, ActionId::new(7)),
            action_output(SlotValue::I64(42)),
        ),
        Err(RuntimeError::InvalidActionCompletion)
    );

    Ok(())
}

/// K6: cancel after kill is idempotent (no RunCancelled appended).
#[test]
fn cancel_after_kill_is_idempotent() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(70006);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.kill_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_before = runtime.counters_snapshot();

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter unchanged after cancel-on-killed"
    );

    let events = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    let cancelled_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run))
        .count();
    assert_eq!(cancelled_count, 0, "no RunCancelled after cancel-on-killed");

    Ok(())
}
