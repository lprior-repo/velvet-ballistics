#![forbid(unsafe_code)]
//! Behavior tests for vb_validate YAML parsing validation pipeline.
//!
//! Tests schema validation behavior, validation pipeline composition,
//! and exact error type assertions for workflow document validation.
//!
//! NOTE: vb_validate does not directly parse YAML — it validates document
//! models (WorkflowDoc, FieldValue) that result from YAML parsing. These
//! tests verify the validation behavior that downstream YAML parsers rely on.

use vb_core::action::ActionContract;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_validate::ValidationError;
use vb_validate::schema::{
    FieldValue, StepDoc, WorkflowDoc, validate_ids, validate_single_primitive,
    validate_step_fields, validate_trigger, validate_version, validate_workflow_schema,
};
use vb_validate::shared::{ValidationPipeline, validate, validate_with_contracts};

// ---------------------------------------------------------------------------
// Helper constructors
// ---------------------------------------------------------------------------

fn make_workflow(fields: Vec<(&str, FieldValue)>) -> WorkflowDoc {
    WorkflowDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

fn make_step(fields: Vec<(&str, FieldValue)>) -> StepDoc {
    StepDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result_slot),
        },
    }
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

// ---------------------------------------------------------------------------
// YAML parse error detection — duplicate key detection via validate_workflow_schema
// ---------------------------------------------------------------------------

#[test]
fn validate_workflow_schema_detects_duplicate_top_level_key() {
    // Given a workflow doc with duplicate "name" keys (simulates YAML duplicate key parse)
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
    // When validate_workflow_schema is called
    let result = validate_workflow_schema(&doc);
    // Then it returns DuplicateKey error exactly
    assert_eq!(result, Err(ValidationError::DuplicateKey));
}

#[test]
fn validate_workflow_schema_detects_duplicate_step_field_key() {
    // Given a workflow doc with a step containing duplicate keys
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
                ("set", FieldValue::String("duplicate".to_owned())),
            ])]),
        ),
    ]);
    // When validate_workflow_schema is called
    let result = validate_workflow_schema(&doc);
    // Then it returns DuplicateKey before counting primitives
    assert_eq!(result, Err(ValidationError::DuplicateKey));
}

// ---------------------------------------------------------------------------
// Schema validation behavior — required field detection via validate_workflow_schema
// ---------------------------------------------------------------------------

#[test]
fn validate_workflow_schema_detects_missing_version() {
    // Given a workflow doc without "version"
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
    // When validate_workflow_schema is called
    let result = validate_workflow_schema(&doc);
    // Then it returns MissingRequiredField for "version"
    assert_eq!(
        result,
        Err(ValidationError::MissingRequiredField {
            field: "version".to_owned(),
        })
    );
}

#[test]
fn validate_workflow_schema_detects_missing_name() {
    // Given a workflow doc without "name"
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
    // When validate_workflow_schema is called
    let result = validate_workflow_schema(&doc);
    // Then it returns MissingRequiredField for "name"
    assert_eq!(
        result,
        Err(ValidationError::MissingRequiredField {
            field: "name".to_owned(),
        })
    );
}

#[test]
fn validate_workflow_schema_detects_missing_when() {
    // Given a workflow doc without "when"
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("test".to_owned())),
        (
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    // When validate_workflow_schema is called
    let result = validate_workflow_schema(&doc);
    // Then it returns MissingRequiredField for "when"
    assert_eq!(
        result,
        Err(ValidationError::MissingRequiredField {
            field: "when".to_owned(),
        })
    );
}

#[test]
fn validate_workflow_schema_detects_missing_steps() {
    // Given a workflow doc without "steps"
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
    // When validate_workflow_schema is called
    let result = validate_workflow_schema(&doc);
    // Then it returns MissingRequiredField for "steps"
    assert_eq!(
        result,
        Err(ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        })
    );
}

// ---------------------------------------------------------------------------
// Schema validation behavior — unknown field detection via validate_workflow_schema
// ---------------------------------------------------------------------------

