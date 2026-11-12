#![forbid(unsafe_code)]
//! Differential test for VB-EXPR-001: bytecode evaluation against a
//! tree-walking AST evaluator over the same expression, slots, and store.
//!
//! The AST evaluator exists only in this module. It shares the leaf
//! semantics with the bytecode path (`eval_binary_op`, `eval_unary_op`,
//! `eval_helper_with_store`) and differs only in traversal: tree walk
//! against the compiler's postfix stack machine, so agreement pins the
//! compiler's lowering. Each path builds its own fixture store with the
//! same insertion order, so store-allocated ids agree. Recursion depth
//! is bounded by the parser's 64-level limit for parsed sources and by
//! construction (at most 3) for generated ASTs.

use crate::bytecode::compile_expr_with_resolver;
use crate::eval::{
    eval_binary_op, eval_expr_program_with_store, eval_helper_with_store, eval_unary_op,
};
use crate::lexer::{BinaryOp, UnaryOp, lex_expr};
use crate::parser::{ExprAst, ExprLiteral, parse_expr};
use crate::{ExprError, ExprResult};
use proptest::prelude::*;
use vb_core::value::FiniteF64;
use vb_core::value_store::{ObjectField, ValueStore};
use vb_core::{SlotIdx, SlotValue};

fn resolve_reference(reference: &str) -> Option<SlotIdx> {
    match reference {
        "$a" => Some(SlotIdx::new(0)),
        "$b" => Some(SlotIdx::new(1)),
        "$c" => Some(SlotIdx::new(2)),
        "$x" => Some(SlotIdx::new(3)),
        "$d" => Some(SlotIdx::new(4)),
        "$e" => Some(SlotIdx::new(5)),
        "$f" => Some(SlotIdx::new(6)),
        "$g" => Some(SlotIdx::new(7)),
        "$h" => Some(SlotIdx::new(8)),
        "$i" => Some(SlotIdx::new(9)),
        "$j" => Some(SlotIdx::new(10)),
        "$u" => Some(SlotIdx::new(11)),
        _ => None,
    }
}

fn eval_ast(
    ast: &ExprAst,
    slots: &[Option<SlotValue>],
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    match ast {
        ExprAst::Literal(lit) => literal_value(lit),
        ExprAst::Reference(name) => reference_value(name, slots),
        ExprAst::Unary { op, expr } => {
            let value = eval_ast(expr, slots, store)?;
            eval_unary_op(*op, value)
        }
        ExprAst::Binary { op, left, right } => {
            let left_value = eval_ast(left, slots, store)?;
            let right_value = eval_ast(right, slots, store)?;
            eval_binary_op(*op, left_value, right_value)
        }
        ExprAst::Helper { name, args } => {
            let values = args
                .iter()
                .map(|arg| eval_ast(arg, slots, store))
                .collect::<ExprResult<Vec<_>>>()?;
            eval_helper_with_store(*name, &values, store)
        }
    }
}

fn literal_value(lit: &ExprLiteral) -> ExprResult<SlotValue> {
    match lit {
        ExprLiteral::Null => Ok(SlotValue::Null),
        ExprLiteral::Bool(value) => Ok(SlotValue::Bool(*value)),
        ExprLiteral::I64(value) => Ok(SlotValue::I64(*value)),
        ExprLiteral::F64(value) => Ok(SlotValue::F64(*value)),
        ExprLiteral::Text(_) => Err(ExprError::UnsupportedLiteral {
            literal: "text".into(),
        }),
    }
}

fn reference_value(name: &str, slots: &[Option<SlotValue>]) -> ExprResult<SlotValue> {
    let slot = resolve_reference(name).ok_or(ExprError::InvalidReference {
        reference: name.into(),
    })?;
    slots
        .get(slot.as_usize())
        .and_then(|value| *value)
        .ok_or(ExprError::StackUnderflow)
}

fn eval_bytecode(
    ast: &ExprAst,
    slots: &[Option<SlotValue>],
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let mut constants = Vec::new();
    let program = compile_expr_with_resolver(ast, &mut constants, &resolve_reference)?;
    eval_expr_program_with_store(&program, slots, &constants, store)
}

