#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledNodeKind, CompiledWorkflow, RunId, SlotIdx, SlotValue, StepBudget,
    StepIdx, ValueStore, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const REPEAT_SOURCE: &str = r#"version: velvet-ballistics/v1
name: f1_repeat_counter_ownership
when:
  manual: {}
steps:
  - id: first
    set:
      output: answer
      value: "42"
  - id: bounded_repeat
    repeat:
      max_attempts: 3
      steps:
        - id: body_value
          set:
            output: body_result
            value: "12884901888"
  - id: done
    finish:
      result: answer
"#;

fn compile_max1() -> Result<CompiledWorkflow, Box<dyn std::error::Error + Send + Sync>> {
    let src = REPEAT_SOURCE.replace("max_attempts: 3", "max_attempts: 1");
    compile_workflow(src.as_bytes()).map_err(|e| format!("compile failed: {e:?}").into())
}

fn compile_max3() -> Result<CompiledWorkflow, Box<dyn std::error::Error + Send + Sync>> {
    compile_workflow(REPEAT_SOURCE.as_bytes()).map_err(|e| format!("compile failed: {e:?}").into())
}

fn drive_and_observe(
    workflow: &CompiledWorkflow,
    step_budget: u64,
) -> Result<(RuntimeSignal, Vec<EvidenceEvent>), Box<dyn std::error::Error + Send + Sync>> {
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

fn find_repeat_slots(workflow: &CompiledWorkflow) -> Option<(StepIdx, StepIdx, SlotIdx, SlotIdx)> {
    let parts = workflow.to_parts();
    for node in &parts.nodes {
        if let CompiledNodeKind::RepeatStart { body, .. } = &node.kind {
            let owner = node.id;
            for check_node in &parts.nodes {
                if let CompiledNodeKind::RepeatCheck { attempt_slot, .. } = &check_node.kind {
                    if check_node.next == Some(*body) {
                        let body_node = parts.nodes.iter().find(|n| n.id == *body)?;
                        let body_output = body_node.output?;
                        return Some((owner, *body, *attempt_slot, body_output));
                    }
                }
            }
        }
    }
    None
}

fn find_check_and_done(workflow: &CompiledWorkflow) -> Option<(StepIdx, StepIdx)> {
    let parts = workflow.to_parts();
    let check = parts
        .nodes
        .iter()
        .find(|n| matches!(&n.kind, CompiledNodeKind::RepeatCheck { .. }))?;
    let done = parts
        .nodes
        .iter()
        .find(|n| matches!(&n.kind, CompiledNodeKind::Finish { .. }))?;
    Some((check.id, done.id))
}

fn expect_repeat_counter_foreign_write(
    result: Result<CompiledWorkflow, vb_core::WorkflowError>,
    owner: StepIdx,
    writer: StepIdx,
    slot: SlotIdx,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match result {
        Ok(_) => Err("expected RepeatCounterForeignWrite error, got Ok".into()),
        Err(vb_core::WorkflowError::RepeatCounterForeignWrite {
            owner: e_owner,
            writer: e_writer,
            slot: e_slot,
        }) => {
            if e_owner != owner || e_writer != writer || e_slot != slot {
                return Err(format!(
                    "expected RepeatCounterForeignWrite {{ owner: {owner:?}, writer: {writer:?}, slot: {slot:?} }}, got {{ owner: {e_owner:?}, writer: {e_writer:?}, slot: {e_slot:?} }}"
                )
                .into());
            }
            Ok(())
        }
        Err(other) => Err(format!("expected RepeatCounterForeignWrite, got {other:?}").into()),
    }
}

#[test]
fn repeat_max1_executes_once() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let workflow = compile_max1()?;
    let (_owner, body, counter_slot, body_output) =
        find_repeat_slots(&workflow).ok_or("could not find repeat slots")?;

    let original_parts = workflow.to_parts();

    let (signal, events) = drive_and_observe(&workflow, 64)?;

    match signal {
        RuntimeSignal::Finished(value) => {
            assert!(
                matches!(&value, SlotValue::I64(42)),
                "expected finished with 42, got {value:?}"
            );
        }
        other => return Err(format!("expected Finished, got {other:?}").into()),
    }

    let body_exec_count = events
        .iter()
        .filter(|event| matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == body))
        .count();
    assert_eq!(body_exec_count, 1, "expected 1 body execution for max1");

    let counter_writes: Vec<SlotValue> = events
        .iter()
        .filter_map(|event| match event {
            EvidenceEvent::SlotWritten { slot, value, .. } if *slot == counter_slot => {
                Some(value.clone())
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        counter_writes,
        vec![SlotValue::I64(4294967296), SlotValue::I64(4294967297)],
        "unexpected counter writes for max1"
    );

    assert_ne!(
        body_output, counter_slot,
        "body output must be distinct from counter slot"
    );

    assert_eq!(
        workflow.to_parts(),
        original_parts,
        "compiled workflow parts mutated during execution"
    );

    Ok(())
}

#[test]
fn repeat_max3_executes_three_times() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let workflow = compile_max3()?;
    let (_owner, body, counter_slot, body_output) =
        find_repeat_slots(&workflow).ok_or("could not find repeat slots")?;

    let original_parts = workflow.to_parts();

    let (signal, events) = drive_and_observe(&workflow, 64)?;

    match signal {
        RuntimeSignal::Finished(value) => {
            assert!(
                matches!(&value, SlotValue::I64(42)),
                "expected finished with 42, got {value:?}"
            );
        }
        other => return Err(format!("expected Finished, got {other:?}").into()),
    }

    let body_exec_count = events
        .iter()
        .filter(|event| matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == body))
        .count();
    assert_eq!(body_exec_count, 3, "expected 3 body executions for max3");

    let counter_writes: Vec<SlotValue> = events
        .iter()
        .filter_map(|event| match event {
            EvidenceEvent::SlotWritten { slot, value, .. } if *slot == counter_slot => {
                Some(value.clone())
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        counter_writes,
        vec![
            SlotValue::I64(12884901888),
            SlotValue::I64(12884901889),
            SlotValue::I64(12884901890),
            SlotValue::I64(12884901891)
        ],
        "unexpected counter writes for max3"
    );

    assert_ne!(
        body_output, counter_slot,
        "body output must be distinct from counter slot"
    );

    assert_eq!(
        workflow.to_parts(),
        original_parts,
        "compiled workflow parts mutated during execution"
    );

    Ok(())
}

/// MALFORMED-IR constructor test: body output slot aliased to repeat counter.
/// Not reachable from canonical source; validates try_from_parts invariant.
#[test]
fn repeat_body_output_mutated_to_counter() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let baseline = compile_max3()?;
    let (owner, body, counter_slot, _body_output) =
        find_repeat_slots(&baseline).ok_or("could not find repeat slots")?;

    let mut parts = baseline.to_parts();
    let body_node = parts
        .nodes
        .iter_mut()
        .find(|n| n.id == body)
        .ok_or("body node not found")?;
    body_node.output = Some(counter_slot);

    let result = CompiledWorkflow::try_from_parts(parts);
    expect_repeat_counter_foreign_write(result, owner, body, counter_slot)
}

