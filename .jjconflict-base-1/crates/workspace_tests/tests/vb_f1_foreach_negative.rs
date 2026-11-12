use vb_compile::CompileError::{InvalidName, StepFieldShape};
use vb_compile::{CompileError, compile_workflow};

fn rejects(source: &str, expected: CompileError) -> Result<(), String> {
    match compile_workflow(source.as_bytes()) {
        Ok(_) => Err("expected compile error but got success".to_string()),
        Err(errors) => {
            assert_eq!(errors.0, vec![expected]);
            Ok(())
        }
    }
}

#[test]
fn valid_source() -> Result<(), String> {
    compile_workflow(SOURCE.as_bytes()).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

const SOURCE: &str = r#"version: velvet-ballistics/v1
name: foreach_negative
when: {manual: {}}
inputs:
  items: {is: list, of: any, max: 3}
steps:
  - id: seed
    set: {output: answer, value: "42"}
  - id: bounded_loop
    for_each:
      variable: entry
      input: "$input.items"
      at_once: 1
      steps:
        - id: copy
          set: {output: copied, value: "$entry"}
  - id: done
    finish: {result: answer}
"#;

#[test]
fn for_each_at_once_must_be_one() -> Result<(), String> {
    for n in [0, 2] {
        let src = SOURCE.replace("at_once: 1", &format!("at_once: {n}"));
        if n == 0 {
            rejects(
                &src,
                StepFieldShape {
                    step: 1,
                    field: "for_each.at_once",
                    expected: "a positive concurrency upper bound",
                },
            )?;
        } else {
            extras::assert_serial_cap(&src)?;
        }
    }
    Ok(())
}

#[test]
fn for_each_variable_name_restricted() -> Result<(), String> {
    let names = ["item", "step"];
    for name in names {
        let src = SOURCE.replace("variable: entry", &format!("variable: {name}"));
        rejects(
            &src,
            InvalidName {
                field: "for_each.variable",
                value: name.into(),
            },
        )?;
    }
    Ok(())
}

#[test]
fn for_each_input_shape_validation() -> Result<(), String> {
    let input_not_list = SOURCE.replace("$input.items", "0");
    rejects(
        &input_not_list,
        CompileError::TypeMismatch {
            field: "for_each.input",
            expected: "list",
            found: "non-list",
        },
    )?;

    let typed_source = SOURCE.replace("of: any", "of: number");
    execute_typed_list(&typed_source)?;

    let input_no_max = SOURCE.replace(", max: 3", "");
    extras::assert_bound(&input_no_max, 65_535, None)?;

    Ok(())
}

#[test]
fn for_each_max_overflow() -> Result<(), String> {
    let overflow = SOURCE.replace("max: 3", "max: 4294967296");
    extras::assert_bound(&overflow, 65_535, Some(4_294_967_296))?;
    let mut store = vb_core::ValueStore::new();
    let too_many = vb_core::limits::MAX_LIST_ITEMS_PER_VALUE
        .checked_add(1)
        .ok_or("list length")?;
    assert_eq!(
        store.insert_list(vec![vb_core::SlotValue::Null; too_many]),
        Err(vb_core::CoreError::ResourceLimitExceeded {
            resource: "list_items"
        })
    );
    rejects(
        &SOURCE.replace("max: 3", "max: -1"),
        CompileError::InvalidInputSchema {
            field: "inputs.min/max",
            expected: "non-negative list length bounds",
        },
    )
}

#[test]
fn for_each_unknown_reference() -> Result<(), String> {
    let missing = SOURCE.replace("$input.items", "$input.missing");
    rejects(
        &missing,
        CompileError::UnknownReferenceName {
            kind: "declaration",
            reference: "$input.missing".into(),
            name: "$input.missing".into(),
        },
    )
}

#[test]
fn rejects_type_mismatch_in_set_value() -> Result<(), String> {
    let modified = SOURCE.replace("value: \"$entry\"", "value: \"$entry + 1\"");
    rejects(
        &modified,
        CompileError::TypeMismatch {
            field: "set.value".into(),
            expected: "known static operand type".into(),
            found: "unknown".into(),
        },
    )
}

#[test]
fn rejects_duplicate_output_name() -> Result<(), String> {
    let modified = SOURCE.replace("output: copied", "output: entry");
    rejects(
        &modified,
        CompileError::DuplicateOutputName {
            name: "entry".into(),
        },
    )
}

#[test]
fn rejects_unknown_reference_in_finish() -> Result<(), String> {
    let modified = SOURCE.replace("result: answer", "result: $copied");
    rejects(
        &modified,
        CompileError::UnknownReferenceName {
            kind: "output",
            reference: "$copied".into(),
            name: "copied".into(),
        },
    )
}

#[path = "vb_f1_foreach_negative/extras.rs"]
mod extras;

fn execute_typed_list(source: &str) -> Result<(), String> {
    use vb_core::{CapabilitySet, RunId, SlotValue, StepBudget, Taint, ValueStore, new_run_frame};
    use vb_runtime::engine::{
        EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
    };
    use vb_runtime::primitives::collect::CollectStates;
    let workflow = compile_workflow(source.as_bytes()).map_err(|error| error.to_string())?;
    let original = workflow.to_parts();
    let input = workflow
        .input_slots()
        .first()
        .ok_or("missing typed list input")?;
    assert_eq!(input.kind, vb_core::InputSlotKind::List);
    assert_eq!(input.constraint.element, Some(vb_core::InputSlotKind::I64));
    let values = [SlotValue::I64(7), SlotValue::I64(9)];
    let mut store = ValueStore::new();
    let list = store
        .insert_list(values.to_vec().into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut frame = new_run_frame(RunId::new(1), &workflow).map_err(|error| error.to_string())?;
    frame
        .write_slot_with_taint(input.slot, SlotValue::List(list), Taint::Clean)
        .map_err(|error| error.to_string())?;
    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(128),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(42)));
    assert_eq!(
        frame
            .read_slot(input.slot)
            .map_err(|error| error.to_string())?,
        &SlotValue::List(list)
    );
    assert_eq!(
        store.list(list).map_err(|error| error.to_string())?,
        values.as_slice()
    );
    assert_eq!(workflow.to_parts(), original);
    Ok(())
}

