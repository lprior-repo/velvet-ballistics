#![forbid(unsafe_code)]

use vb_yaml::{SourceMap, SourceSpan, build_source_map, span_for_node};

fn assert_span(map: &SourceMap, index: u32, expected: SourceSpan) {
    assert_eq!(map.span_for_node(index), Some(expected));
}

#[test]
fn flat_yaml_maps_named_scalars_to_exact_locations() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "a: 1\n";
    let map = build_source_map(yaml)?;
    assert_span(&map, 1, SourceSpan::new(0, 1, 1, 1, 1, 2));
    assert_span(&map, 2, SourceSpan::new(3, 4, 1, 4, 1, 5));
    assert_eq!(
        span_for_node(&map, 2),
        Some(SourceSpan::new(3, 4, 1, 4, 1, 5))
    );
    assert_eq!(map.span_for_node(3), None);
    Ok(())
}

#[test]
fn nested_sequence_maps_root_and_last_value_to_exact_locations()
-> Result<(), Box<dyn std::error::Error>> {
    let yaml = "items:\n  - alpha\n  - beta\n";
    let map = build_source_map(yaml)?;
    assert_span(&map, 0, SourceSpan::new(0, 0, 1, 1, 1, 1));
    assert_span(&map, 1, SourceSpan::new(0, 5, 1, 1, 1, 6));
    assert_span(&map, 3, SourceSpan::new(11, 16, 2, 5, 2, 10));
    assert_span(&map, 4, SourceSpan::new(21, 25, 3, 5, 3, 9));
    assert_eq!(map.span_for_node(5), None);
    Ok(())
}

#[test]
fn utf8_offsets_count_bytes_and_coordinates_are_one_indexed()
-> Result<(), Box<dyn std::error::Error>> {
    let yaml = "x: café\n";
    let map = build_source_map(yaml)?;
    assert_span(&map, 2, SourceSpan::new(3, 8, 1, 4, 1, 8));
    Ok(())
}

#[test]
fn utf8_prefix_keeps_following_nodes_at_byte_offsets() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "é: one\nx: two\n";
    let map = build_source_map(yaml)?;
    assert_span(&map, 3, SourceSpan::new(8, 9, 2, 1, 2, 2));
    assert_span(&map, 4, SourceSpan::new(11, 14, 2, 4, 2, 7));
    Ok(())
}

#[test]
fn cr_and_crlf_line_breaks_keep_following_scalar_coordinates()
-> Result<(), Box<dyn std::error::Error>> {
    let cr_yaml = "a: one\rx: two\r";
    let cr_map = build_source_map(cr_yaml)?;
    assert_span(&cr_map, 3, SourceSpan::new(7, 8, 2, 1, 2, 2));
    assert_span(&cr_map, 4, SourceSpan::new(10, 13, 2, 4, 2, 7));
    let crlf_yaml = "a: one\r\nx: two\r\n";
    let crlf_map = build_source_map(crlf_yaml)?;
    assert_span(&crlf_map, 3, SourceSpan::new(8, 9, 2, 1, 2, 2));
    assert_span(&crlf_map, 4, SourceSpan::new(11, 14, 2, 4, 2, 7));
    Ok(())
}

#[test]
fn malformed_yaml_returns_typed_parse_error() {
    let result = build_source_map("a: [1, 2\n");
    assert!(matches!(result, Err(vb_yaml::YamlError::ParseError { .. })));
}
