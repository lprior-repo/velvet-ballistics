#![forbid(unsafe_code)]

use crate::vb_validate::{make_step, make_workflow, valid_workflow_doc};
use crate::vb_validate::ValidationError;
use crate::vb_validate::schema_support::schema_doc::FieldValue;
use crate::vb_validate::schema_support::schema_fields::validate_ids;

#[test]
fn validate_ids_accepts_valid_workflow() {
    let doc = valid_workflow_doc();
    assert_eq!(validate_ids(&doc), Ok(()));
}

#[test]
fn validate_ids_rejects_missing_name() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        (
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        ),
        (
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::MissingRequiredField {
            field: "name".to_owned()
        })
    );
}

#[test]
fn validate_ids_rejects_invalid_name() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("Bad-Name".to_owned())),
        (
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        ),
        (
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert!(matches!(
        validate_ids(&doc),
        Err(ValidationError::InvalidId { .. })
    ));
}

#[test]
fn validate_ids_rejects_empty_steps() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("test".to_owned())),
        (
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        ),
        ("steps", FieldValue::Sequence(vec![])),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::MissingRequiredField {
            field: "steps".to_owned()
        })
    );
}

#[test]
fn validate_ids_rejects_step_without_id() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("test".to_owned())),
        (
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        ),
        (
            "steps",
            FieldValue::Sequence(vec![make_step(vec![("finish", FieldValue::Empty)])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::MissingRequiredField {
            field: "step id".to_owned()
        })
    );
}
