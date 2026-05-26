#![forbid(unsafe_code)]
//! Tests for schema validation.

#![allow(unreachable_pub)]
use crate::ValidationError;
use crate::schema_doc::{FieldValue, StepDoc, WorkflowDoc};
use crate::schema_fields::{
    validate_ids, validate_single_primitive, validate_step_fields, validate_trigger,
    validate_version, validate_workflow_schema,
};
use crate::schema_id::{is_valid_id, validate_single_id};
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

#[test]
fn accepts_valid_workflow() {
    let doc = valid_workflow_doc();
    assert_eq!(validate_workflow_schema(&doc), Ok(()));
}

#[test]
fn rejects_missing_version() {
    let doc = make_workflow(vec![
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
    ]);
    assert!(matches!(
        validate_workflow_schema(&doc),
        Err(ValidationError::MissingRequiredField { .. })
    ));
}

#[test]
fn rejects_wrong_version() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("other-language/v1".to_owned()),
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
    ]);
    assert!(matches!(
        validate_version(&doc),
        Err(ValidationError::InvalidVersion { .. })
    ));
}

#[test]
fn rejects_http_trigger() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("test".to_owned())),
        (
            "when",
            FieldValue::Mapping(vec![("http".to_owned(), FieldValue::Empty)]),
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
        validate_trigger(&doc),
        Err(ValidationError::HttpTriggerOutOfCore { span: Span::ZERO })
    ));
}

#[test]
fn rejects_invalid_id() {
    let result = validate_single_id("123bad", &[]);
    assert!(matches!(result, Err(ValidationError::InvalidId { .. })));
}

#[test]
fn rejects_reserved_id() {
    let result = validate_single_id("runtime", &[]);
    assert!(matches!(result, Err(ValidationError::ReservedId { .. })));
}

#[test]
fn rejects_duplicate_id() {
    let result = validate_single_id("step1", &["step1"]);
    assert!(matches!(result, Err(ValidationError::DuplicateId { .. })));
}

#[test]
fn rejects_step_without_primitive() {
    let step = make_step(vec![("id", FieldValue::String("s1".to_owned()))]);
    assert!(matches!(
        validate_single_primitive(&step),
        Err(ValidationError::MissingStepPrimitive { span: Span::ZERO })
    ));
}

#[test]
fn rejects_step_with_multiple_primitives() {
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("set", FieldValue::Empty),
        ("finish", FieldValue::Empty),
    ]);
    assert!(matches!(
        validate_single_primitive(&step),
        Err(ValidationError::MultipleStepPrimitives { span: Span::ZERO })
    ));
}

#[test]
fn accepts_valid_id() {
    assert!(is_valid_id("step_1"));
    assert!(is_valid_id("a"));
    assert!(is_valid_id("abc_def_123"));
}

#[test]
fn rejects_uppercase_id() {
    assert!(!is_valid_id("StepOne"));
}

#[test]
fn rejects_id_starting_with_digit() {
    assert!(!is_valid_id("1step"));
}

#[test]
fn rejects_too_long_id() {
    let long_id = "a".repeat(65);
    assert!(!is_valid_id(&long_id));
}

#[test]
fn accepts_max_length_id() {
    let max_id = "a".repeat(64);
    assert!(is_valid_id(&max_id));
}

// ---------------------------------------------------------------------------
// BDD exact-assertion tests
// ---------------------------------------------------------------------------

#[test]
fn validate_workflow_schema_returns_unknown_top_level_field_for_invalid_field() {
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
        ("bogus_field", FieldValue::Empty),
    ]);
    assert_eq!(
        validate_workflow_schema(&doc),
        Err(ValidationError::UnknownTopLevelField { span: Span::ZERO })
    );
}

