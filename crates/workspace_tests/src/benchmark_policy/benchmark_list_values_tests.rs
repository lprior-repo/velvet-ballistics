use super::list_values::normalize_lists;
use crate::benchmark_policy::error::{Context, Result};
use proptest::prelude::*;
use serde_yaml::Value;

fn list_value_field(text: &str, field: &str) -> Result<Vec<Value>> {
    let normalized = normalize_lists(text)?;
    let value: Value = serde_yaml::from_str(&normalized)?;
    value
        .get(field)
        .and_then(Value::as_sequence)
        .cloned()
        .with_context(|| format!("expected normalized {field} list"))
}

fn list_value(text: &str) -> Result<Vec<Value>> {
    list_value_field(text, "deferred_modes")
}

fn literal_item(token: &str, expected: &str) -> Result<()> {
    let text = format!("deferred_modes:\n  - {token}\n");
    assert_eq!(list_value(&text)?, vec![Value::String(expected.to_owned())]);
    Ok(())
}

fn normalization_error(input: &str, expected: &str) -> Result<()> {
    match normalize_lists(input) {
        Err(error) => assert_eq!(error.to_string(), expected),
        Ok(_) => crate::benchmark_policy::error::bail!("expected {expected}"),
    }
    Ok(())
}

#[test]
fn preserves_null_and_flow_looking_literals_without_yaml_reinterpretation() -> Result<()> {
    [
        "null",
        "~",
        "[a, b]",
        "{ key: value }",
        "[unclosed",
        "{unclosed",
        "word # retained",
    ]
    .iter()
    .try_for_each(|token| literal_item(token, token))
}

#[test]
fn preserves_noninteger_numeric_looking_literals() -> Result<()> {
    ["+42", "42.0", "0x2a", "1e3", "NaN", "Infinity", "----"]
        .iter()
        .try_for_each(|token| literal_item(token, token))
}

#[test]
fn decodes_only_original_double_quoted_escapes() -> Result<()> {
    literal_item(
        r#""line\ncolumn\t\"quote\"\\end""#,
        "line\ncolumn\t\"quote\"\\end",
    )?;
    literal_item("\"\"", "")?;
    literal_item("\"42\"", "42")
}

#[test]
fn decodes_original_single_quote_doubling() -> Result<()> {
    literal_item("'hello''s'", "hello's")?;
    literal_item("''", "")?;
    literal_item("'\\n'", "\\n")
}

#[test]
fn unmatched_quotes_remain_original_literal_text() -> Result<()> {
    ["'", "\"", "'open", "\"open", "\"closed\" # text"]
        .iter()
        .try_for_each(|token| literal_item(token, token))
}

#[test]
fn boolean_spellings_and_arbitrary_length_ascii_integers_remain_typed() -> Result<()> {
    let input = "deferred_modes:\n  - True\n  - FALSE\n  - -42\n  - 184467440737095516160\n";
    assert_eq!(
        list_value(input)?,
        vec![
            Value::Bool(true),
            Value::Bool(false),
            Value::Number(0.into()),
            Value::Number(0.into())
        ]
    );
    Ok(())
}

#[test]
fn unsupported_escape_reports_the_actual_physical_line() -> Result<()> {
    normalization_error(
        "# heading\ndeferred_modes:\n\n  - \"bad\\x\"\n",
        "performance budget line 4: unsupported escape \\x",
    )
}

#[test]
fn trailing_escape_reports_the_original_scalar_failure() -> Result<()> {
    normalization_error(
        "deferred_modes:\n  - \"trailing\\\"\n",
        "performance budget line 2: trailing backslash in quoted scalar",
    )
}

#[test]
fn repeated_minus_integer_reports_original_integer_conversion_failure() -> Result<()> {
    normalization_error(
        "deferred_modes:\n  - --42\n",
        "performance budget line 2: integer out of range: --42",
    )
}

#[test]
fn physical_tabs_are_not_hidden_by_requoting() -> Result<()> {
    normalization_error(
        "deferred_modes:\n  - word\tword\n",
        "performance budget line 2: tab indentation is not allowed",
    )
}

#[test]
fn blank_and_comment_lines_preserve_the_active_target_list() -> Result<()> {
    let input = "deferred_modes:\n  - null\n\n  # keep comment\n  - [a, b]\n";
    let normalized = normalize_lists(input)?;
    assert_eq!(
        normalized,
        "deferred_modes:\n  - \"null\"\n\n  # keep comment\n  - \"[a, b]\"\n"
    );
    assert_eq!(
        list_value(input)?,
        vec![
            Value::String("null".to_owned()),
            Value::String("[a, b]".to_owned())
        ]
    );
    Ok(())
}

