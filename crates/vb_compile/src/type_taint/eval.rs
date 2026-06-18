#![forbid(unsafe_code)]
//! Expression-level type-and-taint evaluation.
//!
//! Walks `AstExpression` / `ParsedExpression` trees, resolving references into
//! the fact engine and propagating `ValueType` + `Taint` through operators.

use crate::CompileError;
use crate::ast::{AstExpression, AstMapEntry, AstValue};
use crate::expression::{BinaryOp, ExpressionHelper, ExpressionLiteral, ParsedExpression, UnaryOp};

use super::engine::Facts;
use super::types::{ValueFact, ValueType};

// ── Top-level expression resolution ─────────────────────────────────────────

pub(crate) fn expression_fact(
    expression: &AstExpression,
    facts: &Facts<'_>,
    field: &'static str,
) -> Result<ValueFact, CompileError> {
    match expression {
        AstExpression::Slot(slot) => facts.read_slot(slot.as_usize(), field),
        AstExpression::Reference(reference) => Ok(reference_fact(reference, Some(facts))),
        AstExpression::Parsed(expression) => parsed_expression_fact(expression, facts, field),
        AstExpression::Literal(value) => Ok(value_fact(value, Some(facts))),
    }
}

fn parsed_expression_fact(
    expression: &ParsedExpression,
    facts: &Facts<'_>,
    field: &'static str,
) -> Result<ValueFact, CompileError> {
    match expression {
        ParsedExpression::Literal(value) => Ok(expression_literal_fact(value)),
        ParsedExpression::Reference(reference) => Ok(reference_fact(reference, Some(facts))),
        ParsedExpression::Unary { op, expr } => unary_fact(*op, expr, facts, field),
        ParsedExpression::Binary { op, left, right } => binary_fact(*op, left, right, facts, field),
        ParsedExpression::HelperCall { name, args } => helper_fact(*name, args, facts, field),
    }
}

// ── Literal & AST-value helpers ─────────────────────────────────────────────

fn expression_literal_fact(value: &ExpressionLiteral) -> ValueFact {
    match value {
        ExpressionLiteral::Null => ValueFact::clean(ValueType::Null),
        ExpressionLiteral::Bool(_) => ValueFact::clean(ValueType::Boolean),
        ExpressionLiteral::I64(_) => ValueFact::clean(ValueType::Number),
        ExpressionLiteral::F64(_) => ValueFact::clean(ValueType::Number),
        ExpressionLiteral::Text(_) => ValueFact::clean(ValueType::Text),
    }
}

pub(crate) fn value_fact(value: &AstValue, facts: Option<&Facts<'_>>) -> ValueFact {
    match value {
        AstValue::Null => ValueFact::clean(ValueType::Null),
        AstValue::Bool(_) => ValueFact::clean(ValueType::Boolean),
        AstValue::I64(_) => ValueFact::clean(ValueType::Number),
        AstValue::Text(_) => ValueFact::clean(ValueType::Text),
        AstValue::Reference(reference) => reference_fact(reference, facts),
        AstValue::Sequence(values) => sequence_fact(values, facts),
        AstValue::Mapping(entries) => optional_object_fact(entries, facts),
    }
}

fn sequence_fact(values: &[AstValue], facts: Option<&Facts<'_>>) -> ValueFact {
    let mut fact = ValueFact::clean(ValueType::List);
    for value in values {
        fact = fact.merge(value_fact(value, facts));
    }
    fact
}

pub(crate) fn optional_object_fact(
    entries: &[AstMapEntry<AstValue>],
    facts: Option<&Facts<'_>>,
) -> ValueFact {
    let mut fact = ValueFact::clean(ValueType::Object);
    for entry in entries {
        fact = fact.merge(value_fact(&entry.value, facts));
    }
    fact
}

// ── Reference resolution ────────────────────────────────────────────────────

pub(crate) fn reference_fact(reference: &str, facts: Option<&Facts<'_>>) -> ValueFact {
    let Some(facts) = facts else {
        return ValueFact::clean(ValueType::Any);
    };
    let Some(body) = reference.strip_prefix('$') else {
        return ValueFact::clean(ValueType::Text);
    };
    let Some((root, tail)) = body.split_once('.') else {
        return ValueFact::clean(ValueType::Any);
    };
    let name = match tail.split_once('.') {
        Some((name, _)) => name,
        None => tail,
    };
    let table = match root {
        "input" => Some(&facts.inputs),
        "var" | "vars" => Some(&facts.vars),
        "secrets" => Some(&facts.secrets),
        _ => None,
    };
    match table.and_then(|values| values.get(name)) {
        Some(fact) => *fact,
        None => ValueFact::clean(ValueType::Any),
    }
}

