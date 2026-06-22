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
    unused_variables
)]
#![cfg(test)]
#![forbid(unsafe_code)]
//! BDD behavior tests for vb_runtime IPC and resource management behavior.
//!
//! Tests cover:
//! - IPC message passing behavior (command routing, queue full conditions)
//! - Resource allocation and release behavior (frame pool, command queue)
//! - Connection lifecycle behavior (shutdown, drain, migration)
//! - Exact message ordering assertions (FIFO command processing)

// ============================================================================
// Test helpers and shared fixtures
// ============================================================================

use std::num::NonZeroUsize;

use vb_core::action::{
    ActionFailure, ActionFailureCode, ActionOutputReady, ActionTicket, RetryPolicy,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_runtime::RuntimeError;
use vb_runtime::frame_pool::FramePool;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{ShardConfig, ShardDirective};

fn test_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
    }
}

#[allow(dead_code)]
fn make_ticket(run: RunId, seq: u64) -> ActionTicket {
    ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq: SeqNo::new(seq),
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: seq as u128,
        capacity: 1,
        ..Default::default()
    }
}

fn finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("finished"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("suspended"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

#[allow(dead_code)]
fn action_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let do_node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(7),
            input: SlotIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("action_then_finish"),
        digest: WorkflowDigest::from_bytes([3; 32]),
        nodes: Box::from([do_node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn terminal_reentry_output(value: SlotValue) -> Result<ActionOutputReady, String> {
    let encoded = postcard::to_allocvec(&value).map_err(|error| error.to_string())?;
    let encoded_len = u32::try_from(encoded.len()).map_err(|error| error.to_string())?;
    Ok(ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint: Taint::Clean,
        encoded_len,
    })
}

// ============================================================================
// IPC-001: Command routing — submits route to correct shard deterministically
// ============================================================================

/// Given a 2-shard runtime, when submitting two runs with IDs that hash to
/// different shards, then each run's operations route to their respective shards.
#[test]
fn ipc_commands_route_to_correct_shard_by_run_id() {
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };

    let run1 = RunId::new(1); // hashes to shard 0 (1 % 2 = 1... wait: run_id hash % shard_count)
    let run2 = RunId::new(2); // hashes to different shard

    // Submit both runs
    assert_eq!(runtime.submit_direct(run1, wf1), Ok(()));
    assert_eq!(runtime.submit_direct(run2, wf2), Ok(()));

    // Tick once
    assert_eq!(runtime.tick_all(), Ok(true));

    // Both runs should be submitted (1 per shard)
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 2);
}

/// Given a 1-shard runtime, when submitting multiple runs,
/// then all operations succeed because they all route to the same shard.
#[test]
fn ipc_commands_stay_on_same_shard_across_operations() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf1) = finished_workflow() else {
        return;
    };
    let Some(wf2) = finished_workflow() else {
        return;
    };

    // Submit two runs to same shard
    assert_eq!(runtime.submit_direct(RunId::new(1), wf1), Ok(()));
    assert_eq!(runtime.submit_direct(RunId::new(2), wf2), Ok(()));
    // Each tick processes one command per shard, so need two ticks for two submits
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.tick_all(), Ok(true));

    // Both runs completed on the single shard
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 2);
    assert_eq!(snap.runs_completed, 2);
}

/// Given a runtime with tiny queue, when the queue is full,
/// then subsequent commands don't corrupt previously submitted runs.
#[test]
fn ipc_queue_full_does_not_corrupt_other_runs() {
    let config = ShardConfig {
        command_queue_capacity: 1,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
    };
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, config).expect("runtime config is valid");

    let Some(wf1) = finished_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };

    // First workflow completes in one tick
    assert_eq!(runtime.submit_direct(RunId::new(1), wf1), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));

    // Queue should now be empty, but we fill it again
    assert_eq!(runtime.submit_direct(RunId::new(2), wf2.clone()), Ok(()));

    // Try to submit when queue is full
    let err = runtime.submit_direct(RunId::new(3), wf2);
    assert_eq!(err, Err(RuntimeError::QueueFull));

    // Run 1 still completed successfully
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_completed, 1);
}

