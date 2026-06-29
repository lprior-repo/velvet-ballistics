#![forbid(unsafe_code)]

use super::make_workflow;
use crate::ValidationError;
use crate::schema_support::schema_doc::FieldValue;
use crate::schema_support::schema_fields::validate_trigger;

#[test]
fn validate_trigger_accepts_supported_triggers() {
    for (kind, body) in [
        ("manual", FieldValue::Empty),
        ("webhook", FieldValue::Mapping(vec![])),
        (
            "schedule",
            FieldValue::Mapping(vec![(
                "cron".to_owned(),
                FieldValue::String("0 0 * * *".to_owned()),
            )]),
        ),
        (
            "event",
            FieldValue::Mapping(vec![(
                "name".to_owned(),
                FieldValue::String("job.created".to_owned()),
            )]),
        ),
    ] {
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![(kind.to_owned(), body)]),
        )]);
        assert_eq!(validate_trigger(&doc), Ok(()));
    }
}

#[test]
fn validate_trigger_rejects_unsupported_trigger_shapes() {
    for (kind, body, expected) in [
        (
            "ipc",
            FieldValue::Empty,
            ValidationError::UnsupportedTrigger {
                trigger: "ipc".to_owned(),
            },
        ),
        (
            "schedule",
            FieldValue::Mapping(vec![("cron".to_owned(), FieldValue::String(String::new()))]),
            ValidationError::UnsupportedTrigger {
                trigger: "schedule".to_owned(),
            },
        ),
        (
            "http",
            FieldValue::Empty,
            ValidationError::HttpTriggerOutOfCore,
        ),
        (
            "cron",
            FieldValue::Empty,
            ValidationError::UnsupportedTrigger {
                trigger: "cron".to_owned(),
            },
        ),
    ] {
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![(kind.to_owned(), body)]),
        )]);
        assert_eq!(validate_trigger(&doc), Err(expected));
    }
}

#[test]
fn validate_trigger_rejects_empty_or_missing() {
    let empty = make_workflow(vec![("when", FieldValue::Mapping(vec![]))]);
    assert_eq!(
        validate_trigger(&empty),
        Err(ValidationError::MissingRequiredField {
            field: "when".to_owned()
        })
    );

    let missing = make_workflow(vec![]);
    assert_eq!(
        validate_trigger(&missing),
        Err(ValidationError::MissingRequiredField {
            field: "when".to_owned()
        })
    );
}

#[test]
fn validate_trigger_rejects_multiple_triggers() {
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![
            ("manual".to_owned(), FieldValue::Empty),
            ("schedule".to_owned(), FieldValue::Mapping(vec![])),
        ]),
    )]);
    assert_eq!(
        validate_trigger(&doc),
        Err(ValidationError::UnsupportedTrigger {
            trigger: "multiple triggers".to_owned()
        })
    );
}
