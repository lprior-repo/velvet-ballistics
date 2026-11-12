#![forbid(unsafe_code)]
use crate::ast::{AstMapEntry, AstValue, StepKindAst, WorkflowAst};
use crate::{CompileError, CompileErrors};
use std::collections::HashMap;
use vb_validate::type_taint::Taint;

#[path = "type_taint_expr.rs"]
mod expr;

use expr::{optional_object_fact, validate_condition, validate_public_result, value_fact};

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

#[cfg(test)]
mod tests;
