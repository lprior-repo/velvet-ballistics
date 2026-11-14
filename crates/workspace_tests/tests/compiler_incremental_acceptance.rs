#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::budget::WholeWorkflowBudget;
use vb_core::engine::StepBudget;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledWorkflow, WorkflowIrProfile};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const SOURCE: &str = "version: velvet-ballistics/v1\nname: incremental_acceptance\nwhen: {manual: {}}\ninputs:\n  items: {is: list, of: any, max: 3}\nsteps:\n  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.items'\n      variable: entry\n      at_once: 2\n      steps:\n        - id: leaf\n          set: {output: picked, value: '$entry'}\n  - id: done\n    finish: {result: '$outer'}\n";

struct DriveResult {
    signal: RuntimeSignal,
    events: Vec<EvidenceEvent>,
    result_taint: Taint,
    values: Vec<SlotValue>,
}

fn compiled_source() -> Result<CompiledWorkflow, String> {
    compile_workflow(SOURCE.as_bytes()).map_err(|errors| format!("source compile: {errors:?}"))
}

#[test]
fn public_source_at_once_two_emits_checked_concurrent_ir() -> Result<(), String> {
    let workflow = compiled_source()?;
    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    let starts: Vec<u16> = workflow
        .to_parts()
        .nodes
        .into_iter()
        .filter_map(|node| match node.kind {
            vb_core::workflow::CompiledNodeKind::ForEachConcurrentStart { at_once, .. } => {
                Some(at_once.get())
            }
            _ => None,
        })
        .collect();
    assert_eq!(starts, vec![2]);
    Ok(())
}

fn run_with_repeated_budget(
    workflow: &CompiledWorkflow,
    items: &[i64],
    quantum: u64,
) -> Result<DriveResult, String> {
    let mut frame = RunFrame::new(
        RunId::new(1),
        StepIdx::new(0),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|error| format!("frame: {error}"))?;
    let mut store = ValueStore::new();
    let input = workflow
        .input_slots()
        .first()
        .ok_or("compiled input missing")?
        .slot;
    let input_values: Vec<SlotValue> = items.iter().copied().map(SlotValue::I64).collect();
    let input_list = store
        .insert_list(input_values.into_boxed_slice())
        .map_err(|error| format!("input list: {error}"))?;
    frame
        .write_slot_with_taint(input, SlotValue::List(input_list), Taint::Secret)
        .map_err(|error| format!("input slot: {error}"))?;

    let parts = workflow.to_parts();
    let upper_bound = WholeWorkflowBudget::compute(
        &parts.nodes,
        workflow.entry(),
        &workflow.resource_contract(),
    )
    .map_err(|error| format!("workflow bound: {error:?}"))?
    .max_total_steps;
    let mut evidence = EvidenceCollector::new();
    let mut events = Vec::new();
    let mut collect_states = CollectStates::new();
    let capabilities = vb_core::capability::CapabilitySet::empty();
    let mut final_signal = None;

    for _ in 0..upper_bound {
        let mut budget = StepBudget::new(quantum);
        let signal = drive_deterministic_full(
            workflow,
            &mut frame,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &capabilities,
            1,
        )
        .map_err(|error| format!("drive: {error:?}"))?;
        events.extend(evidence.drain());
        if matches!(signal, RuntimeSignal::Finished(_)) {
            final_signal = Some(signal);
            break;
        }
        if signal != RuntimeSignal::StepBudgetExhausted {
            return Err(format!("unexpected nonterminal signal: {signal:?}"));
        }
    }
    let signal = final_signal.ok_or_else(|| {
        format!("budgeted execution did not finish within workflow bound {upper_bound}")
    })?;
    let finish = workflow
        .to_parts()
        .nodes
        .into_iter()
        .find(|node| {
            matches!(
                node.kind,
                vb_core::workflow::CompiledNodeKind::Finish { .. }
            )
        })
        .ok_or("finish node missing")?;
    let vb_core::workflow::CompiledNodeKind::Finish { result } = finish.kind else {
        return Err("finish node kind changed".into());
    };
    let result_taint = frame
        .read_taint(result)
        .map_err(|error| format!("finish taint: {error}"))?;
    let RuntimeSignal::Finished(SlotValue::List(result_list)) = signal else {
        return Err(format!("finished with unexpected value: {signal:?}"));
    };
    let values = store
        .list(result_list)
        .map_err(|error| format!("result list: {error}"))?
        .to_vec();
    Ok(DriveResult {
        signal: RuntimeSignal::Finished(SlotValue::List(result_list)),
        events,
        result_taint,
        values,
    })
}

fn lane_body_output(workflow: &CompiledWorkflow) -> Result<(StepIdx, vb_core::SlotIdx), String> {
    let parts = workflow.to_parts();
    let mut body = None;
    for node in &parts.nodes {
        if let vb_core::workflow::CompiledNodeKind::ForEachConcurrentStart {
            body: candidate, ..
        }
        | vb_core::workflow::CompiledNodeKind::ForEachStart {
            body: candidate, ..
        } = &node.kind
        {
            if body.replace(*candidate).is_some() {
                return Err("source must compile to one concurrent ForEach body".into());
            }
        }
    }
    let body = body.ok_or_else(|| "compiled source lacks concurrent ForEach".to_owned())?;
    let output = workflow
        .node(body)
        .and_then(|node| node.output.slot())
        .ok_or_else(|| format!("lane body {body:?} has no output slot"))?;
    Ok((body, output))
}