fn execute_policy_bound(workflow: &vb_core::CompiledWorkflow) -> Result<(), String> {
    use vb_core::{
        CapabilitySet, CompiledNodeKind as K, ExprOp, RunId, SlotValue, StepBudget, Taint,
        ValueStore, new_run_frame, value::join_taint,
    };
    use vb_runtime::engine::{
        EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
    };
    use vb_runtime::primitives::collect::CollectStates;

    let original = workflow.to_parts();
    let input = workflow
        .input_slots()
        .first()
        .ok_or("missing policy list input")?;
    let starts = original
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, K::ForEachStart { .. }))
        .collect::<Vec<_>>();
    let [start] = starts.as_slice() else {
        return Err("expected one ForEachStart".into());
    };
    let K::ForEachStart {
        item_slot,
        body,
        done,
        ..
    } = &start.kind
    else {
        return Err("missing ForEachStart".into());
    };
    let cursor = start.output.slot().ok_or("missing cursor")?;
    let body_node = workflow.node(*body).ok_or("missing body")?;
    let copied = body_node.output.slot().ok_or("missing copied output")?;
    let K::EvalExpr { expr } = &body_node.kind else {
        return Err("body must evaluate the item".into());
    };
    assert_eq!(
        workflow
            .expression(*expr)
            .ok_or("missing body expression")?
            .ops
            .as_ref(),
        &[ExprOp::LoadSlot(*item_slot)]
    );
    let join = workflow.node(*done).ok_or("missing join")?;
    let K::ForEachJoin {
        output: accumulator,
    } = &join.kind
    else {
        return Err("missing ordered join".into());
    };
    let output = join.output.slot().ok_or("missing public result")?;
    let values = [SlotValue::I64(-7), SlotValue::Bool(true), SlotValue::Null];
    let item_taints = [Taint::Secret, Taint::Random, Taint::Clean];
    for root in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ] {
        let mut store = ValueStore::new();
        let list = store
            .insert_list_with_taint(Box::from(values), Box::from(item_taints))
            .map_err(|e| e.to_string())?;
        let mut frame = new_run_frame(RunId::new(1), workflow).map_err(|e| e.to_string())?;
        frame
            .write_slot_with_taint(input.slot, SlotValue::List(list), root)
            .map_err(|e| e.to_string())?;
        let mut evidence = EvidenceCollector::new();
        let signal = drive_deterministic_full(
            workflow,
            &mut frame,
            &mut StepBudget::new(128),
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut CollectStates::new(),
            &CapabilitySet::empty(),
            1,
        )
        .map_err(|e| e.to_string())?;
        assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(42)));
        assert_eq!(frame.executed(), 15);
        assert_eq!(
            frame.read_slot(input.slot).map_err(|e| e.to_string())?,
            &SlotValue::List(list)
        );
        assert_eq!(
            frame.read_taint(input.slot).map_err(|e| e.to_string())?,
            root
        );
        let source = values.into_iter().zip(item_taints).collect::<Vec<_>>();
        assert_eq!(
            store
                .list_items_with_taint(list)
                .map_err(|e| e.to_string())?
                .collect::<Vec<_>>(),
            source
        );
        let SlotValue::List(result) = *frame.read_slot(output).map_err(|e| e.to_string())? else {
            return Err("ordered result must be a list".into());
        };
        assert_ne!(result, list);
        assert_eq!(
            frame.read_slot(*accumulator).map_err(|e| e.to_string())?,
            &SlotValue::List(result)
        );
        assert_eq!(
            store.list(result).map_err(|e| e.to_string())?,
            values.as_slice()
        );
        let expected = values
            .into_iter()
            .zip(item_taints)
            .map(|(value, tag)| (value, join_taint(root, tag)))
            .collect::<Vec<_>>();
        assert_eq!(
            store
                .list_items_with_taint(result)
                .map_err(|e| e.to_string())?
                .collect::<Vec<_>>(),
            expected
        );
        let expected_root = item_taints.into_iter().fold(root, join_taint);
        assert_eq!(
            frame.read_taint(output).map_err(|e| e.to_string())?,
            expected_root
        );
        assert_eq!(frame.read_taint(cursor).map_err(|e| e.to_string())?, root);
        let events = evidence.drain();
        let started = events
            .iter()
            .filter_map(|event| match event {
                EvidenceEvent::StepStarted { step } if step == body => Some(*step),
                _ => None,
            })
            .collect::<Vec<_>>();
        let succeeded = events
            .iter()
            .filter_map(|event| match event {
                EvidenceEvent::StepSucceeded { step, .. } if step == body => Some(*step),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(started, vec![*body; values.len()]);
        assert_eq!(succeeded, vec![*body; values.len()]);
        let writes = events
            .iter()
            .filter_map(|event| match event {
                EvidenceEvent::SlotWritten {
                    slot, value, taint, ..
                } if *slot == copied => Some((*value, *taint)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(writes, expected);
        let published = events
            .iter()
            .filter_map(|event| match event {
                EvidenceEvent::SlotWritten {
                    slot, value, taint, ..
                } if *slot == output => Some((*value, *taint)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(published, vec![(SlotValue::List(result), expected_root)]);
        assert_eq!(workflow.to_parts(), original);
    }
    Ok(())
}