#[test]
fn validate_workflow_schema_detects_unknown_top_level_field() {
    // Given a workflow doc with a field not in ALLOWED_TOP_LEVEL_FIELDS
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
        ("unknown_toplevel", FieldValue::Empty),
    ]);
    // When validate_workflow_schema is called
    let result = validate_workflow_schema(&doc);
    // Then it returns UnknownTopLevelField
    assert_eq!(result, Err(ValidationError::UnknownTopLevelField));
}

#[test]
fn validate_workflow_schema_rejects_bogus_field() {
    // Given a workflow doc with "bogus_field" at top level
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
    // When validate_workflow_schema is called
    let result = validate_workflow_schema(&doc);
    // Then it returns UnknownTopLevelField
    assert_eq!(result, Err(ValidationError::UnknownTopLevelField));
}

#[test]
fn validate_step_fields_detects_unknown_step_field() {
    // Given a workflow doc where a step has a field not in ALLOWED_STEP_FIELDS
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
    // When validate_step_fields is called
    let result = validate_step_fields(&doc);
    // Then it returns UnknownStepField
    assert_eq!(result, Err(ValidationError::UnknownStepField));
}

#[test]
fn validate_step_fields_rejects_legacy_save_field() {
    // Given a step using obsolete "save" instead of "set"
    let doc = make_workflow(vec![(
        "steps",
        FieldValue::Sequence(vec![make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("save", FieldValue::Empty),
        ])]),
    )]);
    // When validate_step_fields is called
    let result = validate_step_fields(&doc);
    // Then it returns UnknownStepField
    assert_eq!(result, Err(ValidationError::UnknownStepField));
}

// ---------------------------------------------------------------------------
// Schema validation behavior — version string validation
// ---------------------------------------------------------------------------

#[test]
fn validate_version_accepts_canonical_version() {
    // Given a workflow doc with the canonical version string
    let doc = make_workflow(vec![(
        "version",
        FieldValue::String("velvet-ballistics/v1".to_owned()),
    )]);
    // When validate_version is called
    let result = validate_version(&doc);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_version_rejects_missing_version() {
    // Given a workflow doc with no version field
    let doc = make_workflow(vec![("name", FieldValue::String("test".to_owned()))]);
    // When validate_version is called
    let result = validate_version(&doc);
    // Then it returns MissingRequiredField for "version"
    assert_eq!(
        result,
        Err(ValidationError::MissingRequiredField {
            field: "version".to_owned(),
        })
    );
}

#[test]
fn validate_version_rejects_wrong_version() {
    // Given a workflow doc with "not-velvet/v1"
    let doc = make_workflow(vec![(
        "version",
        FieldValue::String("not-velvet/v1".to_owned()),
    )]);
    // When validate_version is called
    let result = validate_version(&doc);
    // Then it returns InvalidVersion with exact version string
    assert_eq!(
        result,
        Err(ValidationError::InvalidVersion {
            version: "not-velvet/v1".to_owned(),
        })
    );
}

#[test]
fn validate_version_rejects_empty_version_string() {
    // Given a workflow doc with empty version string
    let doc = make_workflow(vec![("version", FieldValue::String(String::new()))]);
    // When validate_version is called
    let result = validate_version(&doc);
    // Then it returns InvalidVersion with empty string
    assert_eq!(
        result,
        Err(ValidationError::InvalidVersion {
            version: String::new(),
        })
    );
}

#[test]
fn validate_version_rejects_future_version() {
    // Given a workflow doc with a future version
    let doc = make_workflow(vec![(
        "version",
        FieldValue::String("velvet-ballistics/v99".to_owned()),
    )]);
    // When validate_version is called
    let result = validate_version(&doc);
    // Then it returns InvalidVersion
    assert_eq!(
        result,
        Err(ValidationError::InvalidVersion {
            version: "velvet-ballistics/v99".to_owned(),
        })
    );
}

// ---------------------------------------------------------------------------
// Schema validation behavior — ID validation
// ---------------------------------------------------------------------------

#[test]
fn validate_ids_accepts_valid_step_ids() {
    // Given a workflow doc with valid lowercase snake_case IDs
    let doc = make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("valid_ids".to_owned())),
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
    // When validate_ids is called
    let result = validate_ids(&doc);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_ids_rejects_step_id_starting_with_digit() {
    // Given a workflow doc with a step id starting with a digit
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
    // When validate_ids is called
    let result = validate_ids(&doc);
    // Then it returns InvalidId
    assert_eq!(
        result,
        Err(ValidationError::InvalidId {
            id: "1bad".to_owned(),
        })
    );
}

#[test]
fn validate_ids_rejects_step_id_with_dash() {
    // Given a workflow doc with a step id containing a dash
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
    // When validate_ids is called
    let result = validate_ids(&doc);
    // Then it returns InvalidId
    assert_eq!(
        result,
        Err(ValidationError::InvalidId {
            id: "bad-id".to_owned(),
        })
    );
}

#[test]
fn validate_ids_rejects_reserved_id_runtime() {
    // Given a workflow doc with step using reserved id "runtime"
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
    // When validate_ids is called
    let result = validate_ids(&doc);
    // Then it returns ReservedId
    assert_eq!(
        result,
        Err(ValidationError::ReservedId {
            id: "runtime".to_owned(),
        })
    );
}

#[test]
fn validate_ids_rejects_reserved_id_now() {
    // Given a workflow doc with step using reserved id "now"
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
                ("id", FieldValue::String("now".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    // When validate_ids is called
    let result = validate_ids(&doc);
    // Then it returns ReservedId
    assert_eq!(
        result,
        Err(ValidationError::ReservedId {
            id: "now".to_owned(),
        })
    );
}

#[test]
fn validate_ids_rejects_duplicate_step_id() {
    // Given a workflow doc with duplicate step IDs
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
            FieldValue::Sequence(vec![
                make_step(vec![
                    ("id", FieldValue::String("step1".to_owned())),
                    ("finish", FieldValue::Empty),
                ]),
                make_step(vec![
                    ("id", FieldValue::String("step1".to_owned())),
                    ("finish", FieldValue::Empty),
                ]),
            ]),
        ),
    ]);
    // When validate_ids is called
    let result = validate_ids(&doc);
    // Then it returns DuplicateId
    assert_eq!(
        result,
        Err(ValidationError::DuplicateId {
            id: "step1".to_owned(),
        })
    );
}

