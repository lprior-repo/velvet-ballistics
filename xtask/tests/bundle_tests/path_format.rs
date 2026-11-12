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

