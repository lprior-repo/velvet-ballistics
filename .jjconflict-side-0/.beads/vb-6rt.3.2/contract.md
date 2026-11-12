# vb-6rt.3.2 Source Map Profile Contract

`vb_yaml::build_source_map(text)` validates the strict YAML profile before constructing a source map, so each rejected input returns its exact typed `YamlError`.

Given `a: 1\na: 2\n`, duplicate detection returns `YamlError::DuplicateKeyAt { key: "a", line: 2, column: 0 }`; `line` and `column` are copied from the parser event span as raw parser coordinates, not converted to one-based `SourceSpan` coordinates.

Given `a: !custom value\n`, profile validation returns `YamlError::CustomTag { tag: "!custom" }`; given `a: !!binary SGVsbG8=\n`, tag conversion returns `YamlError::CustomTag { tag: "tag:yaml.org,2002:binary" }`.

Given `a: yes\n`, strict scalar validation returns `YamlError::AmbiguousScalar { scalar: "yes" }`; given `---\na: 1\n---\nb: 2\n`, document validation returns `YamlError::MultipleDocuments { count: 2 }`.

The five existing scenario names and exact source inputs remain: `duplicate_keys_reject_with_key_location`, `custom_tag_rejected_with_tag_name`, `binary_tag_rejected_as_custom_tag`, `ambiguous_yaml11_scalar_rejected_with_value`, and `multiple_documents_rejected_with_count`.

Each scenario asserts `build_source_map(source) == Err(expected_error)` directly, so successful parsing and every wrong error payload fail the test without panic-based extraction. Only `crates/workspace_tests/tests/vb_eirmk_source_map_profile.rs` and this contract are in scope; no production parser changes are required.