#[test]
fn validate_workflow_schema_returns_duplicate_key_for_duplicate_top_level_field() {
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

#[test]
fn validate_workflow_schema_returns_duplicate_key_for_duplicate_step_field() {
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
                ("set", FieldValue::Empty),
                ("set", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_workflow_schema(&doc),
        Err(ValidationError::DuplicateKey { span: Span::ZERO })
    );
}

#[test]
fn validate_workflow_schema_returns_unknown_step_field_for_invalid_step_field() {
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
                ("nonsense", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_step_fields(&doc),
        Err(ValidationError::UnknownStepField { span: Span::ZERO })
    );
}

#[test]
fn validate_workflow_schema_returns_missing_required_field_for_absent_name() {
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
        validate_workflow_schema(&doc),
        Err(ValidationError::MissingRequiredField {
            field: "name".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_workflow_schema_returns_missing_required_field_for_absent_steps() {
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
    ]);
    assert_eq!(
        validate_workflow_schema(&doc),
        Err(ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_version_returns_invalid_version_for_bad_version() {
    let doc = make_workflow(vec![("version", FieldValue::String("2.0".to_owned()))]);
    assert_eq!(
        validate_version(&doc),
        Err(ValidationError::InvalidVersion {
            version: "2.0".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_version_accepts_current_version() {
    let doc = make_workflow(vec![(
        "version",
        FieldValue::String("velvet-ballistics/v1".to_owned()),
    )]);
    assert_eq!(validate_version(&doc), Ok(()));
}

#[test]
fn validate_version_rejects_empty_version() {
    let doc = make_workflow(vec![("version", FieldValue::String(String::new()))]);
    assert_eq!(
        validate_version(&doc),
        Err(ValidationError::InvalidVersion {
            version: String::new(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_version_rejects_unknown_version() {
    let doc = make_workflow(vec![(
        "version",
        FieldValue::String("other-language/v2".to_owned()),
    )]);
    assert_eq!(
        validate_version(&doc),
        Err(ValidationError::InvalidVersion {
            version: "other-language/v2".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_version_returns_missing_required_field_when_absent() {
    let doc = make_workflow(vec![("name", FieldValue::String("test".to_owned()))]);
    assert_eq!(
        validate_version(&doc),
        Err(ValidationError::MissingRequiredField {
            field: "version".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_ids_returns_invalid_id_for_malformed_id() {
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
                ("id", FieldValue::String("1bad".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::InvalidId {
            id: "1bad".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_ids_returns_reserved_id_for_system_id() {
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
                ("id", FieldValue::String("runtime".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::ReservedId {
            id: "runtime".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_ids_returns_duplicate_id_for_same_step_id() {
    let seen = vec!["step1"];
    assert_eq!(
        validate_single_id("step1", &seen),
        Err(ValidationError::DuplicateId {
            id: "step1".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_single_primitive_returns_multiple_step_primitives_for_two_primitives() {
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("set", FieldValue::Empty),
        ("finish", FieldValue::Empty),
    ]);
    assert_eq!(
        validate_single_primitive(&step),
        Err(ValidationError::MultipleStepPrimitives { span: Span::ZERO })
    );
}

#[test]
fn validate_ids_accepts_valid_step_ids() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("my_workflow".to_owned())),
        (
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        ),
        (
            "steps",
            FieldValue::Sequence(vec![
                make_step(vec![
                    ("id", FieldValue::String("step_one".to_owned())),
                    ("finish", FieldValue::Empty),
                ]),
                make_step(vec![
                    ("id", FieldValue::String("step_two".to_owned())),
                    ("finish", FieldValue::Empty),
                ]),
            ]),
        ),
    ]);
    assert_eq!(validate_ids(&doc), Ok(()));
}

#[test]
fn validate_ids_rejects_step_id_with_spaces() {
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
                ("id", FieldValue::String("has space".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::InvalidId {
            id: "has space".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_ids_rejects_step_id_starting_with_digit() {
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
                ("id", FieldValue::String("9lead".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::InvalidId {
            id: "9lead".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_ids_rejects_step_id_with_special_chars() {
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
                ("id", FieldValue::String("bad-id".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::InvalidId {
            id: "bad-id".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_trigger_rejects_ipc_trigger() {
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
fn validate_trigger_accepts_schedule_trigger() {
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
fn validate_trigger_accepts_event_trigger() {
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
fn validate_trigger_accepts_webhook_trigger() {
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![("webhook".to_owned(), FieldValue::Empty)]),
    )]);
    assert_eq!(validate_trigger(&doc), Ok(()));
}

#[test]
fn validate_trigger_rejects_schedule_without_cron() {
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![("schedule".to_owned(), FieldValue::Mapping(vec![]))]),
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
fn validate_trigger_accepts_manual_trigger() {
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
    )]);
    assert_eq!(validate_trigger(&doc), Ok(()));
}

#[test]
fn validate_trigger_rejects_unsupported_trigger() {
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
fn validate_trigger_rejects_empty_when_mapping() {
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
fn validate_step_fields_accepts_valid_do_step() {
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("do", FieldValue::Empty),
        ])]),
    )]);
    assert_eq!(validate_step_fields(&doc), Ok(()));
}

#[test]
fn validate_step_fields_accepts_valid_set_step() {
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("set", FieldValue::Empty),
        ])]),
    )]);
    assert_eq!(validate_step_fields(&doc), Ok(()));
}

#[test]
fn validate_step_fields_accepts_master_metadata_fields() {
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("name", FieldValue::String("Step One".to_owned())),
            ("if", FieldValue::String("$input.enabled".to_owned())),
            ("with", FieldValue::Mapping(vec![])),
            ("try_again", FieldValue::Mapping(vec![])),
            ("on_error", FieldValue::String("fail".to_owned())),
            ("then", FieldValue::String("done".to_owned())),
            ("set", FieldValue::Empty),
        ])]),
    )]);
    assert_eq!(validate_step_fields(&doc), Ok(()));
}

#[test]
fn validate_step_fields_rejects_legacy_save_field() {
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("save", FieldValue::Empty),
        ])]),
    )]);
    assert_eq!(
        validate_step_fields(&doc),
        Err(ValidationError::UnknownStepField { span: Span::ZERO })
    );
}

#[test]
fn validate_step_fields_accepts_valid_branch_step() {
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("choose", FieldValue::Empty),
        ])]),
    )]);
    assert_eq!(validate_step_fields(&doc), Ok(()));
}

#[test]
fn validate_step_fields_rejects_step_without_kind() {
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![make_step(vec![(
            "id",
            FieldValue::String("s1".to_owned()),
        )])]),
    )]);
    assert_eq!(
        validate_step_fields(&doc),
        Err(ValidationError::MissingStepPrimitive { span: Span::ZERO })
    );
}

