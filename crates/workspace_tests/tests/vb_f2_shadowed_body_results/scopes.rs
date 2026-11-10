use super::{TAILS, TestResult, indent, repeated, seeded_source};
use vb_core::{
    CapabilitySet, CompiledNodeKind, RunId, SlotValue, StepBudget, Taint, ValueStore, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

fn compound(family: &str, body: &str) -> TestResult<String> {
    let primitive = match family {
        "repeat" => return Ok(repeated(body, "scope")),
        "together" => format!(
            "together:\n  branches:\n    - label: first\n      steps:\n{}",
            indent(body, 8)
        ),
        "reduce" => format!(
            "reduce:\n  variable: accumulator\n  input: $input.items\n  initial: '0'\n  steps:\n{}",
            indent(body, 4)
        ),
        "collect" => format!(
            "collect:\n  variable: page_scope\n  source: $input.items\n  pages: 2\n  items: 1\n  steps:\n{}",
            indent(body, 4)
        ),
        "for_each" => format!(
            "for_each:\n  variable: outer_item\n  input: $input.items\n  steps:\n{}",
            indent(body, 4)
        ),
        _ => return Err("unknown family".into()),
    };
    Ok(format!("- id: scope\n{}", indent(&primitive, 2)))
}

fn execute_with_secret_source(yaml: &str, quantum: u64) -> TestResult {
    let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
    let original = plan.to_parts();
    vb_validate::shared::validate(&original)?;
    let input = plan.input_slots().first().ok_or("input")?.slot;
    let mut store = ValueStore::with_max_slots(plan.resource_contract().max_slots);
    if let Some(metadata) = plan.source_metadata() {
        vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
    }
    let values: Box<[SlotValue]> = Box::from([SlotValue::I64(3), SlotValue::I64(5)]);
    let list = store.insert_list_with_taint(
        values.clone(),
        Box::from([Taint::Secret, Taint::DerivedFromSecret]),
    )?;
    let mut frame = new_run_frame(RunId::new(1), &plan)?;
    frame.write_slot_with_taint(input, SlotValue::List(list), Taint::Secret)?;
    let mut collect = CollectStates::new();
    let mut evidence = EvidenceCollector::new();
    for _ in 0..512 {
        let signal = drive_deterministic_full(
            &plan,
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
        assert_eq!(*frame.read_slot(input)?, SlotValue::List(list));
        assert_eq!(frame.read_taint(input)?, Taint::Secret);
        assert_eq!(store.list(list)?, values.as_ref());
        assert_eq!(
            store.list_item_with_taint(list, 0)?,
            (SlotValue::I64(3), Taint::Secret)
        );
        assert_eq!(
            store.list_item_with_taint(list, 1)?,
            (SlotValue::I64(5), Taint::DerivedFromSecret)
        );
        assert!(
            !evidence
                .drain()
                .iter()
                .any(|event| matches!(event, EvidenceEvent::StepFailed { .. }))
        );
        if let RuntimeSignal::Finished(value) = signal {
            assert_eq!(value, SlotValue::Bool(true));
            for node in &original.nodes {
                if let CompiledNodeKind::RepeatFinish { result } = node.kind {
                    assert_eq!(*frame.read_slot(result)?, SlotValue::I64(7));
                    assert_eq!(frame.read_taint(result)?, Taint::Secret);
                }
            }
            assert_eq!(plan.to_parts(), original);
            return Ok(());
        }
        assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    }
    Err("workflow did not finish".into())
}

#[test]
fn nested_shared_callers_preserve_selected_values_taints_and_list_source() -> TestResult {
    for family in ["repeat", "together", "reduce", "collect", "for_each"] {
        for (_, tail, waits) in TAILS {
            if waits != 0 {
                continue;
            }
            let leaf = format!(
                "- id: value\n  set: {{output: value, value: '$input.items.0 + 4'}}\n{tail}"
            );
            let outer = compound(family, &repeated(&leaf, "inner"))?;
            let result = match family {
                "repeat" | "reduce" => "$step.scope == 7",
                "together" => "$step.scope.0 == 7",
                "collect" => "$step.scope.0 == 3 and $step.scope.1 == 5",
                "for_each" => "$saved == 99",
                _ => return Err("unknown family".into()),
            };
            let body = format!(
                "{outer}- id: checked\n  set: {{output: checked, value: '{result} and $saved == 99 and $input.items.0 == 3 and $input.items.1 == 5'}}\n"
            );
            let yaml = seeded_source(&body, "$step.checked");
            for quantum in [1, 64] {
                execute_with_secret_source(&yaml, quantum)?;
            }
        }
    }
    Ok(())
}

#[test]
fn direct_reduce_keeps_its_body_value_through_nonproducing_shadows() -> TestResult {
    for (_, tail, waits) in TAILS {
        if waits != 0 {
            continue;
        }
        let body =
            format!("- id: value\n  set: {{output: value, value: '$input.items.0 + 4'}}\n{tail}");
        let outer = compound("reduce", &body)?;
        let checked = format!(
            "{outer}- id: checked\n  set: {{output: checked, value: '$step.scope == 7 and $saved == 99'}}\n"
        );
        let yaml = seeded_source(&checked, "$step.checked");
        for quantum in [1, 64] {
            execute_with_secret_source(&yaml, quantum)?;
        }
    }
    Ok(())
}

#[test]
fn reduce_does_not_inherit_an_ancestor_as_its_required_body_value() -> TestResult {
    for (_, tail, _) in TAILS {
        let yaml = seeded_source(&compound("reduce", tail)?, "$step.scope");
        let result = vb_compile::compile_workflow(yaml.as_bytes());
        assert!(
            matches!(result, Err(vb_compile::CompileErrors(ref errors)) if errors.iter().any(|error| matches!(
                error,
                vb_compile::CompileError::StepFieldShape {
                    field: "reduce.steps", expected: "at least one set or do step", ..
                }
            ))),
            "{result:?}"
        );
    }
    Ok(())
}
