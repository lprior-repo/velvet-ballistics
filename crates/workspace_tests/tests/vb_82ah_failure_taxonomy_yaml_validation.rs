#![forbid(unsafe_code)]

use velvet_ballastics_workspace_tests::acceptance_catalog::{
    ArtifactProbe, ExpectedDiagnostic, FailureFamily, FailureTaxonomyScenario,
    run_failure_taxonomy_scenario,
};

#[test]
fn malformed_yaml_returns_exact_validation_code() {
    let scenario = FailureTaxonomyScenario::yaml_fixture("VB-82AH-YAML-DUPLICATE-KEY")
        .with_source("version: velvet-ballastics/v1\nname: one\nname: two\nsteps: []\n")
        .with_expected_diagnostic(ExpectedDiagnostic::new(
            FailureFamily::Yaml,
            "YamlError::DuplicateKey",
            "DUPLICATE_KEY",
            1,
        ))
        .with_required_schema_fields(["path", "span", "message", "repair"])
        .with_artifact_probe(ArtifactProbe::success_artifacts_absent());

    let evidence = run_failure_taxonomy_scenario(&scenario);

    assert_eq!(evidence.diagnostic_code(), "DUPLICATE_KEY");
    assert_eq!(evidence.cli_exit_code(), Some(1));
    assert_eq!(evidence.created_success_artifacts(), Vec::<String>::new());
}

#[test]
fn forbidden_yaml_features_return_exact_diagnostic() {
    let cases = [
        (
            "VB-82AH-YAML-ALIAS",
            "root: &base {a: 1}\ncopy: *base\n",
            "YamlError::AnchorAliasMerge",
        ),
        (
            "VB-82AH-YAML-TAG",
            "value: !Custom tagged\n",
            "YamlError::CustomTag",
        ),
        (
            "VB-82AH-YAML-BINARY",
            "blob: !!binary SGVsbG8=\n",
            "YamlError::BinaryScalar",
        ),
        (
            "VB-82AH-YAML-MULTI-DOC",
            "---\na: 1\n---\nb: 2\n",
            "YamlError::MultipleDocuments",
        ),
        (
            "VB-82AH-YAML-AMBIGUOUS",
            "flag: yes\n",
            "YamlError::AmbiguousScalar",
        ),
    ];

    for (id, source, typed_error) in cases {
        let scenario = FailureTaxonomyScenario::yaml_fixture(id)
            .with_source(source)
            .with_expected_diagnostic(ExpectedDiagnostic::new(
                FailureFamily::Yaml,
                typed_error,
                "FORBIDDEN_YAML_FEATURE",
                1,
            ))
            .with_artifact_probe(ArtifactProbe::success_artifacts_absent());

        let evidence = run_failure_taxonomy_scenario(&scenario);

        assert_eq!(evidence.typed_error(), typed_error);
        assert_eq!(evidence.diagnostic_code(), "FORBIDDEN_YAML_FEATURE");
        assert_eq!(evidence.created_success_artifacts(), Vec::<String>::new());
    }
}

#[test]
fn yaml_size_and_shape_limits_return_payload_or_limit_diagnostic() {
    let cases = [
        (
            "VB-82AH-YAML-SOURCE-TOO-LARGE",
            "YamlError::SourceTooLarge",
            "PAYLOAD_TOO_LARGE",
            "source",
        ),
        (
            "VB-82AH-YAML-NESTING-TOO-DEEP",
            "YamlError::NestingTooDeep",
            "LIMIT_EXCEEDED",
            "steps[0].then[0].then[0]",
        ),
        (
            "VB-82AH-YAML-NODE-LIMIT",
            "YamlError::NodeLimitExceeded",
            "LIMIT_EXCEEDED",
            "document",
        ),
        (
            "VB-82AH-YAML-SCALAR-TOO-LONG",
            "YamlError::ScalarTooLong",
            "LIMIT_EXCEEDED",
            "steps[0].ask.prompt",
        ),
        (
            "VB-82AH-YAML-SEQUENCE-TOO-LONG",
            "YamlError::SequenceTooLong",
            "LIMIT_EXCEEDED",
            "steps",
        ),
        (
            "VB-82AH-YAML-MAPPING-TOO-LARGE",
            "YamlError::MappingTooLarge",
            "LIMIT_EXCEEDED",
            "steps[0]",
        ),
    ];

    for (id, typed_error, code, path) in cases {
        let scenario = FailureTaxonomyScenario::yaml_fixture(id)
            .with_source(&yaml_limit_or_shape_source(id))
            .with_expected_diagnostic(ExpectedDiagnostic::new(
                FailureFamily::Yaml,
                typed_error,
                code,
                1,
            ))
            .with_expected_path(path)
            .with_artifact_probe(ArtifactProbe::success_artifacts_absent());

        let evidence = run_failure_taxonomy_scenario(&scenario);

        assert_eq!(evidence.typed_error(), typed_error);
        assert_eq!(evidence.diagnostic_code(), code);
        assert_eq!(evidence.diagnostic_path(), path);
        assert_eq!(evidence.created_success_artifacts(), Vec::<String>::new());
    }
}