#[test]
fn validate_workflow_schema_accepts_minimal_valid_workflow() {
    let doc = valid_workflow_doc();
    assert_eq!(validate_workflow_schema(&doc), Ok(()));
}

#[test]
fn validate_workflow_schema_rejects_empty_workflow() {
    let doc = make_workflow(vec![]);
    assert_eq!(
        validate_workflow_schema(&doc),
        Err(ValidationError::MissingRequiredField {
            field: "version".to_owned(),
            span: Span::ZERO
        })
    );
}

// ---------------------------------------------------------------------------
// Accessor and query tests
// ---------------------------------------------------------------------------

#[test]
fn get_string_returns_some_for_existing_string_field() {
    let doc = make_workflow(vec![("name", FieldValue::String("hello".to_owned()))]);
    assert_eq!(doc.get_string("name"), Some("hello"));
}

#[test]
fn get_string_returns_none_for_missing_field() {
    let doc = make_workflow(vec![]);
    assert_eq!(doc.get_string("name"), None);
}

#[test]
fn get_string_returns_none_for_non_string_field() {
    let doc = make_workflow(vec![("name", FieldValue::Mapping(vec![]))]);
    assert_eq!(doc.get_string("name"), None);
}

#[test]
fn get_mapping_returns_some_for_existing_mapping() {
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
    )]);
    let result = doc.get_mapping("when");
    assert!(result.is_some());
    let Some(mapping) = result else {
        return;
    };
    assert_eq!(mapping.len(), 1);
    let Some((field, _)) = mapping.first() else {
        return;
    };
    assert_eq!(field, "manual");
}

#[test]
fn get_mapping_returns_none_for_missing_field() {
    let doc = make_workflow(vec![]);
    assert!(doc.get_mapping("when").is_none());
}

#[test]
fn get_sequence_returns_some_for_existing_sequence() {
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![make_step(vec![(
            "id",
            FieldValue::String("s1".to_owned()),
        )])]),
    )]);
    let result = doc.get_sequence("steps");
    assert!(result.is_some());
    let Some(seq) = result else {
        return;
    };
    assert_eq!(seq.len(), 1);
}

#[test]
fn get_sequence_returns_none_for_missing_field() {
    let doc = make_workflow(vec![]);
    assert!(doc.get_sequence("steps").is_none());
}

#[test]
fn has_field_returns_true_for_existing_field() {
    let doc = make_workflow(vec![("name", FieldValue::String("test".to_owned()))]);
    assert!(doc.has_field("name"));
}

#[test]
fn has_field_returns_false_for_missing_field() {
    let doc = make_workflow(vec![]);
    assert!(!doc.has_field("name"));
}

#[test]
fn get_string_with_multiple_fields_returns_correct_one() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("multi_test".to_owned())),
    ]);
    assert_eq!(doc.get_string("version"), Some("velvet-ballistics/v1"));
    assert_eq!(doc.get_string("name"), Some("multi_test"));
}

#[test]
fn get_mapping_with_nested_data_returns_correct_mapping() {
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![(
            "manual".to_owned(),
            FieldValue::String("test".to_owned()),
        )]),
    )]);
    let result = doc.get_mapping("when");
    assert!(result.is_some());
    let Some(mapping) = result else {
        return;
    };
    assert_eq!(mapping.len(), 1);
    let Some((field, value)) = mapping.first() else {
        return;
    };
    assert_eq!(field, "manual");
    let FieldValue::String(s) = value else {
        return;
    };
    assert_eq!(s, "test");
}

