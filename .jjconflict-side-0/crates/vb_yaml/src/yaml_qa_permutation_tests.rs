#![forbid(unsafe_code)]
//! vb-9y985 permutation tests: directives, BOM, merge keys, duplicate-key
//! identity/spans, complex keys, limit counting, encodings, strict optionals.

use super::super::yaml_error::YamlError;
use super::validate_yaml_profile;
use crate::parse_workflow_source;

fn valid_workflow() -> String {
    "version: velvet-ballistics/v1\nname: qa_wf\nwhen:\n  manual: {}\nsteps:\n  - id: only\n    finish:\n      result: '0'\n"
        .to_string()
}

#[test]
fn qa_directives_rejected_all_spellings() {
    let directives = [
        "%YAML 1.1",
        "%YAML 1.2",
        "%YAML 9.9",
        "%TAG !foo! tag:example.com,2024:",
        "%FOO bar",
        "%yaml 1.2",
    ];
    let mut cases = 0_usize;
    for directive in directives {
        for with_marker in [true, false] {
            let marker = if with_marker { "---\n" } else { "" };
            let yaml = format!("{directive}\n{marker}{}", valid_workflow());
            let result = validate_yaml_profile(&yaml);
            assert!(
                matches!(
                    result,
                    Err(YamlError::ForbiddenFeature {
                        detail: "yaml_directive"
                    })
                ),
                "directive {directive:?} marker={with_marker} got {result:?}"
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 12);
}

#[test]
fn qa_bom_valid_workflow_parses() {
    let yaml = format!("\u{feff}{}", valid_workflow());
    let parsed = parse_workflow_source(&yaml);
    assert!(parsed.is_ok(), "BOM workflow rejected: {parsed:?}");
    assert_eq!(validate_yaml_profile(&yaml), Ok(()));
}

#[test]
fn qa_merge_key_spellings_forbidden() {
    let bodies = [
        "a:\n  <<: {b: 1}\n  c: 2\n",
        "a:\n  \"<<\": {b: 1}\n",
        "a:\n  '<<': {b: 1}\n",
        "top:\n  nested:\n    <<: 1\n",
    ];
    for yaml in bodies {
        let result = validate_yaml_profile(yaml);
        assert!(
            matches!(result, Err(YamlError::MergeKeyForbidden { .. })),
            "merge body {yaml:?} got {result:?}"
        );
    }
}

#[test]
fn qa_duplicate_identity_typed() {
    // Quoted vs plain with same text are distinct YAML keys.
    assert_eq!(validate_yaml_profile("true: 1\n\"true\": 2\n"), Ok(()));
    assert_eq!(validate_yaml_profile("1: a\n\"1\": b\n"), Ok(()));
    // Same integer in different spellings duplicates.
    for yaml in ["0x10: a\n16: b\n", "0o20: a\n16: b\n", "true: a\nTrue: b\n"] {
        let result = validate_yaml_profile(yaml);
        assert!(
            matches!(result, Err(YamlError::DuplicateKeyAt { .. })),
            "identity {yaml:?} got {result:?}"
        );
    }
}

#[test]
fn qa_duplicate_span_far_apart() {
    let mut yaml = String::from("name: first\n");
    for i in 0..100 {
        yaml.push_str(&format!("k{i}: {i}\n"));
    }
    yaml.push_str("name: second\n");
    match validate_yaml_profile(&yaml) {
        Err(YamlError::DuplicateKeyAt { key, line, .. }) => {
            assert_eq!(key.as_ref(), "name");
            assert_eq!(line, 102);
        }
        other => panic!("expected spanned duplicate, got {other:?}"),
    }
}

#[test]
fn qa_complex_keys_rejected_at_profile() {
    for yaml in [
        "? [p, q]\n: 1\n? [p, q]\n: 2\n",
        "? {a: 1}\n: 1\n",
        "? [p, q]\n: 1\n",
    ] {
        let result = validate_yaml_profile(yaml);
        assert!(
            matches!(
                result,
                Err(YamlError::FieldShape {
                    field: "mapping key",
                    ..
                })
            ),
            "complex key {yaml:?} got {result:?}"
        );
    }
}

#[test]
fn qa_mapping_limit_counts_per_mapping() {
    // Child mapping with exactly 1024 entries under a parent passes the
    // mapping gate (fails later only if shape is wrong, never MappingTooLarge).
    let mut child_ok = String::from("root:\n");
    for i in 0..1024 {
        child_ok.push_str(&format!("  k{i}: {i}\n"));
    }
    assert_ne!(
        validate_yaml_profile(&child_ok),
        Err(YamlError::MappingTooLarge {
            count: 1029,
            max: 1024
        })
    );
    let mut child_over = String::from("root:\n");
    for i in 0..1025 {
        child_over.push_str(&format!("  k{i}: {i}\n"));
    }
    assert_eq!(
        validate_yaml_profile(&child_over),
        Err(YamlError::MappingTooLarge {
            count: 1025,
            max: 1024
        })
    );
}

#[test]
fn qa_sequence_limit_counts_container_elements() {
    let mut over = String::from("items:\n");
    for _ in 0..10_001 {
        over.push_str("  - {}\n");
    }
    assert_eq!(
        validate_yaml_profile(&over),
        Err(YamlError::SequenceTooLong {
            len: 10_001,
            max: 10_000
        })
    );
    let mut at = String::from("items:\n");
    for _ in 0..10_000 {
        at.push_str("  - {}\n");
    }
    assert_ne!(
        validate_yaml_profile(&at),
        Err(YamlError::SequenceTooLong {
            len: 10_001,
            max: 10_000
        })
    );
}

#[test]
fn qa_utf16_and_nul_diagnosis() {
    let utf16ish = "v\u{0}e\u{0}r\u{0}s\u{0}i\u{0}o\u{0}n\u{0}:\u{0} x\u{0}";
    assert_eq!(
        validate_yaml_profile(utf16ish),
        Err(YamlError::UnsupportedEncoding)
    );
    assert_eq!(
        validate_yaml_profile("a: b\u{0}c\n"),
        Err(YamlError::ForbiddenFeature {
            detail: "null_byte_in_source"
        })
    );
}

#[test]
fn qa_wrong_type_optionals_error_not_absent() {
    let bad_name = valid_workflow().replace("  - id: only\n", "  - id: only\n    name: 123\n");
    assert!(parse_workflow_source(&bad_name).is_err());
    let bad_at_once = "version: velvet-ballistics/v1\nname: qa_wf\nwhen:\n  manual: {}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: '$input.entries'\n      at_once: 'one'\n      steps:\n        - id: inner\n          finish:\n            result: '0'\n  - id: done\n    finish:\n      result: '0'\n";
    assert!(parse_workflow_source(bad_at_once).is_err());
    let neg_at_once = bad_at_once.replace("at_once: 'one'", "at_once: -5");
    assert!(parse_workflow_source(&neg_at_once).is_err());
}

#[test]
fn qa_foreach_legacy_alias_rejected_with_hint() {
    let yaml = valid_workflow().replace(
        "  - id: only\n    finish:",
        "  - id: only\n    foreach:\n      variable: entry\n      input: '$input.entries'\n      steps:\n        - id: inner\n          finish:\n            result: '0'\n  - id: done\n    finish:",
    );
    match parse_workflow_source(&yaml) {
        Err(YamlError::LegacyPrimitive {
            primitive,
            canonical,
        }) => {
            assert_eq!(primitive, "foreach");
            assert_eq!(canonical, "for_each");
        }
        other => panic!("expected legacy foreach rejection, got {other:?}"),
    }
}

#[test]
fn qa_schedule_cron_wrong_type_is_type_error() {
    let yaml = "version: velvet-ballistics/v1\nname: qa_wf\nwhen:\n  schedule:\n    cron: 12345\nsteps:\n  - id: only\n    finish:\n      result: '0'\n";
    assert_eq!(
        parse_workflow_source(yaml),
        Err(YamlError::FieldShape {
            field: "when.schedule.cron",
            expected: "string"
        })
    );
    let missing = "version: velvet-ballistics/v1\nname: qa_wf\nwhen:\n  schedule: {}\nsteps:\n  - id: only\n    finish:\n      result: '0'\n";
    assert_eq!(
        parse_workflow_source(missing),
        Err(YamlError::MissingField {
            field: "when.schedule.cron"
        })
    );
}

fn mapping_with_container_values(entries: usize) -> String {
    let mut source = String::new();
    let mut mapping = true;
    for index in 0..entries {
        let value = if mapping { "{}" } else { "[]" };
        source.push_str(&format!("k{index:04}: {value}\n"));
        mapping = !mapping;
    }
    source
}

#[test]
fn qa_mapping_limit_counts_completed_container_values() -> Result<(), YamlError> {
    let at = mapping_with_container_values(1024);
    assert_eq!(validate_yaml_profile(&at), Ok(()));
    let events = crate::parse_yaml_events(&at)?;
    assert_eq!(
        events
            .iter()
            .filter(|event| event.as_scalar().is_some())
            .count(),
        1024
    );
    let over = mapping_with_container_values(1025);
    let expected = YamlError::MappingTooLarge {
        count: 1025,
        max: 1024,
    };
    assert_eq!(validate_yaml_profile(&over), Err(expected.clone()));
    assert_eq!(crate::parse_yaml_events(&over).map(|_| ()), Err(expected));
    Ok(())
}

#[test]
fn qa_container_value_completion_preserves_duplicate_key_rejection() {
    for source in ["same: {}\nsame: []\n", "same: []\nsame: {}\n"] {
        assert!(matches!(crate::parse_yaml_events(source),
            Err(YamlError::DuplicateKeyAt { key, .. }) if key.as_ref() == "same"));
    }
}
