#![forbid(unsafe_code)]
//! Facts collection and management for type taint validation.

use crate::ast::{AstMapEntry, AstValue};
use crate::compile::type_taint::types::{ValueFact, ValueType};
use crate::{CompileError, CompileErrors};
use std::collections::HashMap;
use vb_validate::type_taint::Taint;

/// Manages all facts (type + taint information) for a workflow AST.
pub(crate) struct Facts<'a> {
    /// Facts for input parameters.
    pub(crate) inputs: HashMap<&'a str, ValueFact>,
    /// Facts for workflow variables.
    pub(crate) vars: HashMap<&'a str, ValueFact>,
    /// Facts for secrets.
    pub(crate) secrets: HashMap<&'a str, ValueFact>,
    /// Slot facts indexed by step position.
    pub(crate) slots: Vec<Option<ValueFact>>,
}

impl<'a> Facts<'a> {
    /// Creates a new facts collector from a workflow AST.
    pub(crate) fn new(ast: &'a crate::ast::WorkflowAst) -> Self {
        Self {
            inputs: input_facts(&ast.inputs),
            vars: value_facts(&ast.vars),
            secrets: secret_facts(&ast.secrets),
            slots: vec![None; ast.steps.len()],
        }
    }

    /// Writes a fact to a slot.
    pub(crate) fn write_slot(&mut self, index: usize, fact: ValueFact) {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = Some(fact);
        }
    }

    /// Reads a fact from a slot.
    pub(crate) fn read_slot(
        &self,
        index: usize,
        field: &'static str,
    ) -> Result<ValueFact, CompileError> {
        match self.slots.get(index).and_then(|slot| *slot) {
            Some(fact) => Ok(fact),
            None => Err(CompileError::UnknownSlotType { field, slot: index }),
        }
    }
}

/// Extracts facts from input schema entries.
pub(crate) fn input_facts(entries: &[AstMapEntry<AstValue>]) -> HashMap<&str, ValueFact> {
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

/// Creates a fact from an input schema value.
pub(crate) fn input_schema_fact(value: &AstValue) -> ValueFact {
    match value {
        AstValue::Text(name) => ValueFact::clean(schema_type(name)),
        AstValue::Mapping(entries) => schema_mapping_fact(entries),
        _ => ValueFact::clean(ValueType::Any),
    }
}

/// Creates a fact from a schema mapping entry.
pub(crate) fn schema_mapping_fact(entries: &[AstMapEntry<AstValue>]) -> ValueFact {
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

/// Maps a schema type name to a ValueType.
pub(crate) fn schema_type(name: &str) -> ValueType {
    match name {
        "text" => ValueType::Text,
        "number" => ValueType::Number,
        "boolean" => ValueType::Boolean,
        "object" => ValueType::Object,
        "list" | "list<any>" | "list<text>" | "list<number>" | "list<boolean>" => ValueType::List,
        _ => ValueType::Any,
    }
}

/// Extracts facts from value entries.
pub(crate) fn value_facts(entries: &[AstMapEntry<AstValue>]) -> HashMap<&str, ValueFact> {
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

/// Extracts facts from secret entries (all marked as Secret taint).
pub(crate) fn secret_facts<T>(entries: &[AstMapEntry<T>]) -> HashMap<&str, ValueFact> {
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
