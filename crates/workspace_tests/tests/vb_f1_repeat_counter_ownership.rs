#![forbid(unsafe_code)]
//! F1 repeat counter ownership regression tests for vb-8eocj.3.3.
//!
//! Positive tests: compile canonical repeat sources, execute emitted IR
//! unchanged, and assert body execution counts, final value, and workflow
//! IR immutability.
//!
//! Negative tests: derive mutated IR from the same compiled baseline and
//! attempt to alias a body writer (output slot) to the repeat attempt
//! counter slot. The constructor must reject these with a typed
//! WorkflowError, not accept and execute with counter corruption.
//!
//! These tests do NOT prove source reachability of counter aliasing; they
//! verify constructor-level ownership isolation of the repeat counter slot.

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledNodeKind, CompiledWorkflow, RunId, SlotIdx, SlotValue, StepBudget,
    StepIdx, ValueStore, WorkflowError, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

/// Canonical repeat workflow: sets answer=42, repeats body 3 times, finishes with 42.
const REPEAT_MAX3_SOURCE: &str = "version: velvet-ballistics/v1
name: f1_repeat_counter_ownership_max3
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

/// Repeat workflow with max_attempts=1.
const REPEAT_MAX1_SOURCE: &str = "version: velvet-ballistics/v1
name: f1_repeat_counter_ownership_max1
when:
  manual: {}
steps:
  - id: first
    set:
      output: answer
      value: \"42\"
  - id: bounded_repeat
    repeat:
      max_attempts: 1
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

/// Find the body step and attempt slot of the repeat construct.
fn find_repeat_body_and_attempt_slot(
    workflow: &CompiledWorkflow,
) -> Option<(StepIdx, SlotIdx)> {
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

fn count_body_executions(events: &[EvidenceEvent], body_step: StepIdx) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == body_step)
        })
        .count()
}

fn count_slot_writes(events: &[EvidenceEvent], slot: SlotIdx) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(event, EvidenceEvent::SlotWritten { s, .. } if *s == slot)
        })
        .count()
}

// =========================================================================
// Positive tests: canonical sources compile and execute correctly
// =========================================================================

#[test]
fn repeat_max3_baseline_compiles_and_executes_correctly() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let workflow = compile_workflow(REPEAT_MAX3_SOURCE.as_bytes())
        .map_err(|e| format!("compile failed: {e:?}"))?;

    let (body_step, _attempt_slot) = find_repeat_body_and_attempt_slot(&workflow)
        .ok_or("could not find repeat body/attempt slots")?;

    let (signal, events) = drive_and_observe(&workflow, 64)?;

    match signal {
        RuntimeSignal::Finished(value) => {
            assert!(
                matches!(&value, SlotValue::I64(42)),
                "expected Finished(I64(42)), got {value:?}"
            );
        }
        other => {
            return Err(format!("expected Finished, got {other:?}").into());
        }
    }

    let body_exec_count = count_body_executions(&events, body_step);
    assert_eq!(
        body_exec_count, 3,
        "expected exactly 3 repeat body executions, got {body_exec_count}"
    );

    Ok(())
}

#[test]
fn repeat_max1_baseline_compiles_and_executes_correctly() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let workflow = compile_workflow(REPEAT_MAX1_SOURCE.as_bytes())
        .map_err(|e| format!("compile failed: {e:?}"))?;

    let (body_step, _attempt_slot) = find_repeat_body_and_attempt_slot(&workflow)
        .ok_or("could not find repeat body/attempt slots")?;

    let (signal, events) = drive_and_observe(&workflow, 64)?;

    match signal {
        RuntimeSignal::Finished(value) => {
            assert!(
                matches!(&value, SlotValue::I64(42)),
                "expected Finished(I64(42)), got {value:?}"
            );
        }
        other => {
            return Err(format!("expected Finished, got {other:?}").into());
        }
    }

    let body_exec_count = count_body_executions(&events, body_step);
    assert_eq!(
        body_exec_count, 1,
        "expected exactly 1 repeat body execution, got {body_exec_count}"
    );

    Ok(())
}

#[test]
fn repeat_workflow_ir_is_not_mutated_by_execution() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let workflow = compile_workflow(REPEAT_MAX3_SOURCE.as_bytes())
        .map_err(|e| format!("compile failed: {e:?}"))?;

    let original_parts = workflow.to_parts();

    drive_and_observe(&workflow, 64)?;

    assert_eq!(
        workflow.to_parts(),
        original_parts,
        "compiled workflow parts mutated during execution"
    );

    Ok(())
}

// =========================================================================
// Negative tests: constructor must reject counter slot aliasing in IR
// =========================================================================

