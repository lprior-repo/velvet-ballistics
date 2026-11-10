use super::*;

fn in_for_each(body: &str) -> String {
    source(&format!(
        "  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n{body}"
    )) + finish_tail()
}

#[test]
fn wait_inside_loop_body_checks_its_operand_and_binds_a_known_timeout() -> Result<(), String> {
    let malformed = in_for_each("        - id: pause\n          wait: {event: '\"e\"'}\n");
    rejects(&malformed, |e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "wait.event",
                expected: "integer string",
                ..
            }
        )
    })?;
    let valid = in_for_each("        - id: pause\n          wait: {timeout: '$seed'}\n");
    let workflow = compile(&valid)?;
    assert_eq!(
        node(&workflow, 2)?.kind,
        CompiledNodeKind::WaitUntil {
            deadline_slot: output(&workflow, 0)?,
        }
    );
    assert_eq!(
        node(&workflow, 2)?.routing.next,
        NodeContinuation::Step(StepIdx::new(3))
    );
    Ok(())
}

#[test]
fn aggregate_inside_loop_body_checks_numeric_fields_and_keeps_its_result() -> Result<(), String> {
    let malformed = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: fold\n          reduce:\n            variable: total\n            input: '\"8\"'\n            initial: '\"0\"'\n            steps:\n              - id: update\n                set: {output: local_value, value: '$total'}\n",
    ) + finish_tail();
    rejects(&malformed, |e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "slot reference",
                expected: "integer slot index or named reference",
                ..
            }
        )
    })?;
    let valid = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: fold\n          reduce:\n            variable: total\n            input: '$input.inner_items'\n            initial: '0'\n            steps:\n              - id: update\n                set: {output: local_value, value: '$total'}\n",
    ) + finish_tail();
    let workflow = compile(&valid)?;
    assert!(matches!(
        node(&workflow, 3)?.kind,
        CompiledNodeKind::ReduceStart { limit: 4, .. }
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
fn together_inside_loop_body_keeps_its_scoped_result() -> Result<(), String> {
    let text = in_for_each(
        "        - id: fan\n          together:\n            branches:\n              - label: a\n                steps:\n                  - id: only\n                    set: {output: solo, value: '1'}\n",
    );
    let workflow = compile(&text)?;
    let CompiledNodeKind::TogetherStart { branches, join } = &node(&workflow, 2)?.kind else {
        return Err("missing nested TogetherStart".to_owned());
    };
    assert_eq!(branches.as_ref(), &[StepIdx::new(3)]);
    assert!(matches!(
        node(&workflow, join.get())?.kind,
        CompiledNodeKind::TogetherJoin {
            branch_count: 1,
            ..
        }
    ));
    assert_ne!(output(&workflow, 2)?, output(&workflow, join.get())?);
    Ok(())
}

#[test]
fn nested_loop_variable_collision_stays_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: clash\n          set: {output: entry, value: '\"x\"'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(e, CompileError::DuplicateOutputName { .. })
    })
}

#[test]
fn nested_control_field_checks_boolean_type_and_lowers_a_valid_guard() -> Result<(), String> {
    let malformed = in_for_each(
        "        - id: guarded\n          if: '\"true\"'\n          set: {output: picked, value: '$entry'}\n",
    );
    rejects(&malformed, |e| {
        matches!(
            e,
            CompileError::TypeMismatch {
                field: "if",
                expected: "boolean",
                found: "non-boolean"
            }
        )
    })?;
    let valid = in_for_each(
        "        - id: guarded\n          if: 'true'\n          set: {output: picked, value: '$entry'}\n",
    );
    let workflow = compile(&valid)?;
    let CompiledNodeKind::Choose {
        branches,
        otherwise,
    } = &node(&workflow, 2)?.kind
    else {
        return Err("guard did not lower to Choose".to_owned());
    };
    assert_eq!(branches.len(), 1);
    assert_eq!(
        branches.first().ok_or("guard branch")?.target,
        StepIdx::new(3)
    );
    assert_eq!(*otherwise, Some(StepIdx::new(4)));
    assert_eq!(
        node(&workflow, 3)?.routing.next,
        NodeContinuation::Step(StepIdx::new(4))
    );
    Ok(())
}

#[test]
fn conditional_finish_inside_loop_body_preserves_lexical_item_binding() -> Result<(), String> {
    let text = in_for_each(
        "        - id: early\n          if: 'true'\n          finish: {result: '$entry'}\n",
    );
    let workflow = compile(&text)?;
    let CompiledNodeKind::ForEachStart { item_slot, .. } = node(&workflow, 1)?.kind else {
        return Err("missing ForEachStart".to_owned());
    };
    assert_eq!(
        node(&workflow, 3)?.kind,
        CompiledNodeKind::Finish { result: item_slot }
    );
    let CompiledNodeKind::Choose { otherwise, .. } = node(&workflow, 2)?.kind else {
        return Err("missing Finish guard".to_owned());
    };
    assert_eq!(otherwise, Some(StepIdx::new(4)));
    Ok(())
}
