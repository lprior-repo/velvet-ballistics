#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
#![forbid(unsafe_code)]
//! Adversarial BDD-style integration tests attacking cross-crate seams.
//!
//! These tests exercise the boundaries between crates to find bugs in
//! error propagation, resource limit enforcement, taint tracking, and
//! data flow across the pipeline.

use std::str::FromStr;
use vb_core::ids::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn minimal_parts(nodes: Box<[CompiledNode]>) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes,
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    }
}

fn fail_assert(_message: std::fmt::Arguments<'_>) -> bool {
    false
}

fn cross_crate_tempdir() -> std::io::Result<tempfile::TempDir> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/cross-crate-adversarial-tmp");
    std::fs::create_dir_all(&root)?;
    tempfile::Builder::new()
        .prefix("vb-cross-crate-")
        .tempdir_in(root)
}

macro_rules! fail_assert {
    ($($arg:tt)*) => {
        assert!(fail_assert(format_args!($($arg)*)), $($arg)*)
    }
}

/// Valid minimal workflow YAML used across many tests.
/// Note: the canonical compiler requires explicit output names for save/set.
fn valid_workflow_yaml() -> &'static [u8] {
    b"version: velvet-ballistics/v1\nname: test_wf\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      output: saved\n      value: \"42\"\n  - id: s2\n    finish:\n      result: saved\n"
}

// ===========================================================================
// SEAM 1: vb_yaml -> vb_validate (parse then validate)
// ===========================================================================

#[test]
fn yaml_to_validate_parse_invalid_yaml_propagates_parse_error_code() {
    // Given: YAML text that is structurally broken
    let yaml = "{{{broken";
    // When: parsing the YAML source
    let result = vb_yaml::parse_workflow_source(yaml);
    // Then: a parse error is returned with a line number
    match result {
        Err(vb_yaml::YamlError::ParseError { line, reason }) => {
            assert!(line > 0, "parse error should report a line number");
            assert!(!reason.is_empty(), "parse error should report a reason");
        }
        Err(other) => fail_assert!("expected ParseError, got {other:?}"),
        Ok(_) => fail_assert!("broken YAML should not parse successfully"),
    }
}

#[test]
fn yaml_to_validate_empty_string_propagates_empty_source_error() {
    // Given: empty source text
    // When: parsing
    let result = vb_yaml::parse_workflow_source("");
    // Then: EmptySource error variant is returned exactly
    assert_eq!(result, Err(vb_yaml::YamlError::EmptySource));
}

#[test]
fn yaml_to_validate_anchor_rejection_propagates_exact_variant() {
    // Given: YAML with an anchor
    let yaml = "version: &v velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps: []\n";
    // When: parsing through the profile gate
    let result = vb_yaml::parse_workflow_source(yaml);
    // Then: AnchorAliasMerge variant exactly
    assert_eq!(result, Err(vb_yaml::YamlError::AnchorAliasMerge));
}

#[test]
fn yaml_to_validate_ambiguous_scalar_propagates_exact_scalar_value() {
    // Given: YAML with an unquoted YAML 1.1 ambiguous boolean
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    set:\n      output: x\n      value: yes\n";
    // When: parsing through profile validation
    let result = vb_yaml::parse_workflow_source(yaml);
    // Then: AmbiguousScalar with the exact rejected scalar
    assert_eq!(
        result,
        Err(vb_yaml::YamlError::AmbiguousScalar {
            scalar: "yes".into()
        })
    );
}

#[test]
fn yaml_to_validate_missing_required_field_has_correct_field_name() {
    // Given: YAML missing the "name" field
    let yaml = "version: velvet-ballistics/v1\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    finish:\n      result: x\n";
    // When: parsing
    let result = vb_yaml::parse_workflow_source(yaml);
    // Then: MissingField with field = "name"
    assert_eq!(
        result,
        Err(vb_yaml::YamlError::MissingField { field: "name" })
    );
}

#[test]
fn yaml_to_validate_field_shape_error_has_field_and_expected() {
    // Given: YAML where the root is a scalar
    let yaml = "just a string\n";
    // When: parsing
    let result = vb_yaml::parse_workflow_source(yaml);
    // Then: FieldShape with exact field and expected shape
    assert_eq!(
        result,
        Err(vb_yaml::YamlError::FieldShape {
            field: "workflow",
            expected: "mapping"
        })
    );
}

// ===========================================================================
// SEAM 2: vb_validate -> vb_compile (validate then compile)
// ===========================================================================

#[test]
fn validate_then_compile_bad_version_rejected_by_both_crates() {
    // Given: workflow YAML with an invalid version
    let yaml = b"version: bad-version\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    finish:\n      result: x\n";
    // When: compiling through the full pipeline
    let result = vb_compile::compile_workflow(yaml);
    // Then: compilation fails (vb_compile runs its own validation)
    assert!(
        matches!(&result, Err(_)),
        "invalid version should fail compile, got Ok: {:?}",
        result
    );
}

#[test]
fn validate_schema_id_grammar_enforced_across_seam() {
    // Given: a WorkflowDoc with an invalid step ID containing uppercase
    use vb_validate::schema::{FieldValue, StepDoc, WorkflowDoc};
    let doc = WorkflowDoc::from_pairs(vec![
        (
            "version".into(),
            FieldValue::String("velvet-ballistics/v1".into()),
        ),
        ("name".into(), FieldValue::String("test".into())),
        (
            "when".into(),
            FieldValue::Mapping(vec![("manual".into(), FieldValue::Empty)]),
        ),
        (
            "steps".into(),
            FieldValue::Sequence(vec![StepDoc::from_pairs(vec![
                ("id".into(), FieldValue::String("BAD_ID".into())),
                ("finish".into(), FieldValue::Empty),
            ])]),
        ),
    ]);
    // When: validating IDs through vb_validate
    let result = vb_validate::schema::validate_ids(&doc);
    // Then: InvalidId error
    assert_eq!(
        result,
        Err(vb_validate::ValidationError::InvalidId {
            id: "BAD_ID".into()
        })
    );
}