#[test]
fn get_sequence_with_multiple_entries_returns_correct_one() {
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![
            make_step(vec![("id", FieldValue::String("s1".to_owned()))]),
            make_step(vec![("id", FieldValue::String("s2".to_owned()))]),
        ]),
    )]);
    let result = doc.get_sequence("steps");
    assert!(result.is_some());
    let Some(seq) = result else {
        return;
    };
    assert_eq!(seq.len(), 2);
    let Some(first) = seq.first() else {
        return;
    };
    let Some(second) = seq.get(1) else {
        return;
    };
    assert_eq!(first.get_string("id"), Some("s1"));
    assert_eq!(second.get_string("id"), Some("s2"));
}

#[test]
fn field_names_returns_correct_fields_for_workflow() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("test".to_owned())),
    ]);
    let names = doc.field_names();
    assert_eq!(names, vec!["version", "name"]);
}

#[test]
fn step_doc_get_string_returns_value_for_existing_field() {
    let step = make_step(vec![("id", FieldValue::String("my_step".to_owned()))]);
    assert_eq!(step.get_string("id"), Some("my_step"));
}

#[test]
fn step_doc_get_string_returns_none_for_missing() {
    let step = make_step(vec![("finish", FieldValue::Empty)]);
    assert_eq!(step.get_string("id"), None);
}

#[test]
fn step_doc_field_names_returns_all_names() {
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("finish", FieldValue::Empty),
    ]);
    assert_eq!(step.field_names(), vec!["id", "finish"]);
}

