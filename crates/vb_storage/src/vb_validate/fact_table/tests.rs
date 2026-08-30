//! Tests for fact_table

use crate::vb_validate::ValidationError;
use crate::vb_validate::type_sigs::{InputDecl, Taint, TypedValue, ValueFact, ValueType, WorkflowTypes};

use crate::vb_validate::*;

fn sample_workflow() -> WorkflowTypes {
    WorkflowTypes {
        inputs: vec![
            InputDecl {
                name: "user".to_owned(),
                schema_type: ValueType::Text,
                is_secret: false,
            },
            InputDecl {
                name: "token".to_owned(),
                schema_type: ValueType::Text,
                is_secret: true,
            },
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
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "text".to_owned(),
        }),
        "require_boolean should reject text type"
    );
}

#[test]
fn require_boolean_rejects_number() {
    let result = require_boolean(ValueType::Number);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "number".to_owned(),
        }),
        "require_boolean should reject number type"
    );
}

#[test]
fn require_boolean_rejects_null() {
    let result = require_boolean(ValueType::Null);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "null".to_owned(),
        }),
        "require_boolean should reject null type"
    );
}

#[test]
fn require_boolean_rejects_object() {
    let result = require_boolean(ValueType::Object);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "object".to_owned(),
        }),
        "require_boolean should reject object type"
    );
}

#[test]
fn require_boolean_rejects_list() {
    let result = require_boolean(ValueType::List);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "list".to_owned(),
        }),
        "require_boolean should reject list type"
    );
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
    let fact = resolve_value(
        &TypedValue::Reference("$input.user".to_owned()),
        &facts,
        &slots,
    );
    assert_eq!(fact.value_type, ValueType::Text);
    assert_eq!(fact.taint, Taint::Clean);
}

#[test]
fn resolve_value_reference_resolves_secret() {
    let facts = Facts::build(&sample_workflow());
    let slots = vec![];
    let fact = resolve_value(
        &TypedValue::Reference("$input.token".to_owned()),
        &facts,
        &slots,
    );
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
    let Some(fact) = slots.first() else {
        return;
    };
    assert_eq!(*fact, Some(ValueFact::clean(ValueType::Boolean)));
}

#[test]
fn write_slot_out_of_bounds_is_noop() {
    let mut slots = vec![None];
    write_slot(&mut slots, 5, ValueFact::clean(ValueType::Number));
    let Some(fact) = slots.first() else {
        return;
    };
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
            InputDecl {
                name: "x".to_owned(),
                schema_type: ValueType::Text,
                is_secret: false,
            },
            InputDecl {
                name: "x".to_owned(),
                schema_type: ValueType::Number,
                is_secret: true,
            },
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
