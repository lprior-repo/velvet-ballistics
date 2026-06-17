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

//! CLI Postcard Envelope-Shape Tests
//!
//! vb-k8ut.5: envelope-shape classification. Covers the kind-string
//! discriminant table, the typed `From<EnvelopeKind>` impl, the
//! `classify_envelope` routing of known kinds to typed variants, the
//! `UnknownKind` error for unrecognized kind strings, the
//! `MissingKind` error when the `kind` field is absent, and the
//! `Generic` migration-fallback for kinds without a dedicated typed
//! variant.

use super::super::*;
use crate::cli_envelope::Kind as EnvelopeKind;
use crate::cli_postcard::types::UnknownCliPostcardKind;
use proptest::prelude::*;
use std::collections::BTreeSet;
use std::str::FromStr;

#[test]
fn classify_envelope_routes_validate_report_to_typed_variant() {
    let envelope = serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
        "kind": "validate_report",
        "success": true,
        "status": "valid",
        "exit_code": 0,
        "repair_hints": [],
    });
    let payload = classify_envelope(&envelope).expect("validate_report must classify");

    match payload {
        CliPostcardPayload::Validate(report) => {
            assert!(report.success);
            assert_eq!(report.status, "valid");
            assert_eq!(report.exit_code, 0);
            assert_eq!(
                report.schema_version.as_str(),
                "velvet-ballistics/cli-output/v1"
            );
        }
        other => panic!("expected Validate variant, got {other:?}"),
    }
}

#[test]
fn classify_envelope_routes_unknown_kind_to_unknown_kind_error() {
    let envelope = serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
        "kind": "totally_unknown",
    });
    let result = classify_envelope(&envelope);
    match result {
        Err(ClassifyError::UnknownKind(kind)) => {
            assert_eq!(kind, "totally_unknown");
        }
        other => panic!("expected UnknownKind error, got {other:?}"),
    }
}

#[test]
fn classify_envelope_fails_on_missing_kind_field() {
    let envelope = serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
    });
    let result = classify_envelope(&envelope);
    assert_eq!(result, Err(ClassifyError::MissingKind));
}

#[test]
fn classify_envelope_falls_back_to_generic_for_unmapped_typed_kinds() {
    // DoctorReport has no dedicated typed variant in CliPostcardPayload —
    // it must land in the `Generic` migration-fallback variant with the
    // body bytes round-tripping through `GenericEnvelopeRepr`.
    let original = serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
        "kind": "DoctorReport",
        "status": "healthy",
        "checks": ["ok"],
    });
    let payload =
        classify_envelope(&original).expect("DoctorReport must classify to generic fallback");

    let body = match payload {
        CliPostcardPayload::Generic(generic) => {
            assert_eq!(generic.kind, CliPostcardKind::DoctorReport);
            generic.body
        }
        other => panic!("expected Generic variant, got {other:?}"),
    };

    let recovered = GenericEnvelopeRepr::decode_body_as_json(&body)
        .expect("generic body must decode to json tree");
    assert_eq!(recovered, original);
}

#[test]
fn cli_postcard_kind_from_str_resolves_known_kinds_and_returns_typed_err_for_unknown() {
    // Every known kind string must resolve to its typed `CliPostcardKind`
    // discriminant — no silent coercion to DiagnosticReport.
    let cases: &[(&str, CliPostcardKind)] = &[
        ("VerificationReport", CliPostcardKind::VerificationReport),
        ("DiagnosticReport", CliPostcardKind::DiagnosticReport),
        ("WorkflowExplanation", CliPostcardKind::WorkflowExplanation),
        ("WorkflowGraph", CliPostcardKind::WorkflowGraph),
        ("SimulationReport", CliPostcardKind::SimulationReport),
        ("SubmitRunResult", CliPostcardKind::SubmitRunResult),
        ("RunInspection", CliPostcardKind::RunInspection),
        ("RunEvents", CliPostcardKind::RunEvents),
        ("ReplayReport", CliPostcardKind::ReplayReport),
        ("IncidentReport", CliPostcardKind::IncidentReport),
        ("ActionList", CliPostcardKind::ActionList),
        ("ActionDescription", CliPostcardKind::ActionDescription),
        ("DoctorReport", CliPostcardKind::DoctorReport),
        ("AiContextPacket", CliPostcardKind::AiContextPacket),
        ("CliStatus", CliPostcardKind::CliStatus),
        ("SystemStatus", CliPostcardKind::SystemStatus),
        ("AgentContext", CliPostcardKind::AgentContext),
        ("validate_report", CliPostcardKind::ValidateReport),
        ("verify_report", CliPostcardKind::VerifyReport),
        ("explain_report", CliPostcardKind::ExplainReport),
        ("diff_report", CliPostcardKind::DiffReport),
        ("events_report", CliPostcardKind::EventsReport),
        ("trace_report", CliPostcardKind::TraceReport),
        ("replay_report", CliPostcardKind::ReplayReport),
        ("run_report", CliPostcardKind::RunReport),
        ("inspect_report", CliPostcardKind::InspectReport),
        ("simulate", CliPostcardKind::Simulate),
        ("workflow_diff_report", CliPostcardKind::WorkflowDiffReport),
    ];
    for (input, expected) in cases {
        let actual = CliPostcardKind::from_str(input).unwrap_or_else(|_| {
            panic!("envelope kind {input:?} must parse to typed CliPostcardKind")
        });
        assert_eq!(
            actual, *expected,
            "envelope kind {input:?} parsed to {actual:?}, expected {expected:?}"
        );
    }
    // Unknown kinds return a typed parse error — there is no silent fallback.
    assert_eq!(
        CliPostcardKind::from_str("totally_unknown"),
        Err(UnknownCliPostcardKind("totally_unknown".to_string()))
    );
    assert_eq!(
        CliPostcardKind::from_str(""),
        Err(UnknownCliPostcardKind(String::new()))
    );
}

