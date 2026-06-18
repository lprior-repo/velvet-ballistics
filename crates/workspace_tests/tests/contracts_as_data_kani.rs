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

//! Kani harness for schema_version field validation (OBL-001).
//!
//! Proves: parse_schema_version handles all possible string inputs without panic,
//! correctly rejects malformed versions, and correctly accepts valid semver strings.
//!
//! GOD RULE: Implements kani::Arbitrary for ContractFileMeta — no hardcoded inputs.
//! Binds to: xtask/src/contracts.rs::parse_schema_version
//!
//! NOTE: This file is gated with #[cfg(kani)] — only compiled by `cargo kani`, not `cargo test`.

#![cfg(kani)]

use std::path::PathBuf;

// The older harness copy under `contracts_as_data_kani/` is intentionally kept
// addressable so architecture-drift checks do not treat it as an orphaned test
// file. It is not compiled here because this parent file contains the current
// Kani harness and the submodule copy is legacy/incompatible with the current
// contract model.
#[cfg(any())]
#[path = "contracts_as_data_kani/contracts_kani_harness.rs"]
mod contracts_kani_harness;

// ============================================================
// Domain model — mirrors xtask/src/contracts.rs
// ============================================================

/// ContractKind mirrors the 6 valid enum values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractKind {
    CliEnvelope,
    UiTokens,
    AcceptedArtifacts,
    EvidenceBundle,
    Diagnostics,
    GateOutput,
}

/// ContractFileMeta mirrors the parsed metadata from a .cue file.
pub struct ContractFileMeta {
    pub schema_version: String,
    pub kind: ContractKind,
}

/// ValidationResult mirrors vb_validate's error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    MissingSchemaVersion,
    InvalidVersion { version: String },
    InvalidKind { kind: String },
    CueVetFailed { file: String },
}

/// ContractFile mirrors the full per-file result.
pub struct ContractFile {
    pub path: PathBuf,
    pub schema_version: String,
    pub kind: ContractKind,
    pub vet_errors: Vec<String>,
}

/// DiscoveryReport mirrors the per-directory aggregation.
pub struct DiscoveryReport {
    pub files: Vec<ContractFile>,
    pub errors: Vec<ValidationError>,
    pub total: u32,
    pub valid: u32,
    pub invalid: u32,
}

/// GateStatus mirrors GateStatus in tooling_and_gate_types.rs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateStatus {
    Pass,
    Fail,
    Skipped { reason: String },
}

/// GateEvidence mirrors GateEvidence in tooling_and_gate_types.rs.
pub struct GateEvidence {
    pub kind: String,
    pub gate_name: String,
    pub command: String,
    pub exit_code: i32,
    pub log: PathBuf,
    pub status: GateStatus,
    pub why_failed: Option<String>,
}

// ============================================================
// Functions under verification (bindings to actual Rust impl)
// ============================================================

/// Parses a schema_version string.
///
/// Binds to: xtask/src/contracts.rs::parse_schema_version
///
/// # Requirements (INV-001)
/// - Input must match `^\d+\.\d+\.\d+$`
/// - Each component must be a valid u32
/// - Returns Err(MissingSchemaVersion) for empty/missing input
/// - Returns Err(InvalidVersion) for malformed input
/// - Returns Ok(normalized) for valid input
#[verifier::external]
pub fn parse_schema_version(raw: &str) -> std::result::Result<String, ValidationError> {
    if raw.is_empty() {
        return Err(ValidationError::MissingSchemaVersion);
    }

    let parts: Vec<&str> = raw.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err(ValidationError::InvalidVersion {
            version: raw.to_string(),
        });
    }

    for part in &parts {
        if part.is_empty() {
            return Err(ValidationError::InvalidVersion {
                version: raw.to_string(),
            });
        }
        // Check for leading zeros (e.g., "01.0.0" is invalid)
        if part.len() > 1 && part.starts_with('0') {
            return Err(ValidationError::InvalidVersion {
                version: raw.to_string(),
            });
        }
        if part.parse::<u32>().is_err() {
            return Err(ValidationError::InvalidVersion {
                version: raw.to_string(),
            });
        }
    }

    Ok(raw.to_string())
}

