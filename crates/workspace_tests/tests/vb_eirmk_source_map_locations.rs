#![forbid(unsafe_code)]

use vb_yaml::{SourceMap, SourceSpan, build_source_map, span_for_node};

fn expect_span(map: &SourceMap, index: u32, expected: SourceSpan) {
    assert_eq!(map.span_for_node(index), Some(expected));
    assert_eq!(span_for_node(map, index), Some(expected));
}

fn expect_absent(map: &SourceMap, index: u32) {
    assert_eq!(
        map.span_for_node(index),
        None,
        "expected absent at index {index}"
    );
    assert_eq!(
        span_for_node(map, index),
        None,
        "expected absent at index {index}"
    );
}

#[test]
fn nested_mapping_node_indices_and_spans() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "left:\n  value: one\nright:\n  value: two\n";
    let map = build_source_map(yaml)?;

    // index 0: root mapping - starts at byte 0
    expect_span(&map, 0, SourceSpan::new(0, 0, 1, 1, 1, 1));

    // index 1: key "left" - byte 0..4, line 1, col 1..5
    expect_span(&map, 1, SourceSpan::new(0, 4, 1, 1, 1, 5));

    // index 2: nested mapping under "left" - starts at first content byte 8
    expect_span(&map, 2, SourceSpan::new(8, 8, 2, 3, 2, 3));

    // index 3: key "value" - byte 8..13, line 2, col 3..8
    expect_span(&map, 3, SourceSpan::new(8, 13, 2, 3, 2, 8));

    // index 4: scalar "one" - byte 15..18, line 2, col 10..13
    expect_span(&map, 4, SourceSpan::new(15, 18, 2, 10, 2, 13));

    // index 5: key "right" - byte 19..24, line 3, col 1..6
    expect_span(&map, 5, SourceSpan::new(19, 24, 3, 1, 3, 6));

    // index 6: nested mapping under "right" - byte 28
    expect_span(&map, 6, SourceSpan::new(28, 28, 4, 3, 4, 3));

    // index 7: key "value" - byte 28..33, line 4, col 3..8
    expect_span(&map, 7, SourceSpan::new(28, 33, 4, 3, 4, 8));

    // index 8: scalar "two" - byte 35..38, line 4, col 10..13
    expect_span(&map, 8, SourceSpan::new(35, 38, 4, 10, 4, 13));

    expect_absent(&map, 9);

    Ok(())
}

#[test]
fn utf8_block_scalar_span() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "x: |\n  café\n  end\n";
    // 19 bytes total
    // block scalar content starts at first line byte 7, event index 2

    let map = build_source_map(yaml)?;

    // saphyr-parser 0.0.6 reports block scalar spans from the first line
    // of content through the final newline of the block.
    expect_span(&map, 2, SourceSpan::new(7, 19, 2, 3, 4, 1));

    Ok(())
}

#[test]
fn out_of_range_indices_do_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "a: b\n";
    let map = build_source_map(yaml)?;

    expect_absent(&map, 9999);
    expect_absent(&map, u32::MAX);

    Ok(())
}