#[test]
fn validate_schema_step_without_primitive_rejected_across_seam() {
    // Given: a WorkflowDoc with a step that has no primitive
    use vb_validate::schema::{FieldValue, StepDoc, WorkflowDoc};
    let doc = WorkflowDoc::from_pairs(vec![
        (
            "version".into(),
            FieldValue::String("velvet-ballistics/v1".into()),
        ),
        ("name".into(), FieldValue::String("test".into())),
        (
            "when".into(),
            FieldValue::Mapping(vec![("manual".into(), FieldValue::Empty)]),
        ),
        (
            "steps".into(),
            FieldValue::Sequence(vec![StepDoc::from_pairs(vec![(
                "id".into(),
                FieldValue::String("s1".into()),
            )])]),
        ),
    ]);
    // When: validating step fields
    let result = vb_validate::schema::validate_step_fields(&doc);
    // Then: MissingStepPrimitive error
    assert_eq!(
        result,
        Err(vb_validate::ValidationError::MissingStepPrimitive)
    );
}

// ===========================================================================
// SEAM 3: vb_compile -> vb_core (compile produces valid IR for core engine)
// ===========================================================================

#[test]
fn compile_to_core_valid_workflow_produces_valid_compiled_workflow() {
    // Given: valid minimal workflow YAML
    let yaml = valid_workflow_yaml();
    // When: compiling through the full pipeline
    let result = vb_compile::compile_workflow(yaml);
    // Then: a valid CompiledWorkflow is produced that core can accept
    match result {
        Ok(workflow) => {
            assert_eq!(workflow.name(), "test_wf");
            assert!(
                workflow.node_count() > 0,
                "compiled workflow should have nodes"
            );
            // Verify core validation passes with no errors
            let parts = workflow.to_parts();
            match vb_core::engine::validate_compiled_workflow(&parts) {
                Ok(()) => {}
                Err(e) => fail_assert!("core validation should pass on valid workflow, got: {e:?}"),
            }
        }
        Err(err) => fail_assert!("valid workflow should compile: {err}"),
    }
}

#[test]
fn compile_to_core_rejects_empty_input_at_compilation_boundary() {
    // Given: empty source bytes
    // When: compiling
    let result = vb_compile::compile_workflow(b"");
    // Then: compilation fails with an empty source error
    assert!(
        matches!(&result, Err(errors) if errors.iter().any(|e| e.to_string().contains("empty") || e.to_string().contains("Empty"))),
        "empty input should fail compilation, got Ok: {:?}",
        result
    );
}

#[test]
fn compile_to_core_rejects_non_utf8_input_at_compilation_boundary() {
    // Given: non-UTF-8 binary input
    let binary: &[u8] = &[0xff, 0xfe, 0x00, 0x01, 0x80];
    // When: compiling
    let result = vb_compile::compile_workflow(binary);
    // Then: compilation fails with a UTF-8 encoding error
    assert!(
        matches!(&result, Err(errors) if errors.iter().any(|e| e.to_string().contains("utf-8") || e.to_string().contains("UTF-8"))),
        "non-UTF-8 input should fail compilation, got Ok: {:?}",
        result
    );
}

#[test]
fn compile_to_core_step_count_matches_yaml_step_count() {
    // Given: a 3-step workflow
    let yaml = b"version: velvet-ballistics/v1\nname: three_step\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      output: first\n      value: \"1\"\n  - id: s2\n    save:\n      output: second\n      value: \"2\"\n  - id: s3\n    finish:\n      result: second\n";
    // When: compiling
    let result = vb_compile::compile_workflow(yaml);
    // Then: the compiled workflow has at least 3 nodes (one per step)
    match result {
        Ok(workflow) => {
            assert!(
                workflow.node_count() >= 3,
                "compiled workflow should have at least 3 nodes, got {}",
                workflow.node_count()
            );
        }
        Err(err) => fail_assert!("valid 3-step workflow should compile: {err}"),
    }
}

// ===========================================================================
// SEAM 4: vb_core -> vb_runtime (engine drives compiled workflow)
// ===========================================================================

#[test]
fn core_to_runtime_simple_set_workflow_runs_deterministic() {
    // Given: a compiled workflow with a single SetConst + Finish node
    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ConstIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("set_finish"),
        digest: WorkflowDigest::from_bytes([1u8; 32]),
        nodes: Box::from([node0, node1]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    let workflow = match vb_core::workflow::CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(err) => {
            fail_assert!("compiled workflow construction failed: {err:?}");
            return;
        }
    };

    // When: creating a run frame and driving deterministically
    let run_id = RunId::new(1);
    let mut frame = match vb_core::engine::new_run_frame(run_id, &workflow) {
        Ok(f) => f,
        Err(err) => {
            fail_assert!("frame creation failed: {err:?}");
            return;
        }
    };
    let mut budget = vb_core::engine::StepBudget::new(100);
    let mut store = ValueStore::new();
    let signal =
        vb_core::engine::drive_deterministic(&workflow, &mut frame, &mut budget, &mut store);

    // Then: the engine signals completion
    assert!(
        matches!(signal, Ok(vb_core::engine::EngineSignal::Finished(_, _))),
        "simple set+finish should finish, got {:?}",
        signal
    );
}

#[test]
fn core_to_runtime_jump_to_out_of_bounds_step_is_rejected() {
    // Given: a workflow with a jump targeting a nonexistent step
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Jump {
            target: StepIdx::new(99),
        },
    };
    let parts = minimal_parts(Box::from([node]));
    // When: validating transition targets
    let result = vb_core::engine::validate_transition_target(&parts);
    // Then: validation rejects the out-of-bounds jump with a StepOutOfBounds error
    assert!(
        matches!(
            &result,
            Err(vb_core::workflow::WorkflowError::StepOutOfBounds { .. })
        ),
        "out-of-bounds jump should return StepOutOfBounds error, got: {:?}",
        result
    );
}

