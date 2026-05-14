#![forbid(unsafe_code)]
//! Bytecode compilation tests.

mod adversarial;

#[allow(unused_imports, dead_code)]
use vb_core::{ConstIdx, ConstValue, ExprOp, SlotIdx};

#[allow(unused_imports)]
use crate::bytecode::{
    ReferenceResolver, check_expr_stack_bound, compile_expr, compile_expr_to_bytecode,
    compile_expr_with_pool, compile_expr_with_resolver, const_fold_expr, push_constant,
};
use crate::lexer::lex_expr;
use crate::parser::parse_expr;

#[allow(dead_code)]
fn resolve_test_reference(reference: &str) -> Option<SlotIdx> {
    match reference {
        "$a" => Some(SlotIdx::new(0)),
        "$b" => Some(SlotIdx::new(1)),
        "$c" => Some(SlotIdx::new(2)),
        "$x" => Some(SlotIdx::new(3)),
        _ => None,
    }
}

#[test]
fn compiles_binary_addition() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("1 + 2 * 3")?;
    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::LoadConst(ConstIdx::new(2)),
        ExprOp::Mul,
        ExprOp::Add,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(
        constants,
        vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)]
    );
    assert_eq!(program.max_stack, 3);
    Ok(())
}

#[test]
fn compiles_not_negation() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("not -1")?;
    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Sub,
        ExprOp::Not,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants, vec![ConstValue::I64(0), ConstValue::I64(1)]);
    assert_eq!(program.max_stack, 2);
    Ok(())
}

#[test]
fn compiles_helper_call() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("contains(1, 2)")?;
    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Contains,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants, vec![ConstValue::I64(1), ConstValue::I64(2)]);
    Ok(())
}

#[test]
fn constant_folds_addition() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 + 2")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::I64(3)));
    Ok(())
}

#[test]
fn constant_folds_boolean_logic() -> crate::ExprResult<()> {
    let tokens = lex_expr("true and false")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(false)));
    Ok(())
}

#[test]
fn does_not_fold_references() -> crate::ExprResult<()> {
    let tokens = lex_expr("$x + 1")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None);
    Ok(())
}

#[test]
fn compiles_all_comparison_ops() -> crate::ExprResult<()> {
    compile("1 == 2")?;
    compile("1 != 2")?;
    compile("1 < 2")?;
    compile("1 <= 2")?;
    compile("1 > 2")?;
    compile("1 >= 2")?;
    Ok(())
}

#[test]
fn compiles_all_arithmetic_ops() -> crate::ExprResult<()> {
    compile("1 + 2")?;
    compile("1 - 2")?;
    compile("1 * 2")?;
    compile("1 / 2")?;
    Ok(())
}

// --- F64 bytecode tests ---

#[test]
fn compiles_float_literal_to_f64_constant() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("3.14")?;
    let expected_ops = vec![ExprOp::LoadConst(ConstIdx::new(0))];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants.len(), 1);
    let ConstValue::F64(finite) = constants.first().unwrap() else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected ConstValue::F64".into(),
        });
    };
    assert_eq!(finite.get(), 3.14);
    Ok(())
}

#[test]
fn compiles_float_literal_with_leading_zero() -> crate::ExprResult<()> {
    let (_program, constants) = compile_with_pool("0.5")?;
    assert_eq!(constants.len(), 1);
    let ConstValue::F64(finite) = constants.first().unwrap() else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected ConstValue::F64".into(),
        });
    };
    assert_eq!(finite.get(), 0.5);
    Ok(())
}

#[test]
fn constant_folds_float_literal() -> crate::ExprResult<()> {
    let tokens = lex_expr("2.5")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    let Some(ConstValue::F64(finite)) = folded else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected ConstValue::F64 from folding".into(),
        });
    };
    assert_eq!(finite.get(), 2.5);
    Ok(())
}

#[test]
fn compiles_all_helpers() -> crate::ExprResult<()> {
    compile_expr("contains($a, $b)", &resolve_test_reference)?;
    compile_expr("starts_with($a, $b)", &resolve_test_reference)?;
    compile_expr("ends_with($a, $b)", &resolve_test_reference)?;
    compile_expr("has($a, $b)", &resolve_test_reference)?;
    compile_expr("exists($a)", &resolve_test_reference)?;
    compile_expr("length($a)", &resolve_test_reference)?;
    compile_expr("empty($a)", &resolve_test_reference)?;
    compile_expr("append($a, $b)", &resolve_test_reference)?;
    compile_expr("append_if($a, $b, $c)", &resolve_test_reference)?;
    compile_expr("merge($a, $b)", &resolve_test_reference)?;
    compile_expr("sum($a)", &resolve_test_reference)?;
    compile_expr("count($a)", &resolve_test_reference)?;
    compile_expr("unique($a)", &resolve_test_reference)?;
    Ok(())
}