#[test]
fn yaml_field_and_parse_failures_return_exact_field_diagnostic() {
    let cases = [
        (
            "VB-82AH-YAML-UNKNOWN-TOP-FIELD",
            "YamlError::UnknownTopLevelField",
            "UNKNOWN_TOP_LEVEL_FIELD",
            "unexpected_root_field",
        ),
        (
            "VB-82AH-YAML-UNKNOWN-STEP-FIELD",
            "YamlError::UnknownStepField",
            "UNKNOWN_STEP_FIELD",
            "steps[0].unexpected_step_field",
        ),
        (
            "VB-82AH-YAML-MISSING-REQUIRED",
            "YamlError::MissingRequiredField",
            "MISSING_REQUIRED_FIELD",
            "steps[0].id",
        ),
        (
            "VB-82AH-YAML-WRONG-SHAPE",
            "YamlError::WrongFieldShape",
            "WRONG_FIELD_SHAPE",
            "steps",
        ),
        (
            "VB-82AH-YAML-PARSE-ERROR",
            "YamlError::ParseError",
            "YAML_PARSE_ERROR",
            "line:3:column:12",
        ),
    ];

    for (id, typed_error, code, path) in cases {
        let scenario = FailureTaxonomyScenario::yaml_fixture(id)
            .with_source(&yaml_field_or_parse_source(id))
            .with_expected_diagnostic(ExpectedDiagnostic::new(
                FailureFamily::Yaml,
                typed_error,
                code,
                1,
            ))
            .with_expected_path(path)
            .with_required_schema_fields(["path", "span", "message", "repair"])
            .with_artifact_probe(ArtifactProbe::success_artifacts_absent());

        let evidence = run_failure_taxonomy_scenario(&scenario);

        assert_eq!(evidence.typed_error(), typed_error);
        assert_eq!(evidence.diagnostic_code(), code);
        assert_eq!(evidence.diagnostic_path(), path);
        assert_eq!(evidence.missing_cli_schema_fields(), Vec::<String>::new());
        assert_eq!(evidence.created_success_artifacts(), Vec::<String>::new());
    }
}

#[test]
fn validation_reference_and_id_failures_return_exact_section_16_codes() {
    let cases = [
        (
            "VB-82AH-VAL-INVALID-VERSION",
            "ValidationError::InvalidVersion",
            "INVALID_VERSION",
        ),
        (
            "VB-82AH-VAL-INVALID-ID",
            "ValidationError::InvalidId",
            "INVALID_ID",
        ),
        (
            "VB-82AH-VAL-RESERVED-ID",
            "ValidationError::ReservedId",
            "RESERVED_ID",
        ),
        (
            "VB-82AH-VAL-DUPLICATE-ID",
            "ValidationError::DuplicateId",
            "DUPLICATE_ID",
        ),
    ];

    for (id, typed_error, code) in cases {
        let scenario = FailureTaxonomyScenario::validation_fixture(id).with_expected_diagnostic(
            ExpectedDiagnostic::new(FailureFamily::Validation, typed_error, code, 1),
        );

        let evidence = run_failure_taxonomy_scenario(&scenario);

        assert_eq!(evidence.typed_error(), typed_error);
        assert_eq!(evidence.diagnostic_code(), code);
        assert_eq!(evidence.compile_attempted(), false);
        assert_eq!(evidence.run_attempted(), false);
    }
}