#[test]
fn from_pairs_creates_workflow_with_given_pairs() {
    let pairs = vec![
        (
            "version".to_owned(),
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        (
            "name".to_owned(),
            FieldValue::String("roundtrip".to_owned()),
        ),
    ];
    let doc = WorkflowDoc::from_pairs(pairs);
    assert_eq!(doc.get_string("version"), Some("velvet-ballistics/v1"));
    assert_eq!(doc.get_string("name"), Some("roundtrip"));
}

#[test]
fn from_pairs_creates_step_with_given_pairs() {
    let pairs = vec![
        ("id".to_owned(), FieldValue::String("s1".to_owned())),
        ("do".to_owned(), FieldValue::Empty),
    ];
    let step = StepDoc::from_pairs(pairs);
    assert_eq!(step.get_string("id"), Some("s1"));
    assert_eq!(step.field_names(), vec!["id", "do"]);
}

// ---------------------------------------------------------------------------
// Adversarial BDD tests: validation bypass attacks
// ---------------------------------------------------------------------------

#[test]
fn adversarial_version_v2_is_rejected_as_invalid_version() {
    let doc = make_workflow(vec![(
        "version",
        FieldValue::String("velvet-ballistics/v2".to_owned()),
    )]);
    assert_eq!(
        validate_version(&doc),
        Err(ValidationError::InvalidVersion {
            version: "velvet-ballistics/v2".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_reserved_id_input_is_rejected_as_reserved() {
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
                ("id", FieldValue::String("input".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::ReservedId {
            id: "input".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_reserved_id_vars_is_rejected_as_reserved() {
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
                ("id", FieldValue::String("vars".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::ReservedId {
            id: "vars".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_reserved_id_secrets_is_rejected_as_reserved() {
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
                ("id", FieldValue::String("secrets".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::ReservedId {
            id: "secrets".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_reserved_id_steps_is_rejected_as_reserved() {
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
                ("id", FieldValue::String("steps".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::ReservedId {
            id: "steps".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_reserved_id_error_is_rejected_as_reserved() {
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
                ("id", FieldValue::String("error".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::ReservedId {
            id: "error".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_reserved_id_attempt_is_rejected_as_reserved() {
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
                ("id", FieldValue::String("attempt".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::ReservedId {
            id: "attempt".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_step_with_set_and_do_primitives_is_rejected() {
    let step = make_step(vec![
        ("id", FieldValue::String("sneaky".to_owned())),
        ("set", FieldValue::Empty),
        ("do", FieldValue::Empty),
    ]);
    assert_eq!(
        validate_single_primitive(&step),
        Err(ValidationError::MultipleStepPrimitives { span: Span::ZERO })
    );
}

#[test]
fn adversarial_step_with_choose_and_finish_primitives_is_rejected() {
    let step = make_step(vec![
        ("id", FieldValue::String("dual_action".to_owned())),
        ("choose", FieldValue::Empty),
        ("finish", FieldValue::Empty),
    ]);
    assert_eq!(
        validate_single_primitive(&step),
        Err(ValidationError::MultipleStepPrimitives { span: Span::ZERO })
    );
}

#[test]
fn adversarial_step_with_all_primitives_is_rejected() {
    let step = make_step(vec![
        ("id", FieldValue::String("greedy".to_owned())),
        ("set", FieldValue::Empty),
        ("choose", FieldValue::Empty),
        ("for_each", FieldValue::Empty),
        ("parallel", FieldValue::Empty),
        ("collect", FieldValue::Empty),
        ("aggregate", FieldValue::Empty),
        ("repeat", FieldValue::Empty),
        ("wait", FieldValue::Empty),
        ("ask", FieldValue::Empty),
        ("finish", FieldValue::Empty),
        ("do", FieldValue::Empty),
    ]);
    assert_eq!(
        validate_single_primitive(&step),
        Err(ValidationError::MultipleStepPrimitives { span: Span::ZERO })
    );
}

#[test]
fn adversarial_step_with_only_non_primitive_fields_is_rejected() {
    let step = make_step(vec![
        ("id", FieldValue::String("no_op".to_owned())),
        ("name", FieldValue::String("No Operation".to_owned())),
        ("then", FieldValue::String("next_step".to_owned())),
        ("on_error", FieldValue::Empty),
        ("retry", FieldValue::Empty),
    ]);
    assert_eq!(
        validate_single_primitive(&step),
        Err(ValidationError::MissingStepPrimitive { span: Span::ZERO })
    );
}

#[test]
fn adversarial_http_trigger_is_rejected_as_out_of_core() {
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
fn adversarial_duplicate_step_ids_in_full_workflow_is_rejected() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("dup_test".to_owned())),
        (
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        ),
        (
            "steps",
            FieldValue::Sequence(vec![
                make_step(vec![
                    ("id", FieldValue::String("clone".to_owned())),
                    ("set", FieldValue::Empty),
                ]),
                make_step(vec![
                    ("id", FieldValue::String("clone".to_owned())),
                    ("finish", FieldValue::Empty),
                ]),
            ]),
        ),
    ]);
    assert_eq!(
        validate_ids(&doc),
        Err(ValidationError::DuplicateId {
            id: "clone".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_uppercase_step_id_is_rejected() {
    assert_eq!(
        validate_single_id("MyStep", &[]),
        Err(ValidationError::InvalidId {
            id: "MyStep".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_hyphenated_step_id_is_rejected() {
    assert_eq!(
        validate_single_id("my-step", &[]),
        Err(ValidationError::InvalidId {
            id: "my-step".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_step_id_starting_with_digit_is_rejected() {
    assert_eq!(
        validate_single_id("0step", &[]),
        Err(ValidationError::InvalidId {
            id: "0step".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_empty_step_id_is_rejected() {
    assert_eq!(
        validate_single_id("", &[]),
        Err(ValidationError::InvalidId {
            id: String::new(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_multiple_triggers_are_rejected() {
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

#[test]
fn adversarial_unknown_trigger_kind_is_rejected() {
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![("timer".to_owned(), FieldValue::Empty)]),
    )]);
    assert_eq!(
        validate_trigger(&doc),
        Err(ValidationError::UnsupportedTrigger {
            trigger: "timer".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_empty_steps_sequence_is_rejected() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("empty_steps".to_owned())),
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
fn adversarial_unknown_top_level_field_webhook_is_rejected() {
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
fn adversarial_unknown_step_field_payload_is_rejected() {
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("finish", FieldValue::Empty),
            ("payload", FieldValue::Empty),
        ])]),
    )]);
    assert_eq!(
        validate_step_fields(&doc),
        Err(ValidationError::UnknownStepField { span: Span::ZERO })
    );
}

#[test]
fn adversarial_reserved_id_result_is_rejected_in_step() {
    assert_eq!(
        validate_single_id("result", &[]),
        Err(ValidationError::ReservedId {
            id: "result".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_reserved_id_when_is_rejected_in_step() {
    assert_eq!(
        validate_single_id("when", &[]),
        Err(ValidationError::ReservedId {
            id: "when".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_reserved_id_item_is_rejected_in_step() {
    assert_eq!(
        validate_single_id("item", &[]),
        Err(ValidationError::ReservedId {
            id: "item".to_owned(),
            span: Span::ZERO
        })
    );
}

#[test]
fn adversarial_step_without_id_field_is_rejected() {
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("no_id_test".to_owned())),
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
