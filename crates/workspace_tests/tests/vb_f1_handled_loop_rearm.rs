use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{CompiledNodeKind as K, RunId, RuntimePolicy, SlotValue};
use vb_runtime::{
    journal::{RuntimeJournalEvent as E, VolatileRuntimeJournal},
    runtime::Runtime,
    shard::ShardConfig,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "vb_f1_handled_loop_rearm/ask_operands.rs"]
mod ask_operands;
#[path = "vb_f1_handled_loop_rearm/budget.rs"]
mod budget;
#[path = "vb_f1_handled_loop_rearm/durable.rs"]
mod durable;
#[path = "vb_f1_handled_loop_rearm/owned.rs"]
mod owned;

fn indent(body: &str, spaces: usize) -> String {
    body.lines()
        .map(|line| format!("{}{line}\n", " ".repeat(spaces)))
        .collect()
}

fn repeated(id: &str, body: &str) -> String {
    format!(
        "- id: {id}\n  repeat:\n    max_attempts: 2\n    steps:\n{}",
        indent(body, 6)
    )
}

fn source(body: &str) -> String {
    source_with("", body, "$step.outer")
}

fn source_with(declarations: &str, body: &str, result: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: handled_loop_rearm\nwhen: {{manual: {{}}}}\n{declarations}steps:\n  - id: zero\n    set: {{output: zero, value: '0'}}\n{}  - id: done\n    finish: {{result: '{result}'}}\n",
        indent(body, 2)
    )
}

const RECOVERED_BODY: &str = "- id: failing\n  on_error: {handler: recovered}\n  set: {output: quotient, value: '1 / $zero'}\n- id: recovered\n  set: {output: recovered, value: '7'}\n";

#[test]
fn yaml_engine_accepts_successful_owned_backedge_after_recovery() -> TestResult {
    let workflow =
        vb_compile::compile_workflow(source(&repeated("outer", RECOVERED_BODY)).as_bytes())?;
    let mut frame = vb_core::new_run_frame(RunId::new(1), &workflow)?;
    let signal = vb_runtime::engine::drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut vb_core::StepBudget::new(128),
        &mut vb_core::ValueStore::new(),
        &[],
        vb_runtime::engine::RetryPolicy::NEVER,
        &mut vb_runtime::engine::EvidenceCollector::new(),
        &mut vb_runtime::primitives::collect::CollectStates::new(),
        &vb_core::CapabilitySet::empty(),
        1,
    )?;
    assert_eq!(
        signal,
        vb_runtime::engine::RuntimeSignal::Finished(SlotValue::I64(7))
    );
    Ok(())
}