#[test]
fn validate_ids_rejects_too_long_id() {
    // Given a workflow doc with a step id exceeding 64 characters
    let long_id = "a".repeat(65);
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
                ("id", FieldValue::String(long_id.clone())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    // When validate_ids is called
    let result = validate_ids(&doc);
    // Then it returns InvalidId
    assert_eq!(result, Err(ValidationError::InvalidId { id: long_id }));
}

#[test]
fn validate_ids_accepts_max_length_id() {
    // Given a workflow doc with a step id at exactly 64 characters
    let max_id = "a".repeat(64);
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
                ("id", FieldValue::String(max_id.clone())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]);
    // When validate_ids is called
    let result = validate_ids(&doc);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

// ---------------------------------------------------------------------------
// Schema validation behavior — trigger validation
// ---------------------------------------------------------------------------

#[test]
fn validate_trigger_accepts_manual_trigger() {
    // Given a workflow doc with manual trigger
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
    )]);
    // When validate_trigger is called
    let result = validate_trigger(&doc);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_trigger_accepts_webhook_trigger() {
    // Given a workflow doc with webhook trigger (empty mapping)
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![("webhook".to_owned(), FieldValue::Mapping(vec![]))]),
    )]);
    // When validate_trigger is called
    let result = validate_trigger(&doc);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_trigger_accepts_schedule_trigger_with_cron() {
    // Given a workflow doc with schedule trigger containing cron
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
    // When validate_trigger is called
    let result = validate_trigger(&doc);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
#[ignore]
fn validate_trigger_accepts_event_trigger_with_name() {
    // Given a workflow doc with event trigger containing name
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
    // When validate_trigger is called
    let result = validate_trigger(&doc);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_trigger_rejects_http_trigger() {
    // Given a workflow doc with http trigger (out of core)
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![("http".to_owned(), FieldValue::Empty)]),
    )]);
    // When validate_trigger is called
    let result = validate_trigger(&doc);
    // Then it returns HttpTriggerOutOfCore
    assert_eq!(result, Err(ValidationError::HttpTriggerOutOfCore));
}