#[test]
fn preserves_non_target_bytes_and_all_line_endings() -> Result<()> {
    let input = "# keep\r\ndeferred_modes:\r\n  - null\r\nother:\n  - null\nrequired_evidence_fields:\n  - { k: v }";
    let expected = "# keep\r\ndeferred_modes:\r\n  - \"null\"\r\nother:\n  - null\nrequired_evidence_fields:\n  - \"{ k: v }\"";
    let actual = normalize_lists(input)?;
    assert_eq!(actual, expected);
    assert_eq!(actual.lines().count(), input.lines().count());
    Ok(())
}

#[test]
fn nested_same_named_fields_do_not_activate_top_level_normalization() -> Result<()> {
    let input =
        "other:\n  deferred_modes:\n    - null\n  required_evidence_fields:\n    - [a, b]\n";
    assert_eq!(normalize_lists(input)?, input);
    Ok(())
}

#[test]
fn inline_flow_lists_are_original_scalar_values() -> Result<()> {
    let input = "deferred_modes: [generated, maxperf]\nrequired_evidence_fields: [metric]\n";
    assert_eq!(
        normalize_lists(input)?,
        "deferred_modes: \"[generated, maxperf]\"\nrequired_evidence_fields: \"[metric]\"\n"
    );
    Ok(())
}

#[test]
fn handles_empty_input_and_unicode_literal_strings() -> Result<()> {
    assert_eq!(normalize_lists("")?, "");
    literal_item("café🦀", "café🦀")
}

#[test]
fn u1f_quoted_booleans_remain_strings_both_fields() -> Result<()> {
    for field in ["deferred_modes", "required_evidence_fields"] {
        let input = format!("{field}:\n  - \"true\"\n  - 'TRUE'\n  - \"false\"\n  - 'FALSE'\n");
        assert_eq!(
            list_value_field(&input, field)?,
            vec![
                Value::String("true".to_owned()),
                Value::String("TRUE".to_owned()),
                Value::String("false".to_owned()),
                Value::String("FALSE".to_owned())
            ]
        );
    }
    Ok(())
}

#[test]
fn u1f_bare_booleans_become_typed_both_fields() -> Result<()> {
    for field in ["deferred_modes", "required_evidence_fields"] {
        let input = format!("{field}:\n  - \u{1f}true\u{1f}\n  - \u{1f}False\u{1f}\n");
        assert_eq!(
            list_value_field(&input, field)?,
            vec![Value::Bool(true), Value::Bool(false)]
        );
    }
    Ok(())
}

#[test]
fn u1f_bare_integers_become_zero_both_fields() -> Result<()> {
    for field in ["deferred_modes", "required_evidence_fields"] {
        let input = format!("{field}:\n  - \u{1f}2\u{1f}\n  - \u{1f}٢\u{1f}\n");
        assert_eq!(
            list_value_field(&input, field)?,
            vec![Value::Number(0.into()), Value::Number(0.into())]
        );
    }
    Ok(())
}

#[test]
fn u1f_inside_quotes_retained_both_fields() -> Result<()> {
    for field in ["deferred_modes", "required_evidence_fields"] {
        let input = format!(
            "{field}:\n  - \"hello\u{1f}world\"\n  - 'keep\u{1f}internal'\n  - \"\u{1f}2\u{1f}\"\n  - '\u{1f}true\u{1f}'\n"
        );
        assert_eq!(
            list_value_field(&input, field)?,
            vec![
                Value::String("hello\u{1f}world".to_owned()),
                Value::String("keep\u{1f}internal".to_owned()),
                Value::String("\u{1f}2\u{1f}".to_owned()),
                Value::String("\u{1f}true\u{1f}".to_owned())
            ]
        );
    }
    Ok(())
}

#[test]
fn u1f_outside_quotes_stripped_both_fields() -> Result<()> {
    for field in ["deferred_modes", "required_evidence_fields"] {
        let input = format!("{field}:\n  - \u{1f}\"false\"\u{1f}\n");
        assert_eq!(
            list_value_field(&input, field)?,
            vec![Value::String("false".to_owned())]
        );
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn normalized_literal_text_roundtrips_and_is_idempotent(tail in "[a-zA-Z0-9 #:'{}]{0,80}") {
        let token = format!("literal{tail}");
        let input = format!("deferred_modes:\n  - {token}\n");
        let once = normalize_lists(&input)?;
        prop_assert_eq!(normalize_lists(&once)?, once);
        prop_assert_eq!(list_value(&input)?, vec![Value::String(token.trim().to_owned())]);
    }
}
