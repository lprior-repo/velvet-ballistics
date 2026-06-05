// Evidence bundle types, writer, reader, and validator.
//
// Provides a self-contained serialisable document that aggregates gate execution
// evidence with metadata about the execution context, source/test mappings, release
// artifacts, and bead linkage.

// Note: Error, GateEvidence, GateStatus, GateStatusKind, Path, PathBuf, Serialize,
// Deserialize are in scope from the preceding include!() directives in evidence.rs.

include!("bundle/types.rs");
include!("bundle/postcard.rs");
include!("bundle/api.rs");

// ──────────────────────────────────────────────────────────────────────────────
// Miri UB check: OBL-008 — Postcard round-trip must not exhibit undefined behavior
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(miri)]
#[cfg(test)]
mod miri_tests {
    use super::*;

    /// OBL-008: Postcard serialization round-trip must not exhibit undefined behavior.
    ///
    /// Run with: `cargo +nightly miri test -p xtask --lib -- miri_postcard_roundtrip_no_ub`
    ///
    /// Postcard uses unsafe byte-level repr transmutes internally. Miri detects UB
    /// that would be invisible to Kani's safe-only model checker.
    #[test]
    fn miri_postcard_roundtrip_no_ub() {
        let bundle = EvidenceBundle {
            schema_version: "1.0".to_string(),
            executor_context: ExecutorContext {
                agent: "miri-test".to_string(),
                timestamp: "2025-01-01T00:00:00Z".to_string(),
                machine: "miri-host".to_string(),
            },
            linked_bead_id: "vb-miri-ob008".to_string(),
            gates: vec![],
            source_test_mappings: vec![],
            release_artifacts: vec![],
        };

        let bytes = postcard::to_allocvec(&EvidenceBundlePostcard::from_bundle(&bundle))
            .expect("postcard serialise must succeed for valid bundle");

        let restored: EvidenceBundle = postcard::from_bytes::<EvidenceBundlePostcard>(&bytes)
            .expect("postcard deserialise must succeed for valid bytes")
            .into_bundle();

        assert_eq!(bundle.schema_version, restored.schema_version);
        assert_eq!(bundle.linked_bead_id, restored.linked_bead_id);
        assert_eq!(
            bundle.executor_context.agent,
            restored.executor_context.agent
        );
        assert_eq!(
            bundle.executor_context.timestamp,
            restored.executor_context.timestamp
        );
        assert_eq!(
            bundle.executor_context.machine,
            restored.executor_context.machine
        );
        assert_eq!(bundle.gates.len(), restored.gates.len());
        assert_eq!(
            bundle.source_test_mappings.len(),
            restored.source_test_mappings.len()
        );
        assert_eq!(
            bundle.release_artifacts.len(),
            restored.release_artifacts.len()
        );
    }
}
