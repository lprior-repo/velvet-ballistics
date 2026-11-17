#![forbid(unsafe_code)]

use crate::ValidationError;
use crate::schema::{FieldValue, StepDoc, WorkflowDoc, validate_workflow_schema};

fn step(id: &str) -> StepDoc {
    StepDoc::from_pairs(vec![
        ("id".into(), FieldValue::String(id.into())),
        ("set".into(), FieldValue::Empty),
    ])
}

fn workflow_with_trigger(kind: &str, body: FieldValue) -> WorkflowDoc {
    WorkflowDoc::from_pairs(vec![
        (
            "version".into(),
            FieldValue::String("velvet-ballistics/v1".into()),
        ),
        ("name".into(), FieldValue::String("workflow".into())),
        (
            "when".into(),
            FieldValue::Mapping(vec![(kind.into(), body)]),
        ),
        (
            "steps".into(),
            FieldValue::Sequence(vec![step("valid_step")]),
        ),
    ])
}

#[test]
fn public_event_trigger_rejects_extra_fields_before_or_after_valid_type() {
    for extra in ["bogus", "name"] {
        for extra_first in [false, true] {
            let required = (
                "type".into(),
                FieldValue::String("github.pull_request".into()),
            );
            let unknown = (extra.into(), FieldValue::String("legacy_or_unknown".into()));
            let entries = if extra_first {
                vec![unknown, required]
            } else {
                vec![required, unknown]
            };
            let doc = workflow_with_trigger("event", FieldValue::Mapping(entries));
            assert_eq!(
                crate::schema::validate_trigger(&doc),
                Err(ValidationError::UnsupportedTrigger {
                    trigger: "event".into()
                })
            );
            assert_eq!(
                validate_workflow_schema(&doc),
                Err(ValidationError::UnsupportedTrigger {
                    trigger: "event".into()
                })
            );
        }
    }
}

#[test]
fn public_event_trigger_requires_nonempty_string_type() {
    let valid = workflow_with_trigger(
        "event",
        FieldValue::Mapping(vec![(
            "type".into(),
            FieldValue::String("github.pull_request".into()),
        )]),
    );
    assert_eq!(crate::schema::validate_trigger(&valid), Ok(()));
    assert_eq!(validate_workflow_schema(&valid), Ok(()));
    for body in [
        FieldValue::Empty,
        FieldValue::String("github.pull_request".into()),
        FieldValue::Mapping(vec![]),
        FieldValue::Mapping(vec![("type".into(), FieldValue::Empty)]),
        FieldValue::Mapping(vec![("type".into(), FieldValue::String(String::new()))]),
        FieldValue::Mapping(vec![(
            "name".into(),
            FieldValue::String("github.pull_request".into()),
        )]),
    ] {
        let doc = workflow_with_trigger("event", body);
        assert_eq!(
            crate::schema::validate_trigger(&doc),
            Err(ValidationError::UnsupportedTrigger {
                trigger: "event".into()
            })
        );
        assert_eq!(
            validate_workflow_schema(&doc),
            Err(ValidationError::UnsupportedTrigger {
                trigger: "event".into()
            })
        );
    }
}

#[test]
fn event_key_restriction_preserves_other_public_trigger_contracts() {
    for kind in ["manual", "webhook"] {
        for body in [FieldValue::Empty, FieldValue::Mapping(vec![])] {
            assert_eq!(
                validate_workflow_schema(&workflow_with_trigger(kind, body)),
                Ok(())
            );
        }
        let nonempty =
            FieldValue::Mapping(vec![("path".into(), FieldValue::String("/hook".into()))]);
        assert_eq!(
            validate_workflow_schema(&workflow_with_trigger(kind, nonempty)),
            Err(ValidationError::UnsupportedTrigger {
                trigger: kind.into()
            })
        );
    }
    // Schedule has its own allowed-field policy; event's type-only restriction must not leak.
    for entries in [
        vec![("cron".into(), FieldValue::String("0 * * * *".into()))],
        vec![
            ("cron".into(), FieldValue::String("0 * * * *".into())),
            ("timezone".into(), FieldValue::String("UTC".into())),
        ],
    ] {
        assert_eq!(
            validate_workflow_schema(&workflow_with_trigger(
                "schedule",
                FieldValue::Mapping(entries)
            )),
            Ok(())
        );
    }
    assert_eq!(
        validate_workflow_schema(&workflow_with_trigger("http", FieldValue::Empty)),
        Err(ValidationError::HttpTriggerOutOfCore)
    );
}

#[test]
fn public_event_trigger_rejects_duplicate_type_keys() {
    for second in [
        FieldValue::String("github.pull_request".into()),
        FieldValue::String("github.push".into()),
        FieldValue::String(String::new()),
        FieldValue::Empty,
    ] {
        for invalid_first in [false, true] {
            let first = FieldValue::String("github.pull_request".into());
            let values = if invalid_first {
                [second.clone(), first]
            } else {
                [first, second.clone()]
            };
            let body = FieldValue::Mapping(
                values
                    .into_iter()
                    .map(|value| ("type".into(), value))
                    .collect(),
            );
            let doc = workflow_with_trigger("event", body);
            assert_eq!(
                crate::schema::validate_trigger(&doc),
                Err(ValidationError::DuplicateKey)
            );
            assert_eq!(
                validate_workflow_schema(&doc),
                Err(ValidationError::DuplicateKey)
            );
        }
    }
}

#[test]
fn duplicate_event_diagnostic_preserves_unknown_field_precedence() {
    for extra_first in [false, true] {
        let mut entries = vec![
            (
                "type".into(),
                FieldValue::String("github.pull_request".into()),
            ),
            ("type".into(), FieldValue::String("github.push".into())),
        ];
        let extra = ("bogus".into(), FieldValue::Empty);
        if extra_first {
            entries.insert(0, extra);
        } else {
            entries.push(extra);
        }
        let doc = workflow_with_trigger("event", FieldValue::Mapping(entries));
        let expected = Err(ValidationError::UnsupportedTrigger {
            trigger: "event".into(),
        });
        assert_eq!(crate::schema::validate_trigger(&doc), expected);
        assert_eq!(validate_workflow_schema(&doc), expected);
    }
}