#[test]
fn validate_trigger_rejects_empty_when_mapping() {
    // Given a workflow doc with empty when mapping
    let doc = make_workflow(vec![("when", FieldValue::Mapping(vec![]))]);
    // When validate_trigger is called
    let result = validate_trigger(&doc);
    // Then it returns MissingRequiredField for "when"
    assert_eq!(
        result,
        Err(ValidationError::MissingRequiredField {
            field: "when".to_owned(),
        })
    );
}

#[test]
fn validate_trigger_rejects_unsupported_trigger() {
    // Given a workflow doc with an unsupported trigger type
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![("cron".to_owned(), FieldValue::Empty)]),
    )]);
    // When validate_trigger is called
    let result = validate_trigger(&doc);
    // Then it returns UnsupportedTrigger with exact trigger name
    assert_eq!(
        result,
        Err(ValidationError::UnsupportedTrigger {
            trigger: "cron".to_owned(),
        })
    );
}

#[test]
fn validate_trigger_rejects_event_without_name() {
    // Given a workflow doc with event trigger missing name field
    let doc = make_workflow(vec![(
        "when",
        FieldValue::Mapping(vec![("event".to_owned(), FieldValue::Mapping(vec![]))]),
    )]);
    // When validate_trigger is called
    let result = validate_trigger(&doc);
    // Then it returns UnsupportedTrigger for event
    assert_eq!(
        result,
        Err(ValidationError::UnsupportedTrigger {
            trigger: "event".to_owned(),
        })
    );
}

// ---------------------------------------------------------------------------
// Schema validation behavior — step primitive validation
// ---------------------------------------------------------------------------

#[test]
fn validate_single_primitive_accepts_finish_step() {
    // Given a step with "finish" primitive
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("finish", FieldValue::Empty),
    ]);
    // When validate_single_primitive is called
    let result = validate_single_primitive(&step);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_single_primitive_accepts_set_step() {
    // Given a step with "set" primitive
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("set", FieldValue::Empty),
    ]);
    // When validate_single_primitive is called
    let result = validate_single_primitive(&step);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_single_primitive_accepts_do_step() {
    // Given a step with "do" primitive
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("do", FieldValue::Empty),
    ]);
    // When validate_single_primitive is called
    let result = validate_single_primitive(&step);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_single_primitive_rejects_no_primitive() {
    // Given a step with no primitive field (only metadata)
    let step = make_step(vec![("id", FieldValue::String("s1".to_owned()))]);
    // When validate_single_primitive is called
    let result = validate_single_primitive(&step);
    // Then it returns MissingStepPrimitive
    assert_eq!(result, Err(ValidationError::MissingStepPrimitive));
}

#[test]
fn validate_single_primitive_rejects_multiple_primitives() {
    // Given a step with both "set" and "finish"
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("set", FieldValue::Empty),
        ("finish", FieldValue::Empty),
    ]);
    // When validate_single_primitive is called
    let result = validate_single_primitive(&step);
    // Then it returns MultipleStepPrimitives
    assert_eq!(result, Err(ValidationError::MultipleStepPrimitives));
}

#[test]
fn validate_single_primitive_accepts_metadata_fields_with_primitive() {
    // Given a step with metadata fields (if, with, then, on_error) and one primitive
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("name", FieldValue::String("Step One".to_owned())),
        ("if", FieldValue::String("$input.enabled".to_owned())),
        ("with", FieldValue::Mapping(vec![])),
        ("try_again", FieldValue::Mapping(vec![])),
        ("on_error", FieldValue::String("fail".to_owned())),
        ("then", FieldValue::String("done".to_owned())),
        ("set", FieldValue::Empty),
    ]);
    // When validate_single_primitive is called
    let result = validate_single_primitive(&step);
    // Then it returns Ok (metadata fields are not counted as primitives)
    assert_eq!(result, Ok(()));
}

// ---------------------------------------------------------------------------
// Validation pipeline composition tests
// ---------------------------------------------------------------------------