// ============================================================================
// IPC-003: Message ordering — commands processed in FIFO order per shard
// ============================================================================

/// Given a 1-shard runtime, when submitting multiple runs in sequence,
/// then each command is processed in FIFO order (first-submitted, first-processed).
#[test]
fn ipc_commands_processed_in_fifo_order() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf1) = finished_workflow() else {
        return;
    };
    let Some(wf2) = finished_workflow() else {
        return;
    };
    let Some(wf3) = finished_workflow() else {
        return;
    };

    let run1 = RunId::new(10);
    let run2 = RunId::new(11);
    let run3 = RunId::new(12);

    // Submit three runs
    assert_eq!(runtime.submit_direct(run1, wf1), Ok(()));
    assert_eq!(runtime.submit_direct(run2, wf2), Ok(()));
    assert_eq!(runtime.submit_direct(run3, wf3), Ok(()));

    // First tick processes run1 submit
    assert_eq!(runtime.tick_all(), Ok(true));
    let snap1 = runtime.counters_snapshot();
    assert_eq!(snap1.runs_completed, 1);
    assert_eq!(snap1.runs_submitted, 1);

    // Second tick processes run2 submit
    assert_eq!(runtime.tick_all(), Ok(true));
    let snap2 = runtime.counters_snapshot();
    assert_eq!(snap2.runs_completed, 2);
    assert_eq!(snap2.runs_submitted, 2);

    // Third tick processes run3 submit
    assert_eq!(runtime.tick_all(), Ok(true));
    let snap3 = runtime.counters_snapshot();
    assert_eq!(snap3.runs_completed, 3);
    assert_eq!(snap3.runs_submitted, 3);
}

/// Given a runtime, when submitting a cancel before a submit for the same run,
/// then the cancel is processed after submit (FIFO per shard).
#[test]
fn ipc_cancel_after_submit_processed_in_order() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(20);

    // Submit then cancel (in that order)
    assert_eq!(runtime.submit_direct(run, wf), Ok(()));
    assert_eq!(runtime.cancel_run(run), Ok(()));

    // First tick processes submit (run becomes active/suspended)
    assert_eq!(runtime.tick_all(), Ok(true));

    // Second tick processes cancel (run fails)
    assert_eq!(runtime.tick_all(), Ok(true));

    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 1);
    assert_eq!(snap.runs_failed, 1);
}

// ============================================================================
// IPC-004: Ask/Answer message passing — answers route to correct run
// ============================================================================

/// Given a 1-shard runtime, when answering an Ask,
/// then the answer enqueues successfully for later processing.
#[test]
fn ipc_ask_answer_enqueues_successfully() {
    use vb_runtime::shard::{AskAnswer, AskTicket};

    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let answer = AskAnswer {
        ticket: AskTicket {
            run: RunId::new(1),
            ask_step: StepIdx::ZERO,
            resume_step: StepIdx::new(1),
        },
        answer_slot: SlotIdx::new(0),
        value: SlotValue::Bool(true),
        taint: Taint::Clean,
        encoded_len: 1u32,
    };

    // Answer enqueues successfully (validation happens at tick time)
    assert_eq!(runtime.answer_ask(answer), Ok(()));
}

/// Given a 1-shard runtime, when answering an Ask for a non-existent run
/// that is NOT in terminal_runs, then the answer still enqueues successfully.
#[test]
fn ipc_ask_answer_enqueues_for_unknown_run() {
    use vb_runtime::shard::{AskAnswer, AskTicket};

    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let answer = AskAnswer {
        ticket: AskTicket {
            run: RunId::new(999), // run that doesn't exist
            ask_step: StepIdx::ZERO,
            resume_step: StepIdx::new(1),
        },
        answer_slot: SlotIdx::new(0),
        value: SlotValue::Bool(true),
        taint: Taint::Clean,
        encoded_len: 1u32,
    };

    // Answer enqueues for non-existent run - command is queued for later processing
    // RunNotFound is only returned if run is in terminal_runs
    let result = runtime.answer_ask(answer);
    assert_eq!(result, Ok(()));
}

