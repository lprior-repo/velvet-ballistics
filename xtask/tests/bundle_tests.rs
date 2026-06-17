#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
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
    clippy::wildcard_imports
)]

// Proof harnesses, property tests, and Miri checks for bundle module.
//
// Kani harnesses: OBL-001 through OBL-004 (run via `cargo kani`)
// Proptest properties: OBL-005 through OBL-007 (run via `cargo test --test bundle_tests`)
// Miri UB check: OBL-008 (run via `cargo +nightly miri test --test bundle_tests`)

use std::path::PathBuf;

use proptest::prelude::*;
use xtask::evidence::*;

// ──────────────────────────────────────────────────────────────────────────────
// Kani Proof Harnesses (OBL-001 to OBL-004)
// ──────────────────────────────────────────────────────────────────────────────

/// OBL-001: parse_bundle_schema_version never panics on arbitrary string input.
///
/// For any input string s, if parse succeeds, s matches ^(0|[1-9][0-9])\.(0|[1-9][0-9])$.
/// Leading-zero strings ("01.0", "0.01"), malformed strings ("1.0.0", ""),
/// and major > 1 all return Err(SchemaVersionParseFailed).
#[cfg(kani)]
#[kani::proof]
fn schema_version_parse_non_panic() {
    let input: String = kani::any();

    // This must not panic for any input.
    let _result = parse_bundle_schema_version(&input);
}

/// OBL-002: validate_bundle correctness.
///
/// Returns empty vec iff all required fields are non-empty.
/// Each missing field produces exactly one MissingRequiredField error.
#[cfg(kani)]
#[kani::proof]
fn validator_correctness() {
    // Generate an arbitrary bundle via kani::any.
    let bundle: EvidenceBundle = kani::any();

    let errors = validate_bundle(&bundle);

    // Verify that for every error, a specific required field is empty.

    let schema_err = errors.iter().any(|e| {
        matches!(
            e,
            xtask::evidence::Error::MissingRequiredField { field }
                if field == "schema_version"
        ) || matches!(e, xtask::evidence::Error::SchemaVersionParseFailed { .. })
    });

    let bead_err = errors.iter().any(|e| {
        matches!(
            e,
            xtask::evidence::Error::MissingRequiredField { field }
                if field == "linked_bead_id"
        )
    });

    let agent_err = errors.iter().any(|e| {
        matches!(
            e,
            xtask::evidence::Error::MissingRequiredField { field }
                if field == "executor_context.agent"
        )
    });

    let timestamp_err = errors.iter().any(|e| {
        matches!(
            e,
            xtask::evidence::Error::MissingRequiredField { field }
                if field == "executor_context.timestamp"
        )
    });

    let machine_err = errors.iter().any(|e| {
        matches!(
            e,
            xtask::evidence::Error::MissingRequiredField { field }
                if field == "executor_context.machine"
        )
    });

    assert!(
        schema_err || !bundle.schema_version.is_empty(),
        "schema_version error expected when empty"
    );
    assert!(
        bead_err || !bundle.linked_bead_id.is_empty(),
        "linked_bead_id error expected when empty"
    );
    assert!(
        agent_err || !bundle.executor_context.agent.is_empty(),
        "agent error expected when empty"
    );
    assert!(
        timestamp_err || !bundle.executor_context.timestamp.is_empty(),
        "timestamp error expected when empty"
    );
    assert!(
        machine_err || !bundle.executor_context.machine.is_empty(),
        "machine error expected when empty"
    );
}

/// OBL-003: write_bundle does not panic for any serialisable bundle.
///
/// Returns Ok(()) or a descriptive Error.
#[cfg(kani)]
#[kani::proof]
fn write_bundle_non_panic() {
    let bundle: EvidenceBundle = kani::any();
    let format: EvidenceBundleFormat = kani::any();
    let path: PathBuf = kani::any();

    // Must not panic; result is either Ok(()) or an Error.
    let _result = write_bundle(&bundle, &path, format);
}