#[test]
fn validation_pipeline_default_has_all_gates_enabled() {
    // Given the default ValidationPipeline
    let pipeline = ValidationPipeline::default();
    // Then all gates are enabled
    assert!(pipeline.gate_07_expression_stack);
    assert!(pipeline.gate_08_accessor_paths);
    assert!(pipeline.gate_09_slot_references);
    assert!(pipeline.gate_10_node_kind_specific);
    assert!(pipeline.gate_11_loop_body_graph);
    assert!(pipeline.gate_12_action_contracts);
    assert!(pipeline.gate_13_no_slot_cycles);
    assert!(pipeline.gate_14_slot_type_consistency);
    assert!(pipeline.gate_15_determinism_proof);
}

#[test]
fn validation_pipeline_all_gates_creates_all_enabled() {
    // Given ValidationPipeline::all_gates()
    let pipeline = ValidationPipeline::all_gates();
    // Then all gates are enabled
    assert!(pipeline.gate_07_expression_stack);
    assert!(pipeline.gate_08_accessor_paths);
    assert!(pipeline.gate_09_slot_references);
    assert!(pipeline.gate_10_node_kind_specific);
    assert!(pipeline.gate_11_loop_body_graph);
    assert!(pipeline.gate_12_action_contracts);
    assert!(pipeline.gate_13_no_slot_cycles);
    assert!(pipeline.gate_14_slot_type_consistency);
    assert!(pipeline.gate_15_determinism_proof);
}

#[test]
fn validation_pipeline_no_gates_creates_all_disabled() {
    // Given ValidationPipeline::no_gates()
    let pipeline = ValidationPipeline::no_gates();
    // Then all gates are disabled
    assert!(!pipeline.gate_07_expression_stack);
    assert!(!pipeline.gate_08_accessor_paths);
    assert!(!pipeline.gate_09_slot_references);
    assert!(!pipeline.gate_10_node_kind_specific);
    assert!(!pipeline.gate_11_loop_body_graph);
    assert!(!pipeline.gate_12_action_contracts);
    assert!(!pipeline.gate_13_no_slot_cycles);
    assert!(!pipeline.gate_14_slot_type_consistency);
    assert!(!pipeline.gate_15_determinism_proof);
}

