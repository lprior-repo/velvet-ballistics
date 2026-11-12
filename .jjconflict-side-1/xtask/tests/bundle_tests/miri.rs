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