/// MALFORMED-IR constructor test: body error slot aliased to repeat counter.
/// Not reachable from canonical source; validates try_from_parts invariant.
#[test]
fn repeat_body_error_slot_mutated_to_counter()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let baseline = compile_max3()?;
    let (owner, body, counter_slot, _body_output) =
        find_repeat_slots(&baseline).ok_or("could not find repeat slots")?;

    let mut parts = baseline.to_parts();
    let body_node = parts
        .nodes
        .iter_mut()
        .find(|n| n.id == body)
        .ok_or("body node not found")?;
    body_node.error_slot = Some(counter_slot);

    let result = CompiledWorkflow::try_from_parts(parts);
    expect_repeat_counter_foreign_write(result, owner, body, counter_slot)
}

/// MALFORMED-IR constructor test: owner error slot aliased to repeat counter.
/// Not reachable from canonical source; validates try_from_parts invariant.
#[test]
fn repeat_owner_error_slot_mutated_to_counter()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let baseline = compile_max3()?;
    let (owner, _body, counter_slot, _body_output) =
        find_repeat_slots(&baseline).ok_or("could not find repeat slots")?;

    let mut parts = baseline.to_parts();
    let owner_node = parts
        .nodes
        .iter_mut()
        .find(|n| n.id == owner)
        .ok_or("owner node not found")?;
    owner_node.error_slot = Some(counter_slot);

    let result = CompiledWorkflow::try_from_parts(parts);
    expect_repeat_counter_foreign_write(result, owner, owner, counter_slot)
}

/// MALFORMED-IR constructor test: check error slot aliased to repeat counter.
/// Not reachable from canonical source; validates try_from_parts invariant.
#[test]
fn repeat_check_error_slot_mutated_to_counter()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let baseline = compile_max3()?;
    let (owner, _body, counter_slot, _body_output) =
        find_repeat_slots(&baseline).ok_or("could not find repeat slots")?;

    let mut parts = baseline.to_parts();
    let check_node = parts
        .nodes
        .iter_mut()
        .find(|n| matches!(&n.kind, CompiledNodeKind::RepeatCheck { .. }))
        .ok_or("check node not found")?;
    let writer = check_node.id;
    check_node.error_slot = Some(counter_slot);

    let result = CompiledWorkflow::try_from_parts(parts);
    expect_repeat_counter_foreign_write(result, owner, writer, counter_slot)
}

/// MALFORMED-IR constructor test: check next edge severed.
/// Not reachable from canonical source; validates try_from_parts invariant.
#[test]
fn repeat_check_next_none() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let baseline = compile_max3()?;
    let (owner, _body, counter_slot, _body_output) =
        find_repeat_slots(&baseline).ok_or("could not find repeat slots")?;

    let mut parts = baseline.to_parts();
    let check_node = parts
        .nodes
        .iter_mut()
        .find(|n| matches!(&n.kind, CompiledNodeKind::RepeatCheck { .. }))
        .ok_or("check node not found")?;
    let writer = check_node.id;
    check_node.next = None;

    let result = CompiledWorkflow::try_from_parts(parts);
    expect_repeat_counter_foreign_write(result, owner, writer, counter_slot)
}

