#![forbid(unsafe_code)]
//! Step-level validation: forward data-flow over the step sequence.
//!
//! Each step writes a `ValueFact` into its slot (when applicable) and emits
//! compile errors for type/taint violations.

use crate::CompileError;
use crate::CompileErrors;
use crate::ast::{AstExpression, AstMapEntry, AstValue, StepKindAst, WorkflowAst};

use super::engine::Facts;
use super::eval::expression_fact;
use super::types::{ValueFact, ValueType};

// ── Public entry point ──────────────────────────────────────────────────────

pub(crate) fn validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileErrors> {
    let mut facts = Facts::new(ast);
    validate_steps(ast, &mut facts)
}

// ── Step iteration ──────────────────────────────────────────────────────────

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
            StepKindAst::Repeat { body, .. } => {
                if let Some(attempt_slot) = index.checked_add(1) {
                    facts.write_slot(attempt_slot, ValueFact::clean(ValueType::Any));
                } else {
                    errors.push(CompileError::SlotIndexOutOfRange { value: i64::MAX });
                }
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
                taint_repeat_body(body, facts, &mut errors);
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

// ── Repeat body walker ──────────────────────────────────────────────────────

/// Walks a `Repeat` body for type-taint analysis.
///
/// Body sub-steps do not have a slot index in the top-level `ast.steps`
/// layout, so the body walker validates expression/condition/result
/// references but does not write slot facts. This matches the existing
/// taint behavior for the pre-existing `Run`/`Save`/`Choose`/etc. sub-step
/// kinds inside `ForEach`/`Together` bodies, which the lowering handles
/// in `compile_source` rather than the cold AST.
fn taint_repeat_body(
    body: &[crate::ast::StepAst],
    facts: &Facts<'_>,
    errors: &mut Vec<CompileError>,
) {
    use crate::ast::StepKindAst;
    for body_step in body {
        match &body_step.kind {
            StepKindAst::Run { .. }
            | StepKindAst::ForEach { .. }
            | StepKindAst::Together { .. }
            | StepKindAst::Collect { .. }
            | StepKindAst::Wait { .. }
            | StepKindAst::Ask { .. } => {}
            StepKindAst::Repeat { body: inner, .. } => {
                taint_repeat_body(inner, facts, errors);
            }
            StepKindAst::Save { fields } => {
                let _body_fact = save_fact(fields, facts);
            }
            StepKindAst::Choose { condition, .. } => {
                if let Err(e) = validate_condition(condition, facts) {
                    errors.push(e);
                }
            }
            StepKindAst::Reduce { initial, .. } => {
                let _initial_fact = super::eval::value_fact(initial, Some(facts));
            }
            StepKindAst::Finish { result } => {
                if let Err(e) = validate_public_result(result, facts) {
                    errors.push(e);
                }
            }
        }
    }
}

// ── Step-specific validators ────────────────────────────────────────────────

fn save_fact(fields: &[AstMapEntry<AstValue>], facts: &Facts<'_>) -> ValueFact {
    match single_value_field(fields) {
        Some(value) => super::eval::value_fact(value, Some(facts)),
        None => super::eval::optional_object_fact(fields, Some(facts)),
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
