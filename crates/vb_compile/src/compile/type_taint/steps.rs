#![forbid(unsafe_code)]
//! Step validation logic for type taint analysis.

use crate::ast::{AstExpression, AstMapEntry, AstValue, StepKindAst, WorkflowAst};
use crate::compile::type_taint::expressions::{
    expression_fact, validate_condition, validate_public_result,
};
use crate::compile::type_taint::facts::Facts;
use crate::compile::type_taint::types::ValueFact;
use crate::{CompileError, CompileErrors};

/// Validates all steps in a workflow AST.
pub(crate) fn validate_steps(
    ast: &WorkflowAst,
    facts: &mut Facts<'_>,
) -> Result<(), CompileErrors> {
    let mut errors = Vec::new();
    for (index, step) in ast.steps.iter().enumerate() {
        match &step.kind {
            StepKindAst::Run { input, .. } => {
                if let Err(e) = facts.read_slot(input.as_usize(), "run.input") {
                    errors.push(e);
                }
                facts.write_slot(
                    index,
                    ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
                );
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
                facts.write_slot(
                    item.as_usize(),
                    ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
                );
                facts.write_slot(
                    index,
                    ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
                );
            }
            StepKindAst::Together { .. } => {
                facts.write_slot(
                    index,
                    ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
                );
            }
            StepKindAst::Collect { source, .. } => {
                if let Err(e) = facts.read_slot(source.as_usize(), "collect.source") {
                    errors.push(e);
                }
                facts.write_slot(
                    index,
                    ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
                );
            }
            StepKindAst::Reduce {
                input, accumulator, ..
            } => {
                if let Err(e) = facts.read_slot(input.as_usize(), "reduce.input") {
                    errors.push(e);
                }
                facts.write_slot(
                    accumulator.as_usize(),
                    ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
                );
                facts.write_slot(
                    index,
                    ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
                );
            }
            StepKindAst::Repeat { .. } => {
                if let Some(attempt_slot) = index.checked_add(1) {
                    facts.write_slot(
                        attempt_slot,
                        ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
                    );
                } else {
                    errors.push(CompileError::SlotIndexOutOfRange { value: i64::MAX });
                }
                facts.write_slot(
                    index,
                    ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
                );
            }
            StepKindAst::Wait { .. } => facts.write_slot(
                index,
                ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
            ),
            StepKindAst::Ask { answer, .. } => {
                facts.write_slot(
                    answer.as_usize(),
                    ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
                );
                facts.write_slot(
                    index,
                    ValueFact::clean(crate::compile::type_taint::types::ValueType::Any),
                );
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

/// Computes the fact for a save step.
pub(crate) fn save_fact(fields: &[AstMapEntry<AstValue>], facts: &Facts<'_>) -> ValueFact {
    match single_value_field(fields) {
        Some(value) => crate::compile::type_taint::expressions::value_fact(value, Some(facts)),
        None => optional_object_fact(fields, Some(facts)),
    }
}

/// Extracts a single "value" field if present.
pub(crate) fn single_value_field(fields: &[AstMapEntry<AstValue>]) -> Option<&AstValue> {
    match fields {
        [entry] if entry.name.as_ref() == "value" => Some(&entry.value),
        _ => None,
    }
}

/// Computes a fact for an optional object mapping.
pub(crate) fn optional_object_fact(
    entries: &[AstMapEntry<AstValue>],
    facts: Option<&Facts<'_>>,
) -> ValueFact {
    let mut fact = ValueFact::clean(crate::compile::type_taint::types::ValueType::Object);
    for entry in entries {
        fact = fact.merge(crate::compile::type_taint::expressions::value_fact(
            &entry.value,
            facts,
        ));
    }
    fact
}
