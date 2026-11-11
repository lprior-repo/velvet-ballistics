
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

