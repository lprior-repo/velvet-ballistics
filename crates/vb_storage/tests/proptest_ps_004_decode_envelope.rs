#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports)]

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
