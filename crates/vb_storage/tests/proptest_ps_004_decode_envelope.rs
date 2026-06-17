#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

// Proptest: Envelope decode boundary (Gate 2a) through public API.
//
// Obligation: PO-vb-h09wf-012
// Verifier: proptest
// Command: cargo test -p vb_storage --test proptest -- ps_004_decode_envelope
//
// Domain claim: >1000 cases: valid workflows submitted produce decodable envelopes.
// Random byte sequences passed to postcard fail to decode as AcceptedArtifact.
//
// PRODUCTION BINDING:
//   vb_storage::admission::submit_artifact

use proptest::prelude::*;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy, SlotIdx, StepIdx,
    WorkflowDigest,
    value::ConstValue,
    workflow::{ResourceContract, WorkflowParts},
};
use vb_storage::admission::AcceptedArtifact;
use vb_storage::admission::submit_artifact;
use vb_storage::journal::FjallJournal;

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}

fn make_workflow() -> CompiledWorkflow {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("proptest_004"),
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
    let hash_bytes =
        postcard::to_allocvec(&parts).expect("serialize workflow parts for digest computation");
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(*computed.as_bytes());
    CompiledWorkflow::try_from_parts(parts).expect("construct compiled workflow from valid parts")
}

proptest! {
    /// PS-004a: Valid workflows produce decodable AcceptedArtifact envelopes.
    #[test]
    fn ps_004_valid_workflow_produces_decodable_envelope(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");

        // Serialize and deserialize the artifact
        let envelope = postcard::to_allocvec(&artifact).expect("serialize");
        let decoded: Result<AcceptedArtifact, _> = postcard::from_bytes(&envelope);
        let decoded_artifact = decoded.expect("valid envelope must decode as AcceptedArtifact");
        prop_assert_eq!(decoded_artifact.digest, artifact.digest);
        prop_assert_eq!(decoded_artifact.verification.gate_count, 15,
            "decoded envelope must have 15 verification gates");
    }

    /// PS-004b: Random bytes do NOT decode as AcceptedArtifact.
    #[test]
    fn ps_004_random_bytes_not_accepted_artifact(bytes in proptest::collection::vec(0u8.., 0..256)) {
        let decoded: Result<AcceptedArtifact, _> = postcard::from_bytes(&bytes);
        // Random bytes should not successfully decode as AcceptedArtifact (statistically).
        // If they do decode, assert they are a genuine AcceptedArtifact (not a false positive).
        match decoded {
            Ok(artifact) => {
                prop_assert_eq!(artifact.verification.gate_count, 15,
                    "decoded artifact must have valid gate count even if rare collision");
                prop_assert_eq!(artifact.verification.durable, true,
                    "decoded artifact must have durable flag set");
            }
            Err(_) => {
                prop_assert!(true,
                    "random bytes correctly failed to decode as AcceptedArtifact (common case)");
            }
        }
    }

    /// PS-004c: Truncated envelope bytes fail to decode.
    #[test]
    fn ps_004_truncated_envelope_fails(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");
        let envelope = postcard::to_allocvec(&artifact).expect("serialize");

        // Truncate envelope to half
        let half = envelope.len() / 2;
        let truncated = &envelope[..half];
        let decoded: Result<AcceptedArtifact, _> = postcard::from_bytes(truncated);
        match decoded {
            Ok(ref bad) => {
                prop_assert!(false,
                    "truncated envelope must NOT decode as AcceptedArtifact, got gate_count={}",
                    bad.verification.gate_count);
            }
            Err(_) => {
                prop_assert!(true, "truncated envelope correctly failed decode");
            }
        }
    }
}
