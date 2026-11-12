use super::*;

#[test]
fn foreach_outer_variable_visible_two_levels() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$entry'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 15);
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    let CompiledNodeKind::ForEachStart {
        item_slot: outer_item,
        body: outer_body,
        done: outer_done,
        limit: outer_limit,
        ..
    } = &node(&workflow, 3)?.kind
    else {
        return Err("node 3 is not the outer ForEachStart".to_owned());
    };
    assert_eq!(
        (*outer_body, *outer_done, *outer_limit),
        (StepIdx::new(4), StepIdx::new(13), FanoutLimit::new(4))
    );
    let CompiledNodeKind::ForEachStart {
        item_slot: inner_item,
        body: inner_body,
        done: inner_done,
        limit: inner_limit,
        ..
    } = &node(&workflow, 6)?.kind
    else {
        return Err("node 6 is not the inner ForEachStart".to_owned());
    };
    assert_eq!(
        (*inner_body, *inner_done, *inner_limit),
        (StepIdx::new(7), StepIdx::new(10), FanoutLimit::new(4))
    );
    assert_ne!(outer_item, inner_item);
    let CompiledNodeKind::EvalExpr { expr } = &node(&workflow, 7)?.kind else {
        return Err("node 7 is not the leaf expression".to_owned());
    };
    let program = workflow
        .expression(*expr)
        .ok_or("missing expression program")?;
    let ops: &[ExprOp] = &program.ops;
    assert_eq!(ops, [ExprOp::LoadSlot(*outer_item)]);
    Ok(())
}

#[test]
fn foreach_deep_set_capture_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {output: entry, value: '1'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(e, CompileError::DuplicateOutputName { .. })
    })
}

#[test]
fn foreach_deep_step_capture_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            steps:\n              - id: entry\n                set: {output: picked, value: '1'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(e, CompileError::DuplicateOutputName { .. })
    })
}

#[test]
fn foreach_choose_branch_capture_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: flag\n    set: {output: go, value: 'true'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: route\n          choose:\n            branches:\n              - when: '$go'\n                steps:\n                  - id: taken\n                    set: {output: entry, value: '$entry'}\n            otherwise: done\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(e, CompileError::DuplicateOutputName { .. })
    })
}

#[test]
fn foreach_repeat_body_capture_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: retry\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: leaf\n                set: {output: entry, value: '1'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(e, CompileError::DuplicateOutputName { .. })
    })
}

#[test]
fn foreach_at_once_hardened() -> Result<(), String> {
    for at_once in ["0", "2"] {
        let text = source(&format!(
            "  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      at_once: {at_once}\n      steps:\n        - id: leaf\n          set: {{output: picked, value: '$entry'}}\n"
        )) + finish_tail();
        rejects(&text, |e| {
            matches!(
                e,
                CompileError::StepFieldShape {
                    field: "for_each.at_once",
                    ..
                }
            )
        })?;
    }
    let nested = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            at_once: 3\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n",
    ) + finish_tail();
    rejects(&nested, |e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "for_each.at_once",
                ..
            }
        )
    })?;
    let single = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      at_once: 1\n      steps:\n        - id: leaf\n          set: {output: picked, value: '$entry'}\n",
    ) + finish_tail();
    let workflow = compile(&single)?;
    assert_eq!(workflow.to_parts().nodes.len(), 9);
    Ok(())
}

#[test]
fn foreach_nested_input_stays_bounded() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$vars.inner_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "for_each.input",
                ..
            }
        )
    })
}

#[test]
fn foreach_variable_step_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: step\n      steps:\n        - id: leaf\n          set: {output: picked, value: '$entry'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(
            e,
            CompileError::InvalidName {
                field: "for_each.variable",
                value,
            } if value.as_ref() == "step"
        )
    })
}

#[test]
fn aggregate_inside_foreach_capture_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: fold\n          reduce:\n            variable: entry\n            input: '8'\n            initial: '0'\n            steps:\n              - id: update\n                set: {output: local_value, value: '$entry'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(
            e,
            CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "entry"
        )
    })
}
