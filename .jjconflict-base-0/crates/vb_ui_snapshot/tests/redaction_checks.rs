use proptest::prelude::*;
use vb_ui_snapshot::report::{CheckKind, make_fail_result, make_pass_result, make_screen_result};

const RAW_SENTINEL: &str = "vb_nf2u_secret_sentinel";
const RAW_API_KEY: &str = "sk_test_vb_nf2u_raw_secret";
const RAW_TOKEN: &str = "Bearer vb_nf2u_token";
const RAW_PASSWORD: &str = "password=hunter2";
const RAW_IDEMPOTENCY_KEY: &str = "Idempotency-Key: idem_vb_nf2u_secret";
const RAW_TAINT: &str = "tainted_fixture_value_vb_nf2u";

#[test]
fn redaction_report_fails_when_raw_api_key_check_is_failed() {
    let screen = make_screen_result(
        "execution_overview",
        vec![make_fail_result(
            CheckKind::Redaction,
            "secret_class=api_key",
        )],
    );

    assert!(!screen.passed);
    assert_eq!(
        screen
            .checks
            .first()
            .and_then(|check| check.detail.as_deref()),
        Some("secret_class=api_key")
    );
}

#[test]
fn redaction_report_passes_when_approved_placeholder_has_no_raw_secret() {
    let screen = make_screen_result(
        "execution_overview",
        vec![make_pass_result(CheckKind::Redaction)],
    );

    assert!(screen.passed);
    assert_eq!(
        screen
            .checks
            .first()
            .and_then(|check| check.detail.as_ref()),
        None
    );
}

#[test]
fn redaction_scanner_api_rejects_raw_sentinel_with_exact_diagnostic() {
    let result = require_release_redaction_scan(RAW_SENTINEL);

    assert_eq!(
        result,
        Err(String::from(
            "RedactionViolation { code: \"redaction_violation\", secret_class: \"sentinel\", redacted_sample: \"[REDACTED:sentinel]\" }"
        ))
    );
}

#[test]
fn redaction_scanner_api_rejects_raw_api_key_with_exact_diagnostic() {
    let result = require_release_redaction_scan(RAW_API_KEY);

    assert_eq!(
        result,
        Err(String::from(
            "RedactionViolation { code: \"redaction_violation\", secret_class: \"api_key\", redacted_sample: \"[REDACTED:api_key]\" }"
        ))
    );
}

#[test]
fn redaction_scanner_api_rejects_raw_password_without_echoing_secret() {
    let result = require_release_redaction_scan(RAW_PASSWORD);

    assert_eq!(
        result,
        Err(String::from(
            "RedactionViolation { code: \"redaction_violation\", secret_class: \"password\", redacted_sample: \"[REDACTED:password]\" }"
        ))
    );
}

#[test]
fn redaction_scanner_api_rejects_raw_idempotency_key_with_exact_diagnostic() {
    let result = require_release_redaction_scan(RAW_IDEMPOTENCY_KEY);

    assert_eq!(
        result,
        Err(String::from(
            "RedactionViolation { code: \"redaction_violation\", secret_class: \"idempotency_key\", redacted_sample: \"[REDACTED:idempotency_key]\" }"
        ))
    );
}

#[test]
fn redaction_scanner_api_rejects_raw_tainted_fixture_value_with_exact_diagnostic() {
    let result = require_release_redaction_scan(RAW_TAINT);

    assert_eq!(
        result,
        Err(String::from(
            "RedactionViolation { code: \"redaction_violation\", secret_class: \"tainted_fixture_value\", redacted_sample: \"[REDACTED:tainted_fixture_value]\" }"
        ))
    );
}

proptest! {
    #[test]
    fn redaction_scanner_rejects_any_artifact_containing_required_raw_secret(prefix in "[ -~]{0,32}", suffix in "[ -~]{0,32}") {
        let artifact = format!("{prefix}{RAW_TOKEN}{suffix}");

        prop_assert_eq!(
            require_release_redaction_scan(&artifact),
            Err(String::from("RedactionViolation { code: \"redaction_violation\", secret_class: \"token\", redacted_sample: \"[REDACTED:token]\" }"))
        );
    }

    #[test]
    fn approved_redaction_placeholders_never_fail_without_raw_secret(prefix in "[a-z ]{0,32}", suffix in "[a-z ]{0,32}") {
        let artifact = format!("{prefix}[REDACTED:api_key][REDACTED:token][REDACTED:password]{suffix}");

        prop_assert_eq!(require_release_redaction_scan(&artifact), Ok(()));
    }
}

fn require_release_redaction_scan(artifact: &str) -> Result<(), String> {
    vb_ui_snapshot::redaction::scan_release_artifact(artifact)
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}
