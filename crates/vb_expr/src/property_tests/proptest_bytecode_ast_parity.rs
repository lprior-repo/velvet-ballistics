#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_macro,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
#![forbid(unsafe_code)]

//! Section 38 property test: `bytecode_ast_parity`.
//!
//! Master plan §38, row "Bytecode/AST parity":
//! "Bytecode evaluation matches AST semantics for any expression".
//!
//! This file asserts the parity between constant folding (the AST-level
//! path) and the bytecode-evaluator path. For any constant expression,
//! both paths must agree: `const_fold_expr(ast)` and
//! `eval_expr_program(compile(ast), &[], constants)` either both yield
//! the same `ConstValue` or both are unavailable (i.e. the AST
//! contains a `Reference` or `Helper` that prevents folding, in
//! which case the bytecode program is the canonical answer and
//! folding must report `None`).

use crate::bytecode::{compile_expr_to_bytecode, const_fold_expr};
use crate::eval::eval_expr_program;
use crate::lexer::{BinaryOp, UnaryOp};
use crate::parser::{ExprAst, ExprHelper, ExprLiteral};
use vb_core::ConstValue;
use vb_core::value::FiniteF64;

use proptest::prelude::*;

/// Build an AST for a constant binary-i64 expression. Folds when the
/// operation does not overflow.
fn arb_i64_binary_expr() -> impl Strategy<Value = ExprAst> {
    (
        any::<i64>(),
        any::<i64>(),
        prop_oneof![
            Just(BinaryOp::Add),
            Just(BinaryOp::Sub),
            Just(BinaryOp::Mul),
            Just(BinaryOp::Div),
        ],
    )
        .prop_map(|(a, b, op)| ExprAst::Binary {
            op,
            left: Box::new(ExprAst::Literal(ExprLiteral::I64(a))),
            right: Box::new(ExprAst::Literal(ExprLiteral::I64(b))),
        })
}

/// Build an AST for a constant unary expression on an i64.
fn arb_i64_unary_expr() -> impl Strategy<Value = ExprAst> {
    (any::<i64>(), prop_oneof![Just(UnaryOp::Neg), Just(UnaryOp::Not)]).prop_map(|(v, op)| {
        ExprAst::Unary {
            op,
            expr: Box::new(ExprAst::Literal(ExprLiteral::I64(v))),
        }
    })
}

/// Build an AST for a constant comparison expression.
fn arb_i64_comparison_expr() -> impl Strategy<Value = ExprAst> {
    (
        any::<i64>(),
        any::<i64>(),
        prop_oneof![
            Just(BinaryOp::Eq),
            Just(BinaryOp::NotEq),
            Just(BinaryOp::Lt),
            Just(BinaryOp::Lte),
            Just(BinaryOp::Gt),
            Just(BinaryOp::Gte),
        ],
    )
        .prop_map(|(a, b, op)| ExprAst::Binary {
            op,
            left: Box::new(ExprAst::Literal(ExprLiteral::I64(a))),
            right: Box::new(ExprAst::Literal(ExprLiteral::I64(b))),
        })
}

/// Build an AST for a non-constant expression. Folding must yield `None`.
fn arb_reference_expr() -> impl Strategy<Value = ExprAst> {
    "[a-z][a-z0-9_]{0,8}".prop_map(|name| ExprAst::Reference(format!("${name}").into()))
}

