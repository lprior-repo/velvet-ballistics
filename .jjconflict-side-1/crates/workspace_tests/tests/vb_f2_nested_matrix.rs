//! Actual YAML execution for every pair of bounded compound families.
use vb_core::{CapabilitySet, RunId, SlotValue, StepBudget, Taint, ValueStore, new_run_frame};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const FAMILIES: [&str; 6] = [
    "repeat", "for_each", "collect", "reduce", "together", "choose",
];

fn indent(text: &str, count: usize) -> String {
    text.lines()
        .map(|line| format!("{}{line}\n", " ".repeat(count)))
        .collect()
}

fn compound(family: &str, name: &str, body: &str) -> Result<String, String> {
    let primitive = match family {
        "repeat" => format!("repeat:\n  max_attempts: 2\n  steps:\n{}", indent(body, 4)),
        "for_each" => format!(
            "for_each:\n  variable: item_{name}\n  input: $input.items\n  at_once: 1\n  steps:\n{}",
            indent(body, 4)
        ),
        "collect" => format!(
            "collect:\n  variable: page_{name}\n  source: $input.items\n  pages: 2\n  items: 1\n  steps:\n{}",
            indent(body, 4)
        ),
        "reduce" => format!(
            "reduce:\n  variable: accumulator_{name}\n  input: $input.items\n  initial: '0'\n  steps:\n{}",
            indent(body, 4)
        ),
        "together" => format!(
            "together:\n  branches:\n    - label: first_{name}\n      steps:\n{}    - label: second_{name}\n      steps:\n        - id: other_{name}\n          set: {{output: other_{name}, value: '17'}}\n",
            indent(body, 8)
        ),
        "choose" => format!(
            "choose:\n  branches:\n    - when: 'true'\n      steps:\n{}",
            indent(body, 8)
        ),
        _ => return Err(format!("unknown family {family}")),
    };
    Ok(format!("- id: {name}\n{}", indent(&primitive, 2)))
}

#[test]
fn all_nested_pairs_preserve_source_values_taints_and_ir() -> Result<(), Box<dyn std::error::Error>>
{
    let mut exercised = 0usize;
    for outer in FAMILIES {
        for inner in FAMILIES {
            let leaf = "- id: leaf\n  set: {output: leaf, value: '9'}\n";
            let mut body = compound(inner, "inner", leaf)?;
            body.push_str("- id: tail\n  set: {output: tail, value: '$vars.sentinel'}\n");
            let outer_body = compound(outer, "outer", &body)?;
            let source = format!(
                "version: velvet-ballistics/v1\nname: nested_matrix\nwhen: {{manual: {{}}}}\ninputs: {{items: {{is: list, of: number, max: 2}}}}\nvars: {{sentinel: 42, marker: source_payload}}\nsteps:\n{}  - id: done\n    finish: {{result: '$vars.sentinel'}}\n",
                indent(&outer_body, 2)
            );
            let workflow = vb_compile::compile_workflow(source.as_bytes())
                .map_err(|error| format!("{outer}/{inner}: {error}"))?;
            let original = workflow.to_parts();
            let leaf_node = original.nodes.iter().find(|node| matches!(node.kind,
                vb_core::CompiledNodeKind::SetConst { value } if workflow.constant(value) == Some(&vb_core::ConstValue::I64(9))
            )).ok_or("missing authored leaf")?;
            let leaf_slot = leaf_node.output.slot().ok_or("missing leaf output")?;
            let leaf_step = leaf_node.id;
            let expected = multiplicity(outer)
                .checked_mul(multiplicity(inner))
                .ok_or("multiplicity overflow")?;
            let slot = workflow
                .input_slots()
                .first()
                .ok_or("missing source input")?
                .slot;
            let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
            if let Some(metadata) = workflow.source_metadata() {
                vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
            }
            let source_values: Box<[SlotValue]> = Box::from([SlotValue::I64(3), SlotValue::I64(5)]);
            let list = store.insert_list_with_taint(
                source_values.clone(),
                Box::from([Taint::Secret, Taint::DerivedFromSecret]),
            )?;
            let mut frame = new_run_frame(RunId::new(1), &workflow)?;
            frame.write_slot_with_taint(slot, SlotValue::List(list), Taint::Secret)?;
            let mut evidence = EvidenceCollector::new();
            let mut collect = CollectStates::new();
            let mut finished = false;
            for _ in 0..512 {
                let signal = drive_deterministic_full(
                    &workflow,
                    &mut frame,
                    &mut StepBudget::new(1),
                    &mut store,
                    &[],
                    RetryPolicy::NEVER,
                    &mut evidence,
                    &mut collect,
                    &CapabilitySet::empty(),
                    1,
                )
                .map_err(|error| format!("{outer}/{inner}: {error}"))?;
                assert_eq!(
                    *frame.read_slot(slot)?,
                    SlotValue::List(list),
                    "{outer}/{inner}"
                );
                assert_eq!(frame.read_taint(slot)?, Taint::Secret, "{outer}/{inner}");
                assert_eq!(store.list(list)?, source_values.as_ref(), "{outer}/{inner}");
                assert_eq!(
                    store.list_item_with_taint(list, 0)?,
                    (SlotValue::I64(3), Taint::Secret)
                );
                assert_eq!(
                    store.list_item_with_taint(list, 1)?,
                    (SlotValue::I64(5), Taint::DerivedFromSecret)
                );
                if let RuntimeSignal::Finished(value) = signal {
                    assert_eq!(value, SlotValue::I64(42), "{outer}/{inner}");
                    finished = true;
                    break;
                }
                assert_eq!(
                    signal,
                    RuntimeSignal::StepBudgetExhausted,
                    "{outer}/{inner}"
                );
            }
            assert!(finished, "{outer}/{inner} did not terminate");
            assert_eq!(
                *frame.read_slot(leaf_slot)?,
                SlotValue::I64(9),
                "{outer}/{inner}"
            );
            assert_eq!(
                frame.read_taint(leaf_slot)?,
                Taint::Clean,
                "{outer}/{inner}"
            );
            let events = evidence.drain();
            assert_eq!(events.iter().filter(|event| matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == leaf_step)).count(), expected, "{outer}/{inner}");
            assert_eq!(
                leaf_write_count(&events, leaf_step, leaf_slot),
                expected,
                "{outer}/{inner}"
            );
            assert_eq!(workflow.to_parts(), original, "{outer}/{inner}");
            exercised = exercised.checked_add(1).ok_or("case count overflow")?;
        }
    }
    assert_eq!(exercised, 36);
    Ok(())
}

