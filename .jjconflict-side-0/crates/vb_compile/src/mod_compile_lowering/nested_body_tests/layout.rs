use super::*;

#[test]
fn nested_for_each_in_for_each_layout() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 7);
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    let CompiledNodeKind::ForEachStart {
        body: outer_body,
        done: outer_done,
        limit: outer_limit,
        ..
    } = &node(&workflow, 1)?.kind
    else {
        return Err("node 1 is not the outer ForEachStart".to_owned());
    };
    assert_eq!(
        (*outer_body, *outer_done, *outer_limit),
        (StepIdx::new(2), StepIdx::new(6), FanoutLimit::new(4))
    );
    let CompiledNodeKind::ForEachStart {
        body: inner_body,
        done: inner_done,
        limit: inner_limit,
        ..
    } = &node(&workflow, 2)?.kind
    else {
        return Err("node 2 is not the inner ForEachStart".to_owned());
    };
    assert_eq!(
        (*inner_body, *inner_done, *inner_limit),
        (StepIdx::new(3), StepIdx::new(5), FanoutLimit::new(4))
    );
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::ForEachNext {
            iterator_slot: node(&workflow, 2)?
                .output
                .slot()
                .ok_or("missing iterator")?,
            body: StepIdx::new(3),
            done: StepIdx::new(5),
        }
    );
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::ForEachNext {
            iterator_slot: node(&workflow, 1)?
                .output
                .slot()
                .ok_or("missing iterator")?,
            body: StepIdx::new(2),
            done: StepIdx::new(6),
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
    assert_eq!(workflow.to_parts().nodes.len(), 9);
    let leaf = output(&workflow, 3)?;
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::RepeatFinish { result: leaf }
    );
    assert_eq!(
        node(&workflow, 7)?.kind,
        CompiledNodeKind::RepeatFinish { result: leaf }
    );
    Ok(())
}

#[test]
fn nested_choose_in_for_each_layout() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: flag\n    set: {output: go, value: 'true'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: route\n          choose:\n            branches:\n              - when: '$flag'\n                steps:\n                  - id: taken\n                    set: {output: picked, value: '$entry'}\n            otherwise: done\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 7);
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    assert_eq!(
        node(&workflow, 4)?.routing.next,
        NodeContinuation::Step(StepIdx::new(5))
    );
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::ForEachNext {
            iterator_slot: node(&workflow, 2)?
                .output
                .slot()
                .ok_or("missing iterator")?,
            body: StepIdx::new(3),
            done: StepIdx::new(6),
        }
    );
    Ok(())
}

#[test]
fn nested_for_each_in_repeat_with_step_reference() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: sub\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n        - id: grab\n          set: {output: grabbed, value: '$step.sub'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 9);
    let leaf = output(&workflow, 3)?;
    let grab = output(&workflow, 5)?;
    assert_ne!(leaf, grab);
    assert_eq!(
        node(&workflow, 7)?.kind,
        CompiledNodeKind::RepeatFinish { result: grab }
    );
    assert_eq!(node(&workflow, 7)?.output, NodeOutput::Slot(grab));
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
            CompileError::UnsupportedStepPrimitive {
                primitive: "wait",
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
            CompileError::UnsupportedStepPrimitive {
                primitive: "reduce",
                ..
            }
        )
    })
}

#[test]
fn together_inside_loop_body_stays_rejected() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: fan\n          together:\n            branches:\n              - label: a\n                steps:\n                  - id: only\n                    set: {output: solo, value: '1'}\n",
    ) + finish_tail();
    rejects(&text, |e| {
        matches!(
            e,
            CompileError::UnsupportedStepPrimitive {
                primitive: "parallel",
                ..
            }
        )
    })
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
        matches!(e, CompileError::UnsupportedStepControlField { .. })
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
            CompileError::UnsupportedStepPrimitive {
                primitive: "finish",
                ..
            }
        )
    })
}

#[test]
fn foreach_multi_step_body_layout() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: one\n          set: {output: picked, value: '$entry'}\n        - id: two\n          set: {output: second, value: '$picked'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    assert_eq!(workflow.to_parts().nodes.len(), 6);
    let CompiledNodeKind::ForEachStart { body, done, .. } = &node(&workflow, 1)?.kind else {
        return Err("node 1 is not ForEachStart".to_owned());
    };
    assert_eq!((*body, *done), (StepIdx::new(2), StepIdx::new(5)));
    assert_eq!(
        node(&workflow, 2)?.routing.next,
        NodeContinuation::Step(StepIdx::new(3))
    );
    assert_eq!(
        node(&workflow, 3)?.routing.next,
        NodeContinuation::Step(StepIdx::new(4))
    );
    Ok(())
}