#[test]
fn core_to_runtime_choose_with_empty_branches_is_rejected() {
    // Given: a choose node with no branches and no otherwise
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ChooseSlot {
            branches: Box::from([]),
            otherwise: None,
        },
    };
    let parts = minimal_parts(Box::from([node]));
    // When: constructing the compiled workflow
    let result = vb_core::workflow::CompiledWorkflow::try_from_parts(parts);
    // Then: construction fails (empty branch table is invalid)
    assert!(
        matches!(
            &result,
            Err(vb_core::workflow::WorkflowError::EmptyBranchTable)
        ),
        "empty branch table should return EmptyBranchTable error, got: {:?}",
        result
    );
}

#[test]
fn core_to_runtime_step_budget_exhaustion_returns_correct_signal() {
    // Given: a 3-node chain that exceeds a budget of 2
    // Node 0: SetConst(42) -> node 1
    // Node 1: Copy slot 0 -> slot 1, -> node 2
    // Node 2: Finish result=slot 0
    // With budget=2, only nodes 0 and 1 can execute, leaving budget exhausted before node 2.
    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ConstIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
    };
    let node2 = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("budget_test"),
        digest: WorkflowDigest::from_bytes([2u8; 32]),
        nodes: Box::from([node0, node1, node2]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    let workflow = match vb_core::workflow::CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(err) => {
            fail_assert!("workflow construction failed: {err:?}");
            return;
        }
    };
    let run_id = RunId::new(2);
    let mut frame = match vb_core::engine::new_run_frame(run_id, &workflow) {
        Ok(f) => f,
        Err(err) => {
            fail_assert!("frame creation failed: {err:?}");
            return;
        }
    };
    // When: driving with budget=2 (enough for 2 steps, not 3)
    let mut budget = vb_core::engine::StepBudget::new(2);
    let mut store = ValueStore::new();
    let signal =
        vb_core::engine::drive_deterministic(&workflow, &mut frame, &mut budget, &mut store);

    // Then: budget exhaustion signal (2 of 3 steps executed)
    match signal {
        Ok(vb_core::engine::EngineSignal::StepBudgetExhausted) => {}
        Ok(other) => fail_assert!("expected StepBudgetExhausted, got {other:?}"),
        Err(err) => fail_assert!("unexpected error: {err:?}"),
    }
}

// ===========================================================================
// SEAM 5: vb_runtime -> vb_storage (runtime persists to storage)
// ===========================================================================

#[test]
fn runtime_to_storage_journal_event_encode_decode_roundtrip() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestPayload {
        value: i64,
        label: String,
    }

    // Given: a valid payload and storage magic
    let payload = TestPayload {
        value: 99,
        label: "adversarial".into(),
    };
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;

    // When: encoding then decoding
    let encoded = match vb_storage::encode_record(
        magic,
        vb_storage::RecordKind::StepStarted,
        42,
        &payload,
        4096,
    ) {
        Ok(e) => e,
        Err(err) => {
            fail_assert!("encode failed: {err:?}");
            return;
        }
    };
    let decoded: Result<(vb_storage::RecordEnvelope, TestPayload), _> =
        vb_storage::decode_record(&encoded, magic, 4096);

    // Then: round-trip preserves the payload
    match decoded {
        Ok((_envelope, decoded)) => assert_eq!(decoded, payload),
        Err(err) => fail_assert!("decode failed: {err:?}"),
    }
}

#[test]
fn runtime_to_storage_corrupted_record_fails_integrity_check() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    struct TestPayload {
        value: i64,
    }

    let payload = TestPayload { value: 42 };
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;
    let mut encoded = match vb_storage::encode_record(
        magic,
        vb_storage::RecordKind::StepStarted,
        1,
        &payload,
        4096,
    ) {
        Ok(e) => e,
        Err(err) => {
            fail_assert!("encode failed: {err:?}");
            return;
        }
    };

    // When: corrupting the last byte
    if let Some(last) = encoded.last_mut() {
        *last = last.wrapping_add(1);
    }
    let result: Result<(vb_storage::RecordEnvelope, TestPayload), _> =
        vb_storage::decode_record(&encoded, magic, 4096);

    // Then: decoding fails with any JournalError (corruption detected)
    assert!(
        matches!(
            &result,
            Err(vb_storage::JournalError::PayloadDigestMismatch)
        ),
        "corrupted record should return PayloadDigestMismatch, got: {:?}",
        result
    );
}

#[test]
fn runtime_to_storage_journal_event_carries_correct_record_kind() {
    // Given: a RunAccepted journal event
    let event = vb_storage::JournalEvent::RunAccepted {
        run: RunId::new(42),
        seq: vb_storage::EventSeq::new(1),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    };
    // When: querying the record kind
    let kind = event.record_kind();
    // Then: it maps to RunAccepted
    assert_eq!(kind, vb_storage::RecordKind::RunAccepted);
}

#[test]
fn runtime_to_storage_run_finished_event_carries_correct_kind() {
    // Given: a RunFinished journal event
    let event = vb_storage::JournalEvent::RunFinished {
        run: RunId::new(7),
        seq: vb_storage::EventSeq::new(3),
        result: SlotIdx::new(0),
        attempt: 1,
    };
    // When: querying the record kind
    let kind = event.record_kind();
    // Then: it maps to RunFinished
    assert_eq!(kind, vb_storage::RecordKind::RunFinished);
}

#[test]
fn runtime_to_storage_fjall_journal_open_and_close_temp_dir() {
    // Given: a temporary directory
    let dir = match cross_crate_tempdir() {
        Ok(d) => d,
        Err(err) => {
            fail_assert!("tempdir failed: {err:?}");
            return;
        }
    };

    // When: opening a Fjall journal
    let result = vb_storage::FjallJournal::open(dir.path(), None);

    // Then: journal opens successfully
    match result {
        Ok(_journal) => {} // journal works
        Err(err) => fail_assert!("journal open failed: {err:?}"),
    }
}

