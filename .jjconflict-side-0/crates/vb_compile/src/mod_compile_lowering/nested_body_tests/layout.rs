use super::*;

#[test]
fn nested_for_each_in_for_each_layout() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 15);
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    let CompiledNodeKind::ForEachStart {
        body: outer_body,
        done: outer_done,
        limit: outer_limit,
        ..
    } = &node(&workflow, 3)?.kind
    else {
        return Err("node 3 is not the outer ForEachStart".to_owned());
    };
    assert_eq!(
        (*outer_body, *outer_done, outer_limit.get()),
        (StepIdx::new(4), StepIdx::new(13), 4)
    );
    let CompiledNodeKind::ForEachStart {
        body: inner_body,
        done: inner_done,
        limit: inner_limit,
        ..
    } = &node(&workflow, 6)?.kind
    else {
        return Err("node 6 is not the inner ForEachStart".to_owned());
    };
    assert_eq!(
        (*inner_body, *inner_done, inner_limit.get()),
        (StepIdx::new(7), StepIdx::new(10), 4)
    );
    assert_eq!(
        node(&workflow, 9)?.kind,
        CompiledNodeKind::ForEachNext {
            iterator_slot: node(&workflow, 6)?
                .output
                .slot()
                .ok_or("missing iterator")?,
            body: StepIdx::new(7),
            done: StepIdx::new(10),
        }
    );
    assert_eq!(
        node(&workflow, 12)?.kind,
        CompiledNodeKind::ForEachNext {
            iterator_slot: node(&workflow, 3)?
                .output
                .slot()
                .ok_or("missing iterator")?,
            body: StepIdx::new(4),
            done: StepIdx::new(13),
        }
    );
    Ok(())
}

#[test]
fn nested_repeat_in_repeat_threads_inner_value() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: inner\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: leaf\n                set: {output: picked, value: '7'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    assert_eq!(workflow.to_parts().nodes.len(), 11);
    let leaf = output(&workflow, 3)?;
    let inner_result = output(&workflow, 4)?;
    let outer_result = output(&workflow, 7)?;
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::Copy { source: leaf }
    );
    assert_eq!(
        node(&workflow, 6)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: inner_result
        }
    );
    assert_eq!(
        node(&workflow, 7)?.kind,
        CompiledNodeKind::Copy {
            source: inner_result
        }
    );
    assert_eq!(
        node(&workflow, 9)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: outer_result
        }
    );
    Ok(())
}

#[test]
fn nested_choose_in_for_each_layout() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: flag\n    set: {output: go, value: 'true'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: route\n          choose:\n            branches:\n              - when: '$flag'\n                steps:\n                  - id: taken\n                    set: {output: picked, value: '$entry'}\n            otherwise: after_route\n        - id: after_route\n          set: {output: after_route, value: '$entry'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 12);
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    assert_eq!(
        node(&workflow, 6)?.routing.next,
        NodeContinuation::Step(StepIdx::new(7))
    );
    assert_eq!(
        node(&workflow, 9)?.kind,
        CompiledNodeKind::ForEachNext {
            iterator_slot: node(&workflow, 4)?
                .output
                .slot()
                .ok_or("missing iterator")?,
            body: StepIdx::new(5),
            done: StepIdx::new(10),
        }
    );
    let CompiledNodeKind::Choose { otherwise, .. } = node(&workflow, 5)?.kind else {
        return Err("missing nested Choose".to_owned());
    };
    assert_eq!(otherwise.step(), Some(StepIdx::new(7)));
    Ok(())
}

#[test]
fn nested_for_each_in_repeat_with_step_reference() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: sub\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n        - id: grab\n          set: {output: grabbed, value: '$step.sub'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 14);
    let leaf = output(&workflow, 5)?;
    let grab = output(&workflow, 9)?;
    assert_ne!(leaf, grab);
    let aggregate = output(&workflow, 8)?;
    assert_ne!(leaf, aggregate);
    let CompiledNodeKind::EvalExpr { expr } = node(&workflow, 9)?.kind else {
        return Err("grab is not an expression".into());
    };
    assert_eq!(
        workflow
            .expression(expr)
            .ok_or("grab expression")?
            .ops
            .as_ref(),
        &[vb_core::ExprOp::LoadSlot(aggregate)]
    );
    assert_eq!(
        node(&workflow, 10)?.kind,
        CompiledNodeKind::Copy { source: grab }
    );
    assert_eq!(
        node(&workflow, 12)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: output(&workflow, 10)?
        }
    );
    assert_eq!(
        node(&workflow, 12)?.output,
        NodeOutput::Slot(output(&workflow, 10)?)
    );
    Ok(())
}

#[test]
fn wait_inside_loop_body_stays_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: pause\n          wait: {event: '\"e\"'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "wait.event",
                expected: "integer string",
                ..
            }
        )
    })
}

#[test]
fn aggregate_inside_loop_body_stays_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: fold\n          reduce:\n            variable: total\n            input: '\"8\"'\n            initial: '\"0\"'\n            steps:\n              - id: update\n                set: {output: local_value, value: '$total'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "slot reference",
                expected: "integer slot index or named reference",
                ..
            }
        )
    })
}

#[test]
fn together_inside_loop_body_preserves_scoped_outputs() -> Result<(), String> {
    let text = in_for_each(
        "        - id: fan\n          together:\n            branches:\n              - label: a\n                steps:\n                  - id: only\n                    set: {output: solo, value: '1'}\n",
    );
    let workflow = compile(&text)?;
    let CompiledNodeKind::TogetherStart { branches, join } = &node(&workflow, 4)?.kind else {
        return Err("missing nested TogetherStart".to_owned());
    };
    assert_eq!(branches.as_ref(), &[StepIdx::new(5)]);
    assert!(matches!(
        node(&workflow, join.get())?.kind,
        CompiledNodeKind::TogetherJoin {
            branch_count,
            ..
        } if branch_count == 1.into()
    ));
    assert_ne!(output(&workflow, 4)?, output(&workflow, join.get())?);
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
fn nested_control_field_stays_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: guarded\n          if: '\"true\"'\n          set: {output: picked, value: '$entry'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(
            e,
            CompileError::TypeMismatch {
                field: "if",
                expected: "boolean",
                found: "non-boolean"
            }
        )
    })
}

#[test]
fn finish_inside_loop_body_stays_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: early\n          finish: {result: '$entry'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(
            e,
            CompileError::Workflow(vb_core::WorkflowError::UnreachableNode { .. })
        )
    })
}

#[test]
fn foreach_multi_step_body_layout() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: one\n          set: {output: picked, value: '$entry'}\n        - id: two\n          set: {output: second, value: '$picked'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    assert_eq!(workflow.to_parts().nodes.len(), 10);
    let CompiledNodeKind::ForEachStart { body, done, .. } = &node(&workflow, 3)?.kind else {
        return Err("node 3 is not ForEachStart".to_owned());
    };
    assert_eq!((*body, *done), (StepIdx::new(4), StepIdx::new(8)));
    assert_eq!(
        node(&workflow, 4)?.routing.next,
        NodeContinuation::Step(StepIdx::new(5))
    );
    assert_eq!(
        node(&workflow, 5)?.routing.next,
        NodeContinuation::Step(StepIdx::new(6))
    );
    Ok(())
}
