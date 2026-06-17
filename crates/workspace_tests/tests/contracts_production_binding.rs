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
    unused_variables,
)]

//! Production-type binding tests for contracts-as-data (vb-6f02).
//!
//! CRITICAL REPAIR: These tests import types directly from `xtask::contracts`
//! and `xtask::evidence` — they are NOT independent copies.
//!
//! Covers: OBL-001 (parse_schema_version), OBL-002 (parse_contract_kind),
//! OBL-003 (compare_semver), OBL-004 (parse_vet_exit_code),
//! OBL-005 (gate_evidence_from_report), OBL-006 (ContractFile serde).

use std::collections::BTreeMap;
use std::path::PathBuf;

use xtask::contracts::{
    ContractError, ContractFile, ContractKind, DiscoveryReport, ReportSummary, SemverCmp,
    compare_semver, gate_evidence_from_report, parse_schema_version, parse_vet_exit_code,
};
use xtask::evidence::{GateEvidence, GateStatus, WhyFailed};

// ============================================================
// OBL-001: parse_schema_version — bound to production code
// ============================================================

#[test]
fn test_prod_parse_schema_version_valid() {
    assert_eq!(parse_schema_version("1.0.0"), Ok("1.0.0".to_string()));
    assert_eq!(parse_schema_version("2.1.0"), Ok("2.1.0".to_string()));
    assert_eq!(parse_schema_version("0.9.9"), Ok("0.9.9".to_string()));
}

#[test]
fn test_prod_parse_schema_version_invalid() {
    assert!(parse_schema_version("").is_err());
    assert!(parse_schema_version("1.0").is_err());
    assert!(parse_schema_version("1.0.0.0").is_err());
    assert!(parse_schema_version("abc").is_err());
    assert!(parse_schema_version("1.0.abc").is_err());
    assert!(parse_schema_version("v1.0.0").is_err());
}

#[test]
fn test_prod_parse_schema_version_error_display() {
    let err = parse_schema_version("").unwrap_err();
    assert_eq!(err.to_string(), "MISSING_SCHEMA_VERSION");

    let err = parse_schema_version("v1.0.0").unwrap_err();
    assert!(err.to_string().contains("INVALID_VERSION"));
}

// ============================================================
// OBL-002: parse_contract_kind — bound to production code
// ============================================================

#[test]
fn test_prod_parse_contract_kind_all_valid() {
    assert_eq!(
        ContractKind::parse("cli_envelope"),
        Ok(ContractKind::CliEnvelope)
    );
    assert_eq!(ContractKind::parse("ui_tokens"), Ok(ContractKind::UiTokens));
    assert_eq!(
        ContractKind::parse("accepted_artifacts"),
        Ok(ContractKind::AcceptedArtifacts)
    );
    assert_eq!(
        ContractKind::parse("evidence_bundle"),
        Ok(ContractKind::EvidenceBundle)
    );
    assert_eq!(
        ContractKind::parse("diagnostics"),
        Ok(ContractKind::Diagnostics)
    );
    assert_eq!(
        ContractKind::parse("gate_output"),
        Ok(ContractKind::GateOutput)
    );
}

#[test]
fn test_prod_parse_contract_kind_invalid() {
    assert!(ContractKind::parse("").is_err());
    assert!(ContractKind::parse("CLI_ENVELOPE").is_err());
    assert!(ContractKind::parse("cli-envelope").is_err());
    assert!(ContractKind::parse("unknown").is_err());
    assert!(ContractKind::parse("cli_envelope_extra").is_err());
}

#[test]
fn test_prod_parse_contract_kind_error_display() {
    let err = ContractKind::parse("bogus").unwrap_err();
    // ContractKind::parse returns the unrecognised string as the error
    assert_eq!(err, "bogus");
}

#[test]
fn test_prod_contract_kind_round_trip() {
    for kind in ContractKind::all_values() {
        let display = kind.to_string();
        let parsed = ContractKind::parse(&display).unwrap();
        assert_eq!(parsed, *kind);
    }
}

// ============================================================
// OBL-003: compare_semver — bound to production code
// ============================================================

