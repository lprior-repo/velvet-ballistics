//! Admission and evidence chain integration tests.
//!
//! These tests exercise end-to-end flows across multiple crates: submitting
//! artifacts, running workflows under various policies, verifying journal
//! evidence chains, capability enforcement, budget validation, and taint
//! propagation.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::ids::{ActionId, ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fail_assert(_message: std::fmt::Arguments<'_>) -> bool {
    false
}

macro_rules! fail_assert {
    ($($arg:tt)*) => {
        assert!(fail_assert(format_args!($($arg)*)), $($arg)*)
    }
}

/// Creates a simple two-node workflow: SetConst(42) -> Finish(result=slot0).
fn set_const_finish_workflow(digest: WorkflowDigest) -> Option<CompiledWorkflow> {
    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("set_finish"),
        digest,
        nodes: Box::from([node0, node1]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

/// Creates a workflow with a Do node requiring action 7.
fn do_action_workflow(digest: WorkflowDigest) -> Option<CompiledWorkflow> {
    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        kind: CompiledNodeKind::Do {
            action: ActionId::new(7),
            input: SlotIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("do_action"),
        digest,
        nodes: Box::from([node0, node1]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

/// Creates a workflow that evaluates an expression loading slot 0 (which will
/// be tainted) and writes the result to slot 1, then finishes with slot 1.
fn eval_expr_taint_workflow(digest: WorkflowDigest) -> Option<CompiledWorkflow> {
    // Expression program: LoadSlot(0), LoadConst(0), Add -> loads slot 0 (tainted),
    // loads constant 0, adds them, result inherits slot 0 taint.
    let expr_program = ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Add,
    ]))
    .ok()?;

    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        kind: CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("taint_expr"),
        digest,
        nodes: Box::from([node0, node1]),
        expressions: Box::from([expr_program]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(1)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

fn test_config() -> vb_runtime::shard::ShardConfig {
    vb_runtime::shard::ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 256,
        step_budget_per_tick: 100,
        max_active_runs: 16,
    }
}

fn temp_journal() -> Option<(tempfile::TempDir, Arc<vb_storage::FjallJournal>)> {
    let dir = tempfile::tempdir().ok()?;
    let journal = vb_storage::FjallJournal::open(dir.path(), None).ok()?;
    Some((dir, Arc::new(journal)))
}

// ===========================================================================
// Test 1: submit_artifact then run succeeds
// ===========================================================================

#[test]
fn submit_artifact_then_run_succeeds() {
    // Given: a compiled workflow and a Fjall journal.
    // We compile through the full pipeline (vb_yaml -> vb_validate -> vb_compile)
    // and then submit the artifact under Relaxed policy (which skips checksum
    // verification since the compile pipeline sets digest from source hash
    // rather than from serialized IR hash).
    let workflow_yaml = b"version: velvet-ballastics/v1\nname: artifact_test\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 42\n  - id: done\n    finish:\n      result: 0\n";
    let workflow = match vb_compile::compile_workflow(workflow_yaml) {
        Ok(w) => w,
        Err(err) => {
            fail_assert!("compile_workflow failed: {err}");
            return;
        }
    };
    let digest = workflow.digest();

    let Some((_dir, journal)) = temp_journal() else {
        fail_assert!("temp journal open failed");
        return;
    };

    // When: submitting the artifact under Relaxed policy
    let artifact_result =
        vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed);
    match artifact_result {
        Ok(artifact) => {
            assert_eq!(
                artifact.digest, digest,
                "submit_artifact should return the workflow digest"
            );
        }
        Err(err) => {
            fail_assert!("submit_artifact failed: {err}");
            return;
        }
    }

    // Then: the artifact is stored and the workflow can be loaded and run
    let stored = journal.compiled_ir(digest);
    match stored {
        Ok(Some(_record)) => {}
        Ok(None) => {
            fail_assert!("artifact should be stored after submit_artifact");
            return;
        }
        Err(err) => {
            fail_assert!("compiled_ir lookup failed: {err}");
            return;
        }
    }

    // Run the workflow through the runtime to verify end-to-end success
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut runtime =
        vb_runtime::runtime::Runtime::new_with_journal(
            shard_count,
            test_config(),
            vb_runtime::journal::NoopRuntimeJournal::shared(),
        );
    let run_id = RunId::new(1);
    match runtime.submit_direct(run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }
    let snap = runtime.counters_snapshot();
    assert_eq!(
        snap.runs_completed, 1,
        "workflow should complete successfully after artifact submission"
    );
}

// ===========================================================================
// Test 2: run without artifact under relaxed policy
// ===========================================================================

#[test]
fn run_without_artifact_under_relaxed_policy() {
    // Given: a compiled workflow and a relaxed policy
    let Some((_dir, journal)) = temp_journal() else {
        fail_assert!("temp journal open failed");
        return;
    };
    let digest = WorkflowDigest::from_bytes([2u8; 32]);
    let Some(workflow) = set_const_finish_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };

    // When: submitting under Relaxed policy (no verification required)
    let result =
        vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed);
    match result {
        Ok(artifact) => {
            assert_eq!(artifact.digest, digest);
        }
        Err(err) => {
            fail_assert!("relaxed submit_artifact should succeed: {err}");
            return;
        }
    }

    // Then: the artifact is stored and the workflow can run
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut runtime =
        vb_runtime::runtime::Runtime::new_with_journal(
            shard_count,
            test_config(),
            vb_runtime::journal::NoopRuntimeJournal::shared(),
        );
    let run_id = RunId::new(2);
    match runtime.submit_direct(run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }
    let snap = runtime.counters_snapshot();
    assert_eq!(
        snap.runs_completed, 1,
        "relaxed policy should allow running without strict verification"
    );
}

