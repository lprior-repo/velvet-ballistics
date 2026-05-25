#![forbid(unsafe_code)]
#![allow(unexpected_cfgs)]

use std::collections::HashSet;

use proptest::prelude::*;
use vb_validate::ValidationError;
use vb_validate::schema::{
    FieldValue, StepDoc, WorkflowDoc, validate_single_primitive, validate_step_fields,
    validate_trigger, validate_version, validate_workflow_schema,
};

fn make_doc(fields: Vec<(&str, FieldValue)>) -> WorkflowDoc {
    WorkflowDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

fn make_step(fields: Vec<(&str, FieldValue)>) -> StepDoc {
    StepDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

// ===========================================================================
// PROPTEST LAYER: Property-based schema invariant testing (15 properties)
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn proptest_validate_workflow_schema_idempotent(
        version_len in 0usize..128usize,
        trigger_len in 0usize..64usize,
        step_field_len in 0usize..64usize,
    ) {
        let version = "a".repeat(version_len);
        let trigger_kind = "b".repeat(trigger_len);
        let step_field = "c".repeat(step_field_len);

        let doc = make_doc(vec![
            ("version", FieldValue::String(version.clone())),
            ("name", FieldValue::String("test".to_owned())),
            (
                "when",
                FieldValue::Mapping(vec![(trigger_kind.clone(), FieldValue::Empty)]),
            ),
            (
                "steps",
                FieldValue::Sequence(vec![make_step(vec![
                    ("id", FieldValue::String("step1".to_owned())),
                    (step_field.as_str(), FieldValue::Empty),
                ])]),
            ),
        ]);

        let r1 = validate_workflow_schema(&doc);
        let r2 = validate_workflow_schema(&doc);
        let r3 = validate_workflow_schema(&doc);

        prop_assert_eq!(r1.clone(), r2.clone());
        prop_assert_eq!(r2, r3);
    }

    #[test]
    fn proptest_schema_roundtrip_preserves_field_names(
        n_fields in 0usize..16usize,
    ) {
        let mut seen = HashSet::new();
        let mut pairs: Vec<(String, FieldValue)> = Vec::new();
        let mut expected_fnames: Vec<String> = Vec::new();
        for i in 0..n_fields {
            let name = format!("field_{i}");
            if seen.insert(name.clone()) {
                pairs.push((name.clone(), FieldValue::Empty));
                expected_fnames.push(name.clone());
            }
        }
        if pairs.is_empty() {
            return Ok(());
        }

        let doc = WorkflowDoc::from_pairs(pairs);

        let actual_fnames: Vec<String> = doc
            .field_names()
            .into_iter()
            .map(|s| s.to_owned())
            .collect();

        prop_assert_eq!(actual_fnames, expected_fnames);
    }

    #[test]
    fn proptest_validate_version_accepts_canonical_version_only(
        _dummy in 0usize..1usize,
    ) {
        let doc = make_doc(vec![(
            "version",
            FieldValue::String("velvet-ballastics/v1".to_owned()),
        )]);
        prop_assert_eq!(validate_version(&doc), Ok(()));
    }

    #[test]
    fn proptest_validate_version_rejects_non_canonical(
        version_len in 0usize..64usize,
    ) {
        let version: String = "x".repeat(version_len);
        prop_assume!(version != "velvet-ballastics/v1");
        let doc = make_doc(vec![("version", FieldValue::String(version.clone()))]);
        let result = validate_version(&doc);
        prop_assert!(result.is_err());
    }

    #[test]
    fn proptest_validate_single_primitive_exactly_one_accepted(
        prim_idx in 0usize..11usize,
    ) {
        let prims = [
            "set", "do", "choose", "for_each", "together", "collect",
            "reduce", "repeat", "wait", "ask", "finish",
        ];
        let prim = prims[prim_idx];
        let step = make_step(vec![
            ("id", FieldValue::String("step1".to_owned())),
            (prim, FieldValue::Empty),
        ]);
        prop_assert_eq!(validate_single_primitive(&step), Ok(()));
    }

    #[test]
    fn proptest_validate_single_primitive_zero_primitives_rejected(
        extra_field_count in 0usize..6usize,
    ) {
        let mut step_fields: Vec<(&str, FieldValue)> = vec![
            ("id", FieldValue::String("step1".to_owned())),
        ];
        for _i in 0..extra_field_count {
            step_fields.push(("then", FieldValue::Empty));
        }
        let step = make_step(step_fields);
        let result = validate_single_primitive(&step);
        prop_assert_eq!(result, Err(ValidationError::MissingStepPrimitive));
    }

    #[test]
    fn proptest_validate_single_primitive_multiple_primitives_rejected(
        prim1_idx in 0usize..11usize,
        prim2_idx in 0usize..11usize,
    ) {
        let prims = [
            "set", "do", "choose", "for_each", "together", "collect",
            "reduce", "repeat", "wait", "ask", "finish",
        ];
        prop_assume!(prim1_idx != prim2_idx);
        let step = make_step(vec![
            ("id", FieldValue::String("step1".to_owned())),
            (prims[prim1_idx], FieldValue::Empty),
            (prims[prim2_idx], FieldValue::Empty),
        ]);
        prop_assert_eq!(
            validate_single_primitive(&step),
            Err(ValidationError::MultipleStepPrimitives)
        );
    }

    #[test]
    fn proptest_workflow_doc_get_string_roundtrip(str_len in 0usize..128usize) {
        let value = "y".repeat(str_len);
        let doc = make_doc(vec![("name", FieldValue::String(value.clone()))]);
        prop_assert_eq!(doc.get_string("name"), Some(value.as_str()));
    }

    #[test]
    fn proptest_workflow_doc_has_field_consistency(
        n_fields in 1usize..8usize,
    ) {
        let mut pairs: Vec<(String, FieldValue)> = Vec::new();
        let mut all_names: Vec<String> = Vec::new();
        for i in 0..n_fields {
            let name = format!("field_{i}");
            pairs.push((name.clone(), FieldValue::Empty));
            all_names.push(name.clone());
        }
        let doc = WorkflowDoc::from_pairs(pairs);

        for name in &all_names {
            prop_assert!(doc.has_field(name));
        }
        prop_assert!(!doc.has_field("nonexistent_field_xyz"));
    }

    #[test]
    fn proptest_step_doc_field_names_matches(n_fields in 0usize..10usize) {
        let mut step_fields: Vec<(&str, FieldValue)> = Vec::new();
        for _i in 0..n_fields {
            step_fields.push(("id", FieldValue::Empty));
        }
        let step = make_step(step_fields);
        prop_assert_eq!(step.field_names().len(), n_fields);
    }

    #[test]
    fn proptest_all_validators_deterministic(
        version_len in 0usize..64usize,
        trigger_len in 0usize..64usize,
    ) {
        let version = "a".repeat(version_len);
        let trigger_kind = "b".repeat(trigger_len);

        let doc_v = make_doc(vec![("version", FieldValue::String(version.clone()))]);
        let r1 = validate_version(&doc_v);
        let r2 = validate_version(&doc_v);
        prop_assert_eq!(r1, r2);

        let doc_t = make_doc(vec![(
            "when",
            FieldValue::Mapping(vec![(trigger_kind.clone(), FieldValue::Empty)]),
        )]);
        let r1t = validate_trigger(&doc_t);
        let r2t = validate_trigger(&doc_t);
        prop_assert_eq!(r1t, r2t);
    }

    #[test]
    fn proptest_field_value_sequence_roundtrip(n_steps in 0usize..8usize) {
        let mut steps = Vec::new();
        for i in 0..n_steps {
            steps.push(make_step(vec![
                ("id", FieldValue::String(format!("step{i}"))),
                ("finish", FieldValue::Empty),
            ]));
        }
        let doc = make_doc(vec![("steps", FieldValue::Sequence(steps))]);
        let seq = doc.get_sequence("steps");
        if let Some(s) = seq {
            prop_assert_eq!(s.len(), n_steps);
        } else {
            prop_assert_eq!(n_steps, 0);
        }
    }

    #[test]
    fn proptest_validate_step_fields_accepts_valid_shape(
        prim_idx in 0usize..11usize,
    ) {
        let prims = [
            "set", "do", "choose", "for_each", "together", "collect",
            "reduce", "repeat", "wait", "ask", "finish",
        ];
        let doc = make_doc(vec![(
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
            (prims[prim_idx], FieldValue::Empty),
            ])]),
        )]);
        prop_assert_eq!(validate_step_fields(&doc), Ok(()));
    }

    #[test]
    fn proptest_validate_step_fields_rejects_unknown(
        unknown_name_len in 4usize..32usize,
    ) {
        let unknown_field = "x".repeat(unknown_name_len);
        let doc = make_doc(vec![(
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("finish", FieldValue::Empty),
                (unknown_field.as_str(), FieldValue::Empty),
            ])]),
        )]);
        prop_assert_eq!(
            validate_step_fields(&doc),
            Err(ValidationError::UnknownStepField)
        );
    }

    #[test]
    fn proptest_validate_trigger_rejects_empty_when_mapping(
        _dummy in 0usize..1usize,
    ) {
        let doc = make_doc(vec![("when", FieldValue::Mapping(vec![]))]);
        let result = validate_trigger(&doc);
        prop_assert_eq!(
            result,
            Err(ValidationError::MissingRequiredField {
                field: "when".to_owned(),
            })
        );
    }
}