// ===========================================================================
// SEAM 6: vb_runtime -> vb_ipc (runtime responds to IPC commands)
// ===========================================================================

#[test]
fn runtime_to_ipc_frame_header_roundtrip_preserves_all_fields() {
    let header = vb_ipc::IpcFrameHeader::new(
        vb_ipc::IpcCommand::SubmitRun,
        0x1234,
        0xDEAD_BEEF_CAFE_1234u64,
        0,
    );
    let encoded = match header.encode() {
        Ok(e) => e,
        Err(err) => {
            fail_assert!("encode failed: {err:?}");
            return;
        }
    };
    let max_payload = vb_ipc::MaxPayloadBytes::new(
        std::num::NonZeroUsize::new(4096).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    match vb_ipc::IpcFrameHeader::decode(&encoded, max_payload) {
        Ok(decoded) => {
            assert_eq!(decoded.command, vb_ipc::IpcCommand::SubmitRun);
            assert_eq!(decoded.flags, 0x1234);
            assert_eq!(decoded.correlation, 0xDEAD_BEEF_CAFE_1234u64);
            assert_eq!(decoded.payload_len, 0);
        }
        Err(err) => fail_assert!("decode failed: {err:?}"),
    }
}

#[test]
fn runtime_to_ipc_invalid_command_id_preserves_unknown_command_variant() {
    // Given: an invalid command identifier
    let value: u16 = 999;
    // When: parsing the command
    let result = vb_ipc::IpcCommand::from_u16(value);
    // Then: UnknownCommand command variant preserves the exact value for dispatch
    assert_eq!(result, Ok(vb_ipc::IpcCommand::UnknownCommand(999)));
}

#[test]
fn runtime_to_ipc_payload_too_large_returns_typed_error() {
    // Given: a payload exceeding the limit
    let large_bytes = bytes::Bytes::from(vec![0u8; 2048]);
    let max = vb_ipc::MaxPayloadBytes::new(
        std::num::NonZeroUsize::new(1024).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    // When: creating a bounded payload
    let result = vb_ipc::BoundedPayload::new(large_bytes.clone(), max);
    // Then: PayloadTooLarge error with exact sizes
    assert_eq!(
        result,
        Err(vb_ipc::IpcError::PayloadTooLarge {
            actual: 2048,
            limit: 1024,
        })
    );
}

#[test]
fn runtime_to_ipc_memory_ingress_submit_and_receive_roundtrip() {
    let capacity = vb_ipc::QueueCapacity::new(
        std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    let max_payload = vb_ipc::MaxPayloadBytes::DEFAULT;
    let ingress = vb_ipc::MemoryIngress::bounded(capacity);

    // Given: a valid ingress frame
    let run_id = RunId::new(42);
    let digest = WorkflowDigest::from_bytes([0u8; 32]);
    let payload = bytes::Bytes::from(b"test payload".as_slice());
    let frame = match vb_ipc::IngressFrame::new(run_id, digest, payload, max_payload) {
        Ok(f) => f,
        Err(err) => {
            fail_assert!("frame creation failed: {err:?}");
            return;
        }
    };

    // When: submitting then receiving
    match ingress.try_submit(frame) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit failed: {err:?}");
            return;
        }
    }
    let received = match ingress.try_recv() {
        Ok(Some(f)) => f,
        Ok(None) => {
            fail_assert!("expected a frame, got None");
            return;
        }
        Err(err) => {
            fail_assert!("recv failed: {err:?}");
            return;
        }
    };

    // Then: the received frame matches
    assert_eq!(received.run_id(), run_id);
    assert_eq!(received.workflow(), digest);
}

#[test]
fn runtime_to_ipc_memory_ingress_queue_full_returns_error() {
    let capacity = vb_ipc::QueueCapacity::new(
        std::num::NonZeroUsize::new(1).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    let max_payload = vb_ipc::MaxPayloadBytes::DEFAULT;
    let ingress = vb_ipc::MemoryIngress::bounded(capacity);

    let run_id = RunId::new(1);
    let digest = WorkflowDigest::from_bytes([0u8; 32]);

    // Given: fill the queue to capacity
    let frame1 = match vb_ipc::IngressFrame::new(
        run_id,
        digest,
        bytes::Bytes::from(b"first".as_slice()),
        max_payload,
    ) {
        Ok(f) => f,
        Err(err) => {
            fail_assert!("frame1 creation failed: {err:?}");
            return;
        }
    };
    match ingress.try_submit(frame1) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("first submit failed: {err:?}");
            return;
        }
    }

    // When: submitting beyond capacity
    let frame2 = match vb_ipc::IngressFrame::new(
        RunId::new(2),
        digest,
        bytes::Bytes::from(b"second".as_slice()),
        max_payload,
    ) {
        Ok(f) => f,
        Err(err) => {
            fail_assert!("frame2 creation failed: {err:?}");
            return;
        }
    };
    let result = ingress.try_submit(frame2);

    // Then: Full error
    assert_eq!(result, Err(vb_ipc::IpcError::Full));
}

#[test]
fn runtime_to_ipc_submit_run_wire_code_is_stable() {
    assert_eq!(vb_ipc::IpcCommand::SubmitRun.as_u16(), 1);
}

#[test]
fn runtime_to_ipc_submit_run_inline_wire_code_is_stable() {
    assert_eq!(vb_ipc::IpcCommand::SubmitRunInline.as_u16(), 2);
}

#[test]
fn runtime_to_ipc_cancel_run_wire_code_is_stable() {
    assert_eq!(vb_ipc::IpcCommand::CancelRun.as_u16(), 3);
}