// ── Unary / binary operators ────────────────────────────────────────────────

fn unary_fact(
    op: UnaryOp,
    expr: &ParsedExpression,
    facts: &Facts<'_>,
    field: &'static str,
) -> Result<ValueFact, CompileError> {
    let inner = parsed_expression_fact(expr, facts, field)?;
    match op {
        UnaryOp::Not => typed_unary_fact(inner, ValueType::Boolean, field),
        UnaryOp::Neg => typed_unary_fact(inner, ValueType::Number, field),
    }
}

pub(crate) fn typed_unary_fact(
    fact: ValueFact,
    expected: ValueType,
    field: &'static str,
) -> Result<ValueFact, CompileError> {
    if matches_type(fact.value_type, expected) {
        Ok(ValueFact {
            value_type: expected,
            taint: fact.taint,
        })
    } else {
        Err(type_mismatch(field, expected, fact.value_type))
    }
}

fn binary_fact(
    op: BinaryOp,
    left: &ParsedExpression,
    right: &ParsedExpression,
    facts: &Facts<'_>,
    field: &'static str,
) -> Result<ValueFact, CompileError> {
    let left = parsed_expression_fact(left, facts, field)?;
    let right = parsed_expression_fact(right, facts, field)?;
    match op {
        BinaryOp::Or | BinaryOp::And => typed_binary_fact(left, right, ValueType::Boolean, field),
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
            typed_binary_fact(left, right, ValueType::Number, field)
        }
        _ => Ok(ValueFact {
            value_type: ValueType::Boolean,
            taint: left.merge(right).taint,
        }),
    }
}

fn typed_binary_fact(
    left: ValueFact,
    right: ValueFact,
    expected: ValueType,
    field: &'static str,
) -> Result<ValueFact, CompileError> {
    if matches_type(left.value_type, expected) && matches_type(right.value_type, expected) {
        Ok(ValueFact {
            value_type: expected,
            taint: left.merge(right).taint,
        })
    } else {
        Err(type_mismatch(
            field,
            expected,
            first_mismatch(left, right, expected),
        ))
    }
}

fn first_mismatch(left: ValueFact, right: ValueFact, expected: ValueType) -> ValueType {
    if matches_type(left.value_type, expected) {
        right.value_type
    } else {
        left.value_type
    }
}

// ── Helper functions ────────────────────────────────────────────────────────

fn helper_fact(
    name: ExpressionHelper,
    args: &[ParsedExpression],
    facts: &Facts<'_>,
    field: &'static str,
) -> Result<ValueFact, CompileError> {
    let taint = helper_taint(args, facts, field)?;
    Ok(ValueFact {
        value_type: helper_type(name),
        taint,
    })
}

fn helper_taint(
    args: &[ParsedExpression],
    facts: &Facts<'_>,
    field: &'static str,
) -> Result<vb_validate::type_taint::Taint, CompileError> {
    let mut taint = vb_validate::type_taint::Taint::Clean;
    for arg in args {
        taint = taint.merge(parsed_expression_fact(arg, facts, field)?.taint);
    }
    Ok(taint)
}

fn helper_type(name: ExpressionHelper) -> ValueType {
    match name {
        ExpressionHelper::Contains
        | ExpressionHelper::StartsWith
        | ExpressionHelper::EndsWith
        | ExpressionHelper::Has
        | ExpressionHelper::Exists
        | ExpressionHelper::Empty => ValueType::Boolean,
        ExpressionHelper::Length | ExpressionHelper::Sum | ExpressionHelper::Count => {
            ValueType::Number
        }
        ExpressionHelper::Append | ExpressionHelper::AppendIf | ExpressionHelper::Unique => {
            ValueType::List
        }
        ExpressionHelper::Merge => ValueType::Object,
        ExpressionHelper::Coalesce => ValueType::Any,
    }
}

// ── Shared type-matching helpers ────────────────────────────────────────────

pub(crate) fn matches_type(actual: ValueType, expected: ValueType) -> bool {
    matches!(actual, ValueType::Any) || actual == expected
}

pub(crate) fn type_mismatch(
    field: &'static str,
    expected: ValueType,
    found: ValueType,
) -> CompileError {
    CompileError::TypeMismatch {
        field,
        expected: expected.as_str(),
        found: found.as_str(),
    }
}