fn multiplicity(family: &str) -> usize {
    match family {
        "repeat" | "for_each" | "collect" | "reduce" => 2,
        _ => 1,
    }
}

#[test]
fn three_and_eight_level_repeats_observe_every_leaf_and_final_result()
-> Result<(), Box<dyn std::error::Error>> {
    for depth in [3_u32, 8] {
        let mut body = "- id: leaf\n  set: {output: leaf, value: '9'}\n".to_owned();
        for level in (0..depth).rev() {
            body = compound("repeat", &format!("level_{level}"), &body)?;
        }
        let source = format!(
            "version: velvet-ballistics/v1\nname: deep_repeats\nwhen: {{manual: {{}}}}\nvars: {{marker: source_payload}}\nsteps:\n{}  - id: done\n    finish: {{result: '$step.level_0'}}\n",
            indent(&body, 2)
        );
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        let original = workflow.to_parts();
        let leaf = original.nodes.iter().find(|node| matches!(node.kind, vb_core::CompiledNodeKind::SetConst { value } if workflow.constant(value) == Some(&vb_core::ConstValue::I64(9)))).ok_or("missing leaf")?;
        let leaf_slot = leaf.output.slot().ok_or("leaf output")?;
        let expected = 2_usize.checked_pow(depth).ok_or("multiplicity overflow")?;
        let bound = vb_core::budget::WholeWorkflowBudget::compute(
            &original.nodes,
            original.entry,
            &original.resource_contract,
        )?
        .max_total_steps
        .checked_add(1)
        .ok_or("step bound overflow")?;
        for quantum in [1, 64] {
            let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
            vb_core::workflow::source_metadata::seed_source_symbols(
                workflow.source_metadata().ok_or("metadata")?,
                &mut store,
            )?;
            let mut frame = new_run_frame(RunId::new(1), &workflow)?;
            let mut evidence = EvidenceCollector::new();
            let mut collect = CollectStates::new();
            let mut finished = false;
            let mut successes = 0_usize;
            let mut writes = 0_usize;
            for _ in 0..bound {
                let signal = drive_deterministic_full(
                    &workflow,
                    &mut frame,
                    &mut StepBudget::new(quantum),
                    &mut store,
                    &[],
                    RetryPolicy::NEVER,
                    &mut evidence,
                    &mut collect,
                    &CapabilitySet::empty(),
                    1,
                )?;
                let mut executing_leaf = false;
                for event in evidence.drain() {
                    match event {
                        EvidenceEvent::StepStarted { step } => executing_leaf = step == leaf.id,
                        EvidenceEvent::StepSucceeded { step, .. } if step == leaf.id => {
                            successes = successes.checked_add(1).ok_or("count overflow")?
                        }
                        EvidenceEvent::SlotWritten {
                            slot, value, taint, ..
                        } if slot == leaf_slot => {
                            assert_eq!(
                                value,
                                SlotValue::I64(9),
                                "depth={depth}, quantum={quantum}"
                            );
                            assert_eq!(taint, Taint::Clean);
                            if executing_leaf {
                                writes = writes.checked_add(1).ok_or("count overflow")?;
                            }
                        }
                        _ => {}
                    }
                }
                if let RuntimeSignal::Finished(value) = signal {
                    assert_eq!(value, SlotValue::I64(9));
                    finished = true;
                    break;
                }
                assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
            }
            assert!(finished, "depth={depth}, quantum={quantum}");
            assert_eq!(successes, expected);
            assert_eq!(writes, expected);
            assert_eq!(*frame.read_slot(leaf_slot)?, SlotValue::I64(9));
            assert_eq!(frame.read_taint(leaf_slot)?, Taint::Clean);
            assert_eq!(workflow.to_parts(), original);
        }
    }
    Ok(())
}

fn leaf_write_count(
    events: &[EvidenceEvent],
    leaf: vb_core::StepIdx,
    output: vb_core::SlotIdx,
) -> usize {
    let mut active = false;
    events
        .iter()
        .filter(|event| match event {
            EvidenceEvent::StepStarted { step } => {
                active = *step == leaf;
                false
            }
            EvidenceEvent::SlotWritten {
                slot,
                value: SlotValue::I64(9),
                taint: Taint::Clean,
                ..
            } => active && *slot == output,
            _ => false,
        })
        .count()
}