proptest! {
    /// For any constant `I64` literal, the AST fold and the bytecode
    /// evaluation yield the same `ConstValue::I64(v)`.
    #[test]
    fn bap_i64_literal_parity(val in any::<i64>()) {
        let ast = ExprAst::Literal(ExprLiteral::I64(val));
        let folded = const_fold_expr(&ast);
        let program = compile_expr_to_bytecode(&ast).expect("i64 literal compiles");
        let evaluated = eval_expr_program(&program, &[], &[]);
        match (folded, evaluated) {
            (Some(ConstValue::I64(f)), Ok(vb_core::SlotValue::I64(e))) => {
                prop_assert_eq!(f, e, "fold and eval must agree for I64");
            }
            (other_f, other_e) => {
                prop_assert!(false, "fold/eval parity broken: fold={other_f:?} eval={other_e:?}");
            }
        }
    }

    /// For any constant `Bool` literal, the AST fold and the bytecode
    /// evaluation yield the same `ConstValue::Bool(v)`.
    #[test]
    fn bap_bool_literal_parity(val in any::<bool>()) {
        let ast = ExprAst::Literal(ExprLiteral::Bool(val));
        let folded = const_fold_expr(&ast);
        let program = compile_expr_to_bytecode(&ast).expect("bool literal compiles");
        let evaluated = eval_expr_program(&program, &[], &[]);
        match (folded, evaluated) {
            (Some(ConstValue::Bool(f)), Ok(vb_core::SlotValue::Bool(e))) => {
                prop_assert_eq!(f, e, "fold and eval must agree for Bool");
            }
            (other_f, other_e) => {
                prop_assert!(false, "fold/eval parity broken: fold={other_f:?} eval={other_e:?}");
            }
        }
    }

    /// For any i64 binary expression that does NOT overflow, the AST
    /// fold and the bytecode evaluation yield the same numeric result.
    /// For overflow cases, the fold returns `None` and the bytecode
    /// evaluator returns an error — both must agree on the
    /// "unavailable" outcome.
    #[test]
    fn bap_i64_binary_arithmetic_parity(ast in arb_i64_binary_expr()) {
        let folded = const_fold_expr(&ast);
        let program = match compile_expr_to_bytecode(&ast) {
            Ok(p) => p,
            Err(_) => {
                // No bytecode: parity is trivially maintained.
                prop_assert!(folded.is_none());
                return Ok(());
            }
        };
        let evaluated = eval_expr_program(&program, &[], &[]);
        match (folded, evaluated) {
            (Some(ConstValue::I64(f)), Ok(vb_core::SlotValue::I64(e))) => {
                prop_assert_eq!(f, e, "fold and eval must agree for binary i64");
            }
            (None, Err(_)) => {
                // Both agree that the operation is unavailable.
            }
            (other_f, other_e) => {
                prop_assert!(false, "fold/eval parity broken: fold={other_f:?} eval={other_e:?}");
            }
        }
    }

    /// For any i64 comparison expression, the AST fold yields a
    /// `ConstValue::Bool` and the bytecode evaluator returns a
    /// `SlotValue::Bool`. They must agree exactly.
    #[test]
    fn bap_i64_comparison_parity(ast in arb_i64_comparison_expr()) {
        let folded = const_fold_expr(&ast);
        let program = compile_expr_to_bytecode(&ast).expect("comparison compiles");
        let evaluated = eval_expr_program(&program, &[], &[]);
        match (folded, evaluated) {
            (Some(ConstValue::Bool(f)), Ok(vb_core::SlotValue::Bool(e))) => {
                prop_assert_eq!(f, e, "fold and eval must agree for comparison");
            }
            (other_f, other_e) => {
                prop_assert!(false, "fold/eval parity broken: fold={other_f:?} eval={other_e:?}");
            }
        }
    }

    /// For any i64 unary expression, the fold and the eval agree. The
    /// negation of `i64::MIN` overflows, so both paths must agree on
    /// "unavailable" for that case.
    #[test]
    fn bap_i64_unary_parity(ast in arb_i64_unary_expr()) {
        let folded = const_fold_expr(&ast);
        let program = match compile_expr_to_bytecode(&ast) {
            Ok(p) => p,
            Err(_) => {
                prop_assert!(folded.is_none());
                return Ok(());
            }
        };
        let evaluated = eval_expr_program(&program, &[], &[]);
        match (folded, evaluated) {
            (Some(ConstValue::I64(f)), Ok(vb_core::SlotValue::I64(e))) => {
                prop_assert_eq!(f, e, "fold and eval must agree for unary i64");
            }
            (Some(ConstValue::Bool(f)), Ok(vb_core::SlotValue::Bool(e))) => {
                prop_assert_eq!(f, e, "fold and eval must agree for unary Not");
            }
            (None, Err(_)) => {
                // Both agree the operation is unavailable.
            }
            (other_f, other_e) => {
                prop_assert!(false, "fold/eval parity broken: fold={other_f:?} eval={other_e:?}");
            }
        }
    }

    /// For any reference expression, the AST fold returns `None`
    /// (folding does not apply to references). The bytecode, when
    /// evaluated against an empty slot array, must error with a
    /// stack underflow or similar — never silently succeed.
    #[test]
    fn bap_reference_does_not_fold(ast in arb_reference_expr()) {
        let folded = const_fold_expr(&ast);
        prop_assert!(folded.is_none(), "reference must not fold");
        let program = compile_expr_to_bytecode(&ast).expect("reference compiles");
        let result = eval_expr_program(&program, &[], &[]);
        prop_assert!(result.is_err(), "reference evaluated with empty slots must error");
    }

    /// For any constant `Null` literal, the bytecode evaluator yields
    /// `SlotValue::Null` and the fold yields `ConstValue::Null`. They
    /// must agree.
    #[test]
    fn bap_null_literal_parity(_unit in 0u8..1u8) {
        let ast = ExprAst::Literal(ExprLiteral::Null);
        let folded = const_fold_expr(&ast);
        let program = compile_expr_to_bytecode(&ast).expect("null literal compiles");
        let evaluated = eval_expr_program(&program, &[], &[]);
        match (folded, evaluated) {
            (Some(ConstValue::Null), Ok(vb_core::SlotValue::Null)) => {}
            (other_f, other_e) => {
                prop_assert!(false, "fold/eval parity broken: fold={other_f:?} eval={other_e:?}");
            }
        }
    }

    /// For any constant `F64` literal in a bounded range, the AST fold
    /// and the bytecode evaluation yield the same `F64` value. We use
    /// a bounded f64 range (not arbitrary bits) because `FiniteF64`
    /// rejects NaN/+/-Inf and we cannot lift them into a literal.
    #[test]
    fn bap_f64_literal_parity(val in -1_000_000.0_f64..1_000_000.0_f64) {
        let f = match FiniteF64::new(val) {
            Ok(f) => f,
            Err(_) => {
                // Saturated at the edges; not all values in the
                // range are representable as FiniteF64.
                return Ok(());
            }
        };
        let ast = ExprAst::Literal(ExprLiteral::F64(f));
        let folded = const_fold_expr(&ast);
        let program = compile_expr_to_bytecode(&ast).expect("f64 literal compiles");
        let evaluated = eval_expr_program(&program, &[], &[]);
        match (folded, evaluated) {
            (Some(ConstValue::F64(f_folded)), Ok(vb_core::SlotValue::F64(f_eval))) => {
                let f1: f64 = f_folded.get();
                let f2: f64 = f_eval.get();
                prop_assert_eq!(f1.to_bits(), f2.to_bits(), "fold and eval must agree for F64");
            }
            (other_f, other_e) => {
                prop_assert!(false, "fold/eval parity broken: fold={other_f:?} eval={other_e:?}");
            }
        }
    }

    /// For any helper expression (e.g. `length`, `count`), the AST
    /// fold returns `None` (folding does not apply to helpers), and
    /// the bytecode evaluator cannot complete the helper call against
    /// an empty slot array.
    #[test]
    fn bap_helper_does_not_fold(name in 0u8..6u8) {
        let helper = match name {
            0 => ExprHelper::Length,
            1 => ExprHelper::Empty,
            2 => ExprHelper::Exists,
            3 => ExprHelper::Count,
            4 => ExprHelper::Sum,
            _ => ExprHelper::Unique,
        };
        let ast = ExprAst::Helper {
            name: helper,
            args: Box::new([ExprAst::Literal(ExprLiteral::I64(1))]),
        };
        let folded = const_fold_expr(&ast);
        prop_assert!(folded.is_none(), "helper must not fold");
        let program = compile_expr_to_bytecode(&ast).expect("helper compiles");
        let result = eval_expr_program(&program, &[], &[]);
        prop_assert!(result.is_err(), "helper evaluated with empty args must error");
    }

    /// The bytecode compiler is deterministic: compiling the same
    /// AST twice yields byte-equal `ExprProgram` values (same op
    /// sequence, same max_stack).
    #[test]
    fn bap_compile_is_deterministic(ast in arb_i64_binary_expr()) {
        let p1 = compile_expr_to_bytecode(&ast).expect("compiles");
        let p2 = compile_expr_to_bytecode(&ast).expect("compiles");
        prop_assert_eq!(p1.ops, p2.ops);
        prop_assert_eq!(p1.max_stack, p2.max_stack);
    }

    /// For any constant boolean AND/OR expression, the fold yields a
    /// `ConstValue::Bool` and the bytecode evaluator returns a
    /// `SlotValue::Bool`. They must agree.
    #[test]
    fn bap_boolean_short_circuit_parity(
        a in any::<bool>(),
        b in any::<bool>(),
        is_and in any::<bool>(),
    ) {
        let op = if is_and { BinaryOp::And } else { BinaryOp::Or };
        let ast = ExprAst::Binary {
            op,
            left: Box::new(ExprAst::Literal(ExprLiteral::Bool(a))),
            right: Box::new(ExprAst::Literal(ExprLiteral::Bool(b))),
        };
        let folded = const_fold_expr(&ast);
        let program = compile_expr_to_bytecode(&ast).expect("boolean binary compiles");
        let evaluated = eval_expr_program(&program, &[], &[]);
        match (folded, evaluated) {
            (Some(ConstValue::Bool(f)), Ok(vb_core::SlotValue::Bool(e))) => {
                prop_assert_eq!(f, e, "fold and eval must agree for boolean short-circuit");
            }
            (other_f, other_e) => {
                prop_assert!(false, "fold/eval parity broken: fold={other_f:?} eval={other_e:?}");
            }
        }
    }

    /// Nested constant arithmetic: for any (a, b, c) of i64, the
    /// folded `(a + b) * c` and the bytecode-evaluated
    /// `(a + b) * c` must agree (or both report overflow).
    #[test]
    fn bap_nested_constant_arithmetic_parity(
        a in any::<i64>(),
        b in any::<i64>(),
        c in any::<i64>(),
    ) {
        let ast = ExprAst::Binary {
            op: BinaryOp::Mul,
            left: Box::new(ExprAst::Binary {
                op: BinaryOp::Add,
                left: Box::new(ExprAst::Literal(ExprLiteral::I64(a))),
                right: Box::new(ExprAst::Literal(ExprLiteral::I64(b))),
            }),
            right: Box::new(ExprAst::Literal(ExprLiteral::I64(c))),
        };
        let folded = const_fold_expr(&ast);
        let program = compile_expr_to_bytecode(&ast).expect("nested compiles");
        let evaluated = eval_expr_program(&program, &[], &[]);
        match (folded, evaluated) {
            (Some(ConstValue::I64(f)), Ok(vb_core::SlotValue::I64(e))) => {
                prop_assert_eq!(f, e, "fold and eval must agree for nested");
            }
            (None, Err(_)) => {
                // Both agree the operation is unavailable.
            }
            (other_f, other_e) => {
                prop_assert!(false, "fold/eval parity broken: fold={other_f:?} eval={other_e:?}");
            }
        }
    }
}