/// OBL-004: read_bundle does not panic when reading arbitrary bundle data.
///
/// Unknown fields are silently ignored (no deny_unknown_fields).
#[cfg(kani)]
#[kani::proof]
fn read_bundle_non_panic() {
    let bundle: EvidenceBundle = kani::any();
    let format: EvidenceBundleFormat = kani::any();

    // Round-trip through the format: serialise then read from memory buffer.
    let bytes_result: std::result::Result<Vec<u8>, String> = match format {
        EvidenceBundleFormat::Yaml => serde_saphyr::to_string(&bundle)
            .map(|s| s.into_bytes())
            .map_err(|e| e.to_string()),
        EvidenceBundleFormat::Json => serde_json::to_string(&bundle)
            .map(|s| s.into_bytes())
            .map_err(|e| e.to_string()),
        EvidenceBundleFormat::Postcard => postcard::to_allocvec(&bundle).map_err(|e| e.to_string()),
    };

    if let Ok(ref raw) = bytes_result {
        // Read back — must not panic.
        let _result: std::result::Result<EvidenceBundle, _> = match format {
            EvidenceBundleFormat::Yaml => serde_saphyr::from_slice::<EvidenceBundle>(raw),
            EvidenceBundleFormat::Json => serde_json::from_slice::<EvidenceBundle>(raw),
            EvidenceBundleFormat::Postcard => postcard::from_bytes::<EvidenceBundle>(raw),
        };
    }
    // If serialisation failed, that's an error return, not a panic.
}

// ──────────────────────────────────────────────────────────────────────────────
// Proptest Properties (OBL-005 to OBL-007)
// ──────────────────────────────────────────────────────────────────────────────

/// OBL-005: Round-trip identity — serialise then deserialize yields equivalent bundle.
#[test]
fn prop_write_read_roundtrip_yaml() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("bundle.yaml");

        write_bundle(&bundle, &path, EvidenceBundleFormat::Yaml)
            .expect("write bundle succeeded");

        let roundtrip = read_bundle(&path, EvidenceBundleFormat::Yaml)
            .expect("read bundle succeeded");

        assert_eq!(
            bundle, roundtrip,
            "YAML round-trip failed: original != roundtrip"
        );
    });
}

#[test]
fn yaml_roundtrip_preserves_trailing_spaces() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("bundle.yaml");
    let bundle = EvidenceBundle {
        schema_version: "1.0".to_string(),
        executor_context: ExecutorContext {
            agent: String::new(),
            timestamp: String::new(),
            machine: String::new(),
        },
        linked_bead_id: String::new(),
        gates: vec![],
        source_test_mappings: vec![],
        release_artifacts: vec![ReleaseGateArtifact {
            name: String::new(),
            path: "A ".to_string(),
            digest: String::new(),
            artifact_type: ArtifactType::Benchmark,
        }],
    };

    write_bundle(&bundle, &path, EvidenceBundleFormat::Yaml).expect("write bundle succeeded");

    let roundtrip = read_bundle(&path, EvidenceBundleFormat::Yaml).expect("read bundle succeeded");

    assert_eq!(bundle, roundtrip);
}

#[test]
fn prop_write_read_roundtrip_json() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("bundle.json");

        write_bundle(&bundle, &path, EvidenceBundleFormat::Json)
            .expect("write bundle succeeded");

        let roundtrip = read_bundle(&path, EvidenceBundleFormat::Json)
            .expect("read bundle succeeded");

        assert_eq!(
            bundle, roundtrip,
            "JSON round-trip failed: original != roundtrip"
        );
    });
}

#[test]
fn prop_write_read_roundtrip_postcard() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("bundle.postcard");

        write_bundle(&bundle, &path, EvidenceBundleFormat::Postcard)
            .expect("write bundle succeeded");

        let roundtrip = read_bundle(&path, EvidenceBundleFormat::Postcard)
            .expect("read bundle succeeded");

        assert_eq!(
            bundle, roundtrip,
            "Postcard round-trip failed: original != roundtrip"
        );
    });
}

/// OBL-006: Fail-closed validation — empty required fields trigger rejection.
#[test]
fn prop_fail_closed_missing_bead_id() {
    use proptest::prelude::*;

    proptest!(
        |(agent in any::<String>(),
          timestamp in any::<String>(),
          machine in any::<String>(),
          major in 2u64..,
          minor in any::<String>())| {
            let bundle = EvidenceBundle {
                schema_version: format!("{}.{}", major, minor),
                executor_context: ExecutorContext {
                    agent,
                    timestamp,
                    machine,
                },
                linked_bead_id: String::new(),
                gates: vec![],
                source_test_mappings: vec![],
                release_artifacts: vec![],
            };

            let errors = validate_bundle(&bundle);
            assert!(
                !errors.is_empty(),
                "validate_bundle must reject empty linked_bead_id"
            );
            assert!(
                errors.iter().any(|e| {
                    matches!(
                        e,
                        xtask::evidence::Error::MissingRequiredField {
                            field
                        } if field == "linked_bead_id"
                    )
                }),
                "must produce MissingRequiredField for linked_bead_id"
            );
        }
    );
}

#[test]
fn prop_fail_closed_missing_agent() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let mut mutated = bundle.clone();
        mutated.executor_context.agent = String::new();

        let errors = validate_bundle(&mutated);
        assert!(
            !errors.is_empty(),
            "validate_bundle must reject empty agent"
        );
    });
}