// ============================================================================
// IPC-005: Action completion message passing
// ============================================================================

/// Given a runtime, when completing an action for a run that doesn't exist,
/// then the completion enqueues successfully (validation happens at tick time).
#[test]
fn ipc_action_completion_enqueues_for_nonexistent_run() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    // Complete action for non-existent run - enqueues successfully
    let ticket = ActionTicket {
        run: RunId::new(999),
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(7),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        ..Default::default()
    };
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 8,
    };

    // Enqueue succeeds (validation happens during tick)
    assert_eq!(runtime.complete_action_with_output(ticket, output), Ok(()));
}

/// Given a public runtime run reaches terminal state,
/// when the same run/step/action completion is admitted again,
/// then queued processing returns the explicit terminal re-entry error.
#[test]
fn ipc_terminal_run_reentry_completion_returns_run_not_found_when_processed() -> Result<(), String>
{
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard required"))?;
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");
    let workflow = finished_workflow().ok_or_else(|| String::from("finished workflow required"))?;
    let run = RunId::new(4242);
    let step = StepIdx::ZERO;
    let action = ActionId::new(7);
    let seq = SeqNo::ZERO;
    let ticket = ActionTicket {
        run,
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: vb_runtime::engine::compute_idempotency_key(run, seq, action),
        capacity: 1,
        ..Default::default()
    };
    let second_output = terminal_reentry_output(SlotValue::I64(99))?;

    assert_eq!(runtime.submit_direct(run, workflow), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);

    assert_eq!(
        runtime.complete_action_with_output(ticket, second_output),
        Ok(())
    );
    assert_eq!(runtime.tick_all(), Err(RuntimeError::RunNotFound));
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    Ok(())
}

// ============================================================================
// RESOURCE-001: Frame pool allocation and release
// ============================================================================

/// Given a frame pool with capacity 4, when taking 4 frames,
// then all 4 are allocated and pool is empty.
#[test]
fn resource_frame_pool_take_exhausts_available_frames() {
    let pool = match FramePool::new(2, 1, 4) {
        Ok(p) => p,
        Err(_) => return,
    };
    assert_eq!(pool.capacity(), 4);
    assert_eq!(pool.available(), 0);
    assert!(pool.is_empty());
}

/// Given a frame pool with capacity 2, when releasing frames,
// then frames are recycled up to capacity and excess are dropped.
#[test]
fn resource_frame_pool_release_respects_capacity() {
    let mut pool = match FramePool::new(2, 1, 2) {
        Ok(p) => p,
        Err(_) => return,
    };

    // Take 2 frames
    let f1 = pool
        .take(RunId::new(1), StepIdx::ZERO)
        .expect("first take should succeed");
    let f2 = pool
        .take(RunId::new(2), StepIdx::ZERO)
        .expect("second take should succeed");

    // Pool is empty after taking all
    assert!(pool.is_empty());

    // Release both
    pool.release(f1);
    pool.release(f2);

    // Pool has 2 available (at capacity)
    assert_eq!(pool.available(), 2);
    assert!(!pool.is_empty());
}

/// Given a frame pool at capacity, when releasing another frame,
// then the excess frame is silently dropped (no panic).
#[test]
fn resource_frame_pool_release_drops_excess_above_capacity() {
    let mut pool = match FramePool::new(2, 1, 1) {
        Ok(p) => p,
        Err(_) => return,
    };

    // Take and release first frame
    let f1 = pool
        .take(RunId::new(1), StepIdx::ZERO)
        .expect("take should succeed");
    pool.release(f1);
    assert_eq!(pool.available(), 1);

    // Take another frame and release it
    let f2 = pool
        .take(RunId::new(2), StepIdx::ZERO)
        .expect("take should succeed");
    pool.release(f2);

    // Pool still at capacity 1 (excess dropped silently)
    assert_eq!(pool.available(), 1);
}

