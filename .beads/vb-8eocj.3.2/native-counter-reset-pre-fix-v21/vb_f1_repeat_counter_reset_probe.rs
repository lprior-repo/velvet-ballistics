#![forbid(unsafe_code)]
//! F1 repeat counter reset negative probe for vb-8eocj.3.2.
//!
//! Two observational tests that expose the pre-fix repeat counter IR shape:
//! 1. Baseline repeat workflow compiles and executes correctly (finishes with
//!    value 42, exactly 3 body executions in 64 budget steps).
//! 2. Malformed-IR negative probe: compile the same baseline, mutate the
//!    body node's output slot to the counter state slot, reconstruct the
//!    workflow, and observe whether the constructor rejects it or the
//!    runtime accepts and executes it. Pre-fix state should reject; post-fix
//!    state should accept and emit counter writes as evidence.

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledNodeKind, CompiledWorkflow, RunId, SlotIdx, SlotValue, StepBudget,
    StepIdx, ValueStore, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

/// Repeat workflow from canonical source: sets 42, repeats body 3 times, finishes with 42.
const REPEAT_SOURCE: &str = "version: velvet-ballistics/v1
name: f1_repeat_counter_probe
when:
  manual: {}
steps:
  - id: first
    set:
      output: answer
      value: \"42\"
  - id: bounded_repeat
    repeat:
      max_attempts: 3
      steps:
        - id: body_value
          set:
            output: body_result
            value: \"12884901888\"
  - id: done
    finish:
      result: answer
";

fn drive_and_observe(
    workflow: &CompiledWorkflow,
    step_budget: u64,
) -> Result<
    (RuntimeSignal, Vec<EvidenceEvent>),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let mut frame = new_run_frame(RunId::new(1), workflow)?;
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let mut budget = StepBudget::new(step_budget);

    let signal = drive_deterministic_full(
        workflow,
        &mut frame,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
        1,
    )?;

    let events = evidence.drain();
    Ok((signal, events))
}

fn body_and_attempt_slots(workflow: &CompiledWorkflow) -> Option<(StepIdx, SlotIdx)> {
    let parts = workflow.to_parts();
    for node in &parts.nodes {
        if let CompiledNodeKind::RepeatStart { body, .. } = &node.kind {
            for check_node in &parts.nodes {
                if let CompiledNodeKind::RepeatCheck {
                    attempt_slot, ..
                } = &check_node.kind
                {
                    if check_node.next == Some(*body) {
                        return Some((*body, *attempt_slot));
                    }
                }
            }
        }
    }
    None
}

#[test]
fn baseline_repeat_workflow_executes_correctly() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let workflow = compile_workflow(REPEAT_SOURCE.as_bytes())
        .map_err(|e| format!("compile failed: {e:?}"))?;

    let (body_step, _attempt_slot) = body_and_attempt_slots(&workflow)
        .ok_or("could not find repeat body/attempt slots")?;

    let (signal, events) = drive_and_observe(&workflow, 64)?;

    match signal {
        RuntimeSignal::Finished(value) => {
            assert!(matches!(&value, SlotValue::I64(42)), "expected finished with 42, got {value:?}");
        }
        other => {
            return Err(format!("expected Finished, got {other:?}").into());
        }
    }

    let body_exec_count = events.iter().filter(|event| {
        matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == body_step)
    }).count();

    assert_eq!(
        body_exec_count, 3,
        "expected exactly 3 repeat body executions, got {body_exec_count}"
    );

    let original_parts = workflow.to_parts();
    drive_and_observe(&workflow, 64)?;
    assert_eq!(
        workflow.to_parts(),
        original_parts,
        "compiled workflow parts mutated during execution"
    );

    Ok(())
}

#[test]
fn repeat_check_output_slot_negative_probe() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let baseline = compile_workflow(REPEAT_SOURCE.as_bytes())
        .map_err(|e| format!("baseline compile failed: {e:?}"))?;

    let (body_step, attempt_slot) = body_and_attempt_slots(&baseline)
        .ok_or("could not find repeat body/attempt slots")?;

    let mut parts = baseline.to_parts();

    let body_node = parts.nodes.iter_mut().find(|node| node.id == body_step)
        .ok_or_else(|| format!("body node {body_step:?} not found"))?;

    let original_body_output = body_node.output;
    body_node.output = Some(attempt_slot);

    let body_kind = match &body_node.kind {
        CompiledNodeKind::SetConst { value } => {
            let const_value = baseline.constant(*value)
                .ok_or("body constant missing")?;
            format!("SetConst with value {const_value:?}")
        }
        other => format!("{other:?}"),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(error) => {
            println!(
                "Constructor rejected mutated repeat IR: {error:?} (body node {body_step:?} had output {original_body_output:?} mutated to {attempt_slot:?})"
            );
            return Err(format!(
                "REJECTED: constructor did not accept mutated repeat IR with body.output={attempt_slot:?} for {body_kind}: {error:?}"
            ).into());
        }
        Ok(mutated) => {
            println!(
                "Constructor accepted mutated repeat IR (body.output = {attempt_slot:?})"
            );

            match drive_and_observe(&mutated, 64) {
                Ok((signal, events)) => {
                    let body_exec_count = events.iter().filter(|event| {
                        matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == body_step)
                    }).count();

                    let counter_writes: Vec<SlotValue> = events.iter().filter_map(|event| {
                        match event {
                            EvidenceEvent::SlotWritten { slot, value, .. }
                                if *slot == attempt_slot => Some(value.clone()),
                            _ => None,
                        }
                    }).collect();

                    return Err(format!(
                        "ACCEPTED (RED if post-fix): constructor accepted, runtime signal={signal:?}, \
                         body_step_executions={body_exec_count}, counter_slot_writes={}",
                        counter_writes.len()
                    ).into());
                }
                Err(runtime_error) => {
                    return Err(format!(
                        "ACCEPTED but runtime failed: constructor accepted mutated IR, runtime error={runtime_error}"
                    ).into());
                }
            }
        }
    }
}
