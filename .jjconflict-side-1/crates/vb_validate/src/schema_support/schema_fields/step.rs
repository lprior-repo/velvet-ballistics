#![forbid(unsafe_code)]

use super::{make_step, make_workflow};
use crate::ValidationError;
use crate::schema_support::schema_doc::FieldValue;
use crate::schema_support::schema_fields::{validate_single_primitive, validate_step_fields};

#[test]
fn validate_single_primitive_accepts_one_primitive() {
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("set", FieldValue::Empty),
    ]);
    assert_eq!(validate_single_primitive(&step), Ok(()));
}

#[test]
fn validate_single_primitive_rejects_zero_primitives() {
    let step = make_step(vec![("id", FieldValue::String("s1".to_owned()))]);
    assert_eq!(
        validate_single_primitive(&step),
        Err(ValidationError::MissingStepPrimitive)
    );
}

#[test]
fn validate_single_primitive_rejects_two_primitives() {
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("set", FieldValue::Empty),
        ("do", FieldValue::Empty),
    ]);
    assert_eq!(
        validate_single_primitive(&step),
        Err(ValidationError::MultipleStepPrimitives)
    );
}

#[test]
fn validate_step_fields_accepts_valid_step() {
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("finish", FieldValue::Empty),
        ])]),
    )]);
    assert_eq!(validate_step_fields(&doc), Ok(()));
}

#[test]
fn validate_step_fields_rejects_unknown_field() {
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("finish", FieldValue::Empty),
            ("bogus", FieldValue::Empty),
        ])]),
    )]);
    assert_eq!(
        validate_step_fields(&doc),
        Err(ValidationError::UnknownStepField)
    );
}

#[test]
fn validate_step_fields_rejects_missing_steps() {
    let doc = make_workflow(vec![]);
    assert_eq!(
        validate_step_fields(&doc),
        Err(ValidationError::MissingRequiredField {
            field: "steps".to_owned()
        })
    );
}

#[test]
fn all_step_primitives_are_accepted() {
    for prim in [
        "set",
        "do",
        "choose",
        "for_each",
        "parallel",
        "collect",
        "aggregate",
        "repeat",
        "wait",
        "ask",
        "finish",
    ] {
        let step = make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            (prim, FieldValue::Empty),
        ]);
        assert_eq!(validate_single_primitive(&step), Ok(()));
    }
}

#[test]
fn optional_step_fields_are_accepted() {
    for field in ["name", "if", "with", "then", "on_error", "try_again"] {
        let doc = make_workflow(vec![(
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                (field, FieldValue::Empty),
                ("set", FieldValue::Empty),
            ])]),
        )]);
        assert_eq!(validate_step_fields(&doc), Ok(()));
    }
}
