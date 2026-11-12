use super::*;

#[test]
fn nested_repeat_compiles_with_exact_layout() -> Result<(), String> {
    let body = "        - id: inner\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: inner_value\n                set: {output: inside, value: '7'}\n";
    let workflow = compile(&source("", body, "answer", 3))?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 11);
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    let counter = output(&workflow, 1)?;
    let inner_counter = output(&workflow, 2)?;
    let inner_value = output(&workflow, 3)?;
    let inner_result = output(&workflow, 4)?;
    let outer_result = output(&workflow, 7)?;
    assert_eq!(
        node(&workflow, 1)?.kind,
        CompiledNodeKind::RepeatStart {
            max_attempts: 3.into(),
            body: StepIdx::new(2),
            done: StepIdx::new(9),
        }
    );
    assert_eq!(
        node(&workflow, 2)?.kind,
        CompiledNodeKind::RepeatStart {
            max_attempts: 1.into(),
            body: StepIdx::new(3),
            done: StepIdx::new(6),
        }
    );
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::RepeatCheck {
            attempt_slot: inner_counter,
            done: StepIdx::new(6),
        }
    );
    assert_eq!(
        node(&workflow, 5)?.routing.next,
        NodeContinuation::Step(StepIdx::new(3))
    );
    assert_eq!(
        node(&workflow, 6)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: inner_result
        }
    );
    assert_eq!(
        node(&workflow, 8)?.kind,
        CompiledNodeKind::RepeatCheck {
            attempt_slot: counter,
            done: StepIdx::new(9),
        }
    );
    assert_eq!(
        node(&workflow, 8)?.routing.next,
        NodeContinuation::Step(StepIdx::new(2))
    );
    assert_eq!(
        node(&workflow, 9)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: outer_result
        }
    );
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::Copy {
            source: inner_value
        }
    );
    assert_eq!(
        node(&workflow, 7)?.kind,
        CompiledNodeKind::Copy {
            source: inner_result
        }
    );
    Ok(())
}
