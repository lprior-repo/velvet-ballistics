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
    unused_variables,
)]

//! CLI Postcard Multi-Section Typed-Payload Round-Trip Tests
//!
//! vb-k8ut.5: per-command typed `CliPostcardPayload` round-trips for
//! the four "data-flow / multi-section" reports (Verify, Explain,
//! Trace, Diff). Each test builds a typed report struct with
//! multi-section nested fields, encodes it through `postcard`,
//! decodes it back through `decode_cli_payload`, and pattern-matches
//! on the typed enum variant tag to access typed fields directly.
//! No test goes through `serde_json::Value`.
//!
//! The four "core" reports (Diagnostic, Validate, Events, Replay)
//! live in `typed_payloads.rs`. The wire-format contract test lives
//! in `wire_format.rs`.

use super::super::*;

#[test]
fn typed_verify_payload_round_trips() {
    let report = VerifyReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "verify_report".to_string(),
        success: true,
        profile: "strict".to_string(),
        digest: "abc123".to_string(),
        node_count: 7,
        checks: vec!["g0".to_string(), "g1".to_string()],
        warnings: Vec::new(),
        artifact: VerifyArtifactSection {
            source_digest_hex: "src".to_string(),
            ir_digest_hex: "ir".to_string(),
            node_count: 7,
        },
        replay: VerifyReplaySection {
            gates_passed: vec!["g0".to_string()],
            gate_sequence: vec!["g0".to_string(), "g1".to_string()],
            replay_safe: true,
        },
        durability: VerifyDurabilitySection {
            profile: "strict".to_string(),
            journal_written: true,
        },
    };
    let payload = CliPostcardPayload::Verify(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed verify must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed verify must round-trip");

    match decoded {
        CliPostcardPayload::Verify(decoded_report) => {
            assert!(decoded_report.success);
            assert_eq!(decoded_report.profile, "strict");
            assert_eq!(decoded_report.digest, "abc123");
            assert_eq!(decoded_report.node_count, 7);
            assert_eq!(
                decoded_report.checks,
                vec!["g0".to_string(), "g1".to_string()]
            );
            assert!(decoded_report.warnings.is_empty());
            assert_eq!(decoded_report.artifact.source_digest_hex, "src");
            assert_eq!(decoded_report.artifact.ir_digest_hex, "ir");
            assert_eq!(decoded_report.artifact.node_count, 7);
            assert_eq!(decoded_report.replay.gates_passed, vec!["g0".to_string()]);
            assert!(decoded_report.replay.replay_safe);
            assert_eq!(decoded_report.durability.profile, "strict");
            assert!(decoded_report.durability.journal_written);
        }
        other => panic!("expected Verify variant, got {other:?}"),
    }
}

#[test]
fn typed_explain_payload_round_trips() {
    let report = ExplainReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "explain_report".to_string(),
        success: false,
        status: "failed".to_string(),
        phase: "compile".to_string(),
        errors: vec![
            ExplainErrorEntry::Structured {
                phase: "compile".to_string(),
                message: "syntax error at step 3".to_string(),
            },
            ExplainErrorEntry::Message("bottom-line failure".to_string()),
        ],
        repair_hints: vec!["add step".to_string()],
        exit_code: 3,
        body: None,
        artifact: None,
    };
    let payload = CliPostcardPayload::Explain(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed explain must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed explain must round-trip");

    match decoded {
        CliPostcardPayload::Explain(decoded_report) => {
            assert!(!decoded_report.success);
            assert_eq!(decoded_report.status, "failed");
            assert_eq!(decoded_report.phase, "compile");
            assert_eq!(decoded_report.exit_code, 3);
            assert_eq!(decoded_report.repair_hints, vec!["add step".to_string()]);
            assert_eq!(decoded_report.errors.len(), 2);
            match &decoded_report.errors[0] {
                ExplainErrorEntry::Structured { phase, message } => {
                    assert_eq!(phase, "compile");
                    assert_eq!(message, "syntax error at step 3");
                }
                other => panic!("expected Structured error entry, got {other:?}"),
            }
            match &decoded_report.errors[1] {
                ExplainErrorEntry::Message(message) => {
                    assert_eq!(message, "bottom-line failure");
                }
                other => panic!("expected Message error entry, got {other:?}"),
            }
        }
        other => panic!("expected Explain variant, got {other:?}"),
    }
}

#[test]
fn typed_trace_payload_round_trips() {
    let report = TraceReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "trace_report".to_string(),
        run_id: 7,
        trace: vec![TraceEntry {
            seq: 0,
            event_type: "StepBegin".to_string(),
            step: Some(0),
            status: Some("ok".to_string()),
            action: Some("noop".to_string()),
        }],
        total: 1,
    };
    let payload = CliPostcardPayload::Trace(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed trace must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed trace must round-trip");

    match decoded {
        CliPostcardPayload::Trace(decoded_report) => {
            assert_eq!(decoded_report.run_id, 7);
            assert_eq!(decoded_report.total, 1);
            assert_eq!(decoded_report.trace.len(), 1);
            assert_eq!(decoded_report.trace[0].seq, 0);
            assert_eq!(decoded_report.trace[0].event_type, "StepBegin");
            assert_eq!(decoded_report.trace[0].step, Some(0));
            assert_eq!(decoded_report.trace[0].status.as_deref(), Some("ok"));
            assert_eq!(decoded_report.trace[0].action.as_deref(), Some("noop"));
        }
        other => panic!("expected Trace variant, got {other:?}"),
    }
}

#[test]
fn typed_diff_payload_round_trips() {
    let report = DiffReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "diff_report".to_string(),
        run_a: 1,
        run_b: 2,
        events_a: 10,
        events_b: 12,
        diffs: vec![DiffEntry {
            kind: "step".to_string(),
            seq: Some(3),
            step: Some(1),
            slot: None,
            detail: Some("payload differs".to_string()),
        }],
        total_differences: 1,
    };
    let payload = CliPostcardPayload::Diff(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed diff must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed diff must round-trip");

    match decoded {
        CliPostcardPayload::Diff(decoded_report) => {
            assert_eq!(decoded_report.run_a, 1);
            assert_eq!(decoded_report.run_b, 2);
            assert_eq!(decoded_report.events_a, 10);
            assert_eq!(decoded_report.events_b, 12);
            assert_eq!(decoded_report.total_differences, 1);
            assert_eq!(decoded_report.diffs.len(), 1);
            assert_eq!(decoded_report.diffs[0].kind, "step");
            assert_eq!(decoded_report.diffs[0].seq, Some(3));
            assert_eq!(decoded_report.diffs[0].step, Some(1));
            assert_eq!(decoded_report.diffs[0].slot, None);
            assert_eq!(
                decoded_report.diffs[0].detail.as_deref(),
                Some("payload differs")
            );
        }
        other => panic!("expected Diff variant, got {other:?}"),
    }
}
