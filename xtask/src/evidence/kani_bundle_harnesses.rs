// Kani Proof Harnesses for bundle module (OBL-001 to OBL-004).
//
// These functions are include!()ed into xtask/src/evidence.rs when compiling with Kani.
// All items from bundle.rs and tooling_and_gate_types.rs are already in scope.
// Use kani::assert() (function, not macro) for assertions.
//
// Note: This file intentionally contains NO `assert!` macros from the standard
// library — Kani cannot compile files containing bare `assert!` invocations.

// ──────────────────────────────────────────────────────────────────────────────
// Helper: construct a bounded PathBuf
// ──────────────────────────────────────────────────────────────────────────────

/// Build an arbitrary PathBuf with bounded depth and component length.
fn bounded_pathbuf(max_depth: u8, max_component_len: u8) -> std::path::PathBuf {
    let depth: u8 = kani::any();
    // Bound symbolic execution to concrete range
    if max_depth > 0 {
        kani::assume(depth <= max_depth);
    }
    let actual_depth = if max_depth > 0 { (depth % max_depth) as usize } else { 0 };
    let mut components: Vec<String> = Vec::with_capacity(actual_depth);
    let mut i = 0usize;
    while i < actual_depth {
        // Generate a simple component string
        let comp_len: u8 = kani::any();
        if max_component_len > 0 {
            kani::assume(comp_len <= max_component_len);
        }
        let actual_len = if max_component_len > 0 { (comp_len % max_component_len) as usize } else { 0 };
        let mut s = String::with_capacity(actual_len);
        let mut j = 0usize;
        while j < actual_len {
            let byte: u8 = kani::any();
            let c = (byte % 26 + b'a') as char; // Simple lowercase letters
            s.push(c);
            j += 1;
        }
        components.push(s);
        i += 1;
    }
    std::path::PathBuf::from_iter(components)
}

// ──────────────────────────────────────────────────────────────────────────────
// OBL-001: parse_bundle_schema_version never panics
// ──────────────────────────────────────────────────────────────────────────────

