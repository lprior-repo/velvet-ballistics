use super::*;
use vb_core::{CompiledInputConstraint, ConstValue, StepIdx, workflow::node};
use vb_expr::expr_typecheck::ExprType;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn declared_bounds_and_numeric_elements_follow_proven_slot_copies() -> TestResult {
    let mut builder = SlotCompiler::new();
    builder.record_input_slot(
        SlotIdx::new(0),
        InputSlotKind::List,
        CompiledInputConstraint {
            max: Some(2),
            element: Some(InputSlotKind::F64),
            ..CompiledInputConstraint::required()
        },
    );
    builder.push_node(node(
        StepIdx::new(0),
        K::Copy {
            source: SlotIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(1)),
    ));
    let expr = builder.push_expression(vb_core::ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadSlot(SlotIdx::new(1)),
    ]))?)?;
    builder.push_node(node(
        StepIdx::new(1),
        K::EvalExpr { expr },
        Some(SlotIdx::new(2)),
        None,
    ));
    for raw in [0, 1, 2] {
        assert_eq!(input_limit(SlotIdx::new(raw), &builder)?, 2);
        assert_eq!(
            item_type(
                SlotIdx::new(raw),
                &builder,
                crate::mod_compile_lowering::CanonicalScope {
                    outputs: &Default::default(),
                    step_outputs: &Default::default(),
                    slot_types: &Default::default(),
                    parent: None
                }
            ),
            ExprType::F64
        );
    }
    Ok(())
}

#[test]
fn static_lengths_and_homogeneous_or_mixed_element_types_are_exact() -> TestResult {
    for (values, expected) in [
        (vec![], ExprType::Unknown),
        (vec![ConstValue::I64(3), ConstValue::I64(5)], ExprType::I64),
        (
            vec![
                ConstValue::I64(3),
                ConstValue::F64(vb_core::FiniteF64::new(1.5)?),
            ],
            ExprType::F64,
        ),
        (
            vec![ConstValue::I64(3), ConstValue::Bool(true)],
            ExprType::Unknown,
        ),
    ] {
        let mut builder = SlotCompiler::new();
        let mut items = Vec::new();
        for value in values {
            let slot = builder.allocate_slot()?;
            let id = StepIdx::new(u16::try_from(builder.nodes.len())?);
            let value = builder.push_constant(value)?;
            builder.push_node(node(id, K::SetConst { value }, Some(slot), None));
            items.push(slot);
        }
        let len = u32::try_from(items.len())?;
        let output = builder.allocate_slot()?;
        let id = StepIdx::new(u16::try_from(builder.nodes.len())?);
        builder.push_node(node(
            id,
            K::BuildList {
                items: items.into_boxed_slice(),
            },
            Some(output),
            None,
        ));
        assert_eq!(input_limit(output, &builder)?, len);
        assert_eq!(
            item_type(
                output,
                &builder,
                crate::mod_compile_lowering::CanonicalScope {
                    outputs: &Default::default(),
                    step_outputs: &Default::default(),
                    slot_types: &Default::default(),
                    parent: None
                }
            ),
            expected
        );
    }
    Ok(())
}

#[test]
fn unknown_origins_use_the_existing_global_cap_without_inventing_inputs() -> TestResult {
    let builder = SlotCompiler::new();
    assert_eq!(
        input_limit(SlotIdx::new(9), &builder)?,
        u32::try_from(vb_core::limits::MAX_LIST_ITEMS_PER_VALUE)?
    );
    assert_eq!(
        item_type(
            SlotIdx::new(9),
            &builder,
            crate::mod_compile_lowering::CanonicalScope {
                outputs: &Default::default(),
                step_outputs: &Default::default(),
                slot_types: &Default::default(),
                parent: None
            }
        ),
        ExprType::Unknown
    );
    assert_eq!(builder.input_slots.as_slice(), []);
    Ok(())
}

#[test]
fn source_prelude_width_distinguishes_slots_from_accessors_and_expressions() {
    for source in [
        "0",
        "$rows",
        "$input.items",
        "$vars.rows",
        "$step.projected",
    ] {
        assert_eq!(source_width(source), 0);
    }
    for source in [
        "$vars.config.items",
        "$row.0",
        "$step.projected.0",
        "append($rows, 1)",
    ] {
        assert_eq!(source_width(source), 1);
    }
}
