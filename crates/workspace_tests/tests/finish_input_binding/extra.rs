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
fn test_input_expression_save_preserves_value_and_secret() -> Result<(), Box<dyn std::error::Error>>
{
    let yaml = "version: velvet-ballistics/v1\nname: input_finish\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: save\n    save:\n      output: saved\n      value: '$input.count'\n  - id: done\n    finish:\n      result: saved\n";

    let workflow = compile(yaml)?;

    assert_eq!(
        workflow.input_slots(),
        &[CompiledInputSlot {
            slot: SlotIdx::new(1),
            kind: InputSlotKind::I64,
        }]
    );

    assert_eq!(workflow.node_count(), 2);

    let node0 = workflow
        .node(vb_core::StepIdx::new(0))
        .ok_or("node 0 missing")?;
    match &node0.kind {
        CompiledNodeKind::EvalExpr { expr } => {
            assert_eq!(*expr, vb_core::ExprIdx::new(0));
        }
        _ => return Err("node 0 should be EvalExpr".into()),
    }
    assert_eq!(node0.output, Some(SlotIdx::new(0)));
    assert_eq!(node0.next, Some(vb_core::StepIdx::new(1)));

    let expr = workflow
        .expression(vb_core::ExprIdx::new(0))
        .ok_or("expr 0 missing")?;
    assert_eq!(
        expr.ops.as_ref(),
        &[vb_core::ExprOp::LoadSlot(SlotIdx::new(1))]
    );

    let node1 = workflow
        .node(vb_core::StepIdx::new(1))
        .ok_or("node 1 missing")?;
    match &node1.kind {
        CompiledNodeKind::Finish { result } => {
            assert_eq!(*result, SlotIdx::new(0));
        }
        _ => return Err("node 1 should be Finish".into()),
    }
    assert_eq!(node1.next, None);

    for value in [i64::MIN, -1, 0, 42, i64::MAX] {
        let signal = run(&workflow, &[value])?;
        assert_eq!(
            signal,
            EngineSignal::Finished(SlotValue::I64(value), Taint::Secret)
        );
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