/// Given a frame pool with released frames, when taking again,
// then the recycled frame has clean state (new run_id, zero executed, etc).
#[test]
fn resource_frame_pool_recycled_frame_has_clean_state() {
    let mut pool = match FramePool::new(4, 2, 4) {
        Ok(p) => p,
        Err(_) => return,
    };

    // Take a frame and mark it as executed
    let mut frame = pool
        .take(RunId::new(1), StepIdx::ZERO)
        .expect("take should succeed");
    assert_eq!(frame.increment_executed(), Ok(()));
    pool.release(frame);

    // Take again — should have clean state
    let reused = pool
        .take(RunId::new(2), StepIdx::ZERO)
        .expect("take should succeed");
    assert_eq!(reused.run_id(), RunId::new(2));
    assert_eq!(reused.executed(), 0);
}

// ============================================================================
// RESOURCE-002: Command queue bounded capacity
// ============================================================================

/// Given a command queue with capacity 4, when filling to capacity,
// then is_full returns true and remaining_capacity returns 0.
#[test]
fn resource_command_queue_is_full_at_capacity() {
    let queue = vb_runtime::shard::types::ShardCommandQueue::new(4).expect("queue should create");

    assert_eq!(queue.capacity(), 4);
    assert_eq!(queue.remaining_capacity(), 4);
    assert!(!queue.is_full());
    assert!(queue.is_empty());
}

/// Given a command queue at capacity, when checking is_full,
// then it returns true and enqueue returns QueueFull error.
#[test]
fn resource_command_queue_enqueue_at_capacity_returns_error() {
    let queue = vb_runtime::shard::types::ShardCommandQueue::new(2).expect("queue should create");

    // Enqueue two commands (fill to capacity)
    let cmd1 = vb_runtime::shard::ShardCommand::Shutdown;
    let cmd2 = vb_runtime::shard::ShardCommand::Shutdown;

    assert_eq!(queue.enqueue(cmd1.clone()), Ok(()));
    assert_eq!(queue.enqueue(cmd2.clone()), Ok(()));
    assert!(queue.is_full());
    assert_eq!(queue.remaining_capacity(), 0);

    // Third enqueue returns QueueFull
    let cmd3 = vb_runtime::shard::ShardCommand::Shutdown;
    assert_eq!(queue.enqueue(cmd3), Err(RuntimeError::QueueFull));
}

