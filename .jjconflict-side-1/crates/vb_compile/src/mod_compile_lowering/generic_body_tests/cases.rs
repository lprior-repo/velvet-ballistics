use super::*;

#[test]
fn generic_body_empty_multi_step_and_last_typed_output() {
    let (empty, builder) = compare(&[], 1, 0);
    assert_eq!(empty, Ok(None));
    assert_eq!(builder.nodes.len(), 1);
    let body = [step(0, 0), step(1, 1), step(2, 2)];
    let (result, builder) = compare(&body, 1, 0);
    assert_eq!(result, Ok(Some((SlotIdx::new(2), ExprType::Unknown))));
    assert_eq!(
        builder
            .nodes
            .iter()
            .map(|node| node.id.get())
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    let (result, _) = compare(&[step(0, 1), step(1, 0), step(2, 2)], 1, 0);
    assert_eq!(result, Ok(Some((SlotIdx::new(2), ExprType::I64))));
}

#[test]
fn generic_body_first_error_retains_prefix_and_failed_step_allocation() {
    let (result, builder) = compare(&[step(0, 0), step(1, 3), step(2, 0)], 1, 0);
    assert_eq!(
        result,
        Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: 2,
            field: "action",
            expected: "numeric action ID (u16)"
        }]))
    );
    assert_eq!(builder.nodes.len(), 2);
    assert_eq!(builder.max_slot, Some(2));
    assert_eq!(
        builder.constants,
        vec![vb_core::ConstValue::I64(-19), vb_core::ConstValue::I64(7)]
    );
    let (first, builder) = compare(&[step(0, 3), step(1, 5)], 1, 0);
    assert_eq!(
        first,
        Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: 1,
            field: "action",
            expected: "numeric action ID (u16)"
        }]))
    );
    assert_eq!(builder.nodes.len(), 1);
    assert_eq!(builder.max_slot, Some(1));
}

#[test]
fn generic_body_checked_limits_and_duplicate_ids_match_reference() {
    for start in [0, 1, u16::MAX - 1, u16::MAX] {
        let (result, _) = compare(&[step(0, 0), step(1, 1)], start, 0);
        if start == u16::MAX {
            assert_eq!(
                result,
                Err(CompileErrors(vec![CompileError::StepIndexOutOfRange {
                    value: usize::from(u16::MAX) + 1
                }]))
            );
        } else {
            assert_eq!(result, Ok(Some((SlotIdx::new(2), ExprType::Unknown))));
        }
    }
    let (result, builder) = compare(&[step(0, 0)], 1, usize::from(u16::MAX - 1));
    assert_eq!(
        result,
        Err(CompileErrors(vec![CompileError::SlotIndexOutOfRange {
            value: i64::from(u16::MAX)
        }]))
    );
    assert_eq!(builder.max_slot, Some(usize::from(u16::MAX - 1)));
    let (result, _) = compare(&[step(0, 0), step(0, 1)], 1, 0);
    assert_eq!(
        result,
        Err(CompileErrors(vec![CompileError::DuplicateStepId {
            id: "s0".into()
        }]))
    );
}

#[test]
fn generic_body_preparation_error_keeps_its_actual_partial_effects() {
    let mut bad_condition = step(1, 3);
    bad_condition.condition = Some("1".into());
    let (result, builder) = compare(&[step(0, 0), bad_condition, step(2, 0)], 1, 0);
    assert_eq!(
        result,
        Err(CompileErrors(vec![CompileError::TypeMismatch {
            field: "if",
            expected: "boolean",
            found: "non-boolean"
        }]))
    );
    assert_eq!(builder.nodes.len(), 2);
    assert_eq!(builder.max_slot, Some(1));
    assert_eq!(builder.constants.last(), Some(&vb_core::ConstValue::I64(1)));
    let mut bad_with = step(1, 0);
    bad_with.with = Some(crate::AuthorValue::I64(11));
    let (result, builder) = compare(&[step(0, 0), bad_with, step(2, 0)], 1, 0);
    assert_eq!(
        result,
        Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: 2,
            field: "with",
            expected: "an action step"
        }]))
    );
    assert_eq!(builder.nodes.len(), 3);
    assert_eq!(builder.max_slot, Some(2));
    assert_eq!(
        builder.constants.last(),
        Some(&vb_core::ConstValue::I64(11))
    );
}

#[test]
fn generic_body_layout_errors_precede_step_errors_and_controls_complete() {
    let (result, builder) = compare(&[step(0, 3), step(1, 0)], u16::MAX, 0);
    assert_eq!(
        result,
        Err(CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: usize::from(u16::MAX) + 1
        }]))
    );
    assert_eq!(builder.nodes.len(), 1);
    assert_eq!(builder.max_slot, Some(0));
    let mut first = step(0, 0);
    first.name = Some("display".into());
    first.condition = Some("true".into());
    first.then = Some("s2".into());
    let (result, builder) = compare(&[first, step(1, 1), step(2, 0)], 1, 0);
    assert_eq!(result, Ok(Some((SlotIdx::new(3), ExprType::I64))));
    assert_eq!(builder.nodes.len(), 5);
}

#[test]
fn generic_body_public_yaml_compiler_accepts_multiple_body_steps()
-> Result<(), Box<dyn std::error::Error>> {
    let source = crate::parse_workflow_source(
        r#"
version: velvet-ballistics/v1
name: multi_body
when: {manual: {}}
steps:
  - id: branch
    choose:
      branches:
        - when: 'true'
          steps:
            - id: first
              set: {output: left, value: '11'}
            - id: second
              set: {output: right, value: '12'}
result: {answer: 42}
"#,
    )?;
    let workflow = crate::compile_source(&source)?;
    let parts = workflow.to_parts();
    let authored: Vec<_> = parts
        .nodes
        .iter()
        .filter(|node| {
            matches!(node.kind, CompiledNodeKind::SetConst { value }
            if matches!(workflow.constant(value), Some(vb_core::ConstValue::I64(11 | 12))))
        })
        .collect();
    assert_eq!(authored.len(), 2);
    assert_eq!(
        authored.first().and_then(|node| node.routing.next.step()),
        authored.get(1).map(|node| node.id)
    );
    Ok(())
}