fn fixture_slots(store: &mut ValueStore) -> ExprResult<Vec<Option<SlotValue>>> {
    let hello = store.insert_symbol("hello")?;
    let he = store.insert_symbol("he")?;
    let lo = store.insert_symbol("lo")?;
    let key_a = store.insert_symbol("a")?;
    let list123 = store.insert_list(vec![
        SlotValue::I64(1),
        SlotValue::I64(2),
        SlotValue::I64(3),
    ])?;
    let list112 = store.insert_list(vec![
        SlotValue::I64(1),
        SlotValue::I64(1),
        SlotValue::I64(2),
    ])?;
    let object = store.insert_object(vec![ObjectField::clean(key_a, SlotValue::I64(1))])?;
    Ok(vec![
        Some(SlotValue::I64(7)),
        Some(SlotValue::Bool(true)),
        Some(SlotValue::I64(3)),
        Some(SlotValue::F64(FiniteF64::new(2.5)?)),
        Some(SlotValue::List(list123)),
        Some(SlotValue::List(list112)),
        Some(SlotValue::Object(object)),
        Some(SlotValue::Symbol(hello)),
        Some(SlotValue::Symbol(he)),
        Some(SlotValue::Symbol(lo)),
        Some(SlotValue::Symbol(key_a)),
        None,
    ])
}

fn scalar_slots() -> Vec<Option<SlotValue>> {
    vec![
        Some(SlotValue::I64(7)),
        Some(SlotValue::Bool(true)),
        Some(SlotValue::I64(3)),
    ]
}

fn evaluate_both(source: &str) -> ExprResult<(ExprResult<SlotValue>, ExprResult<SlotValue>)> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    let mut ast_store = ValueStore::new();
    let ast_slots = fixture_slots(&mut ast_store)?;
    let ast_result = eval_ast(&ast, &ast_slots, &mut ast_store);
    let mut bytecode_store = ValueStore::new();
    let bytecode_slots = fixture_slots(&mut bytecode_store)?;
    let bytecode_result = eval_bytecode(&ast, &bytecode_slots, &mut bytecode_store);
    Ok((ast_result, bytecode_result))
}

fn assert_source_equiv(source: &str) -> ExprResult<()> {
    let (ast_result, bytecode_result) = evaluate_both(source)?;
    assert_eq!(ast_result, bytecode_result, "source: {source}");
    Ok(())
}

fn assert_source_value(source: &str, expected: SlotValue) -> ExprResult<()> {
    let (ast_result, bytecode_result) = evaluate_both(source)?;
    assert_eq!(ast_result, bytecode_result, "source: {source}");
    assert_eq!(ast_result, Ok(expected), "source: {source}");
    Ok(())
}

#[test]
fn bytecode_vs_ast() -> ExprResult<()> {
    for source in [
        "1 + 2 * 3",
        "(1 + 2) * 3",
        "10 - 4 - 3",
        "2 * 3 + 4 * 5",
        "10 / 3",
        "10 - 2 * 3",
        "-5",
        "-(3 + 4)",
        "not true",
        "not false",
        "not (1 == 2)",
        "1 < 2",
        "2 <= 2",
        "3 > 4",
        "3 >= 3",
        "1 == 1",
        "1 != 2",
        "true and false",
        "true or false",
        "true and true",
        "false or false",
        "1.5 + 2.5",
        "1.5 * 2.0",
        "7 / 2.0",
        "1.5 < 2.5",
        "2.0 == 2.0",
        "null",
        "exists(null)",
        "exists(1)",
        "length(null)",
        "empty(null)",
        "$a + $c",
        "$a * $c + 1",
        "$b and true",
        "$x * 2.0",
        "$a == 7",
        "$c < $a",
        "exists($a)",
        "exists($u)",
        "$u + 1",
        "$zz + 1",
        "\"hello\"",
        "1 / 0",
        "1 + true",
        "true + 1",
        "1 and true",
        "not 1",
        "-true",
        "9223372036854775807 + 1",
        "-(-9223372036854775807 - 1)",
        "1.0 / 0.0",
        "0.0 / 0.0",
        "1 == true",
        "null == null",
        "null == 1",
        "length($d)",
        "count($d)",
        "empty($d)",
        "empty($e)",
        "length($e)",
        "length($f)",
        "sum($d)",
        "sum($e)",
        "contains($g, $h)",
        "contains($g, $i)",
        "contains($g, $g)",
        "starts_with($g, $h)",
        "ends_with($g, $i)",
        "starts_with($g, $i)",
        "has($f, $j)",
        "has($f, $h)",
        "append($d, 4)",
        "append_if($d, 4, true)",
        "append_if($d, 4, false)",
        "unique($e)",
        "unique($d)",
        "merge($f, $f)",
        "exists($d)",
        "exists($f)",
        "exists($g)",
        "count($e)",
    ] {
        assert_source_equiv(source)?;
    }
    Ok(())
}