// ===========================================================================
// Test 3: evidence chain after execution
// ===========================================================================

#[test]
fn evidence_chain_after_execution() {
    // Given: a multi-step workflow that runs an action, then finishes.
    // Using a Do + Finish workflow: the action completion produces
    // SlotWritten and StepSucceeded events in addition to RunSubmitted/RunFinished.
    let digest = WorkflowDigest::from_bytes([3u8; 32]);
    let Some(workflow) = do_action_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let journal = Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let mut runtime =
        vb_runtime::runtime::Runtime::new_with_journal(shard_count, test_config(), journal.clone());
    let run_id = RunId::new(3);

    // When: submitting and ticking (suspends on action)
    match runtime.submit_direct(run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }

    // Complete the action to resume and finish the workflow
    let ticket = vb_core::action::ActionTicket {
        run: run_id,
        step: StepIdx::new(0),
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(7),
        attempt: 1,
        idempotency_key: 0,
    };
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(99),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    match runtime.complete_action_with_output(ticket, output) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("complete_action_with_output failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly after action completion");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed after action completion: {err}");
            return;
        }
    }

    // Then: the journal contains the full evidence chain:
    // RunSubmitted -> StepSucceeded (action) -> SlotWritten -> ActionCompleted ->
    // StepSucceeded (finish) -> RunFinished
    let events = match journal.snapshot() {
        Ok(events) => events,
        Err(err) => {
            fail_assert!("journal snapshot failed: {err}");
            return;
        }
    };

    let mut found_step_succeeded = false;
    let mut found_slot_written = false;
    let mut found_run_submitted = false;
    let mut found_run_finished = false;

    for event in &events {
        match event {
            vb_runtime::journal::RuntimeJournalEvent::RunSubmitted { run, .. }
                if *run == run_id =>
            {
                found_run_submitted = true;
            }
            vb_runtime::journal::RuntimeJournalEvent::RunFinished { run, .. }
                if *run == run_id =>
            {
                found_run_finished = true;
            }
            vb_runtime::journal::RuntimeJournalEvent::StepSucceeded { run, .. }
                if *run == run_id =>
            {
                found_step_succeeded = true;
            }
            vb_runtime::journal::RuntimeJournalEvent::SlotWritten { run, .. }
                if *run == run_id =>
            {
                found_slot_written = true;
            }
            _ => {}
        }
    }

    assert!(
        found_run_submitted,
        "journal should contain RunSubmitted event"
    );
    assert!(
        found_run_finished,
        "journal should contain RunFinished event"
    );
    assert!(
        found_step_succeeded,
        "journal should contain StepSucceeded event"
    );
    assert!(
        found_slot_written,
        "journal should contain SlotWritten event"
    );
}

// ===========================================================================
// Test 4: capability check rejects unauthorized action
// ===========================================================================

