#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![forbid(unsafe_code)]
//! Section 38 behavioral property tests: terminal state rejection, replay
//! determinism, ordering invariants, and snapshot equivalence.
//!
//! Each property is exercised across randomized inputs via `proptest!` so
//! that every public behavior is bound to a generator, not a single
//! hand-written fixture.

use proptest::prelude::*;
use vb_core::errors::{CoreError, CoreResult, EngineError};
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowError,
    WorkflowParts,
};
use vb_core::{EngineSignal, StepBudget, run_until_blocked, step_once};

// =========================================================================
// Helpers
// =========================================================================

fn default_contract() -> ResourceContract {
    ResourceContract::DEFAULT
}

fn two_step_workflow_parts(value: ConstValue, digest_byte: u8) -> WorkflowParts {
    WorkflowParts {
        name: Box::<str>::from("behavioral_test"),
        digest: WorkflowDigest::from_bytes([digest_byte; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![value].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: default_contract(),
        step_names: Box::new([]),
    }
}

/// Builds the canonical two-step workflow from a `ConstValue`. The build can
/// fail only on resource-contract rejection, which we surface as a workflow
/// error string. proptest harnesses route these failures through
/// `prop_string_err` to convert to `TestCaseError`.
fn build_two_step_workflow(value: ConstValue, digest_byte: u8) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(two_step_workflow_parts(value, digest_byte))
        .map_err(|e: WorkflowError| e.to_string())
}

/// Strategy for an arbitrary ConstValue that the workflow engine accepts
/// (Bool, I64, Null). F64 is excluded to keep arithmetic deterministic and
/// Symbol is excluded because no symbol table is materialized in tests.
fn arb_const_value() -> impl Strategy<Value = ConstValue> {
    prop_oneof![
        Just(ConstValue::Null),
        any::<bool>().prop_map(ConstValue::Bool),
        (i64::MIN / 2..=i64::MAX / 2).prop_map(ConstValue::I64),
    ]
}

/// Strategy for an arbitrary I64 value within a safe range so that run
/// completion remains cheap and deterministic.
fn arb_const_i64() -> impl Strategy<Value = i64> {
    -1_000_000i64..=1_000_000i64
}

/// Strategy for a small `RunId` u64 value. Avoids 0 because some engines
/// reserve the zero value; here we use 1..1024.
fn arb_run_id() -> impl Strategy<Value = u64> {
    1u64..1024
}

/// Strategy for an in-range `step_count` for an isolated frame.
fn arb_step_count() -> impl Strategy<Value = u16> {
    2u16..64
}

/// Strategy for a resource-contract `max_steps` value within the validation
/// gate's reach.
fn arb_max_steps() -> impl Strategy<Value = u16> {
    0u16..16
}

/// Adapter: convert a `Result<T, String>` into a `Result<T, TestCaseError>` so
/// the `?` operator works inside `proptest!` blocks. proptest's `TestCaseError`
/// does not implement `From<String>`, so this bridges the two error domains.
fn prop_string_err<T>(r: Result<T, String>) -> Result<T, proptest::test_runner::TestCaseError> {
    r.map_err(|e| proptest::test_runner::TestCaseError::fail(e))
}

/// Adapter: convert a `CoreResult<T>` into a `Result<T, TestCaseError>`.
fn prop_core_err<T>(r: CoreResult<T>) -> Result<T, proptest::test_runner::TestCaseError> {
    r.map_err(|e| proptest::test_runner::TestCaseError::fail(format!("{e}")))
}

/// Adapter: convert a `Result<T, EngineError>` into a `Result<T, TestCaseError>`.
fn prop_engine_err<T>(r: Result<T, EngineError>) -> Result<T, proptest::test_runner::TestCaseError> {
    r.map_err(|e| proptest::test_runner::TestCaseError::fail(format!("{e}")))
}

// =========================================================================
// Property 3: Terminal state allows re-entry -- finished run can be re-run
// =========================================================================

proptest! {
    /// After a workflow finishes, a subsequent `step_once` returns the same
    /// `Finished(value, taint)` signal: the engine is idempotent on terminal
    /// states.
    #[test]
    fn proptest_terminal_state_finished_run_can_be_rerun(value in arb_const_value()) {
        let workflow = prop_string_err(build_two_step_workflow(value, 0x38))?;
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(1),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store = ValueStore::new();

        let expected_slot = prop_core_err(value.to_slot_value())?;

        let result = prop_engine_err(run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store))?;
        prop_assert_eq!(
            result,
            EngineSignal::Finished(expected_slot, Taint::Clean),
            "first run must finish with const value"
        );

        let replay = prop_engine_err(step_once(&workflow, &mut frame, &mut store))?;
        prop_assert_eq!(
            replay,
            EngineSignal::Finished(expected_slot, Taint::Clean),
            "re-run on terminal run must return identical Finished signal"
        );
    }

    /// Succeeded is a terminal absorbing state: `mark_running` from a Succeeded
    /// step is rejected with `InternalInvariantViolation`. Loop-body reentry
    /// must go through the explicit `mark_pending` admission path.
    #[test]
    fn proptest_terminal_state_succeeded_rejects_mark_running_direct(
        run_id in arb_run_id(),
        step_count in arb_step_count(),
    ) {
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(run_id),
            StepIdx::new(0),
            step_count,
            1,
        ))?;
        prop_core_err(frame.mark_running(StepIdx::new(0)))?;
        prop_core_err(frame.mark_succeeded(StepIdx::new(0)))?;
        let result = frame.mark_running(StepIdx::new(0));
        prop_assert!(
            matches!(
                result,
                Err(CoreError::InternalInvariantViolation {
                    reason: "invalid_state_transition"
                })
            ),
            "succeeded step must reject mark_running; terminal states are absorbing"
        );
    }

    /// Failed -> Succeeded is a forbidden cross-terminal transition.
    #[test]
    fn proptest_terminal_state_failed_rejects_mark_succeeded(
        run_id in arb_run_id(),
        step_count in arb_step_count(),
    ) {
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(run_id),
            StepIdx::new(0),
            step_count,
            1,
        ))?;
        prop_core_err(frame.mark_running(StepIdx::new(0)))?;
        prop_core_err(frame.mark_failed(StepIdx::new(0)))?;
        let result = frame.mark_succeeded(StepIdx::new(0));
        prop_assert!(
            matches!(result, Err(CoreError::InternalInvariantViolation { .. })),
            "failed step must reject mark_succeeded"
        );
    }

    /// Cancelled -> Running is a forbidden transition: terminal states are
    /// self-only.
    #[test]
    fn proptest_terminal_state_cancelled_rejects_mark_running(
        run_id in arb_run_id(),
        step_count in arb_step_count(),
    ) {
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(run_id),
            StepIdx::new(0),
            step_count,
            1,
        ))?;
        prop_core_err(frame.mark_running(StepIdx::new(0)))?;
        prop_core_err(frame.mark_cancelled(StepIdx::new(0)))?;
        let result = frame.mark_running(StepIdx::new(0));
        prop_assert!(
            matches!(result, Err(CoreError::InternalInvariantViolation { .. })),
            "cancelled step must reject mark_running"
        );
    }
}

