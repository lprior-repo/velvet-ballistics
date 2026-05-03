//! Fact table for type/taint validation.

#![allow(unreachable_pub)]
//!
//! Builds and resolves facts for inputs, vars, and secrets.

use std::collections::HashMap;

use crate::ValidationResult;

use crate::type_sigs::{
    InputDecl, Taint, TypedValue, ValueFact, ValueType, WorkflowTypes,
};

/// Requires a value type to be boolean (or Any).
pub(crate) fn require_boolean(actual: ValueType) -> ValidationResult<()> {
    if matches!(actual, ValueType::Boolean | ValueType::Any) {
        Ok(())
    } else {
        Err(crate::ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: actual.as_str().to_owned(),
        })
    }
}

/// Resolves a typed value to its fact (type + taint).
pub(crate) fn resolve_value(
    value: &TypedValue,
    facts: &Facts,
    slots: &[Option<ValueFact>],
) -> ValueFact {
    match value {
        TypedValue::Literal(vt) => ValueFact::clean(*vt),
        TypedValue::Reference(name) => facts.resolve_reference(name),
        TypedValue::Slot(index) => match slots.get(*index).and_then(|s| *s) {
            Some(value) => value,
            None => ValueFact::clean(ValueType::Any),
        },
        TypedValue::Composite(values) => resolve_composite(values, facts, slots),
    }
}

fn resolve_composite(
    values: &[TypedValue],
    facts: &Facts,
    slots: &[Option<ValueFact>],
) -> ValueFact {
    let mut taint = Taint::Clean;
    for value in values {
        let fact = resolve_value(value, facts, slots);
        taint = taint.merge(fact.taint);
    }
    ValueFact {
        value_type: ValueType::Any,
        taint,
    }
}

/// Writes a fact into a slot index.
pub(crate) fn write_slot(slots: &mut [Option<ValueFact>], index: usize, fact: ValueFact) {
    if let Some(slot) = slots.get_mut(index) {
        *slot = Some(fact);
    }
}

/// Resolved facts for inputs, vars, and secrets.
pub(crate) struct Facts {
    inputs: HashMap<String, ValueFact>,
    vars: HashMap<String, ValueFact>,
    secrets: HashMap<String, ValueFact>,
}

impl Facts {
    pub(crate) fn build(workflow: &WorkflowTypes) -> Self {
        Self {
            inputs: input_facts(&workflow.inputs),
            vars: var_facts(&workflow.vars),
            secrets: secret_facts(&workflow.secrets),
        }
    }

    fn resolve_reference(&self, reference: &str) -> ValueFact {
        let Some(body) = reference.strip_prefix('$') else {
            return ValueFact::clean(ValueType::Text);
        };
        let Some((root, tail)) = body.split_once('.') else {
            return ValueFact::clean(ValueType::Any);
        };
        let name = reference_name(tail);
        let fact = match root {
            "input" => self.inputs.get(name).copied(),
            "var" | "vars" => self.vars.get(name).copied(),
            "secrets" => self.secrets.get(name).copied(),
            _ => None,
        };
        match fact {
            Some(value) => value,
            None => ValueFact::clean(ValueType::Any),
        }
    }
}

fn input_facts(inputs: &[InputDecl]) -> HashMap<String, ValueFact> {
    let mut facts = HashMap::with_capacity(inputs.len());
    for input in inputs {
        let taint = if input.is_secret {
            Taint::Secret
        } else {
            Taint::Clean
        };
        match facts.entry(input.name.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.insert(ValueFact {
                    value_type: input.schema_type,
                    taint,
                });
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ValueFact {
                    value_type: input.schema_type,
                    taint,
                });
            }
        }
    }
    facts
}

fn var_facts(vars: &[(String, ValueType)]) -> HashMap<String, ValueFact> {
    let mut facts = HashMap::with_capacity(vars.len());
    for (name, vt) in vars {
        match facts.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.insert(ValueFact::clean(*vt));
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ValueFact::clean(*vt));
            }
        }
    }
    facts
}

fn secret_facts(secrets: &[String]) -> HashMap<String, ValueFact> {
    let mut facts = HashMap::with_capacity(secrets.len());
    for name in secrets {
        match facts.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.insert(ValueFact::secret(ValueType::Any));
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ValueFact::secret(ValueType::Any));
            }
        }
    }
    facts
}

fn reference_name(tail: &str) -> &str {
    match tail.split_once('.') {
        Some((name, _)) => name,
        None => tail,
    }
}
