use super::*;
use crate::benchmark_policy::error::Context;

fn extra_item(field: &str, token: &str) -> Result<String> {
    let marker = format!("{field}:\n");
    let (prefix, suffix) = VALID_BUDGET
        .split_once(&marker)
        .context("fixture list header")?;
    Ok(format!("{prefix}{marker}  - {token}\n{suffix}"))
}

fn replace_field(field: &str, replacement: &str) -> Result<String> {
    let end = match field {
        "deferred_modes" => "required_evidence_fields:\n",
        "required_evidence_fields" => "benchmarks:\n",
        _ => crate::benchmark_policy::error::bail!("unknown fixture field {field}"),
    };
    let marker = format!("{field}:\n");
    let (prefix, rest) = VALID_BUDGET.split_once(&marker).context("fixture header")?;
    let (_, suffix) = rest.split_once(end).context("fixture next field")?;
    Ok(format!("{prefix}{replacement}{end}{suffix}"))
}

fn rejected(text: &str, expected: &str) -> Result<()> {
    match parse_budget(text) {
        Err(error) => assert_eq!(error.to_string(), expected),
        Ok(_) => crate::benchmark_policy::error::bail!("expected {expected}"),
    }
    Ok(())
}

fn accepted_item(field: &str, token: &str, expected: &str) -> Result<()> {
    let budget = parse_budget(&extra_item(field, token)?)?;
    let baseline = parse_budget(VALID_BUDGET)?;
    let (actual, original) = match field {
        "deferred_modes" => (&budget.deferred_modes, baseline.deferred_modes),
        "required_evidence_fields" => (
            &budget.required_evidence_fields,
            baseline.required_evidence_fields,
        ),
        _ => crate::benchmark_policy::error::bail!("unknown fixture field {field}"),
    };
    let expected_values: Vec<_> = std::iter::once(expected.to_owned())
        .chain(original)
        .collect();
    assert_eq!(actual, &expected_values);
    let error = validate_evidence(Path::new("."), &budget, "", "0e24bc9")
        .err()
        .context("empty evidence must fail after schema")?;
    assert_eq!(
        error.to_string(),
        "benchmark evidence missing budgeted metrics: transition_set"
    );
    Ok(())
}

// Original parse_scalar returns these unquoted tokens unchanged as strings.
#[test]
fn accepts_deferred_bare_null_as_original_literal() -> Result<()> {
    accepted_item("deferred_modes", "null", "null")
}

#[test]
fn accepts_required_bare_null_as_original_literal() -> Result<()> {
    accepted_item("required_evidence_fields", "null", "null")
}

#[test]
fn accepts_deferred_flow_sequence_as_original_literal() -> Result<()> {
    accepted_item("deferred_modes", "[a, b]", "[a, b]")
}

#[test]
fn accepts_required_flow_sequence_as_original_literal() -> Result<()> {
    accepted_item("required_evidence_fields", "[a, b]", "[a, b]")
}

#[test]
fn accepts_deferred_flow_mapping_as_original_literal() -> Result<()> {
    accepted_item("deferred_modes", "{ key: value }", "{ key: value }")
}

#[test]
fn accepts_required_flow_mapping_as_original_literal() -> Result<()> {
    accepted_item(
        "required_evidence_fields",
        "{ key: value }",
        "{ key: value }",
    )
}

#[test]
fn accepts_quoted_numeric_strings_in_both_fields() -> Result<()> {
    ["deferred_modes", "required_evidence_fields"]
        .iter()
        .try_for_each(|field| {
            accepted_item(field, "\"42\"", "42")?;
            accepted_item(field, "'42'", "42")
        })
}

#[test]
fn rejects_numeric_and_boolean_items_in_both_fields() -> Result<()> {
    ["deferred_modes", "required_evidence_fields"]
        .iter()
        .try_for_each(|field| {
            [
                "42",
                "-42",
                "00042",
                "184467440737095516160",
                "true",
                "FALSE",
            ]
            .iter()
            .try_for_each(|token| {
                rejected(
                    &extra_item(field, token)?,
                    &format!("performance budget {field} must be a list of strings"),
                )
            })
        })
}

#[test]
fn rejects_missing_fields_with_the_original_field_diagnostic() -> Result<()> {
    ["deferred_modes", "required_evidence_fields"]
        .iter()
        .try_for_each(|field| {
            rejected(
                &replace_field(field, "")?,
                &format!("performance budget {field} must be a list of strings"),
            )
        })
}

#[test]
fn rejects_nonlist_fields_including_inline_flow_lists() -> Result<()> {
    ["deferred_modes", "required_evidence_fields"]
        .iter()
        .try_for_each(|field| {
            [
                "42",
                "null",
                "true",
                "[generated, maxperf]",
                "{ key: value }",
                "\n  key: value",
            ]
            .iter()
            .try_for_each(|value| {
                rejected(
                    &replace_field(field, &format!("{field}: {value}\n"))?,
                    &format!("performance budget {field} must be a list of strings"),
                )
            })
        })
}