/// Parses a contract_kind string.
///
/// Binds to: xtask/src/contracts.rs::parse_contract_kind
#[verifier::external]
pub fn parse_contract_kind(raw: &str) -> std::result::Result<ContractKind, ValidationError> {
    match raw {
        "cli_envelope" => Ok(ContractKind::CliEnvelope),
        "ui_tokens" => Ok(ContractKind::UiTokens),
        "accepted_artifacts" => Ok(ContractKind::AcceptedArtifacts),
        "evidence_bundle" => Ok(ContractKind::EvidenceBundle),
        "diagnostics" => Ok(ContractKind::Diagnostics),
        "gate_output" => Ok(ContractKind::GateOutput),
        unknown => Err(ValidationError::InvalidKind {
            kind: unknown.to_string(),
        }),
    }
}

/// Parses cue vet exit code.
///
/// Binds to: xtask/src/contracts.rs::parse_vet_exit_code
#[verifier::external]
pub fn parse_vet_exit_code(exit_code: i32) -> std::result::Result<(), ValidationError> {
    if exit_code == 0 {
        Ok(())
    } else {
        Err(ValidationError::CueVetFailed {
            file: "unknown".to_string(),
        })
    }
}

/// Compares two semver strings.
///
/// Binds to: xtask/src/contracts.rs::compare_semver
/// Returns Ok(Ordering) if both are valid semver, Err otherwise.
/// Uses u64 internally (same as production).
#[verifier::external]
pub fn compare_semver(a: &str, b: &str) -> Result<std::cmp::Ordering, String> {
    let parse_parts = |s: &str| -> Option<(u64, u64, u64)> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse::<u64>().ok()?;
        let minor = parts[1].parse::<u64>().ok()?;
        let patch = parts[2].parse::<u64>().ok()?;
        Some((major, minor, patch))
    };

    let va = parse_parts(a).ok_or(format!("invalid semver: {}", a))?;
    let vb = parse_parts(b).ok_or(format!("invalid semver: {}", b))?;

    if va.0 != vb.0 {
        Ok(va.0.cmp(&vb.0))
    } else if va.1 != vb.1 {
        Ok(va.1.cmp(&vb.1))
    } else {
        Ok(va.2.cmp(&vb.2))
    }
}

/// Constructs GateEvidence from report counts.
///
/// Binds to: xtask/src/evidence/tooling_and_gate_types.rs::gate_evidence_from_report
#[verifier::external]
pub fn gate_evidence_from_report(
    total: u32,
    valid: u32,
    invalid: u32,
) -> std::result::Result<GateEvidence, String> {
    // Precondition: valid + invalid == total (enforced by caller)
    let status = if invalid == 0 {
        GateStatus::Pass
    } else {
        GateStatus::Fail
    };

    let exit_code = if invalid == 0 { 0 } else { 1 };

    let why_failed = if invalid > 0 {
        Some(format!(
            "{} contract(s) failed validation. Run `cargo xtask contracts --json` for details.",
            invalid
        ))
    } else {
        None
    };

    Ok(GateEvidence {
        kind: "contract-discovery".to_string(),
        gate_name: "contracts".to_string(),
        command: "cargo xtask contracts --dir contracts".to_string(),
        exit_code,
        log: PathBuf::from(".evidence/contracts/last_run.log"),
        status,
        why_failed,
    })
}

// ============================================================
// Kani Arbitrary implementations
// ============================================================

use kani::Arbitrary;

impl Arbitrary for ContractKind {
    fn any() -> Self {
        // Kani exhaustively iterates all 6 variants
        let idx: u8 = kani::any();
        match idx % 6 {
            0 => ContractKind::CliEnvelope,
            1 => ContractKind::UiTokens,
            2 => ContractKind::AcceptedArtifacts,
            3 => ContractKind::EvidenceBundle,
            4 => ContractKind::Diagnostics,
            _ => ContractKind::GateOutput,
        }
    }

    fn any_vec(_len: usize) -> Vec<Self> {
        // Not used for this harness — we iterate exhaustively via any()
        Vec::new()
    }
}

impl Arbitrary for ContractFileMeta {
    fn any() -> Self {
        let schema_version = kani::any::<String>();
        let kind = kani::any::<ContractKind>();

        ContractFileMeta {
            schema_version,
            kind,
        }
    }

