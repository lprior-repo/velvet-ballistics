#![forbid(unsafe_code)]
//! Field and document structure validation for schema validation.

#![allow(unreachable_pub)]
use crate::schema_doc::{FieldValue, StepDoc, WorkflowDoc};
use crate::schema_id::{is_reserved_id, is_valid_id, validate_single_id};
use crate::{ValidationError, ValidationResult};
use vb_core::span::Span;

const CANONICAL_VERSION: &str = "velvet-ballistics/v1";
const REQUIRED_TOP_LEVEL_FIELDS: &[&str] = &["version", "name", "when", "steps"];
const ALLOWED_TOP_LEVEL_FIELDS: &[&str] = &[
    "version", "name", "when", "inputs", "vars", "secrets", "result", "examples", "steps",
];
const ALLOWED_STEP_FIELDS: &[&str] = &[
    "id",
    "name",
    "if",
    "with",
    "then",
    "set",
    "choose",
    "for_each",
    "parallel",
    "collect",
    "aggregate",
    "repeat",
    "wait",
    "ask",
    "finish",
    "do",
    "on_error",
    "try_again",
];
const STEP_PRIMITIVES: &[&str] = &[
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
];

pub fn validate_workflow_schema(doc: &WorkflowDoc) -> ValidationResult<()> {
    validate_duplicate_fields(doc)?;
    validate_required_fields(doc)?;
    validate_unknown_fields(doc)?;
    validate_version(doc)?;
    validate_trigger(doc)?;
    validate_ids(doc)?;
    validate_step_fields(doc)?;
    Ok(())
}

fn validate_duplicate_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    validate_no_duplicate_names(&doc.fields)?;
    if let Some(steps) = doc.get_sequence("steps") {
        for step in steps {
            validate_no_duplicate_names(&step.fields)?;
        }
    }
    Ok(())
}

fn validate_no_duplicate_names(fields: &[(String, FieldValue)]) -> ValidationResult<()> {
    let mut seen: Vec<&str> = Vec::with_capacity(fields.len());
    for (name, _) in fields {
        if seen.contains(&name.as_str()) {
            return Err(ValidationError::DuplicateKey { span: Span::ZERO });
        }
        seen.push(name.as_str());
    }
    Ok(())
}

pub fn validate_version(doc: &WorkflowDoc) -> ValidationResult<()> {
    match doc.get_string("version") {
        Some(v) if v == CANONICAL_VERSION => Ok(()),
        Some(v) => Err(ValidationError::InvalidVersion {
            version: v.to_owned(),
            span: Span::ZERO,
        }),
        None => Err(ValidationError::MissingRequiredField {
            field: "version".to_owned(),
            span: Span::ZERO,
        }),
    }
}

pub fn validate_trigger(doc: &WorkflowDoc) -> ValidationResult<()> {
    let trigger = doc
        .get_mapping("when")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "when".to_owned(),
            span: Span::ZERO,
        })?;
    if trigger.is_empty() {
        return Err(ValidationError::MissingRequiredField {
            field: "when".to_owned(),
            span: Span::ZERO,
        });
    }
    if trigger.len() > 1 {
        return Err(ValidationError::UnsupportedTrigger {
            trigger: "multiple triggers".to_owned(),
            span: Span::ZERO,
        });
    }
    let (kind, body) = trigger
        .first()
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "when".to_owned(),
            span: Span::ZERO,
        })?;
    match kind.as_str() {
        "manual" | "webhook" => validate_empty_trigger(kind, body),
        "schedule" => validate_named_string_trigger(kind, body, "cron"),
        "event" => validate_named_string_trigger(kind, body, "name"),
        "http" => Err(ValidationError::HttpTriggerOutOfCore { span: Span::ZERO }),
        other => Err(ValidationError::UnsupportedTrigger {
            trigger: other.to_owned(),
            span: Span::ZERO,
        }),
    }
}