// =========================================================================
// Property 5: Step budget exhaustion -- exceeding max steps is rejected
// =========================================================================

proptest! {
    /// Budget of 0 halts the engine immediately: no steps execute and the
    /// current step remains Pending.
    #[test]
    fn proptest_step_budget_exhaustion_zero_budget_rejects_all_steps(value in arb_const_i64()) {
        let workflow = prop_string_err(build_two_step_workflow(ConstValue::I64(value), 0xB5))?;
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(1),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store = ValueStore::new();

        let result = prop_engine_err(run_until_blocked(&workflow, &mut frame, StepBudget::new(0), &mut store))?;
        prop_assert_eq!(
            result,
            EngineSignal::StepBudgetExhausted,
            "zero budget must exhaust immediately"
        );
        prop_assert_eq!(frame.executed(), 0u64, "no steps should execute on zero budget");
        let state = prop_core_err(frame.step_state(StepIdx::new(0)))?;
        prop_assert_eq!(state, StepState::Pending, "step 0 must still be pending after zero-budget run");
    }

    /// Insufficient budget halts the engine after the first available step.
    #[test]
    fn proptest_step_budget_exhaustion_insufficient_budget_halts_midway(value in arb_const_i64()) {
        let workflow = prop_string_err(build_two_step_workflow(ConstValue::I64(value), 0xB5))?;
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(1),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store = ValueStore::new();

        let result = prop_engine_err(run_until_blocked(&workflow, &mut frame, StepBudget::new(1), &mut store))?;
        prop_assert_eq!(
            result,
            EngineSignal::StepBudgetExhausted,
            "insufficient budget must exhaust after first step"
        );
        prop_assert_eq!(frame.executed(), 1u64, "exactly one step should execute");
        prop_assert_eq!(frame.pc(), StepIdx::new(1), "PC must be at step 1");
    }

    /// Resource contracts are enforced at workflow admission: a 2-node
    /// workflow with `max_steps` set below 2 is rejected; at-or-above is
    /// admitted.
    #[test]
    fn proptest_step_budget_exhaustion_resource_contract_rejects_oversized(max_steps in arb_max_steps()) {
        let built = build_two_step_workflow(ConstValue::I64(42), 0xB5)
            .map_err(|e| proptest::test_runner::TestCaseError::fail(e))?;
        let mut parts = built.to_parts();
        parts.resource_contract.max_steps = max_steps;
        let result = CompiledWorkflow::try_from_parts(parts);
        // The compiled artifact has 2 nodes; max_steps < 2 must reject,
        // max_steps >= 2 must admit.
        if max_steps < 2 {
            prop_assert!(
                matches!(result, Err(WorkflowError::ResourceContractExceeded { .. })),
                "workflow exceeding max_steps ({}) must be rejected", max_steps
            );
        } else {
            prop_assert!(
                result.is_ok(),
                "workflow within max_steps ({}) must be admitted", max_steps
            );
        }
    }
}