#[test]
fn unresolved_reference_is_typed_error() -> crate::ExprResult<()> {
    let result = compile_expr("$missing + 1", &resolve_test_reference);
    assert!(matches!(
        result,
        Err(crate::ExprError::InvalidReference { reference }) if reference == "$missing"
    ));
    Ok(())
}

#[test]
fn resolver_drives_reference_lowering() -> crate::ExprResult<()> {
    let (program, constants) = compile_expr("$a + 1", &resolve_test_reference)?;
    let expected_ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Add,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants, vec![ConstValue::I64(1)]);
    Ok(())
}

#[test]
fn rejects_text_literals_explicitly() {
    let result = compile_with_pool("\"hello\"");
    assert!(matches!(
        result,
        Err(crate::ExprError::UnsupportedLiteral { literal }) if literal == "text"
    ));
}

// --- BDD bytecode tests ---

#[test]
fn compile_expr_to_bytecode_produces_non_empty_bytecode() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 + 2")?;
    let ast = parse_expr(&tokens)?;
    let program = compile_expr_to_bytecode(&ast)?;
    assert!(
        !program.ops.is_empty(),
        "bytecode should contain at least one op"
    );
    Ok(())
}

#[test]
fn compile_expr_to_bytecode_roundtrips_with_eval() -> crate::ExprResult<()> {
    let tokens = lex_expr("3 + 4 * 2")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_pool(&ast, &mut constants)?;
    let result = crate::eval::eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, vb_core::SlotValue::I64(11));
    Ok(())
}

#[test]
fn f64_literal_roundtrips_through_eval() -> crate::ExprResult<()> {
    let tokens = lex_expr("3.14")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_pool(&ast, &mut constants)?;
    let result = crate::eval::eval_expr_program(&program, &[], &constants)?;
    let vb_core::SlotValue::F64(finite) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected SlotValue::F64".into(),
        });
    };
    assert_eq!(finite.get(), 3.14);
    Ok(())
}

#[test]
fn f64_arithmetic_roundtrips_through_eval() -> crate::ExprResult<()> {
    let tokens = lex_expr("1.5 + 2.5")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_pool(&ast, &mut constants)?;
    let result = crate::eval::eval_expr_program(&program, &[], &constants)?;
    let vb_core::SlotValue::F64(finite) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected SlotValue::F64".into(),
        });
    };
    assert_eq!(finite.get(), 4.0);
    Ok(())
}

#[test]
fn compile_expr_with_pool_uses_constant_pool() -> crate::ExprResult<()> {
    let tokens = lex_expr("10 + 20")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let _program = compile_expr_with_pool(&ast, &mut constants)?;
    assert_eq!(constants.len(), 2);
    assert_eq!(constants.first(), Some(&ConstValue::I64(10)));
    assert_eq!(constants.get(1), Some(&ConstValue::I64(20)));
    Ok(())
}

#[test]
fn compile_expr_with_resolver_resolves_variables() -> crate::ExprResult<()> {
    let (program, constants) = compile_expr("$a + 1", &resolve_test_reference)?;
    assert_eq!(constants, vec![ConstValue::I64(1)]);
    let ops = program.ops.as_ref();
    let first_is_load_slot = ops
        .first()
        .is_some_and(|op| matches!(op, ExprOp::LoadSlot(idx) if idx.get() == 0));
    assert!(first_is_load_slot, "first op should be LoadSlot(0)");
    Ok(())
}

#[test]
fn check_expr_stack_bound_returns_ok_within_limit() -> crate::ExprResult<()> {
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ];
    let max_stack = check_expr_stack_bound(&ops)?;
    assert!(max_stack > 0, "max_stack should be positive");
    Ok(())
}

#[allow(dead_code)]
fn compile(source: &str) -> crate::ExprResult<crate::bytecode::ExprProgram> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    compile_expr_to_bytecode(&ast)
}

#[test]
fn text_literal_in_expression_returns_clear_error() -> crate::ExprResult<()> {
    let tokens = lex_expr(r#""hello""#)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let result = compile_expr_with_pool(&ast, &mut constants);
    let err = result.expect_err("text literal should fail");
    match err {
        crate::ExprError::UnsupportedLiteral { literal } => assert_eq!(literal, "text"),
        other => panic!("expected UnsupportedLiteral error, got {other:?}"),
    }
    Ok(())
}

#[allow(dead_code)]
fn compile_with_pool(
    source: &str,
) -> crate::ExprResult<(crate::bytecode::ExprProgram, Vec<ConstValue>)> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_pool(&ast, &mut constants)?;
    Ok((program, constants))
}
