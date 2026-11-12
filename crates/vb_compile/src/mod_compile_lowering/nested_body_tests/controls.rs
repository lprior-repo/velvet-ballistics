use super::*;

#[test]
fn wait_inside_loop_body_checks_its_operand_and_binds_a_known_timeout() -> Result<(), String> {
    let valid = in_for_each("        - id: pause\n          wait: {timeout: '$seed'}\n");
    let workflow = compile(&valid)?;
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::WaitUntil {
            deadline_slot: output(&workflow, 0)?,
        }
    );
    assert_eq!(
        node(&workflow, 4)?.routing.next,
        NodeContinuation::Step(StepIdx::new(5))
    );
    Ok(())
}

#[test]
fn aggregate_inside_loop_body_checks_numeric_fields_and_keeps_its_result() -> Result<(), String> {
    let valid = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: fold\n          reduce:\n            variable: total\n            input: '$input.inner_items'\n            initial: '0'\n            steps:\n              - id: update\n                set: {output: local_value, value: '$total'}\n",
    ) + finish_tail();
    let workflow = compile(&valid)?;
    assert!(matches!(
        node(&workflow, 3)?.kind,
        CompiledNodeKind::ReduceStart { limit, .. } if limit == 4
    ));
    let result = output(&workflow, 6)?;
    assert_eq!(
        node(&workflow, 7)?.kind,
        CompiledNodeKind::Copy { source: result }
    );
    assert_eq!(
        node(&workflow, 9)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: output(&workflow, 7)?
        }
    );
    Ok(())
}

#[test]
fn nested_control_field_checks_boolean_type_and_lowers_a_valid_guard() -> Result<(), String> {
    let valid = in_for_each(
        "        - id: guarded\n          if: 'true'\n          set: {output: picked, value: '$entry'}\n",
    );
    let workflow = compile(&valid)?;
    let CompiledNodeKind::Choose {
        branches,
        otherwise,
    } = &node(&workflow, 5)?.kind
    else {
        return Err("guard did not lower to Choose".to_owned());
    };
    assert_eq!(branches.len(), 1);
    assert_eq!(
        branches.first().ok_or("guard branch")?.target,
        StepIdx::new(6)
    );
    assert_eq!(otherwise.step(), Some(StepIdx::new(8)));
    assert_eq!(
        node(&workflow, 6)?.routing.next,
        NodeContinuation::Step(StepIdx::new(7))
    );
    Ok(())
}

#[test]
fn conditional_finish_inside_loop_body_preserves_lexical_item_binding() -> Result<(), String> {
    let text = in_for_each(
        "        - id: early\n          if: 'true'\n          finish: {result: '$entry'}\n",
    );
    let workflow = compile(&text)?;
    let CompiledNodeKind::ForEachStart { item_slot, .. } = node(&workflow, 3)?.kind else {
        return Err("missing ForEachStart".to_owned());
    };
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::Finish { result: item_slot }
    );
    let CompiledNodeKind::Choose { otherwise, .. } = node(&workflow, 4)?.kind else {
        return Err("missing Finish guard".to_owned());
    };
    assert_eq!(otherwise.step(), Some(StepIdx::new(6)));
    Ok(())
}

#[test]
fn nested_repeat_in_repeat_with_step_reference() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: sub\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: leaf\n                set: {output: picked, value: '7'}\n        - id: grab\n          set: {output: grabbed, value: '$step.sub'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    assert_eq!(workflow.to_parts().nodes.len(), 12);
    let leaf = output(&workflow, 3)?;
    let inner = output(&workflow, 4)?;
    let grab = output(&workflow, 7)?;
    let result = output(&workflow, 8)?;
    assert_ne!(leaf, grab);
    assert_ne!(leaf, inner);
    assert_eq!(
        node(&workflow, 8)?.kind,
        CompiledNodeKind::Copy { source: grab }
    );
    assert_eq!(
        node(&workflow, 10)?.kind,
        CompiledNodeKind::RepeatFinish { result }
    );
    assert_eq!(node(&workflow, 10)?.output, NodeOutput::Slot(result));
    Ok(())
}

#[test]
fn nested_for_each_in_repeat_does_not_export_leaf_value() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: sub\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n        - id: grab\n          set: {output: grabbed, value: '$step.leaf'}\n",
    ) + finish_tail();
    rejects(
        &text,
        |e| matches!(e, CompileError::UnknownReferenceName { kind: "step", name, .. } if name.as_ref() == "leaf"),
    )
}
