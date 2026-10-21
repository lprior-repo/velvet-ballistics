use super::*;

#[test]
fn test_longform_schema() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = BASE.replace("count: number", "count: {is: number}");
    let workflow = compile(&yaml)?;
    assert_eq!(
        workflow.input_slots(),
        &[CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64
        }]
    );
    let signal = run(&workflow, &[123])?;
    assert_eq!(
        signal,
        EngineSignal::Finished(SlotValue::I64(123), Taint::Secret)
    );
    Ok(())
}

#[test]
fn test_duplicate_input_names() -> Result<(), Box<dyn std::error::Error>> {
    let s = parse_workflow_source(BASE)?;
    let mut inputs = s.inputs().to_vec();
    inputs.extend_from_slice(s.inputs());
    let duplicate = vb_compile::WorkflowSource::new(vb_compile::WorkflowSourceParts {
        version: s.version().to_owned(),
        name: s.name().to_owned(),
        trigger: s.trigger().clone(),
        inputs,
        vars: s.vars().to_vec(),
        secrets: s.secrets().to_vec(),
        steps: s.steps().to_vec(),
        result: s.result().cloned(),
        examples: s.examples().to_vec(),
    });
    match compile_source(&duplicate) {
        Err(errors) => {
            assert_eq!(
                errors,
                CompileErrors(vec![CompileError::DuplicateInputName {
                    name: Box::from("count")
                }])
            );
        }
        Ok(_) => return Err("expected DuplicateInputName".into()),
    }
    Ok(())
}

#[test]
fn test_no_finish_compile_fallback() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "version: velvet-ballistics/v1\nname: input_finish\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: save\n    save:\n      output: saved\n      value: '42'\n";
    let workflow = compile(yaml)?;
    assert_eq!(
        workflow.input_slots(),
        &[CompiledInputSlot {
            slot: SlotIdx::new(1),
            kind: InputSlotKind::I64
        }]
    );
    Ok(())
}

#[test]
fn test_unsupported_nonfinish_consumer() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "version: velvet-ballistics/v1\nname: input_finish\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: save\n    save:\n      output: saved\n      value: '$input.count'\n  - id: done\n    finish:\n      result: saved\n";
    let parsed = parse_workflow_source(yaml)?;
    match compile_source(&parsed) {
        Err(errors) => {
            assert_eq!(
                errors,
                CompileErrors(vec![CompileError::StepFieldShape {
                    step: 0,
                    field: "set.value",
                    expected: "integer string"
                }])
            );
        }
        Ok(_) => return Err("expected StepFieldShape".into()),
    }
    Ok(())
}

proptest::proptest! {
    #[test]
    fn arbitrary_input_value_survives(value: i64) {
        let workflow = compile(BASE).map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
        let actual = run(&workflow, &[value]).map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
        proptest::prop_assert_eq!(actual, EngineSignal::Finished(SlotValue::I64(value), Taint::Secret));
    }
}