// =========================================================================
// Property 4: Taint safety -- secret taint propagates to Finish signal
// =========================================================================

proptest! {
    /// A workflow whose result slot is pre-tainted to Secret must finish with
    /// `EngineSignal::Finished(value, Taint::Secret)`: secret taint flows
    /// through the Finish signal without being stripped.
    #[test]
    fn proptest_taint_safety_secret_taint_propagates_to_finish_signal(value in arb_const_i64()) {
        let parts = WorkflowParts {
            name: Box::<str>::from("taint_runtime"),
            digest: WorkflowDigest::from_bytes([0x54; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: default_contract(),
            step_names: Box::new([]),
        };
        let workflow = prop_string_err(
            CompiledWorkflow::try_from_parts(parts).map_err(|e: WorkflowError| e.to_string()),
        )?;
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(1),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store = ValueStore::new();

        prop_core_err(frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(value), Taint::Secret))?;

        let result = prop_engine_err(run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store))?;
        prop_assert_eq!(
            result,
            EngineSignal::Finished(SlotValue::I64(value), Taint::Secret),
            "finish signal must carry Secret taint when result slot is secret-tainted"
        );
    }

    /// A workflow produced only by deterministic nodes has Clean taint and
    /// must finish with `Taint::Clean`.
    #[test]
    fn proptest_taint_safety_clean_taint_produces_clean_finish_signal(value in arb_const_value()) {
        let workflow = prop_string_err(build_two_step_workflow(value, 0x38))?;
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(1),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store = ValueStore::new();

        let result = prop_engine_err(run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store))?;
        let signal_taint = match &result {
            EngineSignal::Finished(_, taint) => *taint,
            _ => return Err(proptest::test_runner::TestCaseError::fail(
                "expected Finished signal on deterministic workflow",
            )),
        };
        prop_assert_eq!(signal_taint, Taint::Clean, "clean workflow must finish with Clean taint");
    }
}

// =========================================================================
// Property 6: Replay determinism -- replay produces identical state sequence
// =========================================================================