#[test]
fn capability_check_rejects_unauthorized_action() {
    // Given: a capability set that does NOT grant Action(7)
    let empty_caps = vb_core::CapabilitySet::empty();
    let required = vb_core::Capability::Action(ActionId::new(7));

    // When: checking if the capability is granted
    let granted = empty_caps.grants(&required);

    // Then: it is rejected
    assert!(
        !granted,
        "empty capability set should not grant Action(7)"
    );

    // Also verify with a specific but different action grant
    let wrong_caps =
        vb_core::CapabilitySet::from_grants(Box::from([vb_core::Capability::Action(ActionId::new(
            99,
        ))]));
    assert!(
        !wrong_caps.grants(&required),
        "Capability::Action(99) should not grant Action(7)"
    );

    // Verify AnyWorkflow does grant it
    let any_caps =
        vb_core::CapabilitySet::from_grants(Box::from([vb_core::Capability::AnyWorkflow]));
    assert!(
        any_caps.grants(&required),
        "AnyWorkflow should grant any action"
    );

    // Verify Workflow-scoped grant grants any action
    let wf_caps = vb_core::CapabilitySet::from_grants(Box::from([vb_core::Capability::Workflow(
        WorkflowDigest::from_bytes([0u8; 32]),
    )]));
    assert!(
        wf_caps.grants(&required),
        "Workflow-scoped grant should grant Action(7)"
    );

    // Verify the workflow requiring the action can be constructed
    let digest = WorkflowDigest::from_bytes([4u8; 32]);
    let Some(workflow) = do_action_workflow(digest) else {
        fail_assert!("do_action workflow construction failed");
        return;
    };

    // The runtime will suspend the run waiting for the action, which demonstrates
    // that the workflow requires an action that would need capability authorization
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut runtime =
        vb_runtime::runtime::Runtime::new_with_journal(
            shard_count,
            test_config(),
            vb_runtime::journal::NoopRuntimeJournal::shared(),
        );
    let run_id = RunId::new(4);
    match runtime.submit_direct(run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }

    // The run should have been submitted but not completed (suspended on action)
    let snap = runtime.counters_snapshot();
    assert_eq!(
        snap.runs_submitted, 1,
        "run should have been submitted"
    );
    assert_eq!(
        snap.runs_completed, 0,
        "run should be suspended waiting for action, not completed"
    );

    // Trace events should show ActionScheduled
    let trace = runtime.list_events(run_id);
    match trace {
        Ok(events) => {
            let found_scheduled = events
                .iter()
                .any(|e| matches!(e, vb_runtime::trace::TraceEvent::ActionScheduled { run, step, .. } if *run == run_id && *step == StepIdx::new(0)));
            assert!(
                found_scheduled,
                "trace should contain ActionScheduled event for the unauthorized action"
            );
        }
        Err(err) => {
            fail_assert!("list_events failed: {err}");
        }
    }
}

// ===========================================================================
// Test 5: budget validation rejects oversized workflow
// ===========================================================================