fn lane_evidence(events: &[EvidenceEvent], body: StepIdx) -> Result<Vec<EvidenceEvent>, String> {
    let mut active = false;
    let mut lane_events = Vec::new();
    for event in events {
        match event {
            EvidenceEvent::StepStarted { step } if *step == body => {
                if active {
                    return Err(format!("overlapping lane body activation at {body:?}"));
                }
                active = true;
                lane_events.push(*event);
            }
            EvidenceEvent::StepSucceeded { step, .. } if *step == body => {
                if !active {
                    return Err(format!("lane body {body:?} succeeded without a start"));
                }
                lane_events.push(*event);
                active = false;
            }
            EvidenceEvent::StepFailed { step } if *step == body => {
                return Err(format!("lane body {body:?} failed"));
            }
            EvidenceEvent::SlotWritten { .. } if active => lane_events.push(*event),
            EvidenceEvent::StepStarted { .. }
            | EvidenceEvent::StepSucceeded { .. }
            | EvidenceEvent::StepFailed { .. }
                if active =>
            {
                return Err(format!("another step interleaved lane body {body:?}"));
            }
            _ => {}
        }
    }
    if active {
        return Err(format!("lane body {body:?} has no matching success"));
    }
    Ok(lane_events)
}

fn assert_lane_evidence(
    workflow: &CompiledWorkflow,
    events: &[EvidenceEvent],
    items: &[i64],
    taint: Taint,
) -> Result<(), String> {
    let (body, output) = lane_body_output(workflow)?;
    let actual = lane_evidence(events, body)?;
    let mut expected = Vec::new();
    for item in items {
        expected.extend([
            EvidenceEvent::StepStarted { step: body },
            EvidenceEvent::SlotWritten {
                slot: output,
                value: SlotValue::I64(*item),
                taint,
                extra: None,
            },
            EvidenceEvent::StepSucceeded {
                step: body,
                output: Some(output),
            },
        ]);
    }
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, EvidenceEvent::StepStarted { step } if *step == body))
            .count(),
        items.len(),
        "lane StepStarted count"
    );
    assert_eq!(
        events
            .iter()
            .filter(
                |event| matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == body)
            )
            .count(),
        items.len(),
        "lane StepSucceeded count"
    );
    assert_eq!(actual, expected, "exact concurrent lane transitions");
    Ok(())
}

#[test]
fn at_once_one_and_two_produce_same_ordered_values_and_taint() -> Result<(), String> {
    let concurrent = compiled_source()?;
    let serial_source = SOURCE.replace("at_once: 2", "at_once: 1");
    let serial = compile_workflow(serial_source.as_bytes())
        .map_err(|errors| format!("serial source compile: {errors:?}"))?;
    let concurrent_result = run_with_repeated_budget(&concurrent, &[11, 22, 33], 10_000)?;
    let serial_result = run_with_repeated_budget(&serial, &[11, 22, 33], 10_000)?;
    let expected: Vec<SlotValue> = [11, 22, 33].into_iter().map(SlotValue::I64).collect();
    assert_eq!(concurrent_result.values, expected);
    assert_eq!(serial_result.values, expected);
    assert_eq!(concurrent_result.result_taint, Taint::Secret);
    assert_eq!(serial_result.result_taint, Taint::Secret);
    assert_lane_evidence(
        &concurrent,
        &concurrent_result.events,
        &[11, 22, 33],
        Taint::Secret,
    )?;
    assert_lane_evidence(&serial, &serial_result.events, &[11, 22, 33], Taint::Secret)?;
    Ok(())
}

#[test]
fn public_source_at_once_two_resumes_each_lane_under_small_budgets() -> Result<(), String> {
    let workflow = compiled_source()?;
    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert!(
        workflow.to_parts().nodes.iter().any(|node| matches!(
            node.kind,
            vb_core::workflow::CompiledNodeKind::ForEachConcurrentStart { .. }
        )),
        "at_once: 2 must emit the checked concurrent ForEach IR node"
    );
    let expected: Vec<SlotValue> = [11, 22, 33].into_iter().map(SlotValue::I64).collect();
    let mut reference = None;
    for quantum in [1, 2, 4] {
        let result = run_with_repeated_budget(&workflow, &[11, 22, 33], quantum)?;
        assert_eq!(result.values, expected, "quantum {quantum}");
        assert_eq!(result.result_taint, Taint::Secret, "quantum {quantum}");
        assert_lane_evidence(&workflow, &result.events, &[11, 22, 33], Taint::Secret)?;
        if let Some(expected_signal) = &reference {
            assert_eq!(&result.signal, expected_signal, "quantum {quantum}");
        } else {
            reference = Some(result.signal);
        }
    }
    Ok(())
}

#[path = "compiler_incremental_acceptance/profiles.rs"]
mod profiles;
#[path = "compiler_incremental_acceptance/recovery.rs"]
mod recovery;
#[path = "compiler_incremental_acceptance/replay.rs"]
mod replay;