#[test]
fn test_prod_compare_semver_equal() {
    assert_eq!(compare_semver("1.0.0", "1.0.0"), Ok(SemverCmp::Equal));
    assert_eq!(compare_semver("0.0.0", "0.0.0"), Ok(SemverCmp::Equal));
    assert_eq!(compare_semver("99.99.99", "99.99.99"), Ok(SemverCmp::Equal));
}

#[test]
fn test_prod_compare_semver_less() {
    assert_eq!(compare_semver("1.0.0", "2.0.0"), Ok(SemverCmp::Less));
    assert_eq!(compare_semver("1.0.0", "1.1.0"), Ok(SemverCmp::Less));
    assert_eq!(compare_semver("1.0.0", "1.0.1"), Ok(SemverCmp::Less));
    assert_eq!(compare_semver("0.0.0", "0.0.1"), Ok(SemverCmp::Less));
}

#[test]
fn test_prod_compare_semver_greater() {
    assert_eq!(compare_semver("2.0.0", "1.0.0"), Ok(SemverCmp::Greater));
    assert_eq!(compare_semver("1.1.0", "1.0.0"), Ok(SemverCmp::Greater));
    assert_eq!(compare_semver("1.0.1", "1.0.0"), Ok(SemverCmp::Greater));
    assert_eq!(compare_semver("0.0.1", "0.0.0"), Ok(SemverCmp::Greater));
}

#[test]
fn test_prod_compare_semver_invalid_format() {
    assert!(compare_semver("1.0", "1.0.0").is_err());
    assert!(compare_semver("1.0.0", "1.0").is_err());
    assert!(compare_semver("abc", "1.0.0").is_err());
    assert!(compare_semver("1.0.0.0", "1.0.0").is_err());
}

// ============================================================
// OBL-004: parse_vet_exit_code — bound to production code
// ============================================================

#[test]
fn test_prod_parse_vet_exit_code_success() {
    assert!(parse_vet_exit_code(0).is_ok());
}

#[test]
fn test_prod_parse_vet_exit_code_failure() {
    assert!(parse_vet_exit_code(1).is_err());
    assert!(parse_vet_exit_code(-1).is_err());
    assert!(parse_vet_exit_code(255).is_err());
    assert!(parse_vet_exit_code(127).is_err());
}

#[test]
fn test_prod_parse_vet_exit_code_error_message() {
    let err = parse_vet_exit_code(1).unwrap_err();
    assert!(err.contains("cue vet exited with code 1"));
}

// ============================================================
// OBL-005: gate_evidence_from_report — bound to production code
// ============================================================

#[test]
fn test_prod_gate_evidence_pass() {
    let report = DiscoveryReport {
        files: vec![
            ContractFile {
                path: PathBuf::from("contracts/cli_envelope.cue"),
                schema_version: "1.0.0".to_string(),
                kind: ContractKind::CliEnvelope,
                vet_errors: Vec::new(),
            },
            ContractFile {
                path: PathBuf::from("contracts/ui_tokens.cue"),
                schema_version: "1.0.0".to_string(),
                kind: ContractKind::UiTokens,
                vet_errors: Vec::new(),
            },
        ],
        errors: Vec::new(),
        summary: ReportSummary {
            total: 2,
            valid: 2,
            invalid: 0,
            errors_by_kind: BTreeMap::new(),
            version_violations: Vec::new(),
        },
    };

    let evidence = gate_evidence_from_report(&report);

    assert_eq!(evidence.kind, "contract-discovery");
    assert_eq!(evidence.gate_name, "contracts");
    assert_eq!(evidence.exit_code, 0);
    assert_eq!(evidence.status, GateStatus::Pass);
    assert!(evidence.why_failed.is_none());
    assert_eq!(evidence.command, "cargo xtask contracts --dir contracts");
}