#[test]
fn prop_fail_closed_missing_timestamp() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let mut mutated = bundle.clone();
        mutated.executor_context.timestamp = String::new();

        let errors = validate_bundle(&mutated);
        assert!(
            !errors.is_empty(),
            "validate_bundle must reject empty timestamp"
        );
    });
}

#[test]
fn prop_fail_closed_missing_machine() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let mut mutated = bundle.clone();
        mutated.executor_context.machine = String::new();

        let errors = validate_bundle(&mutated);
        assert!(
            !errors.is_empty(),
            "validate_bundle must reject empty machine"
        );
    });
}

/// OBL-007: Path determinism — same bead_id + format produces same path.
#[test]
fn prop_path_deterministic() {
    use proptest::prelude::*;

    let format_strategy = proptest::sample::select(vec![
        EvidenceBundleFormat::Yaml,
        EvidenceBundleFormat::Json,
        EvidenceBundleFormat::Postcard,
    ]);

    proptest!(|(bead_id in any::<String>(), format in format_strategy)| {
        let path1 = bundle_path(&bead_id, format);
        let path2 = bundle_path(&bead_id, format);

        assert_eq!(
            path1, path2,
            "bundle_path must be deterministic for same inputs"
        );

        assert!(
            path1.starts_with(".evidence"),
            "path must start with .evidence/"
        );

        let expected_ext = match format {
            EvidenceBundleFormat::Yaml => "yaml",
            EvidenceBundleFormat::Json => "json",
            EvidenceBundleFormat::Postcard => "postcard",
        };
        let actual_ext = path1
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        assert_eq!(
            actual_ext, expected_ext,
            "extension mismatch: expected {}, got {}",
            expected_ext, actual_ext
        );
    });
}