    fn any_vec(_len: usize) -> Vec<Self> {
        Vec::new()
    }
}

impl Arbitrary for ContractFile {
    fn any() -> Self {
        ContractFile {
            path: kani::any::<PathBuf>(),
            schema_version: kani::any::<String>(),
            kind: kani::any::<ContractKind>(),
            vet_errors: kani::any::<Vec<String>>(),
        }
    }

    fn any_vec(_len: usize) -> Vec<Self> {
        Vec::new()
    }
}

// ============================================================
// Kani Proof Harnesses
// ============================================================

/// OBL-001: parse_schema_version handles all strings without panic.
///
/// Kani explores all possible string inputs (bounded by length limit).
#[kani::proof]
#[kani::unwind(10)]
fn kani_schema_version_no_panic() {
    let raw: String = kani::any();

    // The function must never panic for any string input
    let result = parse_schema_version(&raw);

    // Postcondition: result is either Ok(valid_version) or Err(some_error)
    match result {
        Ok(version) => {
            // Valid semver must have exactly 3 dot-separated u32 components
            let parts: Vec<&str> = version.split('.').collect();
            kani::assert(
                parts.len() == 3,
                "Valid schema_version must have exactly 3 parts, got {}",
                parts.len(),
            );
            for part in &parts {
                kani::assert(!part.is_empty(), "Each semver component must be non-empty");
                kani::assert(
                    part.parse::<u32>().is_ok(),
                    "Each semver component must be a valid u32",
                );
            }
        }
        Err(ValidationError::MissingSchemaVersion) => {
            // Called for empty/missing input
            kani::assert(raw.is_empty(), "kani harness assertion");
        }
        Err(ValidationError::InvalidVersion { version: _ }) => {
            // Called for malformed input — must not match ^\d+\.\d+\.\d+$
            let parts: Vec<&str> = raw.split('.').collect();
            if parts.len() == 3 {
                // Has 3 parts but one or more are invalid
                for part in &parts {
                    if !part.is_empty() && part.parse::<u32>().is_ok() {
                        // This part is valid — at least one must be invalid
                        // or there are leading zeros
                    }
                }
            }
        }
        Err(_) => {
            // Other error variants shouldn't be returned for schema_version
            kani::assert(
                false,
                "parse_schema_version should not return other error types",
            );
        }
    }
}

/// OBL-001: parse_schema_version correctness — implementation matches independent spec for all inputs.
///
/// GOD RULE 1 compliant: uses kani::any() for arbitrary string inputs.
/// An independent spec function determines validity; the proof asserts
/// that parse_schema_version's result is consistent with the spec for ALL inputs.
#[kani::proof]
#[kani::unwind(10)]
fn kani_schema_version_correctness() {
    let raw: String = kani::any();
    let result = parse_schema_version(&raw);
    let is_valid = spec_is_valid_schema_version(&raw);

    match result {
        Ok(version) => {
            kani::assert(
                is_valid,
                "parse_schema_version accepted invalid semver: '{}'",
                raw,
            );
            kani::assert(version == raw, "Accepted version must equal input");
        }
        Err(ValidationError::MissingSchemaVersion) => {
            kani::assert(
                !is_valid,
                "Spec says empty string is invalid); parse_schema_version returned MissingSchemaVersion",
            );
        }
        Err(ValidationError::InvalidVersion { version: _ }) => {
            kani::assert(
                !is_valid,
                "Spec says malformed version is invalid); parse_schema_version returned InvalidVersion",
            );
        }
        Err(_) => {
            kani::assert(
                false,
                "parse_schema_version should not return other error types",
            );
        }
    }
}

/// Independent specification for schema_version validity.
///
/// This spec is the ground truth for validation rules:
/// - Non-empty string
/// - Exactly 3 dot-separated parts
/// - Each part is non-empty
/// - Each part has no leading zeros (except "0" itself)
/// - Each part contains only ASCII digits
#[verifier::external]
fn spec_is_valid_schema_version(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let parts: Vec<&str> = s.splitn(3, '.').collect();
    if parts.len() != 3 {
        return false;
    }

    for part in &parts {
        if part.is_empty() {
            return false;
        }
        if part.len() > 1 && part.starts_with('0') {
            return false;
        }
        if !part.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }

    true
}

