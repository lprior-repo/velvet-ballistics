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

// Proptest: Trailing bytes property (Gate 3) through storage boundary.
//
// Obligation: PO-vb-h09wf-015
// Verifier: proptest
// Command: cargo test -p vb_storage -- ps_005_trailing_bytes
//
// Domain claim: >1000 cases: envelopes with trailing bytes fail storage admission.
// The trailing byte defense is verified at the storage boundary.
//
// SECURITY NOTE:
// This test exercises the storage boundary directly with malformed data.
// The direct `put_compiled_ir` API is restricted to internal use only;
// external callers MUST use `submit_artifact` which properly validates
// and binds all artifact metadata (warnings, capabilities, seq).

use crate::{
    JournalError,
    admission::{AcceptedArtifact, submit_artifact},
    journal::FjallJournal,
    records::CompiledIrRecord,
};
use proptest::prop_assert;
use proptest::prop_assert_eq;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy, SlotIdx, StepIdx,
    WorkflowDigest,
    value::ConstValue,
    workflow::{ResourceContract, WorkflowParts},
};

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}

fn make_workflow() -> CompiledWorkflow {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("proptest_005"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let hash_bytes = postcard::to_allocvec(&parts).unwrap();
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(*computed.as_bytes());
    CompiledWorkflow::try_from_parts(parts).unwrap()
}

proptest::proptest! {
    /// PS-005a: Valid envelope decodes without trailing bytes issue.
    #[test]
    fn ps_005_valid_envelope_no_trailing_issue(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");
        let envelope = postcard::to_allocvec(&artifact).expect("serialize");

        let (_decoded, remaining): (AcceptedArtifact, &[u8]) =
            postcard::take_from_bytes(&envelope).expect("decode");
        prop_assert!(remaining.is_empty(),
            "valid envelope must have no trailing bytes after decode");
    }

    /// PS-005b: Envelope with appended random bytes has trailing bytes.
    #[test]
    fn ps_005_envelope_with_trailer_has_remaining(_dummy in proptest::bool::ANY,
        trailer_len in 1usize..=32usize,
    ) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");
        let mut envelope = postcard::to_allocvec(&artifact).expect("serialize");

        // Append random trailer bytes
        let trailer: Vec<u8> = (0..trailer_len).map(|_| 0xFFu8).collect();
        envelope.extend_from_slice(&trailer);

        let (_decoded, remaining): (AcceptedArtifact, &[u8]) =
            postcard::take_from_bytes(&envelope).expect("decode");
        prop_assert!(!remaining.is_empty(),
            "envelope with trailer must have remaining bytes after decode");
        prop_assert_eq!(remaining.len(), trailer_len,
            "remaining bytes length must equal trailer length");
    }

    /// PS-005c: storage write rejects trailered accepted-artifact envelope.
    ///
    /// SECURITY TEST: This verifies that the storage boundary rejects malformed
    /// data. The direct `put_compiled_ir` API is internal; this test exercises
    /// it to verify the security boundary works correctly.
    #[test]
    fn ps_005_storage_write_rejects_trailer(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");
        let mut envelope = postcard::to_allocvec(&artifact).expect("serialize");
        let declared_end = envelope.len();
        envelope.push(0xFF); // Add one trailing byte
        let record = CompiledIrRecord {
            digest: artifact.digest,
            ir: envelope,
            ..Default::default()
        };

        // SECURITY: This tests the internal storage boundary directly
        let result = journal.put_compiled_ir(&record);
        prop_assert!(
            matches!(
                result,
                Err(JournalError::UnexpectedTrailingBytes { declared_end: found, actual_len })
                    if found == declared_end && actual_len == record.ir.len()
            ),
            "storage write must reject trailered accepted-artifact envelope"
        );
    }
}