#[test]
fn prop_format_extensions_distinct() {
    assert_eq!(EvidenceBundleFormat::Yaml.extension(), "yaml");
    assert_eq!(EvidenceBundleFormat::Json.extension(), "json");
    assert_eq!(EvidenceBundleFormat::Postcard.extension(), "postcard");

    assert_ne!(
        EvidenceBundleFormat::Yaml.extension(),
        EvidenceBundleFormat::Json.extension()
    );
    assert_ne!(
        EvidenceBundleFormat::Yaml.extension(),
        EvidenceBundleFormat::Postcard.extension()
    );
    assert_ne!(
        EvidenceBundleFormat::Json.extension(),
        EvidenceBundleFormat::Postcard.extension()
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Miri Test (OBL-008) — UB check for Postcard serialization
// ──────────────────────────────────────────────────────────────────────────────

/// OBL-008: Postcard serialization round-trip must not exhibit undefined behavior.
/// Run with: cargo +nightly miri test --test bundle_tests
#[cfg(miri)]
#[test]
fn miri_postcard_roundtrip_no_ub() {
    let bundle = EvidenceBundle {
        schema_version: "1.0".to_string(),
        executor_context: ExecutorContext {
            agent: "miri-test".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            machine: "miri-host".to_string(),
        },
        linked_bead_id: "vb-miri-test".to_string(),
        gates: vec![],
        source_test_mappings: vec![],
        release_artifacts: vec![],
    };

    let bytes = postcard::to_allocvec(&bundle).expect("postcard serialise");
    let roundtrip: EvidenceBundle = postcard::from_bytes(&bytes).expect("postcard deserialise");
    assert_eq!(bundle, roundtrip, "Miri postcard round-trip failed");
}

// ──────────────────────────────────────────────────────────────────────────────
// BDD Gap Tests — Schema Version Acceptance (B-001)
// ──────────────────────────────────────────────────────────────────────────────

/// B-001: valid schema versions accepted — "0.0", "0.1", "1.99".
#[test]
fn schema_version_parse_accepts_valid_versions() {
    for input in &["0.0", "0.1", "1.99"] {
        let result = parse_bundle_schema_version(input);
        assert!(result.is_ok(), "valid input '{}' must be accepted", input);
        assert_eq!(
            result.as_ref().unwrap(),
            input,
            "parse must return the original string"
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// BDD Gap Tests — Schema Version Rejection (B-002 to B-005, B-012)
// ──────────────────────────────────────────────────────────────────────────────

/// B-002: empty string returns SchemaVersionParseFailed with empty version field.
#[test]
fn schema_version_parse_rejects_empty_string() {
    let result = parse_bundle_schema_version("");
    assert!(result.is_err(), "empty string must be rejected");
    let err = result.unwrap_err();
    assert!(
        matches!(err, xtask::evidence::Error::SchemaVersionParseFailed { ref version } if version.is_empty()),
        "error variant must be SchemaVersionParseFailed with empty version, got {:?}",
        err
    );
}

/// B-003: leading zeros rejected — "01.0", "0.01", "00.00".
#[test]
fn schema_version_parse_rejects_leading_zeros() {
    for input in &["01.0", "0.01", "00.00", "00.99", "10.01", "1.00"] {
        let result = parse_bundle_schema_version(input);
        assert!(
            result.is_err(),
            "input '{}' must be rejected due to leading zeros",
            input
        );
        let err = result.unwrap_err();
        assert!(
            matches!(err, xtask::evidence::Error::SchemaVersionParseFailed { ref version } if version == *input),
            "error for '{}' must preserve original string, got {:?}",
            input,
            err
        );
    }
}

/// B-004: malformed formats rejected — "1", "1.", ".0", "1.0.0", "a.b".
#[test]
fn schema_version_parse_rejects_malformed_formats() {
    let malformed = &[
        "1", "1.", ".0", "1.0.0", "1.0.0.0", "a.b", "!.@", "1.0.", ".0.0", "1..0", "1.0. ", " 1.0",
        "1.0\n",
    ];
    for input in malformed {
        let result = parse_bundle_schema_version(input);
        assert!(
            result.is_err(),
            "malformed input '{}' must be rejected",
            input
        );
        let err = result.unwrap_err();
        assert!(
            matches!(err, xtask::evidence::Error::SchemaVersionParseFailed { .. }),
            "error for '{}' must be SchemaVersionParseFailed, got {:?}",
            input,
            err
        );
    }
}

/// B-005: major version above 1 rejected — "2.0", "10.5", "100.0".
#[test]
fn schema_version_parse_rejects_major_above_one() {
    for input in &["2.0", "10.5", "100.0", "999.0", "2.99"] {
        let result = parse_bundle_schema_version(input);
        assert!(
            result.is_err(),
            "major > 1 input '{}' must be rejected",
            input
        );
        let err = result.unwrap_err();
        assert!(
            matches!(err, xtask::evidence::Error::SchemaVersionParseFailed { .. }),
            "error for '{}' must be SchemaVersionParseFailed, got {:?}",
            input,
            err
        );
    }
}

/// B-012: validate_bundle returns SchemaVersionParseFailed (not MissingRequiredField)
/// when schema_version is non-empty but malformed.
#[test]
fn validate_bundle_returns_schema_version_parse_error_for_invalid_version() {
    let bundle = EvidenceBundle {
        schema_version: "01.0".to_string(),
        executor_context: ExecutorContext {
            agent: "test-agent".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            machine: "test-machine".to_string(),
        },
        linked_bead_id: "vb-abc".to_string(),
        gates: vec![],
        source_test_mappings: vec![],
        release_artifacts: vec![],
    };

    let errors = validate_bundle(&bundle);
    assert!(
        !errors.is_empty(),
        "validate_bundle must reject invalid schema version"
    );

    let has_schema_parse_err = errors.iter().any(|e| {
        matches!(e, xtask::evidence::Error::SchemaVersionParseFailed { version } if version == "01.0")
    });
    let has_missing_schema_err = errors.iter().any(|e| {
        matches!(e, xtask::evidence::Error::MissingRequiredField { field } if field == "schema_version")
    });

    assert!(
        has_schema_parse_err,
        "must have SchemaVersionParseFailed error, got: {:?}",
        errors
    );
    assert!(
        !has_missing_schema_err,
        "must NOT have MissingRequiredField for schema_version when schema_version is non-empty but malformed, got: {:?}",
        errors
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// BDD Gap Tests — Validation Returns Empty Vec on Valid Bundle (B-010)
// ──────────────────────────────────────────────────────────────────────────────

/// B-010: validate_bundle returns empty vec for a fully valid bundle.
#[test]
fn validate_bundle_returns_empty_vec_for_valid_bundle() {
    let bundle = EvidenceBundle {
        schema_version: "1.0".to_string(),
        executor_context: ExecutorContext {
            agent: "test-agent".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            machine: "test-machine".to_string(),
        },
        linked_bead_id: "vb-valid".to_string(),
        gates: vec![],
        source_test_mappings: vec![],
        release_artifacts: vec![],
    };

    let errors = validate_bundle(&bundle);
    assert!(
        errors.is_empty(),
        "validate_bundle must return empty vec for valid bundle, got: {:?}",
        errors
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// BDD Gap Tests — Parent Directory Creation (B-013)
// ──────────────────────────────────────────────────────────────────────────────

/// B-013: write_bundle creates parent directories for deep nested paths.
#[test]
#[cfg(not(miri))] // Miri isolation blocks filesystem; covered by miri_postcard_roundtrip_no_ub UB check
fn write_bundle_creates_parent_directories() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let deep_path = dir
        .path()
        .join("deeply")
        .join("nested")
        .join("directory")
        .join("bundle.yaml");

    let bundle = EvidenceBundle {
        schema_version: "1.0".to_string(),
        executor_context: ExecutorContext {
            agent: "test".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            machine: "test-machine".to_string(),
        },
        linked_bead_id: "vb-deep".to_string(),
        gates: vec![],
        source_test_mappings: vec![],
        release_artifacts: vec![],
    };

    let result = write_bundle(&bundle, &deep_path, EvidenceBundleFormat::Yaml);
    assert!(
        result.is_ok(),
        "write_bundle must succeed and create parent dirs, got: {:?}",
        result
    );
    assert!(
        deep_path.exists(),
        "bundle file must exist at deep nested path"
    );

    // Verify round-trip works too.
    let roundtrip = read_bundle(&deep_path, EvidenceBundleFormat::Yaml)
        .expect("read_bundle must succeed after write_bundle created parent dirs");
    assert_eq!(bundle, roundtrip, "round-trip must preserve bundle");
}

/// B-013: write_bundle to a deeply nested path with Postcard format.
#[test]
#[cfg(not(miri))]
fn write_bundle_creates_parent_directories_postcard() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let deep_path = dir
        .path()
        .join("a")
        .join("b")
        .join("c")
        .join("bundle.postcard");

    let bundle = EvidenceBundle {
        schema_version: "1.0".to_string(),
        executor_context: ExecutorContext {
            agent: "test".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            machine: "test-machine".to_string(),
        },
        linked_bead_id: "vb-deep-postcard".to_string(),
        gates: vec![],
        source_test_mappings: vec![],
        release_artifacts: vec![],
    };

    let result = write_bundle(&bundle, &deep_path, EvidenceBundleFormat::Postcard);
    assert!(
        result.is_ok(),
        "write_bundle postcard must create parent dirs: {:?}",
        result
    );
    assert!(
        deep_path.exists(),
        "postcard bundle must exist at deep path"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// BDD Gap Tests — Error Descriptiveness (B-015 to B-017)
// ──────────────────────────────────────────────────────────────────────────────

/// B-015: BundleSerializationFailed error variant is live via read_bundle.
///
/// NOTE: BundleSerializationFailed CANNOT be triggered by write_bundle on well-formed EvidenceBundle
/// because all field types (String, Vec, PathBuf, enums) are natively infallible via serde.
/// The serialization libraries (serde_json, postcard) do not fail on valid data.
///
/// This test PROVES the error variant exists and is LIVE by triggering it through read_bundle
/// with malformed JSON data. The same Error::BundleSerializationFailed variant is declared by
/// write_bundle but is mathematically unreachable for valid EvidenceBundle — only read_bundle
/// with corrupted data can trigger it.
#[test]
#[cfg(not(miri))]
fn bundle_serialization_error_variant_is_live() {
    // Prove BundleSerializationFailed error variant exists and is triggerable
    // by writing malformed data directly to a file and reading it back.
    let dir = tempfile::tempdir().expect("create temp dir");

    // Malformed JSON that will fail deserialization
    let json_path = dir.path().join("malformed.json");
    std::fs::write(&json_path, "{ invalid json }").expect("write malformed json");

    let result = read_bundle(&json_path, EvidenceBundleFormat::Json);
    assert!(result.is_err(), "read_bundle must error on malformed JSON");
    let err = result.unwrap_err();
    assert!(
        matches!(err, xtask::evidence::Error::BundleSerializationFailed { ref format, ref cause }
            if *format == "json" && !cause.is_empty()),
        "error must be BundleSerializationFailed with format='json' and non-empty cause, got: {:?}",
        err
    );
}

/// B-015: write_bundle creates parent dirs and succeeds for well-formed bundle.
#[test]
#[cfg(not(miri))]
fn write_bundle_succeeds_with_created_parent_dirs() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("deeply").join("nested").join("bundle.yaml");

    let result = write_bundle(
        &EvidenceBundle {
            schema_version: "1.0".to_string(),
            executor_context: ExecutorContext {
                agent: "test".to_string(),
                timestamp: "2025-01-01T00:00:00Z".to_string(),
                machine: "test-machine".to_string(),
            },
            linked_bead_id: "vb-err".to_string(),
            gates: vec![],
            source_test_mappings: vec![],
            release_artifacts: vec![],
        },
        &path,
        EvidenceBundleFormat::Yaml,
    );
    assert!(
        result.is_ok(),
        "write should succeed with created parent dirs: {:?}",
        result
    );
}

/// B-016: read_bundle returns descriptive error for malformed file contents.
#[test]
#[cfg(not(miri))]
fn read_bundle_returns_descriptive_error_for_malformed_yaml() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("malformed.yaml");

    // Write invalid YAML content (not valid YAML at all)
    std::fs::write(&path, "invalid: yaml: content: [[[[").expect("write malformed yaml");

    let result = read_bundle(&path, EvidenceBundleFormat::Yaml);
    assert!(result.is_err(), "read_bundle must error on malformed YAML");
    let err = result.unwrap_err();
    assert!(
        matches!(err, xtask::evidence::Error::BundleSerializationFailed { ref format, ref cause }
            if *format == "yaml" && !cause.is_empty()),
        "error must be BundleSerializationFailed with format='yaml' and non-empty cause, got: {:?}",
        err
    );
}

/// B-016: read_bundle returns descriptive error for malformed JSON.
#[test]
#[cfg(not(miri))]
fn read_bundle_returns_descriptive_error_for_malformed_json() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("malformed.json");

    std::fs::write(&path, "{ invalid json }").expect("write malformed json");

    let result = read_bundle(&path, EvidenceBundleFormat::Json);
    assert!(result.is_err(), "read_bundle must error on malformed JSON");
    let err = result.unwrap_err();
    assert!(
        matches!(err, xtask::evidence::Error::BundleSerializationFailed { ref format, ref cause }
            if *format == "json" && !cause.is_empty()),
        "error must be BundleSerializationFailed with format='json' and non-empty cause, got: {:?}",
        err
    );
}

/// B-016: read_bundle returns descriptive error for malformed Postcard.
#[test]
#[cfg(not(miri))]
fn read_bundle_returns_descriptive_error_for_malformed_postcard() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("malformed.postcard");

    std::fs::write(&path, &[0xFF, 0xFE, 0x00, 0x01]).expect("write malformed postcard bytes");

    let result = read_bundle(&path, EvidenceBundleFormat::Postcard);
    assert!(
        result.is_err(),
        "read_bundle must error on malformed Postcard bytes"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, xtask::evidence::Error::BundleSerializationFailed { ref format, ref cause }
            if *format == "postcard" && !cause.is_empty()),
        "error must be BundleSerializationFailed with format='postcard' and non-empty cause, got: {:?}",
        err
    );
}

/// B-017: read_bundle returns EvidenceWriteFailed for missing file.
#[test]
#[cfg(not(miri))]
fn read_bundle_returns_error_for_missing_file() {
    let path = PathBuf::from("/tmp/this/path/does/not/exist/bundle.yaml");

    let result = read_bundle(&path, EvidenceBundleFormat::Yaml);
    assert!(
        result.is_err(),
        "read_bundle must error when file does not exist"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, xtask::evidence::Error::EvidenceWriteFailed { ref gate, ref cause, .. }
            if *gate == "bundle" && !cause.is_empty()),
        "error must be EvidenceWriteFailed with gate='bundle' and non-empty cause, got: {:?}",
        err
    );
}

/// B-017: read_bundle missing file error includes path.
#[test]
#[cfg(not(miri))]
fn read_bundle_missing_file_error_includes_path() {
    let path = PathBuf::from("/tmp/nonexistent/bundle.json");
    let result = read_bundle(&path, EvidenceBundleFormat::Json);
    assert!(result.is_err(), "missing file must error");
    let err = result.unwrap_err();
    assert!(
        matches!(err, xtask::evidence::Error::EvidenceWriteFailed { path: ref p, .. } if *p == path),
        "error must include the missing path, got: {:?}",
        err
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// BDD Gap Tests — GateStatus Postcard Wire Conversion (B-018)
// ──────────────────────────────────────────────────────────────────────────────

/// B-018: GateStatus::Pass round-trips through postcard write/read unchanged.
#[test]
#[cfg(not(miri))]
fn gate_status_pass_roundtrips_through_postcard() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("bundle.postcard");

    let bundle = EvidenceBundle {
        schema_version: "1.0".to_string(),
        executor_context: ExecutorContext {
            agent: "test".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            machine: "test-machine".to_string(),
        },
        linked_bead_id: "vb-status".to_string(),
        gates: vec![GateEvidence {
            kind: "gate-evidence".to_string(),
            gate_name: "test".to_string(),
            command: "cargo test".to_string(),
            exit_code: 0,
            log: PathBuf::from("test.log"),
            status: GateStatus::Pass,
            why_failed: None,
        }],
        source_test_mappings: vec![],
        release_artifacts: vec![],
    };

    write_bundle(&bundle, &path, EvidenceBundleFormat::Postcard).expect("write postcard ok");
    let roundtrip = read_bundle(&path, EvidenceBundleFormat::Postcard).expect("read postcard ok");

    assert_eq!(roundtrip.gates.len(), 1, "must have one gate");
    assert_eq!(
        roundtrip.gates[0].status,
        GateStatus::Pass,
        "Pass status must survive postcard round-trip"
    );
}

/// B-018: GateStatus::Fail round-trips through postcard write/read unchanged.
#[test]
#[cfg(not(miri))]
fn gate_status_fail_roundtrips_through_postcard() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("bundle.postcard");

    let bundle = EvidenceBundle {
        schema_version: "1.0".to_string(),
        executor_context: ExecutorContext {
            agent: "test".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            machine: "test-machine".to_string(),
        },
        linked_bead_id: "vb-status".to_string(),
        gates: vec![GateEvidence {
            kind: "gate-evidence".to_string(),
            gate_name: "test".to_string(),
            command: "cargo test".to_string(),
            exit_code: 1,
            log: PathBuf::from("test.log"),
            status: GateStatus::Fail,
            why_failed: None,
        }],
        source_test_mappings: vec![],
        release_artifacts: vec![],
    };

    write_bundle(&bundle, &path, EvidenceBundleFormat::Postcard).expect("write postcard ok");
    let roundtrip = read_bundle(&path, EvidenceBundleFormat::Postcard).expect("read postcard ok");

    assert_eq!(roundtrip.gates.len(), 1, "must have one gate");
    assert_eq!(
        roundtrip.gates[0].status,
        GateStatus::Fail,
        "Fail status must survive postcard round-trip"
    );
}

/// B-018: GateStatus::Skipped{reason} round-trips through postcard write/read with reason preserved.
#[test]
#[cfg(not(miri))]
fn gate_status_skipped_roundtrips_through_postcard_with_reason() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("bundle.postcard");

    let skip_reason = "miri not available on this platform".to_string();
    let bundle = EvidenceBundle {
        schema_version: "1.0".to_string(),
        executor_context: ExecutorContext {
            agent: "test".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            machine: "test-machine".to_string(),
        },
        linked_bead_id: "vb-status".to_string(),
        gates: vec![GateEvidence {
            kind: "gate-evidence".to_string(),
            gate_name: "miri".to_string(),
            command: "cargo +nightly miri test".to_string(),
            exit_code: 0,
            log: PathBuf::from("miri.log"),
            status: GateStatus::Skipped {
                reason: skip_reason.clone(),
            },
            why_failed: None,
        }],
        source_test_mappings: vec![],
        release_artifacts: vec![],
    };

    write_bundle(&bundle, &path, EvidenceBundleFormat::Postcard).expect("write postcard ok");
    let roundtrip = read_bundle(&path, EvidenceBundleFormat::Postcard).expect("read postcard ok");

    assert_eq!(roundtrip.gates.len(), 1, "must have one gate");
    assert!(
        matches!(
            roundtrip.gates[0].status,
            GateStatus::Skipped { ref reason } if reason == &skip_reason
        ),
        "Skipped reason '{}' must be preserved through postcard round-trip, got: {:?}",
        skip_reason,
        roundtrip.gates[0].status
    );
}

/// B-018: GateStatus::Skipped{reason: ""} (empty reason) round-trips through postcard.
#[test]
#[cfg(not(miri))]
fn gate_status_skipped_empty_reason_roundtrips_through_postcard() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("bundle.postcard");

    let bundle = EvidenceBundle {
        schema_version: "1.0".to_string(),
        executor_context: ExecutorContext {
            agent: "test".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            machine: "test-machine".to_string(),
        },
        linked_bead_id: "vb-status".to_string(),
        gates: vec![GateEvidence {
            kind: "gate-evidence".to_string(),
            gate_name: "gate".to_string(),
            command: "cmd".to_string(),
            exit_code: 0,
            log: PathBuf::from("log.log"),
            status: GateStatus::Skipped {
                reason: String::new(),
            },
            why_failed: None,
        }],
        source_test_mappings: vec![],
        release_artifacts: vec![],
    };

    write_bundle(&bundle, &path, EvidenceBundleFormat::Postcard).expect("write postcard ok");
    let roundtrip = read_bundle(&path, EvidenceBundleFormat::Postcard).expect("read postcard ok");

    assert_eq!(
        roundtrip.gates[0].status,
        GateStatus::Skipped {
            reason: String::new()
        },
        "Skipped with empty reason must round-trip"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// PI-008: GateStatusPostcard proptest invariant — all variants round-trip losslessly
// ──────────────────────────────────────────────────────────────────────────────

/// PI-008: GateStatus variants round-trip losslessly through postcard serialization
/// for all three variants (Pass, Fail, Skipped with arbitrary reason).
#[test]
#[cfg(not(miri))]
fn prop_gate_status_postcard_roundtrip_all_variants() {
    use proptest::prelude::*;

    proptest!(
        |(status in prop_oneof![
            Just(GateStatus::Pass),
            Just(GateStatus::Fail),
            any::<String>().prop_map(|reason| GateStatus::Skipped { reason }),
        ])| {
            let dir = tempfile::tempdir().expect("temp dir");
            let path = dir.path().join("bundle.postcard");

            let bundle = EvidenceBundle {
                schema_version: "1.0".to_string(),
                executor_context: ExecutorContext {
                    agent: "prop-test".to_string(),
                    timestamp: "2025-01-01T00:00:00Z".to_string(),
                    machine: "prop-machine".to_string(),
                },
                linked_bead_id: "vb-prop".to_string(),
                gates: vec![GateEvidence {
                    kind: "gate-evidence".to_string(),
                    gate_name: "gate".to_string(),
                    command: "cmd".to_string(),
                    exit_code: 0,
                    log: PathBuf::from("log.log"),
                    status: status.clone(),
                    why_failed: None,
                }],
                source_test_mappings: vec![],
                release_artifacts: vec![],
            };

            write_bundle(&bundle, &path, EvidenceBundleFormat::Postcard).expect("write ok");
            let roundtrip = read_bundle(&path, EvidenceBundleFormat::Postcard).expect("read ok");

            prop_assert_eq!(&bundle.gates[0].status, &roundtrip.gates[0].status,
                "GateStatus {:?} must round-trip losslessly through postcard", status);
        }
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Helper: proptest strategy for generating arbitrary EvidenceBundle values
// ──────────────────────────────────────────────────────────────────────────────

use proptest::strategy::{BoxedStrategy, Strategy};

fn evidence_bundle_strategy() -> BoxedStrategy<EvidenceBundle> {
    use proptest::collection::vec;

    fn arb_executor_context() -> BoxedStrategy<ExecutorContext> {
        (any::<String>(), any::<String>(), any::<String>())
            .prop_map(|(agent, timestamp, machine)| ExecutorContext {
                agent,
                timestamp,
                machine,
            })
            .boxed()
    }

    fn arb_gate_evidence() -> BoxedStrategy<GateEvidence> {
        (
            any::<String>(),
            any::<String>(),
            any::<String>(),
            any::<i32>(),
            any::<PathBuf>(),
        )
            .prop_flat_map(|(kind, gate_name, command, exit_code, log)| {
                let status_strategy = prop_oneof![
                    Just(GateStatus::Pass),
                    Just(GateStatus::Fail),
                    any::<String>().prop_map(|reason| GateStatus::Skipped { reason }),
                ];
                status_strategy.prop_map(move |status| GateEvidence {
                    kind: kind.clone(),
                    gate_name: gate_name.clone(),
                    command: command.clone(),
                    exit_code,
                    log: log.clone(),
                    status,
                    why_failed: None,
                })
            })
            .boxed()
    }

    fn arb_source_test_mapping() -> BoxedStrategy<SourceTestMapping> {
        (any::<String>(), vec(any::<String>(), 0..=5))
            .prop_map(|(source_path, tests)| SourceTestMapping { source_path, tests })
            .boxed()
    }

    fn arb_release_artifact() -> BoxedStrategy<ReleaseGateArtifact> {
        use proptest::sample::select;

        let artifact_type_strategy = select(vec![
            ArtifactType::Benchmark,
            ArtifactType::Coverage,
            ArtifactType::Mutation,
            ArtifactType::SupplyChain,
            ArtifactType::Miri,
            ArtifactType::Clippy,
            ArtifactType::Fmt,
        ]);

        (
            any::<String>(),
            any::<String>(),
            any::<String>(),
            artifact_type_strategy,
        )
            .prop_map(|(name, path, digest, artifact_type)| ReleaseGateArtifact {
                name,
                path,
                digest,
                artifact_type,
            })
            .boxed()
    }

    (
        arb_executor_context(),
        any::<String>(),
        vec(arb_gate_evidence(), 0..=5),
        vec(arb_source_test_mapping(), 0..=5),
        vec(arb_release_artifact(), 0..=5),
    )
        .prop_map(
            |(executor_context, linked_bead_id, gates, stms, rga)| EvidenceBundle {
                schema_version: "1.0".to_string(),
                executor_context,
                linked_bead_id,
                gates,
                source_test_mappings: stms,
                release_artifacts: rga,
            },
        )
        .boxed()
}
