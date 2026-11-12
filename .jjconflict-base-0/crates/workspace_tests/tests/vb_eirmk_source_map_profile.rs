use vb_yaml::{YamlError, build_source_map};

#[test]
fn duplicate_keys_reject_with_key_location() {
    // Given: duplicate key text with parser coordinates on the second key.
    let source = "a: 1\na: 2\n";
    // When: source-map construction validates the profile.
    // Then: the exact parser-coordinate diagnostic is returned.
    assert_eq!(
        build_source_map(source),
        Err(YamlError::DuplicateKeyAt {
            key: "a".into(),
            line: 2,
            column: 0,
        })
    );
}

#[test]
fn custom_tag_rejected_with_tag_name() {
    // Given: a scalar with a local custom tag.
    let source = "a: !custom value\n";
    // When: source-map construction validates the profile.
    // Then: the exact custom tag payload is returned.
    assert_eq!(
        build_source_map(source),
        Err(YamlError::CustomTag {
            tag: "!custom".into(),
        })
    );
}

#[test]
fn binary_tag_rejected_as_custom_tag() {
    // Given: a scalar marked with the YAML binary tag.
    let source = "a: !!binary SGVsbG8=\n";
    // When: source-map construction validates the profile.
    // Then: the expanded binary tag is returned as a custom-tag error.
    assert_eq!(
        build_source_map(source),
        Err(YamlError::CustomTag {
            tag: "tag:yaml.org,2002:binary".into(),
        })
    );
}

#[test]
fn ambiguous_yaml11_scalar_rejected_with_value() {
    // Given: a plain YAML 1.1 ambiguous boolean scalar.
    let source = "a: yes\n";
    // When: source-map construction validates the profile.
    // Then: the exact ambiguous scalar payload is returned.
    assert_eq!(
        build_source_map(source),
        Err(YamlError::AmbiguousScalar {
            scalar: "yes".into(),
        })
    );
}

#[test]
fn multiple_documents_rejected_with_count() {
    // Given: two YAML documents in one source string.
    let source = "---\na: 1\n---\nb: 2\n";
    // When: source-map construction validates the profile.
    // Then: the exact document count is returned.
    assert_eq!(
        build_source_map(source),
        Err(YamlError::MultipleDocuments { count: 2 })
    );
}