#[test]
fn validation_pipeline_validate_accepts_valid_parts() {
    // Given a valid WorkflowParts
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    // When validate is called via convenience function
    let result = validate(&parts);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validation_pipeline_validate_rejects_out_of_range_slot_reference() {
    // Given a WorkflowParts with slot_count=1 but slot=99 in output
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(99)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1);
    // When validate is called
    let result = validate(&parts);
    // Then it returns SlotReferenceOutOfRange error
    assert!(matches!(
        result,
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn validation_pipeline_short_circuits_on_first_error() {
    // Given a WorkflowParts that would fail gate 9 (slot ref)
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(99)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1);
    // When validate is called with default pipeline
    let result = ValidationPipeline::default().validate(&parts);
    // Then it returns the gate 9 error (first gate that would fail)
    assert!(matches!(
        result,
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn validation_pipeline_selective_gates_skip_disabled() {
    // Given a WorkflowParts that would fail gate 9
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(99)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1);
    // When validate is called with gate 9 disabled
    let pipeline = ValidationPipeline {
        gate_09_slot_referencesvb_validate::shared::GateStatus::Disabled,
        ..ValidationPipeline::no_gates()
    };
    let result = pipeline.validate(&parts);
    // Then it returns Ok (skipped the failing gate)
    assert_eq!(result, Ok(()));
}

#[test]
fn validation_pipeline_validate_with_contracts_accepts_valid_parts() {
    // Given a valid WorkflowParts and empty contracts
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    let contracts: Vec<ActionContract> = vec![];
    // When validate_with_contracts is called
    let result = validate_with_contracts(&parts, &contracts);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

// ---------------------------------------------------------------------------
// Full schema validation pipeline tests
// ---------------------------------------------------------------------------

#[test]
fn validate_workflow_schema_accepts_valid_minimal_workflow() {
    // Given a minimal valid workflow document
    let doc = valid_workflow_doc();
    // When validate_workflow_schema is called
    let result = validate_workflow_schema(&doc);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_workflow_schema_rejects_empty_workflow() {
    // Given a workflow doc with no fields
    let doc = make_workflow(vec![]);
    // When validate_workflow_schema is called
    let result = validate_workflow_schema(&doc);
    // Then it returns MissingRequiredField for first required field
    assert_eq!(
        result,
        Err(ValidationError::MissingRequiredField {
            field: "version".to_owned(),
        })
    );
}

#[test]
fn validate_workflow_schema_composes_all_validators() {
    // Given a workflow doc that passes all sub-validators
    let doc = valid_workflow_doc();
    // When validate_workflow_schema is called (which calls all validators in order)
    let result = validate_workflow_schema(&doc);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_workflow_schema_fails_on_first_error_in_sequence() {
    // The pipeline runs validators in sequence: duplicate -> required -> unknown -> version -> trigger -> ids -> step_fields
    // Given a doc with duplicate key at top level
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
    // When validate_workflow_schema is called
    let result = validate_workflow_schema(&doc);
    // Then it returns DuplicateKey (first validator that fails)
    assert_eq!(result, Err(ValidationError::DuplicateKey));
}

// ---------------------------------------------------------------------------
// Exact error type assertions for all ValidationError variants
// ---------------------------------------------------------------------------

#[test]
fn validation_error_duplicate_key_display() {
    let err = ValidationError::DuplicateKey;
    assert_eq!(format!("{}", err), "DUPLICATE_KEY");
}

#[test]
fn validation_error_unknown_top_level_field_display() {
    let err = ValidationError::UnknownTopLevelField;
    assert_eq!(format!("{}", err), "UNKNOWN_TOP_LEVEL_FIELD");
}

#[test]
fn validation_error_unknown_step_field_display() {
    let err = ValidationError::UnknownStepField;
    assert_eq!(format!("{}", err), "UNKNOWN_STEP_FIELD");
}

#[test]
fn validation_error_missing_required_field_with_field() {
    let err = ValidationError::MissingRequiredField {
        field: "version".to_owned(),
    };
    assert_eq!(format!("{}", err), "MISSING_REQUIRED_FIELD: version");
}

#[test]
fn validation_error_invalid_version_with_version() {
    let err = ValidationError::InvalidVersion {
        version: "v2.0".to_owned(),
    };
    assert_eq!(format!("{}", err), "INVALID_VERSION: v2.0");
}

#[test]
fn validation_error_invalid_id_with_id() {
    let err = ValidationError::InvalidId {
        id: "bad-id".to_owned(),
    };
    assert_eq!(format!("{}", err), "INVALID_ID: bad-id");
}

#[test]
fn validation_error_reserved_id_with_id() {
    let err = ValidationError::ReservedId {
        id: "runtime".to_owned(),
    };
    assert_eq!(format!("{}", err), "RESERVED_ID: runtime");
}

#[test]
fn validation_error_duplicate_id_with_id() {
    let err = ValidationError::DuplicateId {
        id: "step1".to_owned(),
    };
    assert_eq!(format!("{}", err), "DUPLICATE_ID: step1");
}

#[test]
fn validation_error_multiple_step_primitives_display() {
    let err = ValidationError::MultipleStepPrimitives;
    assert_eq!(format!("{}", err), "MULTIPLE_STEP_PRIMITIVES");
}

#[test]
fn validation_error_missing_step_primitive_display() {
    let err = ValidationError::MissingStepPrimitive;
    assert_eq!(format!("{}", err), "MISSING_STEP_PRIMITIVE");
}

#[test]
fn validation_error_unsupported_trigger_with_trigger() {
    let err = ValidationError::UnsupportedTrigger {
        trigger: "http".to_owned(),
    };
    assert_eq!(format!("{}", err), "UNSUPPORTED_TRIGGER: http");
}

#[test]
fn validation_error_http_trigger_out_of_core_display() {
    let err = ValidationError::HttpTriggerOutOfCore;
    assert_eq!(format!("{}", err), "HTTP_TRIGGER_OUT_OF_CORE");
}

#[test]
fn validation_error_slot_reference_out_of_range_display() {
    let err = ValidationError::SlotReferenceOutOfRange {
        slot: 5,
        slot_count: 3,
        context: "output".to_owned(),
    };
    assert_eq!(
        format!("{}", err),
        "SLOT_REFERENCE_OUT_OF_RANGE: slot 5, slot_count 3, context output"
    );
}