#[test]
fn test_prod_gate_evidence_fail() {
    let report = DiscoveryReport {
        files: vec![ContractFile {
            path: PathBuf::from("contracts/bad.cue"),
            schema_version: "1.0.0".to_string(),
            kind: ContractKind::CliEnvelope,
            vet_errors: vec!["INVALID_KIND: bogus".to_string()],
        }],
        errors: vec!["INVALID_KIND: bogus".to_string()],
        summary: ReportSummary {
            total: 1,
            valid: 0,
            invalid: 1,
            errors_by_kind: [("INVALID_KIND: bogus".to_string(), 1u32)]
                .into_iter()
                .collect(),
            version_violations: Vec::new(),
        },
    };

    let evidence = gate_evidence_from_report(&report);

    assert_eq!(evidence.status, GateStatus::Fail);
    assert_eq!(evidence.exit_code, 1);
    assert!(evidence.why_failed.is_some());

    let why = evidence.why_failed.unwrap();
    assert_eq!(why.gate_name, "contracts");
    assert!(why.repair_command.contains("cargo xtask contracts"));
    assert!(why.hint.contains("1 contract"));
}

#[test]
fn test_prod_gate_evidence_empty_report() {
    let report = DiscoveryReport {
        files: Vec::new(),
        errors: Vec::new(),
        summary: ReportSummary::new(),
    };

    let evidence = gate_evidence_from_report(&report);

    assert_eq!(evidence.status, GateStatus::Pass);
    assert_eq!(evidence.exit_code, 0);
    assert!(evidence.why_failed.is_none());
}

#[test]
fn test_prod_gate_evidence_multiple_errors() {
    let errors = vec![
        "INVALID_KIND: bogus".to_string(),
        "MISSING_SCHEMA_VERSION".to_string(),
        "INVALID_KIND: unknown".to_string(),
    ];

    let report = DiscoveryReport {
        files: Vec::new(),
        errors: errors.clone(),
        summary: ReportSummary {
            total: 3,
            valid: 0,
            invalid: 3,
            errors_by_kind: [
                ("INVALID_KIND: bogus".to_string(), 1u32),
                ("INVALID_KIND: unknown".to_string(), 1u32),
                ("MISSING_SCHEMA_VERSION".to_string(), 1u32),
            ]
            .into_iter()
            .collect(),
            version_violations: Vec::new(),
        },
    };

    let evidence = gate_evidence_from_report(&report);

    assert_eq!(evidence.status, GateStatus::Fail);
    assert!(evidence.why_failed.is_some());

    let why = evidence.why_failed.unwrap();
    // Errors should be sorted and deduplicated
    assert!(why.hint.contains("3 contract"));
}

// ============================================================
// OBL-006: ContractFile / DiscoveryReport serde — bound to production code
// ============================================================

#[test]
fn test_prod_contract_file_serialization() {
    let file = ContractFile {
        path: PathBuf::from("contracts/cli_envelope.cue"),
        schema_version: "1.0.0".to_string(),
        kind: ContractKind::CliEnvelope,
        vet_errors: vec!["CUE_VET_FAILED: syntax error".to_string()],
    };

    let json = serde_json::to_string(&file).expect("ContractFile serialization should not fail");
    let parsed: ContractFile =
        serde_json::from_str(&json).expect("ContractFile deserialization should succeed");

    assert_eq!(parsed.path, file.path);
    assert_eq!(parsed.schema_version, file.schema_version);
    assert_eq!(parsed.kind, file.kind);
    assert_eq!(parsed.vet_errors, file.vet_errors);
}

#[test]
fn test_prod_discovery_report_serialization() {
    let report = DiscoveryReport {
        files: vec![ContractFile {
            path: PathBuf::from("contracts/cli_envelope.cue"),
            schema_version: "1.0.0".to_string(),
            kind: ContractKind::CliEnvelope,
            vet_errors: Vec::new(),
        }],
        errors: vec!["INVALID_KIND: bogus".to_string()],
        summary: ReportSummary {
            total: 2,
            valid: 1,
            invalid: 1,
            errors_by_kind: BTreeMap::from_iter(vec![("INVALID_KIND: bogus".to_string(), 1)]),
            version_violations: Vec::new(),
        },
    };

    let json = serde_json::to_string_pretty(&report)
        .expect("DiscoveryReport serialization should not fail");

    let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON must be parseable");

    assert!(
        parsed.get("summary").is_some(),
        "JSON must have 'summary' key for moon task consumers"
    );
    assert!(
        parsed.get("errors").is_some(),
        "JSON must have 'errors' key for moon task consumers"
    );
    assert!(
        parsed.get("files").is_some(),
        "JSON must have 'files' key for moon task consumers"
    );
}