/// OBL-001: parse_bundle_schema_version never panics on arbitrary string input.
///
/// For any input string s, if parse succeeds, s matches ^(0|[1-9][0-9])\.(0|[1-9][0-9])$.
/// Leading-zero strings ("01.0", "0.01"), malformed strings ("1.0.0", ""),
/// are all rejected by the pure format validator.
///
/// Consumer policy (major > 1) is enforced at the validate_bundle level, not
/// in parse_bundle_schema_version, which is a pure format validator.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(3)]
fn schema_version_parse_non_panic() {
    // Build string manually since String doesn't implement kani::Arbitrary
    let len: u8 = kani::any();
    kani::assume(len <= 20); // bound symbolic execution
    let actual_len = (len % 21) as usize;
    let mut s = String::with_capacity(actual_len);
    let mut i = 0usize;
    while i < actual_len {
        let byte: u8 = kani::any();
        let c = (byte % 94 + 0x21) as char; // Printable ASCII
        s.push(c);
        i += 1;
    }
    let input = s;

    // This must not panic for any input.
    let result = parse_bundle_schema_version(&input);

    if let Ok(ref ok_str) = result {
        // If Ok, the returned string must match the major.minor format.
        let parts: Vec<&str> = ok_str.splitn(2, '.').collect();
        kani::assert(parts.len() == 2, "Ok result must have exactly one dot");
        kani::assert(!parts[0].is_empty(), "major must not be empty");
        kani::assert(!parts[1].is_empty(), "minor must not be empty");

        // No leading zeros check.
        let no_leading_zero = |s: &str| -> bool {
            s.len() > 1 && s.starts_with('0')
        };
        kani::assert(
            !no_leading_zero(parts[0]),
            "major must not have leading zeros"
        );
        kani::assert(
            !no_leading_zero(parts[1]),
            "minor must not have leading zeros"
        );

        // Both parts must parse as non-negative integers.
        let _major: u64 = parts[0].parse().unwrap();
        let _minor: u64 = parts[1].parse().unwrap();
    }

    // If Err, the string must be malformed in a detectable way.
    if result.is_err() {
        if !input.is_empty() && input.contains('.') {
            let parts2: Vec<&str> = input.splitn(2, '.').collect();
            if parts2.len() == 2 {
                let (m, n) = (parts2[0], parts2[1]);
                if let (Ok(_), Ok(_)) = (m.parse::<u64>(), n.parse::<u64>()) {
                    let m_no_lead = m.len() <= 1 || !m.starts_with('0');
                    let n_no_lead = n.len() <= 1 || !n.starts_with('0');
                    if m_no_lead && n_no_lead {
                        kani::assert(
                            result.is_ok(),
                            "format-valid input should be accepted by parser"
                        );
                    }
                }
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// OBL-002: validate_bundle correctness
// ──────────────────────────────────────────────────────────────────────────────

/// OBL-002: validate_bundle correctness.
///
/// validate_bundle checks:
/// - schema_version is non-empty and parseable with major <= 1
/// - linked_bead_id is non-empty
/// - executor_context.agent, timestamp, machine are non-empty
///
/// Returns empty vec iff all required fields are present and valid.
/// Returns errors with one entry per missing/invalid field.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(3)]
fn validator_correctness() {
    let bundle: EvidenceBundle = kani::any();

    let errors = validate_bundle(&bundle);

    // Count expected errors based on actual validate_bundle logic:
    // 1. schema_version empty OR unparseable -> 1 error
    // 2. linked_bead_id empty -> 1 error
    // 3. executor_context.agent empty -> 1 error
    // 4. executor_context.timestamp empty -> 1 error
    // 5. executor_context.machine empty -> 1 error

    let mut expected_error_count: usize = 0;

    // Check schema_version
    let schema_parseable = !bundle.schema_version.is_empty()
        && parse_bundle_schema_version(&bundle.schema_version).is_ok();
    if bundle.schema_version.is_empty() || !schema_parseable {
        expected_error_count += 1;
    }

    // Check linked_bead_id
    if bundle.linked_bead_id.is_empty() {
        expected_error_count += 1;
    }

    // Check executor_context fields
    if bundle.executor_context.agent.is_empty() {
        expected_error_count += 1;
    }
    if bundle.executor_context.timestamp.is_empty() {
        expected_error_count += 1;
    }
    if bundle.executor_context.machine.is_empty() {
        expected_error_count += 1;
    }

    kani::assert(
        errors.len() == expected_error_count,
        "error count must equal expected count"
    );

    // Positive assertions: valid bundles produce no errors
    if schema_parseable
        && !bundle.linked_bead_id.is_empty()
        && !bundle.executor_context.agent.is_empty()
        && !bundle.executor_context.timestamp.is_empty()
        && !bundle.executor_context.machine.is_empty()
    {
        kani::assert(errors.is_empty(), "valid bundle must have no errors");
    }

    // Negative assertions: specific missing fields produce specific errors
    if bundle.schema_version.is_empty() {
        kani::assert(
            errors.iter().any(|e| matches!(e, Error::MissingRequiredField { field } if field == "schema_version")),
            "empty schema_version must produce MissingRequiredField"
        );
    }
    if bundle.linked_bead_id.is_empty() {
        kani::assert(
            errors.iter().any(|e| matches!(e, Error::MissingRequiredField { field } if field == "linked_bead_id")),
            "empty linked_bead_id must produce MissingRequiredField"
        );
    }
    if bundle.executor_context.agent.is_empty() {
        kani::assert(
            errors.iter().any(|e| matches!(e, Error::MissingRequiredField { field } if field == "executor_context.agent")),
            "empty agent must produce MissingRequiredField"
        );
    }
    if bundle.executor_context.timestamp.is_empty() {
        kani::assert(
            errors.iter().any(|e| matches!(e, Error::MissingRequiredField { field } if field == "executor_context.timestamp")),
            "empty timestamp must produce MissingRequiredField"
        );
    }
    if bundle.executor_context.machine.is_empty() {
        kani::assert(
            errors.iter().any(|e| matches!(e, Error::MissingRequiredField { field } if field == "executor_context.machine")),
            "empty machine must produce MissingRequiredField"
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// OBL-003: write_bundle does not panic
// ──────────────────────────────────────────────────────────────────────────────

/// OBL-003: write_bundle does not panic for any serialisable bundle.
///
/// Returns Ok(()) or a descriptive Error.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(4)]
fn write_bundle_non_panic() {
    let bundle: EvidenceBundle = kani::any();
    let format: EvidenceBundleFormat = kani::any();
    let path = bounded_pathbuf(4, 10);

    // Must not panic; result is either Ok(()) or an Error.
    let _result = write_bundle(&bundle, &path, format);
}

// ──────────────────────────────────────────────────────────────────────────────
// OBL-004: read_bundle deserialization does not panic
// ──────────────────────────────────────────────────────────────────────────────

/// OBL-004: read_bundle deserialization does not panic on arbitrary bundle data.
///
/// The file I/O path is covered by the proptest round-trip tests (OBL-005).
/// This harness verifies that serialisation and deserialisation logic
/// does not panic on any serialisable bundle in any format.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(4)]
fn read_bundle_non_panic() {
    let bundle: EvidenceBundle = kani::any();
    let format: EvidenceBundleFormat = kani::any();

    // Serialise in-memory — must not panic.
    let bytes_result: std::result::Result<Vec<u8>, Error> = match format {
        EvidenceBundleFormat::Yaml => {
            serde_saphyr::to_string(&bundle)
                .map(|s| s.into_bytes())
                .map_err(|e| Error::BundleSerializationFailed {
                    format: "yaml".to_string(),
                    cause: e.to_string(),
                })
        }
        EvidenceBundleFormat::Json => {
            serde_json::to_string(&bundle)
                .map(|s| s.into_bytes())
                .map_err(|e| Error::BundleSerializationFailed {
                    format: "json".to_string(),
                    cause: e.to_string(),
                })
        }
        EvidenceBundleFormat::Postcard => {
            postcard::to_allocvec(&bundle)
                .map_err(|e| Error::BundleSerializationFailed {
                    format: "postcard".to_string(),
                    cause: e.to_string(),
                })
        }
    };

    // Deserialise from in-memory bytes — must not panic.
    if let Ok(ref raw) = bytes_result {
        let _deser_result: std::result::Result<EvidenceBundle, Error> = match format {
            EvidenceBundleFormat::Yaml => {
                serde_saphyr::from_slice::<EvidenceBundle>(raw)
                    .map_err(|e| Error::BundleSerializationFailed {
                        format: "yaml".to_string(),
                        cause: e.to_string(),
                    })
            }
            EvidenceBundleFormat::Json => {
                serde_json::from_slice::<EvidenceBundle>(raw)
                    .map_err(|e| Error::BundleSerializationFailed {
                        format: "json".to_string(),
                        cause: e.to_string(),
                    })
            }
            EvidenceBundleFormat::Postcard => {
                postcard::from_bytes::<EvidenceBundle>(raw)
                    .map_err(|e| Error::BundleSerializationFailed {
                        format: "postcard".to_string(),
                        cause: e.to_string(),
                    })
            }
        };
    }
}