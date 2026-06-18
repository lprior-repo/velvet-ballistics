#![forbid(unsafe_code)]
//! Fact-engine: the analysis state machine and schema-to-facts builders.

use crate::ast::{AstMapEntry, AstValue, WorkflowAst};
use crate::CompileError;
use std::collections::HashMap;
use vb_validate::type_taint::Taint;

use super::types::{ValueFact, ValueType};
use super::eval::value_fact;

/// The mutable analysis state for a single workflow.
///
/// Tracks typed facts about inputs, variables, secrets, and per-step output
/// slots during a forward data-flow walk over the step sequence.
pub(crate) struct Facts<'a> {
    pub(crate) inputs: HashMap<&'a str, ValueFact>,
    pub(crate) vars: HashMap<&'a str, ValueFact>,
    pub(crate) secrets: HashMap<&'a str, ValueFact>,
    slots: Vec<Option<ValueFact>>,
}

impl<'a> Facts<'a> {
    pub(crate) fn new(ast: &'a WorkflowAst) -> Self {
        Self {
            inputs: input_facts(&ast.inputs),
            vars: value_facts(&ast.vars),
            secrets: secret_facts(&ast.secrets),
            slots: vec![None; ast.steps.len()],
        }
    }

    pub(crate) fn write_slot(&mut self, index: usize, fact: ValueFact) {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = Some(fact);
        }
    }

    pub(crate) fn read_slot(&self, index: usize, field: &'static str) -> Result<ValueFact, CompileError> {
        match self.slots.get(index).and_then(|slot| *slot) {
            Some(fact) => Ok(fact),
            None => Err(CompileError::UnknownSlotType { field, slot: index }),
        }
    }
}

// ── Schema → facts helpers ──────────────────────────────────────────────────

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
