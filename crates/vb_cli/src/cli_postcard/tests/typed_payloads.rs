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

//! CLI Postcard Typed-Payload Round-Trip Tests
//!
//! vb-k8ut.5: per-command typed `CliPostcardPayload` round-trips for
//! the four "core" reports (Diagnostic, Validate, Events, Replay).
//! Each test builds a typed report struct, encodes it through
//! `postcard`, decodes it back through `decode_cli_payload`, and
//! pattern-matches on the typed enum variant tag to access typed
//! fields directly. No test goes through `serde_json::Value`.
//!
//! The four "data-flow / multi-section" reports (Verify, Explain,
//! Trace, Diff) live in `typed_payloads_reports.rs`. The wire-format
//! contract test lives in `wire_format.rs`.

use super::super::*;
use crate::exit_code::CliExitCode;

#[test]
fn typed_diagnostic_payload_round_trips() {
    let report = DiagnosticReport::from_code("boom".to_string(), CliExitCode::ValidationFailed);
    let payload = CliPostcardPayload::Diagnostic(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed diagnostic must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed diagnostic must round-trip");

    match decoded {
        CliPostcardPayload::Diagnostic(report) => {
            assert_eq!(report.code, CliExitCode::ValidationFailed);
            assert_eq!(report.kind, CliPostcardKind::DiagnosticReport);
            assert_eq!(report.message, "boom");
            assert_eq!(
                report.schema_version.as_str(),
                "velvet-ballistics/cli-output/v1"
            );
        }
        other => panic!("expected Diagnostic variant, got {other:?}"),
    }
}

#[test]
fn typed_diagnostic_payload_preserves_supplied_code_not_message_substrings() {
    let report = DiagnosticReport::from_code(
        "storage error; compile failed; validation failed; replay divergence".to_string(),
        CliExitCode::RuntimeFailed,
    );
    let payload = CliPostcardPayload::Diagnostic(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed diagnostic must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed diagnostic must round-trip");

    match decoded {
        CliPostcardPayload::Diagnostic(report) => {
            assert_eq!(report.code, CliExitCode::RuntimeFailed);
            assert_eq!(
                report.message,
                "storage error; compile failed; validation failed; replay divergence"
            );
        }
        other => panic!("expected Diagnostic variant, got {other:?}"),
    }
}

#[test]
fn output_utils_diagnostic_value_classifies_to_typed_payload() {
    let envelope = serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": crate::cli_envelope::kind::DIAGNOSTIC_REPORT,
        "code": "RuntimeFailed",
        "exit_code": u8::from(CliExitCode::RuntimeFailed),
        "message": "storage error; compile failed; validation failed; replay divergence",
    });
    let payload = classify_envelope(&envelope).expect("diagnostic envelope must classify");

    match payload {
        CliPostcardPayload::Diagnostic(report) => {
            assert_eq!(report.code, CliExitCode::RuntimeFailed);
            assert_eq!(report.kind, CliPostcardKind::DiagnosticReport);
            assert_eq!(
                report.message,
                "storage error; compile failed; validation failed; replay divergence"
            );
        }
        other => panic!("expected Diagnostic variant, got {other:?}"),
    }
}

#[test]
fn typed_validate_payload_round_trips() {
    let report = ValidateReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "validate_report".to_string(),
        success: true,
        status: "valid".to_string(),
        exit_code: 0,
        repair_hints: Vec::new(),
    };
    let payload = CliPostcardPayload::Validate(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed validate must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed validate must round-trip");

    match decoded {
        CliPostcardPayload::Validate(decoded_report) => {
            assert!(decoded_report.success);
            assert_eq!(decoded_report.status, "valid");
            assert_eq!(decoded_report.exit_code, 0);
            assert!(decoded_report.repair_hints.is_empty());
            assert_eq!(decoded_report.kind, "validate_report");
            assert_eq!(
                decoded_report.schema_version.as_str(),
                "velvet-ballistics/cli-output/v1"
            );
        }
        other => panic!("expected Validate variant, got {other:?}"),
    }
}

#[test]
fn typed_events_payload_round_trips() {
    let report = EventsReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "events_report".to_string(),
        run_id: 42,
        events: vec![EventEntry {
            seq: 1,
            attempt: 0,
            event_type: "Started".to_string(),
            step: Some(0),
            slot: None,
        }],
        total: 1,
    };
    let payload = CliPostcardPayload::Events(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed events must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed events must round-trip");

    match decoded {
        CliPostcardPayload::Events(decoded_report) => {
            assert_eq!(decoded_report.run_id, 42);
            assert_eq!(decoded_report.total, 1);
            assert_eq!(decoded_report.events.len(), 1);
            assert_eq!(decoded_report.events[0].seq, 1);
            assert_eq!(decoded_report.events[0].attempt, 0);
            assert_eq!(decoded_report.events[0].event_type, "Started");
            assert_eq!(decoded_report.events[0].step, Some(0));
            assert_eq!(decoded_report.events[0].slot, None);
        }
        other => panic!("expected Events variant, got {other:?}"),
    }
}

#[test]
fn typed_replay_payload_round_trips() {
    let report = ReplayReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "replay_report".to_string(),
        run_id: 100,
        recovered: 5,
        events: vec![EventEntry {
            seq: 0,
            attempt: 1,
            event_type: "Recovered".to_string(),
            step: None,
            slot: Some(2),
        }],
        terminal: "Succeeded".to_string(),
    };
    let payload = CliPostcardPayload::Replay(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed replay must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed replay must round-trip");

    match decoded {
        CliPostcardPayload::Replay(decoded_report) => {
            assert_eq!(decoded_report.run_id, 100);
            assert_eq!(decoded_report.recovered, 5);
            assert_eq!(decoded_report.terminal, "Succeeded");
            assert_eq!(decoded_report.events.len(), 1);
            assert_eq!(decoded_report.events[0].event_type, "Recovered");
            assert_eq!(decoded_report.events[0].slot, Some(2));
        }
        other => panic!("expected Replay variant, got {other:?}"),
    }
}