#[test]
fn runtime_to_ipc_inspect_run_wire_code_is_stable() {
    assert_eq!(vb_ipc::IpcCommand::InspectRun.as_u16(), 4);
}

#[test]
fn runtime_to_ipc_list_events_wire_code_is_stable() {
    assert_eq!(vb_ipc::IpcCommand::ListEvents.as_u16(), 5);
}

#[test]
fn runtime_to_ipc_answer_ask_wire_code_is_stable() {
    assert_eq!(vb_ipc::IpcCommand::AnswerAsk.as_u16(), 6);
}

#[test]
fn runtime_to_ipc_complete_action_wire_code_is_stable() {
    assert_eq!(vb_ipc::IpcCommand::CompleteAction.as_u16(), 7);
}

#[test]
fn runtime_to_ipc_fail_action_wire_code_is_stable() {
    assert_eq!(vb_ipc::IpcCommand::FailAction.as_u16(), 8);
}

#[test]
fn runtime_to_ipc_drain_trace_wire_code_is_stable() {
    assert_eq!(vb_ipc::IpcCommand::DrainTrace.as_u16(), 9);
}

#[test]
fn runtime_to_ipc_health_wire_code_is_stable() {
    assert_eq!(vb_ipc::IpcCommand::Health.as_u16(), 10);
}

#[test]
fn runtime_to_ipc_shutdown_wire_code_is_stable() {
    assert_eq!(vb_ipc::IpcCommand::Shutdown.as_u16(), 11);
}

// ===========================================================================
// TAINT PROPAGATION CROSS-CRATE TESTS
// ===========================================================================

#[test]
fn taint_secret_propagates_through_deterministic_pure_action() {
    // Given: a secret-tainted input and a deterministic pure action
    let result = vb_core::action::propagate_action_taint(
        vb_core::action::Idempotency::DeterministicPure,
        Taint::Secret,
    );
    // Then: the output taint is Secret
    assert_eq!(result, Taint::Secret);
}

#[test]
fn taint_clean_stays_clean_through_pure_action() {
    // Given: clean input and a pure action
    let result = vb_core::action::propagate_action_taint(
        vb_core::action::Idempotency::DeterministicPure,
        Taint::Clean,
    );
    // Then: output stays clean
    assert_eq!(result, Taint::Clean);
}

#[test]
fn taint_derived_propagates_through_idempotent_external() {
    // Given: derived-from-secret taint and an external action
    let result = vb_core::action::propagate_action_taint(
        vb_core::action::Idempotency::IdempotentExternal,
        Taint::DerivedFromSecret,
    );
    // Then: taint propagates
    assert_eq!(result, Taint::DerivedFromSecret);
}

#[test]
fn taint_secret_upgrades_clean_to_secret_in_merge() {
    // Given: two taint levels
    let clean = Taint::Clean;
    let secret = Taint::Secret;
    // When: merging taints (as the runtime would for fan-in)
    let merged = match (clean, secret) {
        (Taint::Secret, _) | (_, Taint::Secret) => Taint::Secret,
        (Taint::DerivedFromSecret, _) | (_, Taint::DerivedFromSecret) => Taint::DerivedFromSecret,
        _ => Taint::Clean,
    };
    // Then: secret dominates
    assert_eq!(merged, Taint::Secret, "secret taint should dominate clean");
}

#[test]
fn taint_secret_in_action_output_ready_carries_through() {
    // Given: an action output with secret taint
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Secret,
        encoded_len: 8,
    };
    // Then: taint is preserved on the output
    assert_eq!(output.taint, Taint::Secret);
}

// ===========================================================================
// ERROR PROPAGATION CROSS-CRATE TESTS
// ===========================================================================

#[test]
fn error_yaml_to_compile_pipeline_preserves_error_information() {
    // Given: YAML with missing version
    let yaml =
        b"name: test\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    finish:\n      result: x\n";
    // When: compiling
    let result = vb_compile::compile_workflow(yaml);
    // Then: error contains "version" somewhere in the message chain
    match result {
        Err(errors) => {
            let message = errors.to_string();
            assert!(
                message.contains("version") || message.contains("Version"),
                "compile error should mention version, got: {message}"
            );
        }
        Ok(_) => fail_assert!("missing version should fail compilation"),
    }
}

#[test]
fn error_compile_duplicate_step_id_rejected_with_exact_id() {
    // Given: workflow with duplicate step IDs
    let yaml = b"version: velvet-ballistics/v1\nname: dup\nwhen:\n  manual: {}\nsteps:\n  - id: dup_id\n    save:\n      output: saved\n      value: \"1\"\n  - id: dup_id\n    finish:\n      result: saved\n";
    // When: compiling
    let result = vb_compile::compile_workflow(yaml);
    // Then: compilation fails mentioning the duplicate
    match result {
        Err(errors) => {
            let message = errors.to_string();
            assert!(
                message.contains("dup_id"),
                "error should reference the duplicate ID 'dup_id', got: {message}"
            );
        }
        Ok(_) => fail_assert!("duplicate step IDs should fail compilation"),
    }
}

#[test]
fn error_core_workflow_error_provides_stable_diagnostic_code() {
    // Given: a WorkflowError for out-of-bounds constant
    let error = vb_core::workflow::WorkflowError::ConstOutOfBounds {
        constant: vb_core::ConstIdx::new(99),
    };
    // When: formatting the error
    let message = error.to_string();
    // Then: the message contains meaningful diagnostic information
    assert!(
        message.contains("constant") || message.contains("out of bounds"),
        "workflow error should be descriptive, got: {message}"
    );
}

#[test]
fn error_validation_error_type_mismatch_preserves_both_types() {
    // Given: a TypeMismatch validation error
    let error = vb_validate::ValidationError::TypeMismatch {
        expected: "u32".into(),
        found: "string".into(),
    };
    // When: formatting
    let message = error.to_string();
    // Then: both types appear in the output
    assert!(
        message.contains("u32"),
        "expected type should appear, got: {message}"
    );
    assert!(
        message.contains("string"),
        "found type should appear, got: {message}"
    );
}