#[test]
fn bytecode_vs_ast_anchored_values() -> ExprResult<()> {
    assert_source_value("3 + 4 * 2", SlotValue::I64(11))?;
    assert_source_value("(3 + 4) * 2", SlotValue::I64(14))?;
    assert_source_value("$a + $c", SlotValue::I64(10))?;
    assert_source_value("length($d)", SlotValue::I64(3))?;
    assert_source_value("sum($d)", SlotValue::I64(6))?;
    assert_source_value("contains($d, $c)", SlotValue::Bool(true))?;
    assert_source_value("has($f, $j)", SlotValue::Bool(true))?;
    assert_source_value("starts_with($g, $h)", SlotValue::Bool(true))?;
    assert_source_value("ends_with($g, $i)", SlotValue::Bool(true))?;
    assert_source_value("empty($d)", SlotValue::Bool(false))?;
    assert_source_value("exists(null)", SlotValue::Bool(false))?;
    assert_source_value("1.5 + 2.5", SlotValue::F64(FiniteF64::new(4.0)?))?;
    Ok(())
}

include!("bytecode_ast_equiv_contains_tests.rs");

#[test]
fn bytecode_vs_ast_error_shapes() -> ExprResult<()> {
    let (ast, bytecode) = evaluate_both("1 / 0")?;
    assert_eq!(ast, bytecode);
    assert!(matches!(ast, Err(ExprError::DivisionByZero)));
    let (ast, bytecode) = evaluate_both("9223372036854775807 + 1")?;
    assert_eq!(ast, bytecode);
    assert!(matches!(ast, Err(ExprError::IntegerOverflow)));
    let (ast, bytecode) = evaluate_both("1 + true")?;
    assert_eq!(ast, bytecode);
    assert!(matches!(ast, Err(ExprError::TypeMismatch { .. })));
    let (ast, bytecode) = evaluate_both("$zz + 1")?;
    assert_eq!(ast, bytecode);
    assert!(matches!(ast, Err(ExprError::InvalidReference { .. })));
    let (ast, bytecode) = evaluate_both("\"hello\"")?;
    assert_eq!(ast, bytecode);
    assert!(matches!(ast, Err(ExprError::UnsupportedLiteral { .. })));
    let (ast, bytecode) = evaluate_both("$u + 1")?;
    assert_eq!(ast, bytecode);
    assert!(matches!(ast, Err(ExprError::StackUnderflow)));
    let (ast, bytecode) = evaluate_both("1.0 / 0.0")?;
    assert_eq!(ast, bytecode);
    assert!(matches!(ast, Err(ExprError::NonFiniteFloat)));
    Ok(())
}

fn scalar_leaf() -> impl Strategy<Value = ExprAst> {
    prop_oneof![
        (0i64..=20).prop_map(|value| ExprAst::Literal(ExprLiteral::I64(value))),
        (-5i64..=5).prop_map(|value| ExprAst::Literal(ExprLiteral::I64(value))),
        any::<bool>().prop_map(|value| ExprAst::Literal(ExprLiteral::Bool(value))),
        Just(ExprAst::Literal(ExprLiteral::Null)),
        Just(ExprAst::Reference("$a".into())),
        Just(ExprAst::Reference("$b".into())),
    ]
}

fn binary_op_strategy() -> impl Strategy<Value = BinaryOp> {
    prop_oneof![
        Just(BinaryOp::Add),
        Just(BinaryOp::Sub),
        Just(BinaryOp::Mul),
        Just(BinaryOp::Div),
        Just(BinaryOp::Eq),
        Just(BinaryOp::NotEq),
        Just(BinaryOp::Lt),
        Just(BinaryOp::Lte),
        Just(BinaryOp::Gt),
        Just(BinaryOp::Gte),
        Just(BinaryOp::And),
        Just(BinaryOp::Or),
    ]
}

fn scalar_ast(depth: u32) -> BoxedStrategy<ExprAst> {
    if depth == 0 {
        return scalar_leaf().boxed();
    }
    prop_oneof![
        scalar_leaf(),
        scalar_ast(depth - 1).prop_map(|expr| ExprAst::Unary {
            op: UnaryOp::Neg,
            expr: Box::new(expr),
        }),
        scalar_ast(depth - 1).prop_map(|expr| ExprAst::Unary {
            op: UnaryOp::Not,
            expr: Box::new(expr),
        }),
        (
            scalar_ast(depth - 1),
            scalar_ast(depth - 1),
            binary_op_strategy(),
        )
            .prop_map(|(left, right, op)| ExprAst::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            }),
    ]
    .boxed()
}

proptest! {
    #[test]
    fn bytecode_vs_ast_proptest(ast in scalar_ast(3)) {
        let slots = scalar_slots();
        let mut ast_store = ValueStore::new();
        let ast_result = eval_ast(&ast, &slots, &mut ast_store);
        let mut bytecode_store = ValueStore::new();
        let bytecode_result = eval_bytecode(&ast, &slots, &mut bytecode_store);
        prop_assert_eq!(ast_result, bytecode_result);
    }
}