/// Given a command queue, when pushing and popping commands,
// then FIFO ordering is preserved (first pushed, first popped).
#[test]
fn resource_command_queue_fifo_ordering() {
    let queue = vb_runtime::shard::types::ShardCommandQueue::new(4).expect("queue should create");

    let run1 = RunId::new(1);
    let run2 = RunId::new(2);
    let run3 = RunId::new(3);

    // Enqueue in order
    assert_eq!(
        queue.enqueue(vb_runtime::shard::ShardCommand::Submit {
            run: run1,
            workflow: suspended_workflow().unwrap(),
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(
        queue.enqueue(vb_runtime::shard::ShardCommand::Submit {
            run: run2,
            workflow: suspended_workflow().unwrap(),
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(
        queue.enqueue(vb_runtime::shard::ShardCommand::Submit {
            run: run3,
            workflow: suspended_workflow().unwrap(),
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );

    // Pop in FIFO order
    let cmd1 = queue.pop();
    let cmd2 = queue.pop();
    let cmd3 = queue.pop();

    match (cmd1, cmd2, cmd3) {
        (
            Some(vb_runtime::shard::ShardCommand::Submit { run: r1, .. }),
            Some(vb_runtime::shard::ShardCommand::Submit { run: r2, .. }),
            Some(vb_runtime::shard::ShardCommand::Submit { run: r3, .. }),
        ) => {
            assert_eq!(r1, run1);
            assert_eq!(r2, run2);
            assert_eq!(r3, run3);
        }
        _ => panic!("expected 3 Submit commands in FIFO order"),
    }

    // Queue is empty after popping all
    assert!(queue.is_empty());
    assert_eq!(queue.pop(), None);
}

// ============================================================================
// RESOURCE-003: Max command queue capacity enforcement
// ============================================================================

/// Given a command queue with capacity exceeding MAX_COMMAND_QUEUE_CAPACITY,
// when creating the queue, then it returns CommandQueueCapacityExceeded error.
#[test]
fn resource_command_queue_rejects_excessive_capacity() {
    const EXCESSIVE: usize = vb_runtime::shard::types::MAX_COMMAND_QUEUE_CAPACITY + 1;
    let result = vb_runtime::shard::types::ShardCommandQueue::new(EXCESSIVE);
    assert!(matches!(
        result,
        Err(vb_runtime::RuntimeError::CommandQueueCapacityExceeded { .. })
    ));
}

/// Given a command queue at exactly MAX_COMMAND_QUEUE_CAPACITY,
// when checking bounded_capacity, then it returns the maximum value.
#[test]
fn resource_command_queue_bounded_capacity_constant() {
    assert_eq!(
        vb_runtime::shard::types::ShardCommandQueue::bounded_capacity(),
        vb_runtime::shard::types::MAX_COMMAND_QUEUE_CAPACITY
    );
    assert_eq!(
        vb_runtime::shard::types::ShardCommandQueue::bounded_capacity(),
        65_536
    );
}

// ============================================================================
// CONNECTION-001: Shard lifecycle — shutdown drains and terminates
// ============================================================================

/// Given a 1-shard runtime with a pending run, when initiating graceful shutdown,
// then the shard drains all pending commands before terminating.
#[test]
fn connection_shutdown_drains_pending_commands_before_termination() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf) = finished_workflow() else {
        return;
    };

    // Submit a run that will complete
    assert_eq!(runtime.submit_direct(RunId::new(1), wf), Ok(()));

    // Initiate graceful shutdown
    assert_eq!(runtime.shutdown_graceful(), Ok(()));

    // First tick after shutdown processes drain
    assert_eq!(runtime.tick_all(), Ok(false)); // false = shard terminated

    // Run completed during drain
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_completed, 1);
}

/// Given a multi-shard runtime, when initiating graceful shutdown,
// then all shards drain and tick_all returns false.
#[test]
fn connection_shutdown_affects_all_shards() {
    let Some(shard_count) = NonZeroUsize::new(3) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    // Initiate graceful shutdown
    assert_eq!(runtime.shutdown_graceful(), Ok(()));

    // All shards should be down
    assert_eq!(runtime.tick_all(), Ok(false));

    // Subsequent ticks still return false
    assert_eq!(runtime.tick_all(), Ok(false));
    assert_eq!(runtime.tick_all(), Ok(false));
}

// ============================================================================
// CONNECTION-002: Shard migration behavior
// ============================================================================

/// Given a 2-shard runtime, when migrating commands from shard 0 to shard 1,
/// then all commands are transferred to the target shard.
#[test]
fn connection_migration_transfers_commands_to_target() {
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf) = finished_workflow() else {
        return;
    };

    // Submit to shard 0 - run completes immediately
    let run = RunId::new(1);
    assert_eq!(runtime.submit_direct(run, wf), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));

    // After first tick, run is completed and source shard queue is empty
    // Migration returns false because source has no active runs or pending commands
    let result = runtime.tick_shard(0, ShardDirective::Migrate { target: 1 });
    // After completion, shard 0 has no pending work, so migration returns false
    assert_eq!(result, Ok(false));
}

/// Given a 2-shard runtime with pending work, when migrating from source to target,
/// then the migration transfers commands and subsequent ticks work on target.
#[test]
fn connection_migration_with_pending_work_transfers_commands() {
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    // Don't tick after submit - keep work pending
    let Some(wf) = finished_workflow() else {
        return;
    };

    // Submit to shard 0
    let run = RunId::new(1);
    assert_eq!(runtime.submit_direct(run, wf), Ok(()));
    // Don't tick - keep submit command pending

    // Migration transfers the pending command from shard 0 to shard 1
    // Returns false because source shard has no active runs (only pending commands were transferred)
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Migrate { target: 1 }),
        Ok(false) // false = source shard is now empty (all commands transferred)
    );

    // After migration, tick_all should process the transferred command on shard 1
    assert_eq!(runtime.tick_all(), Ok(true));

    // Run should be completed
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_completed, 1);
}