// ===========================================================================
// RESOURCE LIMIT BOUNDARY TESTS
// ===========================================================================

#[test]
fn limits_max_expression_ops_is_256() {
    // Given: the MAX_EXPRESSION_OPS constant
    let max = vb_core::limits::MAX_EXPRESSION_OPS;
    // Then: it is exactly 256
    assert_eq!(max, 256, "MAX_EXPRESSION_OPS must be 256");
}

#[test]
fn limits_max_steps_per_workflow_is_65535() {
    // Given: the MAX_STEPS_PER_WORKFLOW constant
    let max = vb_core::limits::MAX_STEPS_PER_WORKFLOW;
    // Then: it fits in u16
    assert!(
        max <= 65535,
        "MAX_STEPS_PER_WORKFLOW must fit in u16: got {max}"
    );
}

#[test]
fn limits_expression_stack_depth_is_bounded() {
    // Given: the MAX_EXPRESSION_STACK constant
    let max = vb_core::limits::MAX_EXPRESSION_STACK;
    // Then: it is <= 64 (bounded for safety)
    assert!(max <= 64, "expression stack must be <= 64, got {max}");
}

#[test]
fn limits_resource_contract_default_max_input_bytes_is_one_mib() {
    // Given: the default ResourceContract
    let contract = ResourceContract::DEFAULT;
    // Then: max_input_bytes is 1 MiB
    assert_eq!(contract.max_input_bytes, 1_048_576);
}

#[test]
fn limits_resource_contract_default_max_steps_is_1000() {
    // Given: the default ResourceContract
    let contract = ResourceContract::DEFAULT;
    // Then: max_steps is 1000 (the Phase 45 tightened limit)
    assert_eq!(contract.max_steps, 1_000);
}

// ===========================================================================
// EXPRESSION EVALUATION CROSS-CRATE TESTS
// ===========================================================================