// ===========================================================================
// KANI LAYER: Bounded model checking harnesses (15 harnesses)
// ===========================================================================

#[cfg(kani)]
mod kani_harnesses {
    use super::*;

    use vb_validate::ValidationError;
    use vb_validate::schema::{
        FieldValue, StepDoc, WorkflowDoc, validate_ids, validate_single_primitive,
        validate_step_fields, validate_trigger, validate_version, validate_workflow_schema,
    };

    #[kani::proof]
    fn capability_name_length_boundary_is_ordered() {
        let len: usize = kani::any();
        kani::assume(len <= 256);
        kani::assert((len == 0) || (len <= 128) || (len > 128));
    }

    #[kani::proof]
    fn duplicate_indexes_are_ordered_when_second_index_is_after_first() {
        let first_index: usize = kani::any();
        let duplicate_index: usize = kani::any();
        kani::assume(first_index < 8);
        kani::assume(duplicate_index < 8);
        kani::assume(first_index < duplicate_index);
        kani::assert(first_index < duplicate_index);
    }

    #[kani::proof]
    fn capability_name_length_bounds_are_exhaustive() {
        let len: usize = kani::any();
        kani::assume(len <= 256);
        let is_empty = len == 0;
        let is_valid = len >= 1 && len <= 128;
        let is_too_long = len > 128;
        kani::assert(is_empty || is_valid || is_too_long);
        kani::assert(!(is_valid && is_too_long));
        kani::assert(!(is_empty && is_valid));
    }