fn exercise(yaml: &str, quantum: u64, expected_iterations: usize) -> TestResult<Vec<E>> {
    let workflow = vb_compile::compile_workflow(yaml.as_bytes())?;
    let parts = workflow.to_parts();
    let budget =
        vb_core::WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)?;
    let failed = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, K::EvalExpr { .. }))
        .ok_or("EvalExpr")?
        .id;
    let handler = workflow
        .node(failed)
        .and_then(|node| node.routing.error.handler())
        .ok_or("handler")?;
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Relaxed,
            step_budget_per_tick: quantum,
            ..ShardConfig::default()
        },
        journal.clone(),
    );
    runtime.submit_direct(RunId::new(1), workflow)?;
    for _ in 0..256 {
        runtime.tick_all()?;
        assert_eq!(
            runtime.counters_snapshot().runs_failed,
            0,
            "{:?}",
            journal.snapshot()?
        );
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = journal.snapshot()?;
    let executed = u64::try_from(
        events
            .iter()
            .filter(|event| matches!(event, E::StepStarted { .. }))
            .count(),
    )?;
    assert!(
        budget.max_total_steps >= executed,
        "budget {} < actual {executed}",
        budget.max_total_steps
    );
    assert!(budget.max_transitions >= executed);
    assert!(budget.max_trace_events >= executed);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::StepFailed { step, .. } if *step == failed))
            .count(),
        expected_iterations
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::StepSucceeded { step, .. } if *step == handler))
            .count(),
        expected_iterations
    );
    let results = events
        .iter()
        .filter_map(|event| match event {
            E::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(results.len(), 1);
    let slot = results.first().ok_or("result slot")?;
    let encoded = events
        .iter()
        .rev()
        .find_map(|event| match event {
            E::SlotWritten {
                slot: written,
                value,
                ..
            } if written == slot => Some(value),
            _ => None,
        })
        .ok_or("result write")?;
    assert_eq!(
        postcard::from_bytes::<SlotValue>(encoded)?,
        SlotValue::I64(7)
    );
    Ok(events)
}

#[test]
fn yaml_repeat_reexecutes_locally_handled_failure() -> TestResult {
    let yaml = source(&repeated("outer", RECOVERED_BODY));
    for quantum in [1, 64] {
        exercise(&yaml, quantum, 2)?;
    }
    Ok(())
}

#[test]
fn yaml_nested_repeat_reexecutes_locally_handled_failure() -> TestResult {
    let yaml = source(&repeated("outer", &repeated("inner", RECOVERED_BODY)));
    for quantum in [1, 64] {
        exercise(&yaml, quantum, 4)?;
    }
    Ok(())
}

#[test]
fn yaml_repeat_reexecutes_chained_local_recovery() -> TestResult {
    let body = "- id: failing\n  on_error: {handler: also_failing}\n  set: {output: quotient, value: '1 / $zero'}\n- id: also_failing\n  on_error: {handler: recovered}\n  set: {output: second_quotient, value: '2 / $zero'}\n- id: recovered\n  set: {output: recovered, value: '7'}\n";
    // The first declared handler also fails twice; the terminal handler
    // supplies the lexical body result and both failures must be rearmed.
    let yaml = source(&repeated("outer", body));
    let workflow = vb_compile::compile_workflow(yaml.as_bytes())?;
    let mut frame = vb_core::new_run_frame(RunId::new(1), &workflow)?;
    let mut evidence = vb_runtime::engine::EvidenceCollector::new();
    let signal = vb_runtime::engine::drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut vb_core::StepBudget::new(128),
        &mut vb_core::ValueStore::new(),
        &[],
        vb_runtime::engine::RetryPolicy::NEVER,
        &mut evidence,
        &mut vb_runtime::primitives::collect::CollectStates::new(),
        &vb_core::CapabilitySet::empty(),
        1,
    )?;
    assert_eq!(
        signal,
        vb_runtime::engine::RuntimeSignal::Finished(SlotValue::I64(7))
    );
    let events = evidence.drain();
    for step in workflow
        .to_parts()
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, K::EvalExpr { .. }))
        .map(|node| node.id)
    {
        assert_eq!(events.iter().filter(|event| matches!(event, vb_runtime::engine::EvidenceEvent::StepFailed { step: failed, .. } if *failed == step)).count(), 2);
        assert_eq!(frame.step_state(step)?, vb_core::StepState::Failed);
    }
    Ok(())
}

#[test]
fn yaml_repeat_reexecutes_locally_recovered_together_branch() -> TestResult {
    let branch = format!(
        "- id: group\n  together:\n    branches:\n      - label: failing_branch\n        steps:\n{}      - label: sibling\n        steps:\n          - id: other\n            set: {{output: other, value: '11'}}\n- id: tail\n  set: {{output: tail, value: '7'}}\n",
        indent(RECOVERED_BODY, 10)
    );
    let yaml = source(&repeated("outer", &branch));
    for quantum in [1, 64] {
        exercise(&yaml, quantum, 2)?;
    }
    Ok(())
}

#[test]
fn yaml_parent_repeat_rearms_abandoned_together_after_outer_handler() -> TestResult {
    let branch = "- id: group\n  on_error: {handler: recovered}\n  together:\n    branches:\n      - label: failing_branch\n        steps:\n          - id: failing\n            set: {output: quotient, value: '1 / $zero'}\n      - label: bypassed_branch\n        steps:\n          - id: bypassed\n            set: {output: bypassed, value: '11'}\n- id: recovered\n  set: {output: recovered, value: '7'}\n";
    let yaml = source(&repeated("outer", branch));
    let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
    let parts = plan.to_parts();
    let join = parts
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            K::TogetherStart { join, .. } => Some(join),
            _ => None,
        })
        .ok_or("join")?;
    let bypassed = parts.nodes.iter().find(|node| matches!(node.kind, K::SetConst { value } if plan.constant(value) == Some(&vb_core::ConstValue::I64(11)))).ok_or("bypassed leaf")?.id;
    for quantum in [1, 64] {
        let events = exercise(&yaml, quantum, 2)?;
        for bypassed in [join, bypassed] {
            assert_eq!(
                events
                    .iter()
                    .filter(
                        |event| matches!(event, E::StepStarted { step, .. } if *step == bypassed)
                    )
                    .count(),
                0
            );
        }
    }
    Ok(())
}