#[test]
fn validation_primitive_shape_failures_return_exact_section_16_codes() {
    let cases = [
        (
            "VB-82AH-VAL-MISSING-PRIMITIVE",
            "ValidationError::MissingStepPrimitive",
            "MISSING_STEP_PRIMITIVE",
        ),
        (
            "VB-82AH-VAL-MULTIPLE-PRIMITIVES",
            "ValidationError::MultipleStepPrimitives",
            "MULTIPLE_STEP_PRIMITIVES",
        ),
        (
            "VB-82AH-VAL-HTTP-TRIGGER",
            "ValidationError::HttpTriggerOutOfCore",
            "HTTP_TRIGGER_OUT_OF_CORE",
        ),
    ];

    for (id, typed_error, code) in cases {
        let scenario = FailureTaxonomyScenario::validation_fixture(id).with_expected_diagnostic(
            ExpectedDiagnostic::new(FailureFamily::Validation, typed_error, code, 1),
        );

        let evidence = run_failure_taxonomy_scenario(&scenario);

        assert_eq!(evidence.typed_error(), typed_error);
        assert_eq!(evidence.diagnostic_code(), code);
        assert_eq!(evidence.contains_raw_secret(), false);
    }
}

fn yaml_limit_or_shape_source(id: &str) -> String {
    match id {
        "VB-82AH-YAML-SOURCE-TOO-LARGE" => "a".repeat(1_048_577),
        "VB-82AH-YAML-NESTING-TOO-DEEP" => nested_yaml(65),
        "VB-82AH-YAML-NODE-LIMIT" => long_sequence_yaml(100_001),
        "VB-82AH-YAML-SCALAR-TOO-LONG" => format!(
            "version: velvet-ballastics/v1\nname: scalar_probe\nsteps:\n  - id: ask\n    ask:\n      prompt: {}\n",
            "x".repeat(65_537)
        ),
        "VB-82AH-YAML-SEQUENCE-TOO-LONG" => long_sequence_yaml(10_001),
        "VB-82AH-YAML-MAPPING-TOO-LARGE" => large_mapping_yaml(1_025),
        _ => String::new(),
    }
}

fn yaml_field_or_parse_source(id: &str) -> String {
    match id {
        "VB-82AH-YAML-UNKNOWN-TOP-FIELD" => "version: velvet-ballastics/v1\nname: field_probe\nunexpected_root_field: true\nsteps: []\n".to_owned(),
        "VB-82AH-YAML-UNKNOWN-STEP-FIELD" => "version: velvet-ballastics/v1\nname: step_field_probe\nsteps:\n  - id: one\n    unexpected_step_field: true\n    set:\n      output: out\n      value: ok\n".to_owned(),
        "VB-82AH-YAML-MISSING-REQUIRED" => "version: velvet-ballastics/v1\nname: missing_probe\nsteps:\n  - set:\n      output: out\n      value: ok\n".to_owned(),
        "VB-82AH-YAML-WRONG-SHAPE" => "version: velvet-ballastics/v1\nname: wrong_shape_probe\nsteps: not-a-sequence\n".to_owned(),
        "VB-82AH-YAML-PARSE-ERROR" => "version: velvet-ballastics/v1\nname: parse_probe\nsteps:\n  - id: one\n    set: [unterminated\n".to_owned(),
        _ => String::new(),
    }
}

fn nested_yaml(depth: usize) -> String {
    let mut source = String::new();
    for level in 0..depth {
        source.push_str(&"  ".repeat(level));
        source.push_str("a:\n");
    }
    source.push_str(&"  ".repeat(depth));
    source.push_str("leaf: value\n");
    source
}

fn long_sequence_yaml(items: usize) -> String {
    let mut source = String::from("items:\n");
    for _ in 0..items {
        source.push_str("  - a\n");
    }
    source
}

fn large_mapping_yaml(items: usize) -> String {
    let mut source = String::new();
    for item in 0..items {
        source.push_str(&format!("k{item}: v\n"));
    }
    source
}