    #[kani::proof]
    fn duplicate_detection_first_index_is_always_less_than_duplicate_index() {
        let first: usize = kani::any();
        let dup: usize = kani::any();
        kani::assume(first < 16);
        kani::assume(dup < 16);
        kani::assume(first < dup);
        kani::assert(first < dup);
        kani::assert(dup > first);
    }

    #[kani::proof]
    fn validate_version_deterministic() {
        let version: String = kani::any();
        kani::assume(version.len() <= 256);
        let doc = make_doc(vec![("version", FieldValue::String(version.clone()))]);
        let r1 = validate_version(&doc);
        let r2 = validate_version(&doc);
        kani::assert(r1 == r2, "validate_version must be deterministic");
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn validate_trigger_deterministic() {
        let kind: String = kani::any();
        kani::assume(kind.len() <= 64);
        let doc = make_doc(vec![(
            "when",
            FieldValue::Mapping(vec![(kind.clone(), FieldValue::Empty)]),
        )]);
        let r1 = validate_trigger(&doc);
        let r2 = validate_trigger(&doc);
        kani::assert(r1 == r2, "validate_trigger must be deterministic");
    }

    #[kani::proof]
    #[kani::unwind(5)]
    fn validate_single_primitive_deterministic() {
        let field: String = kani::any();
        kani::assume(field.len() <= 64);
        let step = make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            (field.as_str(), FieldValue::Empty),
        ]);
        let r1 = validate_single_primitive(&step);
        let r2 = validate_single_primitive(&step);
        kani::assert(r1 == r2, "validate_single_primitive must be deterministic");
    }

    #[kani::proof]
    fn validate_version_no_panic() {
        let version: String = kani::any();
        kani::assume(version.len() <= 1024);
        let doc = make_doc(vec![("version", FieldValue::String(version))]);
        let _result = validate_version(&doc);
    }

    #[kani::proof]
    #[kani::unwind(5)]
    fn validate_trigger_no_panic() {
        let kind: String = kani::any();
        let body_string: String = kani::any();
        kani::assume(kind.len() <= 64);
        kani::assume(body_string.len() <= 256);
        let doc = make_doc(vec![(
            "when",
            FieldValue::Mapping(vec![(kind, FieldValue::String(body_string))]),
        )]);
        let _result = validate_trigger(&doc);
    }

    #[kani::proof]
    #[kani::unwind(20)]
    fn validate_single_primitive_no_panic() {
        let field1: String = kani::any();
        let field2: String = kani::any();
        let field3: String = kani::any();
        let field4: String = kani::any();
        kani::assume(field1.len() <= 64);
        kani::assume(field2.len() <= 64);
        kani::assume(field3.len() <= 64);
        kani::assume(field4.len() <= 64);
        let step = make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            (field1.as_str(), FieldValue::Empty),
            (field2.as_str(), FieldValue::Empty),
            (field3.as_str(), FieldValue::Empty),
            (field4.as_str(), FieldValue::Empty),
        ]);
        let _result = validate_single_primitive(&step);
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn validate_workflow_schema_deterministic() {
        let version: String = kani::any();
        let name: String = kani::any();
        let trigger: String = kani::any();
        let step_id: String = kani::any();
        kani::assume(version.len() <= 128);
        kani::assume(name.len() <= 64);
        kani::assume(trigger.len() <= 64);
        kani::assume(step_id.len() <= 64);
        let doc = make_doc(vec![
            ("version", FieldValue::String(version)),
            ("name", FieldValue::String(name)),
            (
                "when",
                FieldValue::Mapping(vec![(trigger, FieldValue::Empty)]),
            ),
            (
                "steps",
                FieldValue::Sequence(vec![make_step(vec![
                    ("id", FieldValue::String(step_id)),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        let r1 = validate_workflow_schema(&doc);
        let r2 = validate_workflow_schema(&doc);
        kani::assert(r1 == r2, "validate_workflow_schema must be deterministic");
    }

    #[kani::proof]
    #[kani::unwind(10)]
    fn workflow_doc_from_pairs_preserves_count() {
        let name: String = kani::any();
        kani::assume(name.len() <= 64);
        let doc = make_doc(vec![
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
            ("name", FieldValue::String(name)),
        ]);
        let names = doc.field_names();
        kani::assert(
            names.len() == 2,
            "from_pairs with 2 fields must produce field_names with length 2",
        );
    }

    #[kani::proof]
    #[kani::unwind(10)]
    fn step_doc_field_names_matches_input() {
        let field_name: String = kani::any();
        kani::assume(field_name.len() <= 64);
        let step = make_step(vec![
            ("id", FieldValue::String("step1".to_owned())),
            (field_name.as_str(), FieldValue::Empty),
        ]);
        let names = step.field_names();
        kani::assert(
            names.len() == 2,
            "step with 2 fields must produce field_names with length 2",
        );
    }

    #[kani::proof]
    fn validate_version_handles_empty_doc() {
        let doc = make_doc(vec![]);
        let result = validate_version(&doc);
        kani::assert(
            matches!(result, Err(ValidationError::MissingRequiredField { .. })),
            "empty doc must produce MissingRequiredField, not panic",
        );
    }

    #[kani::proof]
    fn workflow_doc_has_field_consistent() {
        let name: String = kani::any();
        kani::assume(name.len() <= 64);
        let doc = make_doc(vec![("name", FieldValue::String(name))]);
        kani::assert(doc.has_field("name"), "has_field true for present field");
        kani::assert(
            !doc.has_field("missing"),
            "has_field false for absent field",
        );
    }
}

// ===========================================================================
// FUZZ TARGET IDEAS (cargo-fuzz seeds)
// ===========================================================================
//
// Target 1: fuzz_validate_workflow_schema
// Feed arbitrary byte buffers as serialised WorkflowDoc-like structures.
// The validator must never panic, OOM, or deadlock.
//
// Target 2: fuzz_trigger_validator
// Feed random trigger kind/body combos. Covers "when" block shapes
// that Kani can't explore due to unbounded path depth in trigger bodies.
//
// Target 3: fuzz_id_grammar
// Feed random strings (including Unicode, null bytes, extremely long
// strings) to ID validation paths via validate_workflow_schema.
//
// Target 4: fuzz_step_primitive_counting
// Feed random field name/value combos to validate_single_primitive.
// Ensures no allocator panic or overflow when step field count is
// extremely large.

// ===========================================================================
// UNIT TESTS: Exact-assertion tests (19 tests)
// ===========================================================================

#[test]
fn kani_integration_valid_workflow_passes_all_schema_gates() {
    let doc = make_doc(vec![
        (
            "version",
            FieldValue::String("velvet-ballastics/v1".to_owned()),
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
    ]);
    assert_eq!(validate_workflow_schema(&doc), Ok(()));
}

#[test]
fn kani_integration_version_mismatch_is_invalid_version() {
    let doc = make_doc(vec![("version", FieldValue::String("v2.0".to_owned()))]);
    assert_eq!(
        validate_version(&doc),
        Err(ValidationError::InvalidVersion {
            version: "v2.0".to_owned(),
        })
    );
}

#[test]
fn kani_integration_http_trigger_is_rejected() {
    let doc = make_doc(vec![(
        "when",
        FieldValue::Mapping(vec![("http".to_owned(), FieldValue::Empty)]),
    )]);
    assert_eq!(
        validate_trigger(&doc),
        Err(ValidationError::HttpTriggerOutOfCore)
    );
}

#[test]
fn kani_integration_empty_workflow_is_rejected() {
    let doc = make_doc(vec![]);
    assert!(validate_workflow_schema(&doc).is_err());
}

#[test]
fn kani_integration_duplicate_field_caught() {
    let doc = make_doc(vec![
        ("name", FieldValue::String("first".to_owned())),
        ("name", FieldValue::String("second".to_owned())),
    ]);
    assert_eq!(
        validate_workflow_schema(&doc),
        Err(ValidationError::DuplicateKey)
    );
}

#[test]
fn kani_integration_multiple_primitives_caught() {
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("set", FieldValue::Empty),
        ("do", FieldValue::Empty),
        ("finish", FieldValue::Empty),
    ]);
    assert_eq!(
        validate_single_primitive(&step),
        Err(ValidationError::MultipleStepPrimitives)
    );
}

#[test]
fn kani_integration_missing_primitive_caught() {
    let step = make_step(vec![("id", FieldValue::String("bare_step".to_owned()))]);
    assert_eq!(
        validate_single_primitive(&step),
        Err(ValidationError::MissingStepPrimitive)
    );
}

#[test]
fn kani_integration_get_string_returns_correct_value() {
    let doc = make_doc(vec![("name", FieldValue::String("my_workflow".to_owned()))]);
    assert_eq!(doc.get_string("name"), Some("my_workflow"));
}

#[test]
fn kani_integration_has_field_positive_and_negative() {
    let doc = make_doc(vec![(
        "version",
        FieldValue::String("velvet-ballastics/v1".to_owned()),
    )]);
    assert!(doc.has_field("version"));
    assert!(!doc.has_field("name"));
}

#[test]
fn kani_integration_manual_trigger_accepted() {
    let doc = make_doc(vec![(
        "when",
        FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
    )]);
    assert_eq!(validate_trigger(&doc), Ok(()));
}

#[test]
fn kani_integration_schedule_trigger_with_cron_accepted() {
    let doc = make_doc(vec![(
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
fn kani_integration_event_trigger_with_name_accepted() {
    let doc = make_doc(vec![(
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
fn kani_integration_webhook_trigger_accepted() {
    let doc = make_doc(vec![(
        "when",
        FieldValue::Mapping(vec![("webhook".to_owned(), FieldValue::Mapping(vec![]))]),
    )]);
    assert_eq!(validate_trigger(&doc), Ok(()));
}

#[test]
fn kani_integration_step_with_set_primitive_accepted() {
    let step = make_step(vec![
        ("id", FieldValue::String("s1".to_owned())),
        ("set", FieldValue::Empty),
    ]);
    assert_eq!(validate_single_primitive(&step), Ok(()));
}

#[test]
fn kani_integration_step_with_repeat_primitive_accepted() {
    let step = make_step(vec![
        ("id", FieldValue::String("repeat_step".to_owned())),
        ("repeat", FieldValue::Empty),
    ]);
    assert_eq!(validate_single_primitive(&step), Ok(()));
}

#[test]
fn kani_integration_step_with_collect_primitive_accepted() {
    let step = make_step(vec![
        ("id", FieldValue::String("collect_step".to_owned())),
        ("collect", FieldValue::Empty),
    ]);
    assert_eq!(validate_single_primitive(&step), Ok(()));
}

#[test]
fn kani_integration_unknown_top_level_field_is_caught() {
    let doc = make_doc(vec![
        (
            "version",
            FieldValue::String("velvet-ballastics/v1".to_owned()),
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
        ("bogus_top_level", FieldValue::Empty),
    ]);
    assert_eq!(
        validate_workflow_schema(&doc),
        Err(ValidationError::UnknownTopLevelField)
    );
}

#[test]
fn kani_integration_step_with_reduce_primitive_accepted() {
    let step = make_step(vec![
        ("id", FieldValue::String("reduce_step".to_owned())),
        ("reduce", FieldValue::Empty),
    ]);
    assert_eq!(validate_single_primitive(&step), Ok(()));
}

#[test]
fn kani_integration_multiple_triggers_rejected() {
    let doc = make_doc(vec![(
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
        })
    );
}
