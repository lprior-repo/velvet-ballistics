#![forbid(unsafe_code)]

use super::{base_valid_fields, make_workflow};
use crate::ValidationError;
use crate::schema_support::schema_doc::FieldValue;
use crate::schema_support::schema_fields::{validate_version, validate_workflow_schema};

#[test]
fn accepts_valid_workflow() {
    assert_eq!(validate_workflow_schema(&super::valid_workflow_doc()), Ok(()));
}

#[test]
fn rejects_empty_workflow() {
    let doc = make_workflow(vec![]);
    assert_eq!(
        validate_workflow_schema(&doc),
        Err(ValidationError::MissingRequiredField {
            field: "version".to_owned()
        })
    );
}

#[test]
fn rejects_unknown_top_level_field() {
    let mut fields = base_valid_fields();
    fields.push(("webhook", FieldValue::Empty));
    let doc = make_workflow(fields);
    assert_eq!(
        validate_workflow_schema(&doc),
        Err(ValidationError::UnknownTopLevelField)
    );
}

#[test]
fn rejects_duplicate_top_level_keys() {
    let mut fields = base_valid_fields();
    fields.push(("name", FieldValue::String("second".to_owned())));
    let doc = make_workflow(fields);
    assert_eq!(
        validate_workflow_schema(&doc),
        Err(ValidationError::DuplicateKey)
    );
}

#[test]
fn validate_version_accepts_canonical() {
    let doc = make_workflow(vec![(
        "version",
        FieldValue::String("velvet-ballistics/v1".to_owned()),
    )]);
    assert_eq!(validate_version(&doc), Ok(()));
}

#[test]
fn validate_version_rejects_wrong_version() {
    let doc = make_workflow(vec![("version", FieldValue::String("wrong/v1".to_owned()))]);
    assert_eq!(
        validate_version(&doc),
        Err(ValidationError::InvalidVersion {
            version: "wrong/v1".to_owned()
        })
    );
}

#[test]
fn validate_version_rejects_missing() {
    let doc = make_workflow(vec![]);
    assert_eq!(
        validate_version(&doc),
        Err(ValidationError::MissingRequiredField {
            field: "version".to_owned()
        })
    );
}

#[test]
fn all_allowed_top_level_fields_are_accepted() {
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
        ("inputs", FieldValue::Empty),
        ("vars", FieldValue::Empty),
        ("secrets", FieldValue::Empty),
        ("result", FieldValue::Empty),
        ("examples", FieldValue::Empty),
        (
            "steps",
            FieldValue::Sequence(vec![super::make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(validate_workflow_schema(&doc), Ok(()));
}