/// NOT a source-reachability test. This derives IR from the compiled
/// baseline and mutates the body node's output slot to the repeat attempt
/// counter slot. The constructor must reject this.
#[test]
fn repeat_body_output_aliased_to_attempt_slot_rejected() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let baseline = compile_workflow(REPEAT_MAX3_SOURCE.as_bytes())
        .map_err(|e| format!("baseline compile failed: {e:?}"))?;

    let (body_step, attempt_slot) = find_repeat_body_and_attempt_slot(&baseline)
        .ok_or("could not find repeat body/attempt slots")?;

    let mut parts = baseline.to_parts();

    // Mutate the body node's output to the attempt (counter) slot.
    let body_node = parts.nodes.iter_mut().find(|node| node.id == body_step)
        .ok_or_else(|| format!("body node {body_step:?} not found"))?;

    let original_body_output = body_node.output;
    body_node.output = Some(attempt_slot);

    // Constructor must reject this mutated IR.
    match CompiledWorkflow::try_from_parts(parts) {
        Err(error) => {
            // Post-fix: expect a typed rejection, not just any error.
            // The exact error variant depends on the fix implementation.
            let error_str = format!("{error:?}");
            println!(
                "Constructor correctly rejected mutated repeat IR: {error_str}"
            );
            // Verify it's a typed WorkflowError, not a runtime panic.
            assert!(
                error_str.contains("SlotOutOfBounds")
                    || error_str.contains("repeat")
                    || error_str.contains("attempt")
                    || error_str.contains("counter")
                    || error_str.contains("output"),
                "rejection should be typed and related to slot/counter/output, got: {error_str}"
            );
        }
        Ok(mutated) => {
            // Pre-fix or regression: constructor accepted the aliasing.
            // Run to demonstrate the counter corruption.
            match drive_and_observe(&mutated, 64) {
                Ok((signal, events)) => {
                    let body_exec_count = count_body_executions(&events, body_step);
                    let counter_writes = count_slot_writes(&events, attempt_slot);
                    return Err(format!(
                        "REGRESSION: constructor accepted repeat body output aliased to attempt slot; \
                         runtime signal={signal:?}, body_step_executions={body_exec_count}, \
                         counter_slot_writes={counter_writes}. Expected constructor rejection."
                    ).into());
                }
                Err(runtime_error) => {
                    return Err(format!(
                        "REGRESSION: constructor accepted repeat body output aliased to attempt slot; \
                         runtime failed: {runtime_error}. Expected constructor rejection."
                    ).into());
                }
            }
        }
    }

    Ok(())
}

/// NOT a source-reachability test. This derives IR from the compiled
/// baseline and mutates the body node's error_slot to the repeat attempt
/// counter slot. The constructor must reject this.
#[test]
fn repeat_body_error_slot_aliased_to_attempt_slot_rejected() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let baseline = compile_workflow(REPEAT_MAX3_SOURCE.as_bytes())
        .map_err(|e| format!("baseline compile failed: {e:?}"))?;

    let (body_step, attempt_slot) = find_repeat_body_and_attempt_slot(&baseline)
        .ok_or("could not find repeat body/attempt slots")?;

    let mut parts = baseline.to_parts();

    // Mutate the body node's error_slot to the attempt (counter) slot.
    let body_node = parts.nodes.iter_mut().find(|node| node.id == body_step)
        .ok_or_else(|| format!("body node {body_step:?} not found"))?;

    let original_body_error_slot = body_node.error_slot;
    body_node.error_slot = Some(attempt_slot);

    // Constructor must reject this mutated IR.
    match CompiledWorkflow::try_from_parts(parts) {
        Err(error) => {
            let error_str = format!("{error:?}");
            println!(
                "Constructor correctly rejected mutated repeat IR (error_slot): {error_str}"
            );
        }
        Ok(mutated) => {
            // Pre-fix or regression: constructor accepted the aliasing.
            match drive_and_observe(&mutated, 64) {
                Ok((signal, events)) => {
                    let counter_writes = count_slot_writes(&events, attempt_slot);
                    return Err(format!(
                        "REGRESSION: constructor accepted repeat body error_slot aliased to attempt slot; \
                         runtime signal={signal:?}, counter_slot_writes={counter_writes}. \
                         Expected constructor rejection."
                    ).into());
                }
                Err(runtime_error) => {
                    return Err(format!(
                        "REGRESSION: constructor accepted repeat body error_slot aliased to attempt slot; \
                         runtime failed: {runtime_error}. Expected constructor rejection."
                    ).into());
                }
            }
        }
    }

    Ok(())
}
