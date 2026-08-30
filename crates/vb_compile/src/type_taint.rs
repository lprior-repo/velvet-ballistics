#![forbid(unsafe_code)]
use crate::ast::{AstExpression, AstMapEntry, AstValue, StepKindAst, WorkflowAst};
use crate::expression::{BinaryOp, ExpressionHelper, ExpressionLiteral, ParsedExpression, UnaryOp};
use crate::{CompileError, CompileErrors};
use std::collections::HashMap;
use vb_storage::vb_validate::type_taint::Taint;

pub(crate) fn validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileErrors> {
    let mut facts = Facts::new(ast);
    validate_steps(ast, &mut facts)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Null,
    Boolean,
    Number,
    Text,
    Object,
    List,
    Any,
}

impl ValueType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::Text => "text",
            Self::Object => "object",
            Self::List => "list",
            Self::Any => "any",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ValueFact {
    value_type: ValueType,
    taint: Taint,
}

impl ValueFact {
    const fn clean(value_type: ValueType) -> Self {
        Self {
            value_type,
            taint: Taint::Clean,
        }
    }

    const fn merge(self, other: Self) -> Self {
        let taint = match (self.taint, other.taint) {
            (Taint::Secret, _) | (_, Taint::Secret) => Taint::Secret,
            (Taint::DerivedFromSecret, _) | (_, Taint::DerivedFromSecret) => {
                Taint::DerivedFromSecret
            }
            (Taint::Clean, Taint::Clean) => Taint::Clean,
            // SAFETY: Taint is marked #[non_exhaustive]. This arm handles any
            // future variants conservatively as Secret (most restrictive).
            #[allow(unreachable_code)]
            (_, _) => Taint::Secret,
        };
        Self {
            value_type: self.value_type,
            taint,
        }
    }
}

struct Facts<'a> {
    inputs: HashMap<&'a str, ValueFact>,
    vars: HashMap<&'a str, ValueFact>,
    secrets: HashMap<&'a str, ValueFact>,
    slots: Vec<Option<ValueFact>>,
}

impl<'a> Facts<'a> {
    fn new(ast: &'a WorkflowAst) -> Self {
        Self {
            inputs: input_facts(&ast.inputs),
            vars: value_facts(&ast.vars),
            secrets: secret_facts(&ast.secrets),
            slots: vec![None; ast.steps.len()],
        }
    }

    fn write_slot(&mut self, index: usize, fact: ValueFact) {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = Some(fact);
        }
    }

    fn read_slot(&self, index: usize, field: &'static str) -> Result<ValueFact, CompileError> {
        match self.slots.get(index).and_then(|slot| *slot) {
            Some(fact) => Ok(fact),
            None => Err(CompileError::UnknownSlotType { field, slot: index }),
        }
    }
}

fn input_facts(entries: &[AstMapEntry<AstValue>]) -> HashMap<&str, ValueFact> {
    let mut facts = HashMap::with_capacity(entries.len());
    for entry in entries {
        match facts.entry(entry.name.as_ref()) {
            std::collections::hash_map::Entry::Occupied(mut fact) => {
                fact.insert(input_schema_fact(&entry.value));
            }
            std::collections::hash_map::Entry::Vacant(fact) => {
                fact.insert(input_schema_fact(&entry.value));
            }
        }
    }
    facts
}

fn input_schema_fact(value: &AstValue) -> ValueFact {
    match value {
        AstValue::Text(name) => ValueFact::clean(schema_type(name)),
        AstValue::Mapping(entries) => schema_mapping_fact(entries),
        _ => ValueFact::clean(ValueType::Any),
    }
}

fn schema_mapping_fact(entries: &[AstMapEntry<AstValue>]) -> ValueFact {
    let mut value_type = ValueType::Any;
    let mut taint = Taint::Clean;
    for entry in entries {
        match (entry.name.as_ref(), &entry.value) {
            ("is", AstValue::Text(name)) => value_type = schema_type(name),
            ("secret", AstValue::Bool(true)) => taint = Taint::Secret,
            _ => {}
        }
    }
    ValueFact { value_type, taint }
}

fn schema_type(name: &str) -> ValueType {
    match name {
        "text" => ValueType::Text,
        "number" => ValueType::Number,
        "boolean" => ValueType::Boolean,
        "object" => ValueType::Object,
        "list" | "list<any>" | "list<text>" | "list<number>" | "list<boolean>" => ValueType::List,
        _ => ValueType::Any,
    }
}

fn value_facts(entries: &[AstMapEntry<AstValue>]) -> HashMap<&str, ValueFact> {
    let mut facts = HashMap::with_capacity(entries.len());
    for entry in entries {
        match facts.entry(entry.name.as_ref()) {
            std::collections::hash_map::Entry::Occupied(mut fact) => {
                fact.insert(value_fact(&entry.value, None));
            }
            std::collections::hash_map::Entry::Vacant(fact) => {
                fact.insert(value_fact(&entry.value, None));
            }
        }
    }
    facts
}

