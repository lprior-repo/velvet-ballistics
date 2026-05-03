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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type_sigs::{InputDecl, Taint, TypedValue, ValueFact, ValueType, WorkflowTypes};

    fn sample_workflow() -> WorkflowTypes {
        WorkflowTypes {
            inputs: vec![
                InputDecl { name: "user".to_owned(), schema_type: ValueType::Text, is_secret: false },
                InputDecl { name: "token".to_owned(), schema_type: ValueType::Text, is_secret: true },
            ],
            vars: vec![
                ("count".to_owned(), ValueType::Number),
                ("label".to_owned(), ValueType::Text),
            ],
            secrets: vec!["db_password".to_owned()],
            ..WorkflowTypes::default()
        }
    }

    // -- Facts::build tests --

    #[test]
    fn facts_build_resolves_clean_input() {
        let facts = Facts::build(&sample_workflow());
        let fact = facts.resolve_reference("$input.user");
        assert_eq!(fact.value_type, ValueType::Text);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn facts_build_resolves_secret_input() {
        let facts = Facts::build(&sample_workflow());
        let fact = facts.resolve_reference("$input.token");
        assert_eq!(fact.value_type, ValueType::Text);
        assert_eq!(fact.taint, Taint::Secret);
    }

    #[test]
    fn facts_build_resolves_var() {
        let facts = Facts::build(&sample_workflow());
        let fact = facts.resolve_reference("$var.count");
        assert_eq!(fact.value_type, ValueType::Number);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn facts_build_resolves_vars_alias() {
        let facts = Facts::build(&sample_workflow());
        let fact = facts.resolve_reference("$vars.count");
        assert_eq!(fact.value_type, ValueType::Number);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn facts_build_resolves_secret() {
        let facts = Facts::build(&sample_workflow());
        let fact = facts.resolve_reference("$secrets.db_password");
        assert_eq!(fact.value_type, ValueType::Any);
        assert_eq!(fact.taint, Taint::Secret);
    }

    #[test]
    fn facts_resolve_unknown_reference_returns_text() {
        let facts = Facts::build(&sample_workflow());
        let fact = facts.resolve_reference("not_a_ref");
        assert_eq!(fact.value_type, ValueType::Text);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn facts_resolve_no_dot_returns_any() {
        let facts = Facts::build(&sample_workflow());
        let fact = facts.resolve_reference("$nodot");
        assert_eq!(fact.value_type, ValueType::Any);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn facts_resolve_unknown_root_returns_any() {
        let facts = Facts::build(&sample_workflow());
        let fact = facts.resolve_reference("$unknown.thing");
        assert_eq!(fact.value_type, ValueType::Any);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn facts_resolve_unknown_name_in_known_root_returns_any() {
        let facts = Facts::build(&sample_workflow());
        let fact = facts.resolve_reference("$input.nonexistent");
        assert_eq!(fact.value_type, ValueType::Any);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn facts_resolve_nested_path_uses_first_segment() {
        let facts = Facts::build(&sample_workflow());
        // $input.user.name should resolve "user" as the name (before the next dot)
        let fact = facts.resolve_reference("$input.user.name");
        assert_eq!(fact.value_type, ValueType::Text);
        assert_eq!(fact.taint, Taint::Clean);
    }

    // -- require_boolean tests --

    #[test]
    fn require_boolean_accepts_boolean() {
        assert_eq!(require_boolean(ValueType::Boolean), Ok(()));
    }

    #[test]
    fn require_boolean_accepts_any() {
        assert_eq!(require_boolean(ValueType::Any), Ok(()));
    }

    #[test]
    fn require_boolean_rejects_text() {
        let result = require_boolean(ValueType::Text);
        assert!(result.is_err());
    }

    #[test]
    fn require_boolean_rejects_number() {
        let result = require_boolean(ValueType::Number);
        assert!(result.is_err());
    }

    #[test]
    fn require_boolean_rejects_null() {
        let result = require_boolean(ValueType::Null);
        assert!(result.is_err());
    }

    #[test]
    fn require_boolean_rejects_object() {
        let result = require_boolean(ValueType::Object);
        assert!(result.is_err());
    }

    #[test]
    fn require_boolean_rejects_list() {
        let result = require_boolean(ValueType::List);
        assert!(result.is_err());
    }

    // -- resolve_value tests --

    #[test]
    fn resolve_value_literal_returns_clean_type() {
        let facts = Facts::build(&sample_workflow());
        let slots = vec![];
        let fact = resolve_value(&TypedValue::Literal(ValueType::Number), &facts, &slots);
        assert_eq!(fact.value_type, ValueType::Number);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn resolve_value_reference_resolves_input() {
        let facts = Facts::build(&sample_workflow());
        let slots = vec![];
        let fact = resolve_value(&TypedValue::Reference("$input.user".to_owned()), &facts, &slots);
        assert_eq!(fact.value_type, ValueType::Text);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn resolve_value_reference_resolves_secret() {
        let facts = Facts::build(&sample_workflow());
        let slots = vec![];
        let fact = resolve_value(&TypedValue::Reference("$input.token".to_owned()), &facts, &slots);
        assert_eq!(fact.value_type, ValueType::Text);
        assert_eq!(fact.taint, Taint::Secret);
    }

    #[test]
    fn resolve_value_slot_returns_some_value() {
        let facts = Facts::build(&sample_workflow());
        let slots = vec![Some(ValueFact::clean(ValueType::Boolean))];
        let fact = resolve_value(&TypedValue::Slot(0), &facts, &slots);
        assert_eq!(fact.value_type, ValueType::Boolean);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn resolve_value_slot_out_of_bounds_returns_any() {
        let facts = Facts::build(&sample_workflow());
        let slots: Vec<Option<ValueFact>> = vec![];
        let fact = resolve_value(&TypedValue::Slot(0), &facts, &slots);
        assert_eq!(fact.value_type, ValueType::Any);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn resolve_value_slot_none_returns_any() {
        let facts = Facts::build(&sample_workflow());
        let slots = vec![None];
        let fact = resolve_value(&TypedValue::Slot(0), &facts, &slots);
        assert_eq!(fact.value_type, ValueType::Any);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn resolve_value_composite_merges_taints() {
        let facts = Facts::build(&sample_workflow());
        let slots = vec![];
        let fact = resolve_value(
            &TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Text),
                TypedValue::Reference("$input.token".to_owned()),
            ]),
            &facts,
            &slots,
        );
        assert_eq!(fact.value_type, ValueType::Any);
        assert_eq!(fact.taint, Taint::Secret);
    }

    #[test]
    fn resolve_value_composite_all_clean_stays_clean() {
        let facts = Facts::build(&sample_workflow());
        let slots = vec![];
        let fact = resolve_value(
            &TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Text),
                TypedValue::Literal(ValueType::Number),
            ]),
            &facts,
            &slots,
        );
        assert_eq!(fact.value_type, ValueType::Any);
        assert_eq!(fact.taint, Taint::Clean);
    }

    // -- write_slot tests --

    #[test]
    fn write_slot_writes_to_valid_index() {
        let mut slots = vec![None, None];
        write_slot(&mut slots, 0, ValueFact::clean(ValueType::Boolean));
        let Some(fact) = slots.first() else { return; };
        assert_eq!(*fact, Some(ValueFact::clean(ValueType::Boolean)));
    }

    #[test]
    fn write_slot_out_of_bounds_is_noop() {
        let mut slots = vec![None];
        write_slot(&mut slots, 5, ValueFact::clean(ValueType::Number));
        let Some(fact) = slots.first() else { return; };
        assert_eq!(*fact, None);
    }

    // -- Facts with empty workflow --

    #[test]
    fn facts_build_empty_workflow() {
        let workflow = WorkflowTypes::default();
        let facts = Facts::build(&workflow);
        let fact = facts.resolve_reference("$input.anything");
        assert_eq!(fact.value_type, ValueType::Any);
    }

    // -- Duplicate input names: last wins --

    #[test]
    fn facts_build_duplicate_input_last_wins() {
        let workflow = WorkflowTypes {
            inputs: vec![
                InputDecl { name: "x".to_owned(), schema_type: ValueType::Text, is_secret: false },
                InputDecl { name: "x".to_owned(), schema_type: ValueType::Number, is_secret: true },
            ],
            ..WorkflowTypes::default()
        };
        let facts = Facts::build(&workflow);
        let fact = facts.resolve_reference("$input.x");
        assert_eq!(fact.value_type, ValueType::Number);
        assert_eq!(fact.taint, Taint::Secret);
    }

    #[test]
    fn facts_build_duplicate_var_last_wins() {
        let workflow = WorkflowTypes {
            vars: vec![
                ("v".to_owned(), ValueType::Text),
                ("v".to_owned(), ValueType::Boolean),
            ],
            ..WorkflowTypes::default()
        };
        let facts = Facts::build(&workflow);
        let fact = facts.resolve_reference("$var.v");
        assert_eq!(fact.value_type, ValueType::Boolean);
        assert_eq!(fact.taint, Taint::Clean);
    }
}