#[test]
fn test_prod_report_summary_deterministic_key_order() {
    // BTreeMap ensures deterministic JSON key order (OBL-006 property).
    let mut errors_by_kind: BTreeMap<String, u32> = BTreeMap::new();
    errors_by_kind.insert("zzz_last".to_string(), 1);
    errors_by_kind.insert("aaa_first".to_string(), 2);
    errors_by_kind.insert("mmm_middle".to_string(), 3);

    let report = DiscoveryReport {
        files: Vec::new(),
        errors: Vec::new(),
        summary: ReportSummary {
            total: 6,
            valid: 4,
            invalid: 2,
            errors_by_kind,
            version_violations: Vec::new(),
        },
    };

    let json = serde_json::to_string(&report).unwrap();

    // The JSON key order must be deterministic: aaa_first before mmm_middle before zzz_last.
    let aaa_pos = json.find("\"aaa_first\"").unwrap();
    let mmm_pos = json.find("\"mmm_middle\"").unwrap();
    let zzz_pos = json.find("\"zzz_last\"").unwrap();

    assert!(
        aaa_pos < mmm_pos,
        "aaa_first must come before mmm_middle in JSON"
    );
    assert!(
        mmm_pos < zzz_pos,
        "mmm_middle must come before zzz_last in JSON"
    );
}

#[test]
fn test_prod_gate_evidence_serialization() {
    let evidence = GateEvidence {
        kind: "contract-discovery".to_string(),
        gate_name: "contracts".to_string(),
        command: "cargo xtask contracts --dir contracts".to_string(),
        exit_code: 0,
        log: PathBuf::from(".evidence/contracts/last_run.log"),
        status: GateStatus::Pass,
        why_failed: None,
    };

    let json =
        serde_json::to_string(&evidence).expect("GateEvidence serialization should not fail");

    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("GateEvidence deserialization should succeed");

    assert_eq!(parsed["kind"].as_str().unwrap(), "contract-discovery");
    assert!(json.contains("contract-discovery"));
    assert!(json.contains("Pass"));
}

#[test]
fn test_prod_gate_evidence_fail_serialization() {
    let why_failed = WhyFailed {
        gate_name: "contracts".to_string(),
        hint: "2 contract(s) failed".to_string(),
        repair_command: "cargo xtask contracts --check".to_string(),
        variant: None,
        fixture_id: None,
        expected_gate: None,
    };

    let evidence = GateEvidence {
        kind: "contract-discovery".to_string(),
        gate_name: "contracts".to_string(),
        command: "cargo xtask contracts --dir contracts".to_string(),
        exit_code: 1,
        log: PathBuf::from(".evidence/contracts/last_run.log"),
        status: GateStatus::Fail,
        why_failed: Some(why_failed),
    };

    let json =
        serde_json::to_string(&evidence).expect("GateEvidence serialization should not fail");

    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("GateEvidence deserialization should succeed");

    assert_eq!(parsed["gate_name"], "contracts");
    assert!(json.contains("Fail"));
    assert!(json.contains("why_failed"));
    assert!(json.contains("contracts"));
    assert!(json.contains("2 contract(s) failed"));
    assert!(json.contains("cargo xtask contracts --check"));
}

// ============================================================
// OBL-005 edge cases: ReportSummary total invariant
// ============================================================

#[test]
fn test_prod_summary_total_invariant_pass() {
    let summary = ReportSummary {
        total: 5,
        valid: 3,
        invalid: 2,
        errors_by_kind: BTreeMap::new(),
        version_violations: Vec::new(),
    };
    assert_eq!(summary.total, summary.valid.saturating_add(summary.invalid));
}

#[test]
fn test_prod_summary_total_invariant_zero() {
    let summary = ReportSummary::new();
    assert_eq!(summary.total, 0u32);
    assert_eq!(summary.valid, 0u32);
    assert_eq!(summary.invalid, 0u32);
    assert_eq!(summary.total, summary.valid.saturating_add(summary.invalid));
}