#[test]
fn milestone_precedes_deferred_list_type_with_a_causal_control() -> Result<()> {
    let bad_list = extra_item("deferred_modes", "42")?;
    rejected(
        &bad_list.replace("ir-interpreter", "wrong"),
        "performance budget milestone must be ir-interpreter, got wrong",
    )?;
    rejected(
        &bad_list,
        "performance budget deferred_modes must be a list of strings",
    )
}

#[test]
fn deferral_flag_precedes_both_list_types_with_causal_controls() -> Result<()> {
    ["deferred_modes", "required_evidence_fields"]
        .iter()
        .try_for_each(|field| {
            let bad_list = extra_item(field, "true")?;
            rejected(
                &bad_list.replace(
                    "generated_maxperf_deferred: true",
                    "generated_maxperf_deferred: false",
                ),
                "performance budget must mark generated/maxperf claims as deferred",
            )?;
            rejected(
                &bad_list,
                &format!("performance budget {field} must be a list of strings"),
            )
        })
}

#[test]
fn deferred_membership_precedes_required_type_with_a_causal_control() -> Result<()> {
    let bad_required = extra_item("required_evidence_fields", "42")?;
    rejected(
        &bad_required.replace("  - maxperf\n", ""),
        "deferred_modes must include generated and maxperf",
    )?;
    rejected(
        &bad_required,
        "performance budget required_evidence_fields must be a list of strings",
    )
}

#[test]
fn missing_deferred_precedes_invalid_required_with_a_causal_control() -> Result<()> {
    let bad_required = extra_item("required_evidence_fields", "42")?;
    let both_bad = bad_required.replace("deferred_modes:\n  - generated\n  - maxperf\n", "");
    rejected(
        &both_bad,
        "performance budget deferred_modes must be a list of strings",
    )?;
    rejected(
        &bad_required,
        "performance budget required_evidence_fields must be a list of strings",
    )
}

#[test]
fn deferred_type_precedes_required_type_when_both_items_are_numeric() -> Result<()> {
    let bad_required = extra_item("required_evidence_fields", "42")?;
    let both_bad = bad_required.replace("deferred_modes:\n", "deferred_modes:\n  - 42\n");
    rejected(
        &both_bad,
        "performance budget deferred_modes must be a list of strings",
    )?;
    rejected(
        &bad_required,
        "performance budget required_evidence_fields must be a list of strings",
    )
}

#[test]
fn required_type_precedes_threshold_validation_with_a_causal_control() -> Result<()> {
    let bad_threshold =
        VALID_BUDGET.replace("max_regression_percent: 3", "max_regression_percent: 0");
    let both_bad = bad_threshold.replace(
        "required_evidence_fields:\n",
        "required_evidence_fields:\n  - 42\n",
    );
    rejected(
        &both_bad,
        "performance budget required_evidence_fields must be a list of strings",
    )?;
    rejected(
        &bad_threshold,
        "benchmark transition_set must define a positive max_regression_percent",
    )
}

#[test]
fn required_coverage_precedes_threshold_validation_with_a_causal_control() -> Result<()> {
    let bad_threshold =
        VALID_BUDGET.replace("max_regression_percent: 3", "max_regression_percent: 0");
    rejected(
        &bad_threshold.replace("  - metric\n", ""),
        "performance budget missing evidence fields: metric",
    )?;
    rejected(
        &bad_threshold,
        "benchmark transition_set must define a positive max_regression_percent",
    )
}

#[test]
fn normalization_keeps_duplicate_target_keys_rejected() -> Result<()> {
    let text = format!("{}deferred_modes:\n  - null\n", VALID_BUDGET);
    let error = parse_budget(&text)
        .err()
        .context("expected duplicate target key rejection")?;
    assert!(
        error
            .to_string()
            .contains("duplicate entry with key \"deferred_modes\"")
    );
    Ok(())
}

#[test]
fn required_coverage_precedes_a_malformed_threshold_value() -> Result<()> {
    let threshold = VALID_BUDGET.replace(
        "max_regression_percent: 3",
        "max_regression_percent: not-a-number",
    );
    rejected(
        &threshold.replace("  - metric\n", ""),
        "performance budget missing evidence fields: metric",
    )?;
    let error = parse_budget(&threshold)
        .err()
        .context("expected threshold decoding failure")?;
    let message = error.to_string();
    assert_eq!(
        message.split(':').next(),
        Some("parse performance budget yaml")
    );
    assert!(message.contains("benchmarks.transition_set.max_regression_percent"));
    Ok(())
}