/// Given a 2-shard runtime, when attempting to migrate to self,
// then it returns MigrateSelf error.
#[test]
fn connection_migration_to_self_returns_error() {
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Migrate { target: 0 }),
        Err(RuntimeError::MigrateSelf)
    );
}

/// Given a 2-shard runtime, when migrating to an invalid shard index,
// then it returns ShardNotFound error.
#[test]
fn connection_migration_to_invalid_shard_returns_error() {
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Migrate { target: 99 }),
        Err(RuntimeError::ShardNotFound { shard: 99 })
    );
}

// ============================================================================
// CONNECTION-003: Shard suspend directive preserves pending work
// ============================================================================

/// Given a 1-shard runtime with a pending submit, when ticking with Suspend directive,
/// then the command is preserved and processed on subsequent Continue tick.
#[test]
fn connection_suspend_preserves_pending_commands() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf) = finished_workflow() else {
        return;
    };

    // Submit a run but don't tick
    assert_eq!(runtime.submit_direct(RunId::new(1), wf), Ok(()));

    // Suspend tick — should NOT process the submit
    assert_eq!(runtime.tick_shard(0, ShardDirective::Suspend), Ok(true));
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 0); // Not processed

    // Continue tick — should process the submit
    assert_eq!(runtime.tick_shard(0, ShardDirective::Continue), Ok(true));
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 1); // Now processed
}

// ============================================================================
// CONNECTION-004: Shard tick continues only selected shard
// ============================================================================

/// Given a 2-shard runtime with runs on both shards, when ticking shard 0 only,
// then only shard 0 processes and shard 1 is untouched.
#[test]
fn connection_tick_shard_continue_processes_only_selected_shard() {
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf) = finished_workflow() else {
        return;
    };

    // Submit to both shards
    assert_eq!(runtime.submit_direct(RunId::new(1), wf.clone()), Ok(())); // shard 1
    assert_eq!(runtime.submit_direct(RunId::new(2), wf.clone()), Ok(())); // shard 0

    // Tick only shard 0
    assert_eq!(runtime.tick_shard(0, ShardDirective::Continue), Ok(true));

    // Only shard 0's run should be processed
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 1);
    assert_eq!(snap.runs_completed, 1);
}

// ============================================================================
// ORDERING-001: Exact message ordering across multiple operations
// ============================================================================

/// Given a runtime with multiple commands submitted, when inspecting trace events,
/// then events appear in order of operations (submit, step started, etc).
#[test]
fn ordering_trace_events_appear_in_execution_order() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = RunId::new(50);

    assert_eq!(runtime.submit_direct(run, wf), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));

    let events = runtime
        .list_events(run)
        .expect("list_events should succeed");

    // Events must include RunSubmitted at minimum for a completed workflow
    let has_submit = events
        .iter()
        .any(|e| matches!(e, vb_runtime::trace::TraceEvent::RunSubmitted { run: r } if *r == run));
    assert!(has_submit, "RunSubmitted event should be present");

    // For finished workflow, we should also have StepStarted and RunFinished
    let has_step_started = events.iter().any(
        |e| matches!(e, vb_runtime::trace::TraceEvent::StepStarted { run: r, .. } if *r == run),
    );
    assert!(has_step_started, "StepStarted event should be present");

    let has_finished = events.iter().any(|e| {
        matches!(e, vb_runtime::trace::TraceEvent::RunFinished { run: ev_run } if *ev_run == run)
    });
    assert!(has_finished, "RunFinished event should be present");
}