fn validate_empty_trigger(kind: &str, body: &FieldValue) -> ValidationResult<()> {
    match body {
        FieldValue::Empty => Ok(()),
        FieldValue::Mapping(entries) if entries.is_empty() => Ok(()),
        _ => Err(ValidationError::UnsupportedTrigger {
            trigger: kind.to_owned(),
            span: Span::ZERO,
        }),
    }
}

fn validate_named_string_trigger(
    kind: &str,
    body: &FieldValue,
    required_field: &str,
) -> ValidationResult<()> {
    let FieldValue::Mapping(entries) = body else {
        return Err(ValidationError::UnsupportedTrigger {
            trigger: kind.to_owned(),
            span: Span::ZERO,
        });
    };
    let valid = entries.iter().any(|(field, value)| match value {
        FieldValue::String(text) => field == required_field && !text.is_empty(),
        _ => false,
    });
    if valid {
        Ok(())
    } else {
        Err(ValidationError::UnsupportedTrigger {
            trigger: kind.to_owned(),
            span: Span::ZERO,
        })
    }
}

pub fn validate_ids(doc: &WorkflowDoc) -> ValidationResult<()> {
    let name = doc
        .get_string("name")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "name".to_owned(),
            span: Span::ZERO,
        })?;
    validate_id("name", name)?;
    let steps = doc
        .get_sequence("steps")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
            span: Span::ZERO,
        })?;
    if steps.is_empty() {
        return Err(ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
            span: Span::ZERO,
        });
    }
    let mut seen: Vec<&str> = Vec::with_capacity(steps.len());
    for step in steps {
        let id = step
            .get_string("id")
            .ok_or_else(|| ValidationError::MissingRequiredField {
                field: "step id".to_owned(),
                span: Span::ZERO,
            })?;
        validate_single_id(id, &seen)?;
        seen.push(id);
    }
    Ok(())
}

pub fn validate_step_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    let steps = doc
        .get_sequence("steps")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
            span: Span::ZERO,
        })?;
    for step in steps {
        validate_step_unknown_fields(step)?;
        validate_single_primitive(step)?;
    }
    Ok(())
}

fn validate_required_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    for field in REQUIRED_TOP_LEVEL_FIELDS {
        if !doc.has_field(field) {
            return Err(ValidationError::MissingRequiredField {
                field: (*field).to_owned(),
                span: Span::ZERO,
            });
        }
    }
    Ok(())
}

fn validate_unknown_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    for field in doc.field_names() {
        if !ALLOWED_TOP_LEVEL_FIELDS.contains(&field) {
            return Err(ValidationError::UnknownTopLevelField { span: Span::ZERO });
        }
    }
    Ok(())
}

fn validate_step_unknown_fields(step: &StepDoc) -> ValidationResult<()> {
    for field in step.field_names() {
        if !ALLOWED_STEP_FIELDS.contains(&field) {
            return Err(ValidationError::UnknownStepField { span: Span::ZERO });
        }
    }
    Ok(())
}

pub fn validate_single_primitive(step: &StepDoc) -> ValidationResult<()> {
    let mut count = 0_usize;
    for (field, _) in &step.fields {
        if STEP_PRIMITIVES.contains(&field.as_str()) {
            count = count.saturating_add(1);
        }
    }
    if count == 0 {
        return Err(ValidationError::MissingStepPrimitive { span: Span::ZERO });
    }
    if count > 1 {
        return Err(ValidationError::MultipleStepPrimitives { span: Span::ZERO });
    }
    Ok(())
}

fn validate_id(field: &str, id: &str) -> ValidationResult<()> {
    if !is_valid_id(id) {
        return Err(ValidationError::InvalidId {
            id: format!("{field}: {id}"),
            span: Span::ZERO,
        });
    }
    if is_reserved_id(id) {
        return Err(ValidationError::ReservedId {
            id: format!("{field}: {id}"),
            span: Span::ZERO,
        });
    }
    Ok(())
}