#[test]
fn test_prod_summary_total_invariant_overflow_safety() {
    // Even with large values, saturating_add should not panic.
    let summary = ReportSummary {
        total: u32::MAX,
        valid: u32::MAX,
        invalid: 1,
        errors_by_kind: BTreeMap::new(),
        version_violations: Vec::new(),
    };
    // saturating_add saturates to u32::MAX on overflow for unsigned types
    let sum = summary.valid.saturating_add(summary.invalid);
    // This tests that saturating_add is used correctly (no overflow panic)
    assert_eq!(sum, u32::MAX);
}

// ============================================================
// OBL-002: ContractKind Display — bound to production code
// ============================================================

#[test]
fn test_prod_contract_kind_display_all() {
    let expected = [
        ("cli_envelope", ContractKind::CliEnvelope),
        ("ui_tokens", ContractKind::UiTokens),
        ("accepted_artifacts", ContractKind::AcceptedArtifacts),
        ("evidence_bundle", ContractKind::EvidenceBundle),
        ("diagnostics", ContractKind::Diagnostics),
        ("gate_output", ContractKind::GateOutput),
    ];

    for (display, kind) in expected {
        let actual = kind.to_string();
        assert_eq!(actual, display);
    }
}

// ============================================================
// OBL-001 + OBL-002: parse_schema_version + parse_contract_kind
// integration — bound to production code
// ============================================================

#[test]
fn test_prod_parse_schema_version_uses_valid() {
    // parse_schema_version returns the original string on success.
    // This means the parsed version is exactly what was validated.
    let input = "3.2.1";
    let result = parse_schema_version(input).unwrap();
    assert_eq!(result, input);
}

#[test]
fn test_prod_parse_contract_kind_case_sensitive() {
    // ContractKind parsing is case-sensitive (lowercase only).
    assert!(ContractKind::parse("cli_envelope").is_ok());
    assert!(ContractKind::parse("cli_Envelope").is_err());
    assert!(ContractKind::parse("CLI_ENVELOPE").is_err());
}

// ============================================================
// OBL-006: GateEvidence status invariant
// ============================================================

#[test]
fn test_prod_gate_evidence_exit_code_matches_status() {
    let pass_report = DiscoveryReport {
        files: Vec::new(),
        errors: Vec::new(),
        summary: ReportSummary {
            total: 0,
            valid: 0,
            invalid: 0,
            errors_by_kind: BTreeMap::new(),
            version_violations: Vec::new(),
        },
    };

    let pass_evidence = gate_evidence_from_report(&pass_report);
    assert_eq!(pass_evidence.status, GateStatus::Pass);
    assert_eq!(pass_evidence.exit_code, 0);

    let fail_report = DiscoveryReport {
        files: Vec::new(),
        errors: vec!["error".to_string()],
        summary: ReportSummary {
            total: 1,
            valid: 0,
            invalid: 1,
            errors_by_kind: BTreeMap::new(),
            version_violations: Vec::new(),
        },
    };

    let fail_evidence = gate_evidence_from_report(&fail_report);
    assert_eq!(fail_evidence.status, GateStatus::Fail);
    assert_eq!(fail_evidence.exit_code, 1);
}

// ============================================================
// ContractError Display — bound to production code
// ============================================================

#[test]
fn test_prod_contract_error_all_variants_display() {
    let err = ContractError::MissingSchemaVersion;
    assert_eq!(err.to_string(), "MISSING_SCHEMA_VERSION");

    let err = ContractError::InvalidVersion {
        version: "1.0".to_string(),
    };
    assert_eq!(err.to_string(), "INVALID_VERSION: 1.0");

    let err = ContractError::InvalidKind {
        kind: "bogus".to_string(),
    };
    assert_eq!(err.to_string(), "INVALID_KIND: bogus");

    let err = ContractError::CueVetFailed {
        file: "foo.cue".to_string(),
    };
    assert_eq!(err.to_string(), "CUE_VET_FAILED: foo.cue");

    let err = ContractError::VersionMonotonicityBreach {
        file: "bar.cue".to_string(),
        expected: "1.0.0".to_string(),
        actual: "0.9.0".to_string(),
    };
    assert!(err.to_string().contains("VERSION_MONOTONICITY_BREACH"));
    assert!(err.to_string().contains("bar.cue"));
    assert!(err.to_string().contains("expected 1.0.0 got 0.9.0"));
}