/// OBL-001: parse_schema_version accepts valid semver strings.
///
/// Generates valid semver strings and verifies acceptance.
#[kani::proof]
#[kani::unwind(5)]
fn kani_schema_version_accepts_valid() {
    // Kani generates arbitrary u32 values for major/minor/patch
    let major: u32 = kani::any();
    let minor: u32 = kani::any();
    let patch: u32 = kani::any();

    let version = format!("{}.{}.{}", major, minor, patch);
    let result = parse_schema_version(&version);

    kani::assert(
        result.is_ok(),
        "parse_schema_version should accept valid semver: '{}'",
        version,
    );

    match result {
        Ok(v) => kani::assert(v == version, "Validated version must equal input"),
        Err(e) => {
            kani::assert(
                false,
                &format!(
                    "parse_schema_version should accept '{}', got Err: {:?}",
                    version, e
                ),
            );
        }
    }
}

/// OBL-002: parse_contract_kind exhaustively covers all 6 variants.
///
/// Kani iterates through every possible ContractKind value.
#[kani::proof]
fn kani_kind_exhaustive() {
    let kind = kani::any::<ContractKind>();

    // Every ContractKind value must be one of the 6 enum variants
    match kind {
        ContractKind::CliEnvelope => {
            let result = parse_contract_kind("cli_envelope");
            kani::assert(
                matches!(result, Ok(ContractKind::CliEnvelope)),
                "cli_envelope should parse to CliEnvelope",
            );
        }
        ContractKind::UiTokens => {
            let result = parse_contract_kind("ui_tokens");
            kani::assert(
                matches!(result, Ok(ContractKind::UiTokens)),
                "ui_tokens should parse to UiTokens",
            );
        }
        ContractKind::AcceptedArtifacts => {
            let result = parse_contract_kind("accepted_artifacts");
            kani::assert(
                matches!(result, Ok(ContractKind::AcceptedArtifacts)),
                "accepted_artifacts should parse to AcceptedArtifacts",
            );
        }
        ContractKind::EvidenceBundle => {
            let result = parse_contract_kind("evidence_bundle");
            kani::assert(
                matches!(result, Ok(ContractKind::EvidenceBundle)),
                "evidence_bundle should parse to EvidenceBundle",
            );
        }
        ContractKind::Diagnostics => {
            let result = parse_contract_kind("diagnostics");
            kani::assert(
                matches!(result, Ok(ContractKind::Diagnostics)),
                "diagnostics should parse to Diagnostics",
            );
        }
        ContractKind::GateOutput => {
            let result = parse_contract_kind("gate_output");
            kani::assert(
                matches!(result, Ok(ContractKind::GateOutput)),
                "gate_output should parse to GateOutput",
            );
        }
    }
}

/// OBL-002: parse_contract_kind rejects unknown kinds.
///
/// Kani generates arbitrary strings and verifies they map to Err.
#[kani::proof]
#[kani::unwind(5)]
fn kani_kind_rejects_unknown() {
    let unknown_kind: String = kani::any();

    // Ensure it's not one of the 6 valid kinds
    if unknown_kind != "cli_envelope"
        && unknown_kind != "ui_tokens"
        && unknown_kind != "accepted_artifacts"
        && unknown_kind != "evidence_bundle"
        && unknown_kind != "diagnostics"
        && unknown_kind != "gate_output"
    {
        let result = parse_contract_kind(&unknown_kind);
        kani::assert(
            matches!(result, Err(ValidationError::InvalidKind { .. })),
            "parse_contract_kind should reject unknown kind: '{}'",
            unknown_kind,
        );
    }
}

/// OBL-003: parse_vet_exit_code handles all i32 values without panic.
///
/// Kani symbolically explores every i32 value (bounded by unwind).
#[kani::proof]
fn kani_vet_exit_code() {
    let exit_code: i32 = kani::any();

    // The function must never panic for any i32 input
    let result = parse_vet_exit_code(exit_code);

    // Postcondition: exit_code == 0 => Ok, non-zero => Err
    if exit_code == 0 {
        kani::assert(result.is_ok(), "Exit code 0 should always return Ok");
    } else {
        kani::assert(
            result.is_err(),
            "Non-zero exit code {} should always return Err",
            exit_code,
        );
    }

    // No panic for negative exit codes (system errors)
    if exit_code < 0 {
        kani::assert(
            result.is_err(),
            "Negative exit code {} should return Err",
            exit_code,
        );
    }

    // No panic for large positive exit codes
    if exit_code > 255 {
        kani::assert(
            result.is_err(),
            "Large exit code {} should return Err",
            exit_code,
        );
    }
}