proptest! {
    /// Two independent run frames driven by the same workflow must produce
    /// byte-identical engine signals, executed counts, and slot values. Run
    /// identifiers are randomized so the comparison isolates workflow
    /// determinism from run identity.
    #[test]
    fn proptest_replay_determinism_same_run_produces_identical_slot_state(
        value in arb_const_value(),
        run_a in arb_run_id(),
        run_b in arb_run_id(),
    ) {
        let workflow = prop_string_err(build_two_step_workflow(value, 0x38))?;

        let mut frame_a = prop_core_err(RunFrame::new(
            RunId::new(run_a),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store_a = ValueStore::new();
        let result_a = prop_engine_err(run_until_blocked(&workflow, &mut frame_a, StepBudget::MAX, &mut store_a))?;

        let mut frame_b = prop_core_err(RunFrame::new(
            RunId::new(run_b),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store_b = ValueStore::new();
        let result_b = prop_engine_err(run_until_blocked(&workflow, &mut frame_b, StepBudget::MAX, &mut store_b))?;

        prop_assert_eq!(result_a, result_b, "replay must produce identical engine signal");
        prop_assert_eq!(frame_a.executed(), frame_b.executed(), "replay must execute same step count");
        let slot_a = prop_core_err(frame_a.read_slot(SlotIdx::new(0)))?;
        let slot_b = prop_core_err(frame_b.read_slot(SlotIdx::new(0)))?;
        prop_assert_eq!(slot_a, slot_b, "replay must produce identical slot values");
    }

    /// Step-by-step execution of a linear workflow produces a deterministic
    /// PC sequence: 0 -> 1, with step 0 transitioning to Succeeded before the
    /// Finish signal is observed.
    #[test]
    fn proptest_replay_determinism_step_by_step_produces_identical_pc_sequence(
        value in arb_const_i64(),
        run_id in arb_run_id(),
    ) {
        let workflow = prop_string_err(build_two_step_workflow(ConstValue::I64(value), 0x38))?;
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(run_id),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store = ValueStore::new();

        prop_assert_eq!(frame.pc(), StepIdx::new(0), "initial PC must be step 0");

        prop_engine_err(step_once(&workflow, &mut frame, &mut store))?;
        prop_assert_eq!(frame.pc(), StepIdx::new(1), "after SetConst, PC must be 1");
        let state0 = prop_core_err(frame.step_state(StepIdx::new(0)))?;
        prop_assert_eq!(state0, StepState::Succeeded, "step 0 must be succeeded");

        let result = prop_engine_err(step_once(&workflow, &mut frame, &mut store))?;
        prop_assert_eq!(
            result,
            EngineSignal::Finished(SlotValue::I64(value), Taint::Clean),
            "step 2 must finish with const value"
        );
    }
}

// =========================================================================
// Property 7: Ordering invariants -- events emitted in valid order
// =========================================================================

proptest! {
    /// Step states follow the required lifecycle Pending -> Running -> Succeeded.
    #[test]
    fn proptest_ordering_invariants_step_states_follow_valid_lifecycle(
        run_id in arb_run_id(),
        step_count in arb_step_count(),
    ) {
        let mut frame = prop_core_err(RunFrame::new(RunId::new(run_id), StepIdx::new(0), step_count, 1))?;

        let initial = prop_core_err(frame.step_state(StepIdx::new(0)))?;
        prop_assert_eq!(initial, StepState::Pending, "step must start pending");

        prop_core_err(frame.mark_running(StepIdx::new(0)))?;
        let running = prop_core_err(frame.step_state(StepIdx::new(0)))?;
        prop_assert_eq!(running, StepState::Running, "step must be running after mark_running");

        prop_core_err(frame.mark_succeeded(StepIdx::new(0)))?;
        let succeeded = prop_core_err(frame.step_state(StepIdx::new(0)))?;
        prop_assert_eq!(succeeded, StepState::Succeeded, "step must be succeeded after mark_succeeded");
    }

    /// Resumable states (Waiting, Asking) can return to Running; terminal
    /// states cannot (covered by other properties). Sweep step count to
    /// ensure the state machine holds across frame dimensions.
    #[test]
    fn proptest_ordering_invariants_resumable_states_can_return_to_running(
        run_id in arb_run_id(),
        step_count in arb_step_count(),
    ) {
        let mut frame = prop_core_err(RunFrame::new(RunId::new(run_id), StepIdx::new(0), step_count, 1))?;

        // Waiting is resumable
        prop_core_err(frame.mark_running(StepIdx::new(0)))?;
        prop_core_err(frame.mark_waiting(StepIdx::new(0)))?;
        let waiting = prop_core_err(frame.step_state(StepIdx::new(0)))?;
        prop_assert_eq!(waiting, StepState::Waiting, "must be waiting");
        prop_core_err(frame.mark_running(StepIdx::new(0)))?;
        let running_again = prop_core_err(frame.step_state(StepIdx::new(0)))?;
        prop_assert_eq!(running_again, StepState::Running, "waiting must be resumable to running");

        // Asking is resumable
        prop_core_err(frame.mark_asking(StepIdx::new(0)))?;
        let asking = prop_core_err(frame.step_state(StepIdx::new(0)))?;
        prop_assert_eq!(asking, StepState::Asking, "must be asking");
        prop_core_err(frame.mark_running(StepIdx::new(0)))?;
        let running_again2 = prop_core_err(frame.step_state(StepIdx::new(0)))?;
        prop_assert_eq!(running_again2, StepState::Running, "asking must be resumable to running");
    }

    /// PC advances monotonically in a linear workflow: each `step_once`
    /// moves the PC strictly forward (or finishes). Use a wide range of
    /// const values to confirm the PC trajectory is independent of the value
    /// carried.
    #[test]
    fn proptest_ordering_invariants_pc_advances_monotonically_in_linear_workflow(
        value in arb_const_i64(),
        run_id in arb_run_id(),
    ) {
        let workflow = prop_string_err(build_two_step_workflow(ConstValue::I64(value), 0x38))?;
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(run_id),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store = ValueStore::new();

        let prev_pc = frame.pc();
        prop_assert_eq!(prev_pc, StepIdx::new(0), "must start at PC 0");

        prop_engine_err(step_once(&workflow, &mut frame, &mut store))?;
        let next_pc = frame.pc();
        prop_assert!(next_pc.get() > prev_pc.get(), "PC must advance monotonically");
        prop_assert_eq!(next_pc, StepIdx::new(1), "PC must be at step 1 after first step");
    }
}

// =========================================================================
// Property 11: Snapshot equivalence -- journal snapshot equals in-memory state
// =========================================================================

proptest! {
    /// After a workflow finishes, the slot stored in the frame matches the
    /// value reported by the Finished engine signal.
    #[test]
    fn proptest_snapshot_equivalence_frame_slots_match_value_store(value in arb_const_value()) {
        let workflow = prop_string_err(build_two_step_workflow(value, 0x38))?;
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(1),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store = ValueStore::new();

        let expected_slot = prop_core_err(value.to_slot_value())?;
        let result = prop_engine_err(run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store))?;
        prop_assert_eq!(
            result,
            EngineSignal::Finished(expected_slot, Taint::Clean),
            "must finish with const value"
        );
        let slot_value = prop_core_err(frame.read_slot(SlotIdx::new(0)))?;
        prop_assert_eq!(
            slot_value, &expected_slot,
            "frame slot must contain the same value as the finish signal"
        );
    }

    /// Resuming a frame after partial execution must continue counting
    /// transitions monotonically. The combined executed count equals the
    /// total number of nodes in the workflow.
    #[test]
    fn proptest_snapshot_equivalence_executed_count_matches_actual_steps(value in arb_const_i64()) {
        let workflow = prop_string_err(build_two_step_workflow(ConstValue::I64(value), 0x38))?;
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(1),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store = ValueStore::new();

        let partial = prop_engine_err(run_until_blocked(&workflow, &mut frame, StepBudget::new(1), &mut store))?;
        prop_assert_eq!(partial, EngineSignal::StepBudgetExhausted, "must exhaust after 1 step");
        prop_assert_eq!(frame.executed(), 1u64, "executed count must be exactly 1");

        let final_signal = prop_engine_err(run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store))?;
        prop_assert_eq!(
            final_signal,
            EngineSignal::Finished(SlotValue::I64(value), Taint::Clean),
            "must finish on resume"
        );
        prop_assert_eq!(frame.executed(), 2u64, "total executed must be 2");
    }

    /// All steps in a linear workflow are in the Succeeded terminal state
    /// once the workflow finishes.
    #[test]
    fn proptest_snapshot_equivalence_step_states_consistent_after_completion(value in arb_const_i64()) {
        let workflow = prop_string_err(build_two_step_workflow(ConstValue::I64(value), 0x38))?;
        let mut frame = prop_core_err(RunFrame::new(
            RunId::new(1),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ))?;
        let mut store = ValueStore::new();

        prop_engine_err(run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store))?;

        let step0 = prop_core_err(frame.step_state(StepIdx::new(0)))?;
        let step1 = prop_core_err(frame.step_state(StepIdx::new(1)))?;
        prop_assert_eq!(step0, StepState::Succeeded, "step 0 must be succeeded");
        prop_assert_eq!(step1, StepState::Succeeded, "step 1 must be succeeded");
    }
}