fn secret_facts<T>(entries: &[AstMapEntry<T>]) -> HashMap<&str, ValueFact> {
    let mut facts = HashMap::with_capacity(entries.len());
    for entry in entries {
        match facts.entry(entry.name.as_ref()) {
            std::collections::hash_map::Entry::Occupied(mut fact) => {
                fact.insert(ValueFact {
                    value_type: ValueType::Any,
                    taint: Taint::Secret,
                });
            }
            std::collections::hash_map::Entry::Vacant(fact) => {
                fact.insert(ValueFact {
                    value_type: ValueType::Any,
                    taint: Taint::Secret,
                });
            }
        }
    }
    facts
}

fn validate_steps(ast: &WorkflowAst, facts: &mut Facts<'_>) -> Result<(), CompileErrors> {
    let mut errors = Vec::new();
    for (index, step) in ast.steps.iter().enumerate() {
        match &step.kind {
            StepKindAst::Run { input, .. } => {
                if let Err(e) = facts.read_slot(input.as_usize(), "run.input") {
                    errors.push(e);
                }
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Save { fields } => facts.write_slot(index, save_fact(fields, facts)),
            StepKindAst::Choose { condition, .. } => {
                if let Err(e) = validate_condition(condition, facts) {
                    errors.push(e);
                }
            }
            StepKindAst::ForEach { input, item, .. } => {
                if let Err(e) = facts.read_slot(input.as_usize(), "for_each.input") {
                    errors.push(e);
                }
                facts.write_slot(item.as_usize(), ValueFact::clean(ValueType::Any));
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Together { .. } => {
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Collect { source, .. } => {
                if let Err(e) = facts.read_slot(source.as_usize(), "collect.source") {
                    errors.push(e);
                }
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Reduce {
                input, accumulator, ..
            } => {
                if let Err(e) = facts.read_slot(input.as_usize(), "reduce.input") {
                    errors.push(e);
                }
                facts.write_slot(accumulator.as_usize(), ValueFact::clean(ValueType::Any));
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Repeat { .. } => {
                if let Some(attempt_slot) = index.checked_add(1) {
                    facts.write_slot(attempt_slot, ValueFact::clean(ValueType::Any));
                } else {
                    errors.push(CompileError::SlotIndexOutOfRange { value: i64::MAX });
                }
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Wait { .. } => facts.write_slot(index, ValueFact::clean(ValueType::Any)),
            StepKindAst::Ask { answer, .. } => {
                facts.write_slot(answer.as_usize(), ValueFact::clean(ValueType::Any));
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Finish { result } => {
                if let Err(e) = validate_public_result(result, facts) {
                    errors.push(e);
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

fn save_fact(fields: &[AstMapEntry<AstValue>], facts: &Facts<'_>) -> ValueFact {
    match single_value_field(fields) {
        Some(value) => value_fact(value, Some(facts)),
        None => optional_object_fact(fields, Some(facts)),
    }
}

fn single_value_field(fields: &[AstMapEntry<AstValue>]) -> Option<&AstValue> {
    match fields {
        [entry] if entry.name.as_ref() == "value" => Some(&entry.value),
        _ => None,
    }
}

fn validate_condition(expression: &AstExpression, facts: &Facts<'_>) -> Result<(), CompileError> {
    let fact = expression_fact(expression, facts, "choose.condition")?;
    if matches!(fact.value_type, ValueType::Boolean | ValueType::Any) {
        Ok(())
    } else {
        Err(CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "boolean",
            found: fact.value_type.as_str(),
        })
    }
}

fn validate_public_result(
    expression: &AstExpression,
    facts: &Facts<'_>,
) -> Result<(), CompileError> {
    // Section 47: No rejection of Secret or DerivedFromSecret results in Finish.
    // Taint is tracked but does not cause rejection.
    let _fact = expression_fact(expression, facts, "finish.result")?;
    Ok(())
}

fn expression_fact(
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

fn expression_literal_fact(value: &ExpressionLiteral) -> ValueFact {
    match value {
        ExpressionLiteral::Null => ValueFact::clean(ValueType::Null),
        ExpressionLiteral::Bool(_) => ValueFact::clean(ValueType::Boolean),
        ExpressionLiteral::I64(_) => ValueFact::clean(ValueType::Number),
        ExpressionLiteral::F64(_) => ValueFact::clean(ValueType::Number),
        ExpressionLiteral::Text(_) => ValueFact::clean(ValueType::Text),
    }
}

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

fn typed_unary_fact(
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
) -> Result<Taint, CompileError> {
    let mut taint = Taint::Clean;
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

fn matches_type(actual: ValueType, expected: ValueType) -> bool {
    matches!(actual, ValueType::Any) || actual == expected
}

fn type_mismatch(field: &'static str, expected: ValueType, found: ValueType) -> CompileError {
    CompileError::TypeMismatch {
        field,
        expected: expected.as_str(),
        found: found.as_str(),
    }
}

fn value_fact(value: &AstValue, facts: Option<&Facts<'_>>) -> ValueFact {
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

fn optional_object_fact(entries: &[AstMapEntry<AstValue>], facts: Option<&Facts<'_>>) -> ValueFact {
    let mut fact = ValueFact::clean(ValueType::Object);
    for entry in entries {
        fact = fact.merge(value_fact(&entry.value, facts));
    }
    fact
}

fn reference_fact(reference: &str, facts: Option<&Facts<'_>>) -> ValueFact {
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

#[cfg(test)]
mod tests;
