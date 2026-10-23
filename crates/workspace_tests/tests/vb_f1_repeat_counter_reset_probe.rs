#![forbid(unsafe_code)]
//! F1 Repeat counter slot reset probe for vb-8eocj.3.2.
//!
//! MALFORMED COMPILER IR negative probe: manually alias the body's output
//! slot to the RepeatAttempt counter state slot to observe whether the
//! body clobbers the counter during execution.
//!
//! Two tests only:
//! 1. Baseline unmutated source control — PASS (finish 42, body count 3)
//! 2. Red malformed-case — FAIL (RED) with actual observations
use vb_compile::compile_workflow;
use vb_core::{
    CompiledNodeKind, CompiledWorkflow, RunId, SlotIdx, SlotValue, StepBudget,
    new_run_frame,
};

use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal,
    drive_deterministic_full,
};

const HEADER: &str = "version: velvet-ballistics/v1\nname: counter_reset_probe\nwhen:\n  manual: {}\nsteps:\n";
const SETUP: &str = "  - id: first\n    set:\n      output: answer\n      value: \"42\"\n";
const REPEAT: &str = "  - id: counter_reset\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: body_value\n          set:\n            output: body_result\n            value: \"12884901888\"\n";
const FINISH: &str = "  - id: done\n    finish:\n      result: answer\n";

fn full_source() -> String {
    format!("{HEADER}{SETUP}{REPEAT}{FINISH}")
}

fn find_repeat_attempt_slot(workflow: &CompiledWorkflow) -> Option<SlotIdx> {
    workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| {
            match &node.kind {
                CompiledNodeKind::RepeatAttempt { attempt_slot, .. } => Some(*attempt_slot),
                _ => None,
            }
        })
}

fn find_body_set_node(workflow: &CompiledWorkflow) -> Option<usize> {
    workflow
        .to_parts()
        .nodes
        .iter()
        .position(|node| {
            matches!(
                &node.kind,
                CompiledNodeKind::SetConst { .. }
                    if node.output == Some(SlotIdx::new(3))
            )
        })
}

fn count_body_executions(evidence: &[EvidenceEvent], workflow: &CompiledWorkflow) -> usize {
    let body_node = find_body_set_node(workflow).expect("body node must exist");
    evidence
        .iter()
        .filter(|event| match event {
            EvidenceEvent::StepSucceeded { step, .. } => step.as_usize() == body_node,
            _ => false,
        })
        .count()
}

#[test]
fn baseline_unmutated_source_control() -> Result<(), String> {
    let source = full_source();
    let workflow = compile_workflow(source.as_bytes())
        .map_err(|e| format!("compile failed: {e:?}"))?;

    let mut frame = new_run_frame(RunId::new(1), &workflow)
        .map_err(|e| format!("frame creation failed: {e:?}"))?;
    let mut budget = StepBudget::new(64);
    let mut store = vb_core::ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();

    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &vb_core::CapabilitySet::empty(),
        1,
    )
    .map_err(|e| format!("drive failed: {e:?}"))?;

    match signal {
        RuntimeSignal::Finished(SlotValue::I64(42)) => {}
        other => return Err(format!("baseline failed: {other:?}")),
    }

    let events = evidence.drain();
    let body_count = count_body_executions(&events, &workflow);
    if body_count != 3 {
        return Err(format!("baseline body count wrong: {body_count}"));
    }

    Ok(())
}

#[test]
fn red_malformed_counter_reset_probe() -> Result<(), String> {
    let source = full_source();
    let workflow = compile_workflow(source.as_bytes())
        .map_err(|e| format!("compile failed: {e:?}"))?;

    let original_parts = workflow.to_parts();

    // Find the attempt slot from RepeatAttempt
    let attempt_slot = find_repeat_attempt_slot(&workflow)
        .ok_or("no RepeatAttempt node found")?;

    // Find the body SetConst node and mutate its output to alias the attempt slot
    let parts = workflow.to_parts();
    let mut nodes: Vec<vb_core::CompiledNode> = parts.nodes.to_vec();
    let body_idx = find_body_set_node(&workflow)
        .ok_or("no body SetConst node found")?;

    nodes[body_idx].output = Some(attempt_slot);

    let mutated_parts = vb_core::WorkflowParts {
        nodes: nodes.into(),
        ..parts
    };

    // Validate the mutated parts
    match CompiledWorkflow::try_from_parts(mutated_parts.clone()) {
        Err(error) => {
            // Constructor rejected the malformed IR
            return Err(format!("rejected by validation: {error:?}"));
        }
        Ok(mutated_workflow) => {
            // Constructor accepted — run the probe
            let mut frame = new_run_frame(RunId::new(1), &mutated_workflow)
                .map_err(|e| format!("frame creation failed: {e:?}"))?;
            let mut budget = StepBudget::new(64);
            let mut store = vb_core::ValueStore::new();
            let mut evidence = EvidenceCollector::new();
            let mut collect_states = CollectStates::new();

            let signal = drive_deterministic_full(
                &mutated_workflow,
                &mut frame,
                &mut budget,
                &mut store,
                &[],
                RetryPolicy::NEVER,
                &mut evidence,
                &mut collect_states,
                &vb_core::CapabilitySet::empty(),
                1,
            );

            let events = evidence.drain();
            let body_count = count_body_executions(&events, &workflow);

            // Read the attempt slot to observe counter values
            let attempt_value = frame.read_slot(attempt_slot)
                .map_err(|e| format!("attempt slot read failed: {e:?}"))?;

            match signal {
                Ok(RuntimeSignal::Finished(SlotValue::I64(42))) => {
                    return Err(format!(
                        "ACCEPTED pre-fix: finished 42 despite counter aliasing; \
                         body count {body_count}; attempt slot {attempt_value:?}"
                    ));
                }
                Ok(RuntimeSignal::StepBudgetExhausted) => {
                    return Err(format!(
                        "ACCEPTED pre-fix: step budget exhausted with counter reset; \
                         body count {body_count} (expected > 3); attempt slot {attempt_value:?}"
                    ));
                }
                Ok(other) => {
                    return Err(format!(
                        "ACCEPTED pre-fix: unexpected signal {other:?}; \
                         body count {body_count}; attempt slot {attempt_value:?}"
                    ));
                }
                Err(e) => {
                    return Err(format!(
                        "runtime error during probe: {e:?}; body count {body_count}; \
                         attempt slot {attempt_value:?}"
                    ));
                }
            }
        }
    }
}