/// MALFORMED-IR constructor test: check next edge bypasses body to done.
/// Not reachable from canonical source; validates try_from_parts invariant.
#[test]
fn repeat_check_next_done() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let baseline = compile_max3()?;
    let (owner, _body, counter_slot, _body_output) =
        find_repeat_slots(&baseline).ok_or("could not find repeat slots")?;

    let parts = baseline.to_parts();
    let done_step = parts
        .nodes
        .iter()
        .find(|n| matches!(&n.kind, CompiledNodeKind::Finish { .. }))
        .map(|n| n.id)
        .ok_or("done node not found")?;

    let mut parts = baseline.to_parts();
    let check_node = parts
        .nodes
        .iter_mut()
        .find(|n| matches!(&n.kind, CompiledNodeKind::RepeatCheck { .. }))
        .ok_or("check node not found")?;
    let writer = check_node.id;
    check_node.next = Some(done_step);

    let result = CompiledWorkflow::try_from_parts(parts);
    expect_repeat_counter_foreign_write(result, owner, writer, counter_slot)
}

/// MALFORMED-IR constructor test: body node kind replaced with RetryCheck.
/// Uses valid forward edges (check, done) from baseline to isolate counter invariant.
/// Not reachable from canonical source; validates try_from_parts invariant.
#[test]
fn repeat_body_kind_retry_check() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let baseline = compile_max3()?;
    let (owner, body, counter_slot, _body_output) =
        find_repeat_slots(&baseline).ok_or("could not find repeat slots")?;
    let (check_id, done_id) =
        find_check_and_done(&baseline).ok_or("could not find check/done nodes")?;

    let mut parts = baseline.to_parts();
    let body_node = parts
        .nodes
        .iter_mut()
        .find(|n| n.id == body)
        .ok_or("body node not found")?;
    body_node.kind = CompiledNodeKind::RetryCheck {
        policy_slot: counter_slot,
        body: check_id,
        exhausted: done_id,
    };

    let result = CompiledWorkflow::try_from_parts(parts);
    expect_repeat_counter_foreign_write(result, owner, body, counter_slot)
}

/// MALFORMED-IR constructor test: body node kind replaced with ErrorHandler.
/// Uses valid forward edges (check, done) from baseline to isolate counter invariant.
/// Outer error_slot stays None; KIND.error_slot targets counter.
/// Not reachable from canonical source; validates try_from_parts invariant.
#[test]
fn repeat_body_kind_error_handler() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let baseline = compile_max3()?;
    let (owner, body, counter_slot, _body_output) =
        find_repeat_slots(&baseline).ok_or("could not find repeat slots")?;
    let (check_id, done_id) =
        find_check_and_done(&baseline).ok_or("could not find check/done nodes")?;

    let mut parts = baseline.to_parts();
    let body_node = parts
        .nodes
        .iter_mut()
        .find(|n| n.id == body)
        .ok_or("body node not found")?;
    body_node.kind = CompiledNodeKind::ErrorHandler {
        body: check_id,
        handler: done_id,
        error_slot: Some(counter_slot),
    };

    let result = CompiledWorkflow::try_from_parts(parts);
    expect_repeat_counter_foreign_write(result, owner, body, counter_slot)
}

#[test]
fn repeat_counter_ask_resume_foreign_write() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let src = REPEAT_SOURCE.replace(
        "  - id: bounded_repeat",
        r#"  - id: query
    ask:
      prompt: "0"
  - id: bounded_repeat"#,
    );

    let workflow = compile_workflow(src.as_bytes()).map_err(|e| format!("{e:?}"))?;

    let mut parts = workflow.to_parts();

    let (owner, _body, counter_slot, _body_output) =
        find_repeat_slots(&workflow).ok_or("missing repeat slots")?;

    let ask_resume_id = parts
        .nodes
        .iter()
        .find(|n| matches!(&n.kind, CompiledNodeKind::AskResume { .. }))
        .map(|n| n.id)
        .ok_or("AskResume node not found")?;

    let ask_node = parts
        .nodes
        .iter()
        .find(|n| n.id == ask_resume_id)
        .ok_or("AskResume node not found")?;
    assert_ne!(
        ask_node.output,
        Some(counter_slot),
        "original output should not be counter slot"
    );

    let original_output = ask_node.output;

    for node in &mut parts.nodes {
        if node.id == ask_resume_id {
            if let CompiledNodeKind::AskResume { answer } = &mut node.kind {
                *answer = counter_slot;
            }
        }
    }

    let ask_node_after = parts
        .nodes
        .iter()
        .find(|n| n.id == ask_resume_id)
        .ok_or("AskResume node not found")?;
    assert_eq!(
        ask_node_after.output, original_output,
        "output should stay the same after mutation"
    );

    expect_repeat_counter_foreign_write(
        CompiledWorkflow::try_from_parts(parts),
        owner,
        ask_resume_id,
        counter_slot,
    )
}