#[test]
fn budget_validation_rejects_oversized_workflow() {
    // Given: a BoundednessPolicy with very tight limits
    let tight_policy = vb_core::BoundednessPolicy {
        max_total_steps: 2,
        max_total_slots: 10,
        max_fanout: 1,
        max_nesting_depth: 1,
    };

    // When: creating a 3-node workflow that exceeds the step limit
    let budget = vb_core::WholeWorkflowBudget {
        max_total_steps: 3,
        max_total_slots: 5,
        max_fanout: 0,
        max_nesting_depth: 0,
    };

    // Then: validation rejects the budget
    match tight_policy.validate(&budget) {
        Err(vb_core::BudgetError::TotalStepsExceeded { actual, limit }) => {
            assert_eq!(actual, 3, "actual should be 3");
            assert_eq!(limit, 2, "limit should be 2");
        }
        Err(other) => {
            fail_assert!("expected TotalStepsExceeded, got {other:?}");
        }
        Ok(()) => {
            fail_assert!("3-step workflow should exceed a 2-step limit");
        }
    }

    // Also verify that compile_workflow rejects a workflow exceeding ResourceContract limits
    // by constructing a WorkflowParts with max_steps=1 but 2 nodes
    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let mut tight_contract = ResourceContract::DEFAULT;
    tight_contract.max_steps = 1;
    let oversized_parts = WorkflowParts {
        name: Box::from("oversized"),
        digest: WorkflowDigest::from_bytes([5u8; 32]),
        nodes: Box::from([node0, node1]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: tight_contract,
    };

    // Then: construction should fail because 2 nodes > max_steps(1)
    let result = CompiledWorkflow::try_from_parts(oversized_parts);
    assert!(
        result.is_err(),
        "workflow with 2 nodes exceeding max_steps=1 should be rejected at construction"
    );

    // Verify slot limit rejection too
    let single_node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let mut slot_contract = ResourceContract::DEFAULT;
    slot_contract.max_slots = 0;
    let slot_parts = WorkflowParts {
        name: Box::from("slot_exceeded"),
        digest: WorkflowDigest::from_bytes([6u8; 32]),
        nodes: Box::from([single_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: slot_contract,
    };
    let slot_result = CompiledWorkflow::try_from_parts(slot_parts);
    assert!(
        slot_result.is_err(),
        "workflow with slot_count=1 exceeding max_slots=0 should be rejected"
    );
}

// ===========================================================================
// Test 6: taint propagates through expression eval
// ===========================================================================

#[test]
fn taint_propagates_through_expression_eval() {
    // Given: a workflow with an EvalExpr node that loads slot 0 into an expression
    let digest = WorkflowDigest::from_bytes([7u8; 32]);
    let Some(workflow) = eval_expr_taint_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };

    // Create a run frame, write a Secret-tainted value to slot 0
    let run_id = RunId::new(7);
    let mut frame = match vb_core::engine::new_run_frame(run_id, &workflow) {
        Ok(f) => f,
        Err(err) => {
            fail_assert!("frame creation failed: {err:?}");
            return;
        }
    };

    // Write a Secret-tainted value into slot 0 (the expression input)
    match frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(41), Taint::Secret) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("write_slot_with_taint failed: {err:?}");
            return;
        }
    }

    // Verify slot 0 is Secret
    match frame.read_taint(SlotIdx::new(0)) {
        Ok(Taint::Secret) => {}
        Ok(other) => {
            fail_assert!("slot 0 should be Secret, got {other:?}");
            return;
        }
        Err(err) => {
            fail_assert!("read_taint failed: {err:?}");
            return;
        }
    }

    // When: evaluating the expression that loads slot 0
    let mut store = vb_core::value_store::ValueStore::new();
    let eval_result = vb_core::engine::eval_expr_with_store(
        &workflow,
        &frame,
        &mut store,
        ExprIdx::new(0),
    );

    // Then: the result value is correct and the taint has propagated
    match eval_result {
        Ok((value, taint)) => {
            // 41 (slot 0) + 1 (const 0) = 42
            assert_eq!(
                value,
                SlotValue::I64(42),
                "expression should compute 41 + 1 = 42"
            );
            assert_eq!(
                taint,
                Taint::Secret,
                "Secret taint from slot 0 should propagate through expression evaluation"
            );
        }
        Err(err) => {
            fail_assert!("eval_expr failed: {err:?}");
        }
    }

    // Also run through the engine to verify taint reaches the Finish signal
    // Reinitialize frame with the tainted slot
    let mut frame2 = match vb_core::engine::new_run_frame(run_id, &workflow) {
        Ok(f) => f,
        Err(err) => {
            fail_assert!("frame2 creation failed: {err:?}");
            return;
        }
    };
    match frame2.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(41), Taint::Secret) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("write_slot_with_taint failed: {err:?}");
            return;
        }
    }
    // Set the PC to step 0 and start execution
    let mut budget = vb_core::engine::StepBudget::new(100);
    let mut store2 = vb_core::value_store::ValueStore::new();
    let signal = vb_core::engine::drive_deterministic(&workflow, &mut frame2, &mut budget, &mut store2);

    match signal {
        Ok(vb_core::engine::EngineSignal::Finished(value, taint)) => {
            assert_eq!(value, SlotValue::I64(42));
            assert_eq!(
                taint,
                Taint::Secret,
                "finished signal taint should be Secret from expression propagation"
            );
        }
        Ok(other) => {
            fail_assert!("expected Finished signal, got {other:?}");
        }
        Err(err) => {
            fail_assert!("drive_deterministic failed: {err:?}");
        }
    }
}