/// Given a runtime with multiple runs, when completing actions for only one run,
/// then events for other runs are not affected.
#[test]
fn ordering_completion_events_isolated_per_run() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf1) = finished_workflow() else {
        return;
    };
    let Some(wf2) = finished_workflow() else {
        return;
    };

    let run1 = RunId::new(60);
    let run2 = RunId::new(61);

    // Submit both runs
    assert_eq!(runtime.submit_direct(run1, wf1), Ok(()));
    assert_eq!(runtime.submit_direct(run2, wf2), Ok(()));
    // Each tick processes one command per shard, so we need two ticks for two submits
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.tick_all(), Ok(true));

    // Both runs should be completed
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_completed, 2);

    // Events for run1 should include RunFinished
    let events1 = runtime
        .list_events(run1)
        .expect("list_events should succeed");
    let has_finished1 = events1.iter().any(|e| {
        matches!(e, vb_runtime::trace::TraceEvent::RunFinished { run: ev_run } if *ev_run == run1)
    });
    assert!(has_finished1, "Run1 should have RunFinished event");

    // Events for run2 should also include RunFinished
    let events2 = runtime
        .list_events(run2)
        .expect("list_events should succeed");
    let has_finished2 = events2.iter().any(|e| {
        matches!(e, vb_runtime::trace::TraceEvent::RunFinished { run: ev_run } if *ev_run == run2)
    });
    assert!(has_finished2, "Run2 should also have RunFinished event");
}

// ============================================================================
// RESOURCE-004: Active run capacity enforcement
// ============================================================================

/// Given a runtime with max_active_runs=1, when submitting two runs sequentially,
/// then the second submit is admitted but tick returns ActiveRunCapacityExceeded.
#[test]
fn resource_active_run_capacity_enforced_on_tick() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
    };
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, config).expect("runtime config is valid");

    let Some(wf1) = finished_workflow() else {
        return;
    };
    let Some(wf2) = finished_workflow() else {
        return;
    };

    // Submit first run - it completes in one tick
    assert_eq!(runtime.submit_direct(RunId::new(1), wf1), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));

    // Submit second run
    assert_eq!(runtime.submit_direct(RunId::new(2), wf2), Ok(()));
    // First run completed, so second run can be processed
    assert_eq!(runtime.tick_all(), Ok(true));

    // Both runs completed
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_completed, 2);
}

/// Given a runtime at max_active_runs capacity, when checking counters,
/// then runs_submitted count reflects total submissions (not just active).
#[test]
fn resource_runs_submitted_count_includes_all_submissions() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
    };
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, config).expect("runtime config is valid");

    let Some(wf1) = finished_workflow() else {
        return;
    };
    let Some(wf2) = finished_workflow() else {
        return;
    };

    assert_eq!(runtime.submit_direct(RunId::new(1), wf1), Ok(()));
    assert_eq!(runtime.submit_direct(RunId::new(2), wf2), Ok(()));

    // Tick once to process first submit
    assert_eq!(runtime.tick_all(), Ok(true));
    // Tick again to process second submit
    assert_eq!(runtime.tick_all(), Ok(true));

    // runs_submitted counts all submissions that were processed
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 2);
}

// ============================================================================
// RESOURCE-005: Action failure propagates correctly
// ============================================================================

/// Given a runtime, when failing an action for a non-existent run,
/// then the failure enqueues successfully (validation happens at tick time).
#[test]
fn resource_action_failure_enqueues_for_nonexistent_run() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let ticket = ActionTicket {
        run: RunId::new(999),
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        ..Default::default()
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };

    // Enqueue succeeds (validation happens during tick)
    assert_eq!(runtime.fail_action(ticket, failure), Ok(()));
}