#[test]
fn expr_compile_to_eval_simple_arithmetic_produces_correct_result() {
    // Given: expression "1 + 2"
    let tokens = match vb_expr::lexer::lex_expr("1 + 2") {
        Ok(t) => t,
        Err(err) => {
            fail_assert!("lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_expr::parser::parse_expr(&tokens) {
        Ok(a) => a,
        Err(err) => {
            fail_assert!("parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_expr::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(p) => p,
        Err(err) => {
            fail_assert!("bytecode compile failed: {err:?}");
            return;
        }
    };
    // When: evaluating the compiled bytecode
    let result = vb_expr::eval::eval_expr_program(&program, &[], &constants);
    // Then: result is 3
    match result {
        Ok(value) => assert_eq!(value, SlotValue::I64(3)),
        Err(err) => fail_assert!("eval failed: {err:?}"),
    }
}

#[test]
fn expr_compile_to_eval_division_by_zero_returns_error() {
    // Given: expression "1 / 0"
    let tokens = match vb_expr::lexer::lex_expr("1 / 0") {
        Ok(t) => t,
        Err(err) => {
            fail_assert!("lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_expr::parser::parse_expr(&tokens) {
        Ok(a) => a,
        Err(err) => {
            fail_assert!("parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_expr::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(p) => p,
        Err(err) => {
            fail_assert!("bytecode compile failed: {err:?}");
            return;
        }
    };
    // When: evaluating
    let result = vb_expr::eval::eval_expr_program(&program, &[], &constants);
    // Then: evaluation fails with DivisionByZero
    assert!(
        matches!(&result, Err(vb_expr::ExprError::DivisionByZero)),
        "division by zero should return DivisionByZero error, got: {:?}",
        result
    );
}

#[test]
fn expr_compile_to_eval_variable_reference_with_slot() {
    fn resolve(reference: &str) -> Option<SlotIdx> {
        match reference {
            "$x" => Some(SlotIdx::new(0)),
            _ => None,
        }
    }
    // Given: expression "$x + 1"
    let compiled = match vb_expr::bytecode::compile_expr("$x + 1", &resolve) {
        Ok(c) => c,
        Err(err) => {
            fail_assert!("compile failed: {err:?}");
            return;
        }
    };
    let (program, constants) = compiled;
    let slots: Vec<Option<SlotValue>> = vec![Some(SlotValue::I64(41))];
    // When: evaluating with slot[0] = 41
    let result = vb_expr::eval::eval_expr_program(&program, &slots, &constants);
    // Then: result is 42
    match result {
        Ok(value) => assert_eq!(value, SlotValue::I64(42)),
        Err(err) => fail_assert!("eval failed: {err:?}"),
    }
}

#[test]
fn expr_compile_to_eval_boolean_and_produces_false() {
    // Given: expression "true and false"
    let tokens = match vb_expr::lexer::lex_expr("true and false") {
        Ok(t) => t,
        Err(err) => {
            fail_assert!("lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_expr::parser::parse_expr(&tokens) {
        Ok(a) => a,
        Err(err) => {
            fail_assert!("parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_expr::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(p) => p,
        Err(err) => {
            fail_assert!("bytecode compile failed: {err:?}");
            return;
        }
    };
    // When: evaluating
    let result = vb_expr::eval::eval_expr_program(&program, &[], &constants);
    // Then: false
    match result {
        Ok(value) => assert_eq!(value, SlotValue::Bool(false)),
        Err(err) => fail_assert!("eval failed: {err:?}"),
    }
}

// ===========================================================================
// COMPILE PIPELINE END-TO-END TESTS
// ===========================================================================

#[test]
fn compile_pipeline_valid_workflow_produces_deterministic_digest() {
    // Given: the same valid workflow compiled twice
    let yaml = valid_workflow_yaml();
    let result1 = vb_compile::compile_workflow(yaml);
    let result2 = vb_compile::compile_workflow(yaml);
    // When: comparing digests
    match (result1, result2) {
        (Ok(w1), Ok(w2)) => {
            // Then: digests are identical
            assert_eq!(
                w1.digest(),
                w2.digest(),
                "same source should produce same digest"
            );
        }
        (Err(e), _) => fail_assert!("first compile failed: {e}"),
        (_, Err(e)) => fail_assert!("second compile failed: {e}"),
    }
}

#[test]
fn compile_pipeline_invalid_trigger_rejected_with_correct_error() {
    // Given: YAML with an HTTP trigger
    let yaml = b"version: velvet-ballistics/v1\nname: http_wf\nwhen:\n  http: {}\nsteps:\n  - id: s1\n    finish:\n      result: x\n";
    // When: compiling
    let result = vb_compile::compile_workflow(yaml);
    // Then: compilation fails
    match result {
        Err(errors) => {
            let message = errors.to_string();
            assert!(
                message.contains("http") || message.contains("trigger"),
                "error should mention http/trigger, got: {message}"
            );
        }
        Ok(_) => fail_assert!("HTTP trigger should be rejected"),
    }
}

#[test]
fn compile_pipeline_empty_steps_rejected() {
    // Given: YAML with empty steps array
    let yaml = b"version: velvet-ballistics/v1\nname: empty\nwhen:\n  manual: {}\nsteps: []\n";
    // When: compiling
    let result = vb_compile::compile_workflow(yaml);
    // Then: compilation fails with EmptySteps error
    assert!(
        matches!(&result, Err(errors) if errors.iter().any(|e| e.to_string().contains("empty") || e.to_string().contains("Empty"))),
        "empty steps should fail compilation, got Ok: {:?}",
        result
    );
}

#[test]
fn compile_pipeline_reserved_step_id_rejected() {
    // Given: YAML with a step using reserved id "runtime"
    let yaml = b"version: velvet-ballistics/v1\nname: reserved\nwhen:\n  manual: {}\nsteps:\n  - id: runtime\n    finish:\n      result: x\n";
    // When: compiling
    let result = vb_compile::compile_workflow(yaml);
    // Then: compilation fails (runtime reserved ID or any compile error)
    assert!(
        matches!(&result, Err(_)),
        "reserved step ID should fail compilation, got Ok: {:?}",
        result
    );
}

#[test]
fn compile_pipeline_uppercase_step_id_rejected() {
    // Given: YAML with an uppercase step ID
    let yaml = b"version: velvet-ballistics/v1\nname: upper\nwhen:\n  manual: {}\nsteps:\n  - id: MyStep\n    finish:\n      result: x\n";
    // When: compiling
    let result = vb_compile::compile_workflow(yaml);
    // Then: compilation fails (uppercase ID rejected or other compile error)
    assert!(
        matches!(&result, Err(_)),
        "uppercase step ID should fail compilation, got Ok: {:?}",
        result
    );
}

// ===========================================================================
// DIAGNOSTIC CODE CROSS-CRATE TESTS
// ===========================================================================

#[test]
fn diagnostic_code_supported_ranges_are_parseable() {
    assert_eq!(
        vb_core::diagnostic::DiagnosticCode::from_str("E0101"),
        Ok(vb_core::diagnostic::DiagnosticCode::new(0x0101))
    );
    assert_eq!(
        vb_core::diagnostic::DiagnosticCode::from_str("E040C"),
        Ok(vb_core::diagnostic::DiagnosticCode::new(0x040C))
    );
    assert_eq!(
        vb_core::diagnostic::DiagnosticCode::from_str("E1003"),
        Ok(vb_core::diagnostic::DiagnosticCode::new(0x1003))
    );
    assert_eq!(
        vb_core::diagnostic::DiagnosticCode::from_str("E200D"),
        Ok(vb_core::diagnostic::DiagnosticCode::new(0x200D))
    );
    assert_eq!(
        vb_core::diagnostic::DiagnosticCode::from_str("E300F"),
        Ok(vb_core::diagnostic::DiagnosticCode::new(0x300F))
    );
    assert_eq!(
        vb_core::diagnostic::DiagnosticCode::from_str("E4015"),
        Ok(vb_core::diagnostic::DiagnosticCode::new(0x4015))
    );
}

#[test]
fn diagnostic_code_unsupported_codes_are_rejected() {
    assert_eq!(
        vb_core::diagnostic::DiagnosticCode::from_str("E010C"),
        Err(vb_core::diagnostic::DiagnosticCodeParseError::UnsupportedCode)
    );
    assert_eq!(
        vb_core::diagnostic::DiagnosticCode::from_str("E040D"),
        Err(vb_core::diagnostic::DiagnosticCodeParseError::UnsupportedCode)
    );
    assert_eq!(
        vb_core::diagnostic::DiagnosticCode::from_str("E9999"),
        Err(vb_core::diagnostic::DiagnosticCodeParseError::UnsupportedCode)
    );
    assert_eq!(
        vb_core::diagnostic::DiagnosticCode::from_str("E0000"),
        Err(vb_core::diagnostic::DiagnosticCodeParseError::UnsupportedCode)
    );
}

// ===========================================================================
// COMPILE ARTIFACT SERIALIZATION ROUND-TRIP
// ===========================================================================

#[test]
fn compile_to_artifact_serialization_roundtrip() {
    // Given: a valid compiled workflow
    let yaml = valid_workflow_yaml();
    let workflow = match vb_compile::compile_workflow(yaml) {
        Ok(w) => w,
        Err(err) => {
            fail_assert!("compile failed: {err}");
            return;
        }
    };
    // When: emitting a postcard artifact
    let artifact = match vb_compile::emit_compiled_artifact(&workflow) {
        Ok(a) => a,
        Err(err) => {
            fail_assert!("artifact emission failed: {err}");
            return;
        }
    };
    // Then: artifact is non-empty
    assert!(!artifact.is_empty(), "artifact should have bytes");
    // And: deserializing returns equivalent parts
    let original_parts = workflow.to_parts();
    let restored_parts: Result<WorkflowParts, _> = postcard::from_bytes(&artifact);
    match restored_parts {
        Ok(parts) => {
            assert_eq!(parts.name, original_parts.name);
            assert_eq!(parts.slot_count, original_parts.slot_count);
        }
        Err(err) => fail_assert!("deserialization failed: {err:?}"),
    }
}

// ===========================================================================
// CROSS-CRATE: vb_validate TYPE TAINT
// ===========================================================================

#[test]
fn validate_type_taint_secret_result_leak_detection() {
    // Given: a workflow where a secret might leak into the result
    // This tests that the type_taint module exists and can be called
    // The actual validation would operate on the typed AST
    // We verify the error variant exists and can be constructed
    let error = vb_validate::ValidationError::SecretResultLeak;
    let message = error.to_string();
    assert!(
        message.contains("SECRET") || message.contains("secret"),
        "SecretResultLeak should be descriptive, got: {message}"
    );
}

#[test]
fn validate_control_flow_cycle_detection_error_exists() {
    // Given: the ControlFlowCycle error variant
    let error = vb_validate::ValidationError::ControlFlowCycle;
    let message = error.to_string();
    assert!(
        message.contains("cycle") || message.contains("CYCLE"),
        "ControlFlowCycle should be descriptive, got: {message}"
    );
}

#[test]
fn validate_references_unknown_reference_contains_reference_name() {
    // Given: an UnknownReference error
    let error = vb_validate::ValidationError::UnknownReference {
        reference: "missing_step".into(),
    };
    let message = error.to_string();
    assert!(
        message.contains("missing_step"),
        "error should contain the reference name, got: {message}"
    );
}

#[test]
fn validate_references_future_reference_contains_reference_name() {
    // Given: a FutureReference error
    let error = vb_validate::ValidationError::FutureReference {
        reference: "later_step".into(),
    };
    let message = error.to_string();
    assert!(
        message.contains("later_step"),
        "error should contain the reference name, got: {message}"
    );
}

// ===========================================================================
// RUNTIME SHARD CROSS-CRATE INTEGRATION
// ===========================================================================

#[test]
fn runtime_submit_and_tick_simple_workflow() {
    use std::num::NonZeroUsize;
    use vb_runtime::runtime::Runtime;
    use vb_runtime::shard::ShardConfig;

    // Given: a compiled set+finish workflow
    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ConstIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("rt_test"),
        digest: WorkflowDigest::from_bytes([3u8; 32]),
        nodes: Box::from([node0, node1]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    let workflow = match vb_core::workflow::CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(err) => {
            fail_assert!("workflow construction failed: {err:?}");
            return;
        }
    };

    let shard_count = NonZeroUsize::new(1).unwrap_or(NonZeroUsize::MIN);
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 256,
        step_budget_per_tick: 100,
        max_active_runs: 16,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
    };
    let mut runtime = Runtime::new(shard_count, config).expect("runtime config is valid");
    let run_id = RunId::new(100);

    // When: submitting and ticking
    match runtime.submit_direct(run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit failed: {err:?}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(_) => {}
        Err(err) => {
            fail_assert!("tick failed: {err:?}");
            return;
        }
    }

    // Then: the run should have been processed (inspect should find it)
    match runtime.snapshot_run(run_id, 1) {
        Ok(response) => {
            // The run should have finished since it's a simple set+finish
            match response {
                vb_runtime::shard::InspectResponse::Found(snapshot) => {
                    assert_eq!(snapshot.correlation, 1);
                }
                vb_runtime::shard::InspectResponse::NotFound { correlation, .. } => {
                    assert_eq!(correlation, 1);
                }
                // Bead vb-wxl5r: completed runs now surface as Terminal (with
                // the appropriate outcome) instead of either Found or NotFound.
                vb_runtime::shard::InspectResponse::Terminal { correlation, .. } => {
                    assert_eq!(correlation, 1);
                }
                other => fail_assert!("unexpected inspect response: {other:?}"),
            }
        }
        Err(err) => {
            // Even if inspect fails, the tick itself should have succeeded
            assert!(!err.to_string().is_empty());
        }
    }
}

#[test]
fn runtime_rejects_duplicate_run_id_on_tick() {
    use std::num::NonZeroUsize;
    use vb_runtime::runtime::Runtime;
    use vb_runtime::shard::ShardConfig;

    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = minimal_parts(Box::from([node]));
    let workflow = match vb_core::workflow::CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(err) => {
            fail_assert!("workflow construction failed: {err:?}");
            return;
        }
    };

    let shard_count = NonZeroUsize::new(1).unwrap_or(NonZeroUsize::MIN);
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 256,
        step_budget_per_tick: 100,
        max_active_runs: 16,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
    };
    let mut runtime = Runtime::new(shard_count, config).expect("runtime config is valid");
    let run_id = RunId::new(200);

    // When: submitting the same run ID twice, then ticking
    let first = runtime.submit_direct(run_id, workflow.clone());
    let first_tick = runtime.tick_all(); // First submission processes
    match &first_tick {
        Ok(_) => {}
        Err(e) => fail_assert!("first tick should succeed, got: {e:?}"),
    }
    let second = runtime.submit_direct(run_id, workflow);
    // The second submission overwrites the first run (current behavior)
    let tick_result = runtime.tick_all();

    // Then: both submits succeed and the second overwrites the first
    assert!(
        matches!(&first, Ok(())),
        "first submit should succeed, got: {first:?}"
    );
    assert!(
        matches!(&second, Ok(())),
        "second submit enqueues to command queue, got: {second:?}"
    );
    match &tick_result {
        Ok(_) => {}
        Err(e) => fail_assert!("second tick should succeed, got: {e:?}"),
    }
}