#[cfg(test)]
mod fields_tests {
    use super::*;
    use crate::schema_doc::{FieldValue, StepDoc, WorkflowDoc};
    use vb_core::span::Span;

    fn make_workflow(fields: Vec<(&str, FieldValue)>) -> WorkflowDoc {
        WorkflowDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }

    fn make_step(fields: Vec<(&str, FieldValue)>) -> StepDoc {
        StepDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }

    fn valid_workflow_doc() -> WorkflowDoc {
        make_workflow(vec![
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
                FieldValue::Sequence(vec![make_step(vec![
                    ("id", FieldValue::String("step1".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ])
    }

    // -- validate_workflow_schema end-to-end --

    #[test]
    fn accepts_valid_workflow() {
        assert_eq!(validate_workflow_schema(&valid_workflow_doc()), Ok(()));
    }

    #[test]
    fn rejects_empty_workflow() {
        let doc = make_workflow(vec![]);
        assert_eq!(
            validate_workflow_schema(&doc),
            Err(ValidationError::MissingRequiredField {
                field: "version".to_owned(),
                span: Span::ZERO
            })
        );
    }

    #[test]
    fn rejects_unknown_top_level_field() {
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
                FieldValue::Sequence(vec![make_step(vec![
                    ("id", FieldValue::String("s1".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
            ("webhook", FieldValue::Empty),
        ]);
        assert_eq!(
            validate_workflow_schema(&doc),
            Err(ValidationError::UnknownTopLevelField { span: Span::ZERO })
        );
    }

    #[test]
    fn rejects_duplicate_top_level_keys() {
        let doc = make_workflow(vec![
            (
                "version",
                FieldValue::String("velvet-ballistics/v1".to_owned()),
            ),
            ("name", FieldValue::String("first".to_owned())),
            ("name", FieldValue::String("second".to_owned())),
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
            validate_workflow_schema(&doc),
            Err(ValidationError::DuplicateKey { span: Span::ZERO })
        );
    }

    // -- validate_version --

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
                version: "wrong/v1".to_owned(),
                span: Span::ZERO
            })
        );
    }

    #[test]
    fn validate_version_rejects_missing() {
        let doc = make_workflow(vec![]);
        assert_eq!(
            validate_version(&doc),
            Err(ValidationError::MissingRequiredField {
                field: "version".to_owned(),
                span: Span::ZERO
            })
        );
    }

    // -- validate_trigger --

    #[test]
    fn validate_trigger_accepts_manual() {
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        )]);
        assert_eq!(validate_trigger(&doc), Ok(()));
    }

    #[test]
    fn validate_trigger_rejects_ipc() {
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("ipc".to_owned(), FieldValue::Empty)]),
        )]);
        assert_eq!(
            validate_trigger(&doc),
            Err(ValidationError::UnsupportedTrigger {
                trigger: "ipc".to_owned(),
                span: Span::ZERO
            })
        );
    }

    #[test]
    fn validate_trigger_accepts_schedule() {
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![(
                "schedule".to_owned(),
                FieldValue::Mapping(vec![(
                    "cron".to_owned(),
                    FieldValue::String("0 0 * * *".to_owned()),
                )]),
            )]),
        )]);
        assert_eq!(validate_trigger(&doc), Ok(()));
    }

    #[test]
    fn validate_trigger_accepts_event() {
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![(
                "event".to_owned(),
                FieldValue::Mapping(vec![(
                    "name".to_owned(),
                    FieldValue::String("job.created".to_owned()),
                )]),
            )]),
        )]);
        assert_eq!(validate_trigger(&doc), Ok(()));
    }

    #[test]
    fn validate_trigger_accepts_webhook() {
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("webhook".to_owned(), FieldValue::Mapping(vec![]))]),
        )]);
        assert_eq!(validate_trigger(&doc), Ok(()));
    }

    #[test]
    fn validate_trigger_rejects_empty_schedule_cron() {
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![(
                "schedule".to_owned(),
                FieldValue::Mapping(vec![("cron".to_owned(), FieldValue::String(String::new()))]),
            )]),
        )]);
        assert_eq!(
            validate_trigger(&doc),
            Err(ValidationError::UnsupportedTrigger {
                trigger: "schedule".to_owned(),
                span: Span::ZERO
            })
        );
    }

    #[test]
    fn validate_trigger_rejects_http() {
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("http".to_owned(), FieldValue::Empty)]),
        )]);
        assert_eq!(
            validate_trigger(&doc),
            Err(ValidationError::HttpTriggerOutOfCore { span: Span::ZERO })
        );
    }

    #[test]
    fn validate_trigger_rejects_unknown() {
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("cron".to_owned(), FieldValue::Empty)]),
        )]);
        assert_eq!(
            validate_trigger(&doc),
            Err(ValidationError::UnsupportedTrigger {
                trigger: "cron".to_owned(),
                span: Span::ZERO
            })
        );
    }

    #[test]
    fn validate_trigger_rejects_empty_mapping() {
        let doc = make_workflow(vec![("when", FieldValue::Mapping(vec![]))]);
        assert_eq!(
            validate_trigger(&doc),
            Err(ValidationError::MissingRequiredField {
                field: "when".to_owned(),
                span: Span::ZERO
            })
        );
    }

    #[test]
    fn validate_trigger_rejects_missing() {
        let doc = make_workflow(vec![]);
        assert_eq!(
            validate_trigger(&doc),
            Err(ValidationError::MissingRequiredField {
                field: "when".to_owned(),
                span: Span::ZERO
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
                trigger: "multiple triggers".to_owned(),
                span: Span::ZERO
            })
        );
    }

    // -- validate_ids --

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
                field: "name".to_owned(),
                span: Span::ZERO
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
                field: "steps".to_owned(),
                span: Span::ZERO
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
                field: "step id".to_owned(),
                span: Span::ZERO
            })
        );
    }

    // -- validate_single_primitive --

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
            Err(ValidationError::MissingStepPrimitive { span: Span::ZERO })
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
            Err(ValidationError::MultipleStepPrimitives { span: Span::ZERO })
        );
    }

    // -- validate_step_fields --

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
            Err(ValidationError::UnknownStepField { span: Span::ZERO })
        );
    }

    #[test]
    fn validate_step_fields_rejects_missing_steps() {
        let doc = make_workflow(vec![]);
        assert_eq!(
            validate_step_fields(&doc),
            Err(ValidationError::MissingRequiredField {
                field: "steps".to_owned(),
                span: Span::ZERO
            })
        );
    }

    // -- All valid step primitives --

    #[test]
    fn all_step_primitives_are_accepted() {
        for prim in &[
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
                (*prim, FieldValue::Empty),
            ]);
            assert_eq!(
                validate_single_primitive(&step),
                Ok(()),
                "primitive {prim} should be accepted"
            );
        }
    }

    // -- Optional fields are accepted --

    #[test]
    fn optional_step_fields_are_accepted() {
        for field in &["name", "if", "with", "then", "on_error", "try_again"] {
            let doc = make_workflow(vec![(
                "steps",
                FieldValue::Sequence(vec![make_step(vec![
                    ("id", FieldValue::String("s1".to_owned())),
                    (*field, FieldValue::Empty),
                    ("set", FieldValue::Empty),
                ])]),
            )]);
            assert_eq!(
                validate_step_fields(&doc),
                Ok(()),
                "optional field {field} should be accepted"
            );
        }
    }

    // -- All allowed top-level fields --

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
                FieldValue::Sequence(vec![make_step(vec![
                    ("id", FieldValue::String("s1".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        assert_eq!(validate_workflow_schema(&doc), Ok(()));
    }
}