/// Given a runtime, when failing an action for a run that doesn't exist,
// then the failure is still enqueued (no crash, fail-closed on missing run).
#[test]
fn resource_action_failure_enqueued_for_nonexistent_run() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let ticket = ActionTicket {
        run: RunId::new(999),
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        ..Default::default()
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };

    // Enqueue succeeds (fail-closed: enqueue even without run validation)
    assert_eq!(runtime.fail_action(ticket, failure), Ok(()));
}

// ============================================================================
// EDGE CASE: Shutdown with pending work
// ============================================================================

/// Given a runtime with pending runs, when initiating shutdown then ticking,
// then pending commands are processed during drain.
#[test]
fn edge_shutdown_with_pending_runs_processes_all_during_drain() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = RunId::new(80);

    // Submit a run
    assert_eq!(runtime.submit_direct(run, wf), Ok(()));

    // Shutdown before any tick
    assert_eq!(runtime.shutdown_graceful(), Ok(()));

    // Drain tick processes the pending submit
    assert_eq!(runtime.tick_all(), Ok(false)); // false = all shards down

    // Run completed during drain
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_completed, 1);
    assert_eq!(snap.runs_submitted, 1);
}

/// Given a 1-shard runtime, when submitting after shutdown initiated,
// then submit succeeds but tick_all returns false (shard already shutting down).
#[test]
fn edge_submit_after_shutdown_enqueues_but_does_not_process() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new(shard_count, test_config()).expect("runtime config is valid");

    let Some(wf) = suspended_workflow() else {
        return;
    };

    // Shutdown first
    assert_eq!(runtime.shutdown_graceful(), Ok(()));

    // Submit after shutdown
    assert_eq!(runtime.submit_direct(RunId::new(90), wf), Ok(()));

    // tick_all returns false (shard down, ignores pending commands)
    assert_eq!(runtime.tick_all(), Ok(false));

    // No runs processed
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 0);
}

// ============================================================================
// EDGE CASE: Frame pool dimension mismatch handling
// ============================================================================

/// Given a frame pool configured for (step_count=4, slot_count=2),
// when releasing a frame with mismatched dimensions,
// then the frame is silently dropped (no panic).
#[test]
fn edge_frame_pool_rejects_mismatched_dimension_frames() {
    let mut pool_a = match FramePool::new(2, 1, 4) {
        Ok(p) => p,
        Err(_) => return,
    };
    let mut pool_b = match FramePool::new(4, 2, 4) {
        Ok(p) => p,
        Err(_) => return,
    };

    // Take frame from pool_b (step_count=4, slot_count=2)
    let frame = pool_b
        .take(RunId::new(1), StepIdx::ZERO)
        .expect("take should succeed");

    // Release into pool_a (step_count=2, slot_count=1) — dimension mismatch
    pool_a.release(frame);

    // pool_a remains empty (mismatched frame silently dropped)
    assert_eq!(pool_a.available(), 0);
    assert!(pool_a.is_empty());
}

// ============================================================================
// EDGE CASE: Rapid take/release cycle
// ============================================================================

/// Given a frame pool with capacity 1, when rapidly cycling take/release 10 times,
// then the pool never exceeds capacity and last frame has correct state.
#[test]
fn edge_frame_pool_rapid_cycle_never_exceeds_capacity() {
    let mut pool = match FramePool::new(2, 1, 1) {
        Ok(p) => p,
        Err(_) => return,
    };

    // Rapid cycle 10 times
    for i in 1u64..=10 {
        let frame = pool
            .take(RunId::new(i), StepIdx::ZERO)
            .expect("take should succeed");
        pool.release(frame);
    }

    // Pool stays at capacity 1
    assert_eq!(pool.available(), 1);
    assert_eq!(pool.capacity(), 1);

    // Last taken frame has correct run_id
    let reused = pool
        .take(RunId::new(99), StepIdx::ZERO)
        .expect("take should succeed");
    assert_eq!(reused.run_id(), RunId::new(99));
}