#[test]
fn cli_postcard_kind_from_cli_envelope_kind_is_total() {
    // The `From<EnvelopeKind> for CliPostcardKind` impl must be total —
    // every `cli_envelope::Kind` variant must map to a typed discriminant
    // without panic or fallback.
    let cases: &[(EnvelopeKind, CliPostcardKind)] = &[
        (
            EnvelopeKind::VerificationReport,
            CliPostcardKind::VerificationReport,
        ),
        (
            EnvelopeKind::DiagnosticReport,
            CliPostcardKind::DiagnosticReport,
        ),
        (
            EnvelopeKind::WorkflowExplanation,
            CliPostcardKind::WorkflowExplanation,
        ),
        (EnvelopeKind::WorkflowGraph, CliPostcardKind::WorkflowGraph),
        (
            EnvelopeKind::SimulationReport,
            CliPostcardKind::SimulationReport,
        ),
        (
            EnvelopeKind::SubmitRunResult,
            CliPostcardKind::SubmitRunResult,
        ),
        (EnvelopeKind::RunInspection, CliPostcardKind::RunInspection),
        (EnvelopeKind::RunEvents, CliPostcardKind::RunEvents),
        (EnvelopeKind::ReplayReport, CliPostcardKind::ReplayReport),
        (
            EnvelopeKind::IncidentReport,
            CliPostcardKind::IncidentReport,
        ),
        (EnvelopeKind::ActionList, CliPostcardKind::ActionList),
        (
            EnvelopeKind::ActionDescription,
            CliPostcardKind::ActionDescription,
        ),
        (EnvelopeKind::DoctorReport, CliPostcardKind::DoctorReport),
        (
            EnvelopeKind::AiContextPacket,
            CliPostcardKind::AiContextPacket,
        ),
        (EnvelopeKind::CliStatus, CliPostcardKind::CliStatus),
        (EnvelopeKind::SystemStatus, CliPostcardKind::SystemStatus),
        (EnvelopeKind::AgentContext, CliPostcardKind::AgentContext),
    ];
    for (input, expected) in cases {
        let actual: CliPostcardKind = CliPostcardKind::from(input.clone());
        assert_eq!(
            actual, *expected,
            "From<EnvelopeKind> must map {input:?} to {expected:?}, got {actual:?}"
        );
    }
}

#[test]
fn cli_postcard_kind_all_has_unique_wire_strings() {
    let mut seen = BTreeSet::new();
    for kind in CliPostcardKind::ALL {
        assert!(
            seen.insert(kind.as_str()),
            "duplicate postcard kind {}",
            kind.as_str()
        );
    }
    assert_eq!(seen.len(), CliPostcardKind::ALL.len());
    assert_eq!(
        seen.get("workflow_diff_report").copied(),
        Some("workflow_diff_report")
    );
}

// Property tests: the closed `CliPostcardKind` enum must round-trip
// through its string discriminant for every variant in the taxonomy,
// and every variant in the taxonomy must round-trip through
// `EnvelopeKind` -> `From<EnvelopeKind>` for the subset of variants
// that have an `EnvelopeKind` counterpart. Keep these at module scope:
// `proptest!` expands to `#[test]` items, and nested test items are denied
// by the repository's `-D warnings` check lane.
proptest! {
    #[test]
    fn prop_kind_round_trips_through_as_str_and_from_str(
        kind in proptest::sample::select(CliPostcardKind::ALL.to_vec())
    ) {
        let s = kind.as_str();
        let parsed = CliPostcardKind::from_str(s)
            .expect("every variant as_str must parse back to itself");
        prop_assert_eq!(parsed, kind);
    }

    #[test]
    fn prop_envelope_kind_subset_round_trips(
        kind in proptest::sample::select(CliPostcardKind::ALL.to_vec())
    ) {
        // For every variant, if there is an `EnvelopeKind` counterpart,
        // then `EnvelopeKind::from_str(<kind>.as_str())` must resolve
        // and `From<EnvelopeKind>` must produce the original variant.
        let s = kind.as_str();
        if let Some(env_kind) = EnvelopeKind::from_str(s) {
            let back: CliPostcardKind = CliPostcardKind::from(env_kind);
            prop_assert_eq!(back, kind);
        } else {
            // No EnvelopeKind counterpart — that's expected for the
            // snake_case variants. They are still parsed by
            // `CliPostcardKind::from_str` but not by `EnvelopeKind`.
            prop_assert!(
                matches!(
                    kind,
                    CliPostcardKind::ValidateReport
                        | CliPostcardKind::VerifyReport
                        | CliPostcardKind::ExplainReport
                        | CliPostcardKind::DiffReport
                        | CliPostcardKind::EventsReport
                        | CliPostcardKind::TraceReport
                        | CliPostcardKind::RunReport
                        | CliPostcardKind::InspectReport
                        | CliPostcardKind::Simulate
                        | CliPostcardKind::WorkflowDiffReport
                        | CliPostcardKind::ReplayReport,
                ),
                "PascalCase variant {kind:?} must have EnvelopeKind counterpart",
            );
        }
    }
}
