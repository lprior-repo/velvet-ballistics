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
    let values = [0, 2];
    for n in values {
        let src = SOURCE.replace("at_once: 1", &format!("at_once: {n}"));
        rejects(
            &src,
            StepFieldShape {
                step: 1,
                field: "for_each.at_once",
                expected: "omitted or 1 until concurrent for_each lowering is implemented",
            },
        )?;
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
        CompileError::StepFieldShape {
            step: 1,
            field: "for_each.input",
            expected: "direct declared list<any> input with finite schema max",
        },
    )?;

    let typed_source = SOURCE.replace("of: any", "of: number");
    execute_typed_list(&typed_source)?;

    let input_no_max = SOURCE.replace(", max: 3", "");
    rejects(
        &input_no_max,
        CompileError::StepFieldShape {
            step: 1,
            field: "for_each.input",
            expected: "direct declared list input with finite schema max",
        },
    )?;

    Ok(())
}

#[test]
fn for_each_max_overflow() -> Result<(), String> {
    let overflow = SOURCE.replace("max: 3", "max: 4294967296");
    rejects(
        &overflow,
        CompileError::InvalidInputSchema {
            field: "inputs.max",
            expected: "list maximum representable as u32",
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
