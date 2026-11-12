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