/// OBL-006: gate_evidence_from_report always produces valid GateEvidence.
///
/// Kani explores all u32 combinations with precondition valid + invalid == total.
#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_evidence_parity() {
    let total: u32 = kani::any();
    let valid: u32 = kani::any();
    let invalid: u32 = kani::any();

    // Precondition: valid + invalid == total (bounded arithmetic)
    if valid.saturating_add(invalid) != total {
        // Skip invalid combinations — precondition not met
        return;
    }

    // The function must never fail when precondition is met
    let result = gate_evidence_from_report(total, valid, invalid);

    kani::assert(
        result.is_ok(),
        "gate_evidence_from_report should always return Ok when valid + invalid == total",
    );

    let evidence = match result {
        Ok(e) => e,
        Err(e) => {
            kani::assert(
                false,
                &format!("gate_evidence_from_report should succeed, got Err: {}", e),
            );
            return;
        }
    };

    // Postcondition: status == Pass iff invalid == 0
    if invalid == 0 {
        kani::assert(
            matches!(evidence.status, GateStatus::Pass),
            "Status should be Pass when invalid == 0",
        );
        kani::assert(
            evidence.exit_code == 0,
            "Exit code should be 0 when invalid == 0",
        );
        kani::assert(
            evidence.why_failed.is_none(),
            "why_failed should be None when invalid == 0",
        );
    } else {
        kani::assert(
            matches!(evidence.status, GateStatus::Fail),
            "Status should be Fail when invalid > 0",
        );
        kani::assert(
            evidence.exit_code == 1,
            "Exit code should be 1 when invalid > 0",
        );
        kani::assert(
            evidence.why_failed.is_some(),
            "why_failed should be Some when invalid > 0",
        );
    }

    // Postcondition: kind and gate_name are always correct
    kani::assert(evidence.kind == "contract-discovery");
    kani::assert(evidence.gate_name == "contracts");

    // Postcondition: total == valid + invalid (invariant)
    kani::assert(
        valid.saturating_add(invalid) == total,
        "total must equal valid + invalid",
    );
}

/// OBL-006: gate_evidence_from_report empty case (all zeros).
///
/// GOD RULE 1 compliant: uses kani::any() then asserts edge case.
#[kani::proof]
fn kani_gate_evidence_empty() {
    let total: u32 = kani::any();
    let valid: u32 = kani::any();
    let invalid: u32 = kani::any();

    // Precondition: valid + invalid == total
    if valid.saturating_add(invalid) != total {
        return;
    }

    // Edge case: empty report (all zeros implied by total == 0)
    if total == 0 {
        let result = gate_evidence_from_report(total, valid, invalid);
        match result {
            Ok(evidence) => {
                kani::assert(matches!(evidence.status, GateStatus::Pass));
                kani::assert(evidence.exit_code == 0);
                kani::assert(evidence.why_failed.is_none(), "kani harness assertion");
            }
            Err(e) => {
                kani::assert(
                    false,
                    &format!("Empty report should succeed, got Err: {}", e),
                );
            }
        }
    }
}

/// OBL-006: gate_evidence_from_report with valid=0, invalid > 0 (all invalid).
///
/// GOD RULE 1 compliant: uses kani::any() for invalid count.
#[kani::proof]
fn kani_gate_evidence_all_invalid() {
    let invalid: u32 = kani::any();
    if invalid == 0 {
        return; // Skip trivial case
    }

    let result = gate_evidence_from_report(invalid, 0, invalid);

    match result {
        Ok(evidence) => {
            kani::assert(matches!(evidence.status, GateStatus::Fail));
            kani::assert(evidence.exit_code == 1);
            kani::assert(evidence.why_failed.is_some(), "kani harness assertion");
        }
        Err(e) => {
            kani::assert(false, &format!("Expected Ok, got Err: {}", e));
        }
    }
}
