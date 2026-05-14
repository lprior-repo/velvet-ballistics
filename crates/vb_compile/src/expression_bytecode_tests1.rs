mod tests {
    use super::{compile_expr_to_bytecode, compile_expr_to_bytecode_with_accessors};
    use crate::CompileError;
    use crate::expression::parse_expression;
    use vb_core::{
        AccessorIdx, AccessorProgram, ConstIdx, ConstValue, ExprOp, PathSegment, SlotIdx,
    };

    type LoweredWithAccessors = (Vec<ExprOp>, Vec<ConstValue>, Vec<AccessorProgram>);

    fn lower(source: &str) -> Result<(Vec<ExprOp>, Vec<ConstValue>, u8), String> {
        let expr = parse_expression(source).map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        let program =
            compile_expr_to_bytecode(&expr, &mut constants).map_err(|error| error.to_string())?;
        Ok((program.ops.into_vec(), constants, program.max_stack))
    }

    fn lower_with_accessors(source: &str) -> Result<LoweredWithAccessors, String> {
        let expr = parse_expression(source).map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();
        let program =
            compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors)
                .map_err(|error| error.to_string())?;
        Ok((program.ops.into_vec(), constants, accessors))
    }

    #[test]
    fn lowers_binary_expression_to_postfix_bytecode() -> Result<(), String> {
        let (ops, constants, max_stack) = lower("1 + 2 * 3")?;

        let expected_ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::LoadConst(ConstIdx::new(2)),
            ExprOp::Mul,
            ExprOp::Add,
        ];
        let expected_constants = vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)];
        if ops != expected_ops {
            return Err(format!(
                "ops mismatch: expected {expected_ops:?}, got {ops:?}"
            ));
        }
        if constants != expected_constants {
            return Err(format!(
                "constants mismatch: expected {expected_constants:?}, got {constants:?}"
            ));
        }
        if max_stack != 3 {
            return Err(format!("max_stack mismatch: expected 3, got {max_stack}"));
        }
        Ok(())
    }

    #[test]
    fn lowers_unary_not_and_numeric_negation() -> Result<(), String> {
        let (ops, constants, max_stack) = lower("not -1")?;

        let expected_ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Sub,
            ExprOp::Not,
        ];
        let expected_constants = vec![ConstValue::I64(0), ConstValue::I64(1)];
        if ops != expected_ops {
            return Err(format!(
                "ops mismatch: expected {expected_ops:?}, got {ops:?}"
            ));
        }
        if constants != expected_constants {
            return Err(format!(
                "constants mismatch: expected {expected_constants:?}, got {constants:?}"
            ));
        }
        if max_stack != 2 {
            return Err(format!("max_stack mismatch: expected 2, got {max_stack}"));
        }
        Ok(())
    }

    #[test]
    fn validates_helper_arity_before_stack_validation() -> Result<(), String> {
        let expr = parse_expression("contains(1)").map_err(|error| error.to_string())?;
        let mut constants = Vec::new();

        match compile_expr_to_bytecode(&expr, &mut constants) {
            Err(CompileError::ExpressionHelperArity {
                helper: "contains",
                actual: 1,
                ..
            }) => Ok(()),
            other => Err(format!("unexpected lowering result: {other:?}")),
        }
    }

    #[test]
    fn rejects_references_until_accessor_table_exists() -> Result<(), String> {
        let expr = parse_expression("$input.value").map_err(|error| error.to_string())?;
        let mut constants = Vec::new();

        match compile_expr_to_bytecode(&expr, &mut constants) {
            Err(CompileError::ExpressionLoweringUnsupported { ref feature })
                if feature.as_ref() == "accessor references" =>
            {
                Ok(())
            }
            other => Err(format!("unexpected lowering result: {other:?}")),
        }
    }

    #[test]
    fn lowers_direct_slot_reference_to_load_slot() -> Result<(), String> {
        let (ops, constants, accessors) = lower_with_accessors("$slot.7 == true")?;
        let expected_ops = vec![
            ExprOp::LoadSlot(SlotIdx::new(7)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Eq,
        ];
        if ops != expected_ops {
            return Err(format!(
                "ops mismatch: expected {expected_ops:?}, got {ops:?}"
            ));
        }
        if constants != vec![ConstValue::Bool(true)] {
            return Err(format!("unexpected constants: {constants:?}"));
        }
        if !accessors.is_empty() {
            return Err(format!("direct slot ref created accessors: {accessors:?}"));
        }
        Ok(())
    }

    #[test]
    fn lowers_numeric_nested_slot_reference_to_accessor_table() -> Result<(), String> {
        let (ops, constants, accessors) = lower_with_accessors("$slots.2.0.3")?;
        let expected_ops = vec![ExprOp::LoadAccessor(AccessorIdx::new(0))];
        let expected_accessors = vec![AccessorProgram {
            root: SlotIdx::new(2),
            path: vec![PathSegment::Index(0), PathSegment::Index(3)].into_boxed_slice(),
        }];
        if ops != expected_ops {
            return Err(format!(
                "ops mismatch: expected {expected_ops:?}, got {ops:?}"
            ));
        }
        if !constants.is_empty() {
            return Err(format!("nested accessor created constants: {constants:?}"));
        }
        if accessors != expected_accessors {
            return Err(format!(
                "accessors mismatch: expected {expected_accessors:?}, got {accessors:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn lowers_single_list_index_accessor_to_table() -> Result<(), String> {
        let (ops, constants, accessors) = lower_with_accessors("$slot.4.12")?;
        let expected_ops = vec![ExprOp::LoadAccessor(AccessorIdx::new(0))];
        let expected_accessors = vec![AccessorProgram {
            root: SlotIdx::new(4),
            path: vec![PathSegment::Index(12)].into_boxed_slice(),
        }];
        if ops != expected_ops {
            return Err(format!(
                "ops mismatch: expected {expected_ops:?}, got {ops:?}"
            ));
        }
        if !constants.is_empty() {
            return Err(format!("list accessor created constants: {constants:?}"));
        }
        if accessors != expected_accessors {
            return Err(format!(
                "accessors mismatch: expected {expected_accessors:?}, got {accessors:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn rejects_field_accessor_without_symbol_table() -> Result<(), String> {
        let expr = parse_expression("$slot.1.name").map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();

        match compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors) {
            Err(CompileError::UnsupportedAccessorReference { root, path, .. })
                if root.as_ref() == "slot.1" && path.as_ref() == "name" =>
            {
                Ok(())
            }
            other => Err(format!("unexpected lowering result: {other:?}")),
        }
    }

    #[test]
    fn rejects_field_accessor_after_list_index_without_mutating_table() -> Result<(), String> {
        let expr = parse_expression("$slots.1.0.name").map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();

        match compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors) {
            Err(CompileError::UnsupportedAccessorReference { root, path, .. })
                if root.as_ref() == "slots.1" && path.as_ref() == "0.name" =>
            {
                if !accessors.is_empty() {
                    return Err(format!("unsupported accessor mutated table: {accessors:?}"));
                }
                Ok(())
            }
            other => Err(format!("unexpected lowering result: {other:?}")),
        }
    }

    #[test]
    fn rejects_empty_accessor_segment_with_exact_diagnostic_code() -> Result<(), String> {
        let expr = parse_expression("$slot.1..0").map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();

        match compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors) {
            Err(error @ CompileError::UnsupportedAccessorReference { .. }) => match error {
                CompileError::UnsupportedAccessorReference {
                    ref root, ref path, ..
                } if root.as_ref() == "slot.1"
                    && path.as_ref() == ".0"
                    && error.diagnostic_code() == "UNSUPPORTED_ACCESSOR_REFERENCE" =>
                {
                    Ok(())
                }
                other => Err(format!("unexpected lowering result: {other:?}")),
            },
            other => Err(format!("unexpected lowering result: {other:?}")),
        }
    }

    // ── Adversarial expression bytecode tests ────────────────────────────────

    fn adv_lower_error(source: &str) -> Result<CompileError, String> {
        let expr = parse_expression(source).map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        match compile_expr_to_bytecode(&expr, &mut constants) {
            Err(error) => Ok(error),
            Ok(program) => Err(format!("lowering unexpectedly succeeded: {program:?}")),
        }
    }

    fn adv_ensure(condition: bool, message: &'static str) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(message.to_owned())
        }
    }

    #[test]
    fn text_literal_rejected_with_expression_lowering_unsupported() -> Result<(), String> {
        let expr = parse_expression("\"hello\"").map_err(|e| e.to_string())?;
        let mut constants = Vec::new();
        let result = compile_expr_to_bytecode(&expr, &mut constants);
        adv_ensure(
            matches!(
                result,
                Err(CompileError::ExpressionLoweringUnsupported {
                    ref feature
                }) if feature.as_ref() == "text constants"
            ),
            "text literal did not produce exact text constants diagnostic",
        )
    }

    #[test]
    fn accessor_reference_without_table_rejected_with_unsupported_feature() -> Result<(), String> {
        let error = adv_lower_error("$slot.5")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionLoweringUnsupported {
                    ref feature
                } if feature.as_ref() == "accessor references"
            ),
            "accessor without table did not produce accessor references diagnostic",
        )
    }

    #[test]
    fn constant_pool_overflow_in_expression_rejected() -> Result<(), String> {
        let expr = parse_expression("1").map_err(|e| e.to_string())?;
        // Pre-fill constants to u16::MAX + 1 (65536) so the next push fails
        let count = usize::from(u16::MAX) + 1;
        let mut constants = Vec::with_capacity(count);
        for i in 0..count {
            let value = i64::try_from(i).map_err(|error| error.to_string())?;
            constants.push(ConstValue::I64(value));
        }
        let result = compile_expr_to_bytecode(&expr, &mut constants);
