use super::{DecodeError, DecodeLimits, IntegerField, decode_integer_field};

#[test]
fn constructor_accepts_inclusive_caps_and_rejects_each_zero_or_over_cap_budget() {
    assert!(DecodeLimits::new(1, 1, 1, 1).is_ok());
    assert!(DecodeLimits::new(1_048_576, 64, 1_048_576, 1_048_576).is_ok());
    for (body, depth, nodes, strings) in [
        (0, 1, 1, 1),
        (1, 0, 1, 1),
        (1, 1, 0, 1),
        (1, 1, 1, 0),
        (1_048_577, 1, 1, 1),
        (1, 65, 1, 1),
        (1, 1, 1_048_577, 1),
        (1, 1, 1, 1_048_577),
    ] {
        assert_eq!(
            DecodeLimits::new(body, depth, nodes, strings),
            Err(DecodeError::InvalidLimits)
        );
    }
}

#[test]
fn input_budget_counts_all_wire_bytes_including_trailing_whitespace() -> Result<(), DecodeError> {
    let body = br#"{"total_count":42}"#;
    let exact = DecodeLimits::new(body.len(), 1, 3, 11)?;
    assert_eq!(
        decode_integer_field(body, exact, "total_count"),
        Ok(IntegerField::Integer {
            negative: false,
            magnitude: Some(42)
        })
    );
    let smaller = DecodeLimits::new(body.len().saturating_sub(1), 1, 3, 11)?;
    assert_eq!(
        decode_integer_field(body, smaller, "total_count"),
        Err(DecodeError::BodyLimit)
    );
    let mut trailing = body.to_vec();
    trailing.push(b' ');
    assert_eq!(
        decode_integer_field(&trailing, exact, "total_count"),
        Err(DecodeError::BodyLimit)
    );
    let non_ascii = r#"{"é":0}"#.as_bytes();
    assert_eq!(
        decode_integer_field(non_ascii, DecodeLimits::new(non_ascii.len(), 1, 3, 2)?, "é"),
        Ok(IntegerField::Integer {
            negative: false,
            magnitude: Some(0)
        })
    );
    assert_eq!(
        decode_integer_field(
            non_ascii,
            DecodeLimits::new(non_ascii.len().saturating_sub(1), 1, 3, 2)?,
            "é"
        ),
        Err(DecodeError::BodyLimit)
    );
    Ok(())
}

#[test]
fn maximum_input_budget_is_inclusive_without_discarding_trailing_bytes() -> Result<(), DecodeError>
{
    let limits = DecodeLimits::new(1_048_576, 1, 1, 1)?;
    let mut body = b"{}".to_vec();
    body.resize(1_048_576, b' ');
    assert_eq!(
        decode_integer_field(&body, limits, "x"),
        Ok(IntegerField::Missing)
    );
    body.push(b' ');
    assert_eq!(
        decode_integer_field(&body, limits, "x"),
        Err(DecodeError::BodyLimit)
    );
    Ok(())
}

#[test]
fn body_limit_precedes_utf8_or_syntax_checks_for_oversized_input() -> Result<(), DecodeError> {
    let limits = DecodeLimits::new(1, 1, 1, 1)?;
    assert_eq!(
        decode_integer_field(b"\xff\xff", limits, "x"),
        Err(DecodeError::BodyLimit)
    );
    assert_eq!(
        decode_integer_field(b"\xff", limits, "x"),
        Err(DecodeError::InvalidJson)
    );
    let malformed_tail = br#"{"total_count":42,"ignored":""#;
    let limits = DecodeLimits::new(malformed_tail.len().saturating_sub(1), 4, 32, 32)?;
    assert_eq!(
        decode_integer_field(malformed_tail, limits, "total_count"),
        Err(DecodeError::BodyLimit)
    );
    Ok(())
}

#[test]
fn depth_counts_root_and_nested_containers_not_primitive_values() -> Result<(), DecodeError> {
    let one = DecodeLimits::new(256, 1, 32, 32)?;
    assert_eq!(
        decode_integer_field(b"{}", one, "x"),
        Ok(IntegerField::Missing)
    );
    assert_eq!(
        decode_integer_field(br#"{"x":0}"#, one, "x"),
        Ok(IntegerField::Integer {
            negative: false,
            magnitude: Some(0)
        })
    );
    for body in [br#"{"x":[]}"#.as_slice(), br#"{"x":{}}"#.as_slice()] {
        assert_eq!(
            decode_integer_field(body, one, "x"),
            Err(DecodeError::DepthLimit)
        );
        assert_eq!(
            decode_integer_field(body, DecodeLimits::new(256, 2, 32, 32)?, "x"),
            Ok(IntegerField::InvalidType)
        );
    }
    Ok(())
}

#[test]
fn quoted_brackets_braces_and_escaped_quote_do_not_consume_container_depth()
-> Result<(), DecodeError> {
    let body = br#"{"x":42,"a":"[]{}\""}"#;
    assert_eq!(
        decode_integer_field(body, DecodeLimits::new(256, 1, 5, 5)?, "x"),
        Ok(IntegerField::Integer {
            negative: false,
            magnitude: Some(42)
        })
    );
    Ok(())
}

#[test]
fn fixed_stack_accepts_root_plus_sixty_three_arrays_and_rejects_one_more() -> Result<(), DecodeError>
{
    let at_cap = format!("{{\"a\":{}0{},\"x\":42}}", "[".repeat(63), "]".repeat(63));
    let over_cap = format!("{{\"a\":{}0{},\"x\":42}}", "[".repeat(64), "]".repeat(64));
    let limits = DecodeLimits::new(4096, 64, 512, 32)?;
    assert_eq!(
        decode_integer_field(at_cap.as_bytes(), limits, "x"),
        Ok(IntegerField::Integer {
            negative: false,
            magnitude: Some(42)
        })
    );
    assert_eq!(
        decode_integer_field(over_cap.as_bytes(), limits, "x"),
        Err(DecodeError::DepthLimit)
    );
    Ok(())
}

#[test]
fn workflow_fixture_requires_three_nodes_for_root_key_and_integer() -> Result<(), DecodeError> {
    let body = br#"{"total_count":42}"#;
    assert_eq!(
        decode_integer_field(body, DecodeLimits::new(128, 1, 3, 11)?, "total_count"),
        Ok(IntegerField::Integer {
            negative: false,
            magnitude: Some(42)
        })
    );
    assert_eq!(
        decode_integer_field(body, DecodeLimits::new(128, 1, 2, 11)?, "total_count"),
        Err(DecodeError::NodeLimit)
    );
    Ok(())
}

#[test]
fn node_caps_count_each_container_primitive_and_object_key_exactly() -> Result<(), DecodeError> {
    let cases: &[(&[u8], usize, IntegerField)] = &[
        (b"{}", 1, IntegerField::Missing),
        (
            br#"{"x":0}"#,
            3,
            IntegerField::Integer {
                negative: false,
                magnitude: Some(0),
            },
        ),
        (br#"{"x":[]}"#, 3, IntegerField::InvalidType),
        (br#"{"x":[0]}"#, 4, IntegerField::InvalidType),
        (br#"{"x":{"a":null}}"#, 5, IntegerField::InvalidType),
        (
            br#"{"a":[{},[]],"x":42}"#,
            7,
            IntegerField::Integer {
                negative: false,
                magnitude: Some(42),
            },
        ),
    ];
    for &(body, nodes, expected) in cases {
        assert_eq!(
            decode_integer_field(body, DecodeLimits::new(256, 16, nodes, 32)?, "x"),
            Ok(expected),
            "{body:?}"
        );
        if nodes > 1 {
            assert_eq!(
                decode_integer_field(
                    body,
                    DecodeLimits::new(256, 16, nodes.saturating_sub(1), 32)?,
                    "x"
                ),
                Err(DecodeError::NodeLimit),
                "{body:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn empty_keys_and_values_are_valid_under_a_positive_one_byte_string_budget()
-> Result<(), DecodeError> {
    let limits = DecodeLimits::new(128, 1, 5, 1)?;
    let body = br#"{"":0,"x":""}"#;
    assert_eq!(
        decode_integer_field(body, limits, ""),
        Ok(IntegerField::Integer {
            negative: false,
            magnitude: Some(0)
        })
    );
    assert_eq!(
        decode_integer_field(body, limits, "x"),
        Ok(IntegerField::InvalidType)
    );
    assert_eq!(
        decode_integer_field(body, limits, "total_count"),
        Ok(IntegerField::Missing)
    );
    assert_eq!(
        decode_integer_field(br#"{"":1,"":2}"#, limits, ""),
        Ok(IntegerField::Duplicate)
    );
    Ok(())
}

#[test]
fn decoded_key_bytes_not_escape_wire_length_determine_string_bounds() -> Result<(), DecodeError> {
    assert_eq!(
        decode_integer_field(br#"{"\u0078":42}"#, DecodeLimits::new(128, 1, 3, 1)?, "x"),
        Ok(IntegerField::Integer {
            negative: false,
            magnitude: Some(42)
        })
    );
    assert_eq!(
        decode_integer_field(br#"{"xx":42}"#, DecodeLimits::new(128, 1, 3, 1)?, "xx"),
        Err(DecodeError::StringLimit)
    );
    assert_eq!(
        decode_integer_field(br#"{"x":"ab"}"#, DecodeLimits::new(128, 1, 3, 1)?, "x"),
        Err(DecodeError::StringLimit)
    );
    Ok(())
}

#[test]
fn decoded_utf8_byte_boundaries_apply_to_literal_and_escaped_keys_and_values()
-> Result<(), DecodeError> {
    for (encoded, field, bytes) in [
        ("é", "é", 2_usize),
        (r"\u00e9", "é", 2),
        ("€", "€", 3),
        (r"\u20ac", "€", 3),
        ("𝄞", "\u{1d11e}", 4),
        (r"\uD834\uDD1E", "\u{1d11e}", 4),
    ] {
        let key = format!(r#"{{"{encoded}":7}}"#);
        let exact = DecodeLimits::new(128, 1, 5, bytes)?;
        let smaller = DecodeLimits::new(128, 1, 5, bytes.saturating_sub(1))?;
        assert_eq!(
            decode_integer_field(key.as_bytes(), exact, field),
            Ok(IntegerField::Integer {
                negative: false,
                magnitude: Some(7)
            }),
            "{encoded}"
        );
        assert_eq!(
            decode_integer_field(key.as_bytes(), smaller, field),
            Err(DecodeError::StringLimit),
            "{encoded}"
        );
        let value = format!(r#"{{"x":42,"a":"{encoded}"}}"#);
        assert_eq!(
            decode_integer_field(value.as_bytes(), exact, "x"),
            Ok(IntegerField::Integer {
                negative: false,
                magnitude: Some(42)
            }),
            "{encoded}"
        );
        assert_eq!(
            decode_integer_field(value.as_bytes(), smaller, "x"),
            Err(DecodeError::StringLimit),
            "{encoded}"
        );
    }
    Ok(())
}

#[test]
fn string_budget_is_per_decoded_key_or_value_not_an_aggregate() -> Result<(), DecodeError> {
    let body = br#"{"xy":"ab","x":42}"#;
    assert_eq!(
        decode_integer_field(body, DecodeLimits::new(128, 1, 5, 2)?, "x"),
        Ok(IntegerField::Integer {
            negative: false,
            magnitude: Some(42)
        })
    );
    Ok(())
}

#[test]
fn ignored_tail_resource_errors_precede_raw_duplicate_or_valid_integer_outcomes()
-> Result<(), DecodeError> {
    assert_eq!(
        decode_integer_field(
            br#"{"x":42,"s":"abcd"}"#,
            DecodeLimits::new(256, 4, 32, 3)?,
            "x"
        ),
        Err(DecodeError::StringLimit)
    );
    assert_eq!(
        decode_integer_field(
            br#"{"x":42,"a":[]}"#,
            DecodeLimits::new(256, 1, 32, 32)?,
            "x"
        ),
        Err(DecodeError::DepthLimit)
    );
    assert_eq!(
        decode_integer_field(
            br#"{"x":1,"x":2,"tail":null}"#,
            DecodeLimits::new(256, 4, 6, 32)?,
            "x"
        ),
        Err(DecodeError::NodeLimit)
    );
    assert_eq!(
        decode_integer_field(
            br#"{"x":false,"s":"abcd"}"#,
            DecodeLimits::new(256, 4, 32, 3)?,
            "x"
        ),
        Err(DecodeError::StringLimit)
    );
    Ok(())
}

#[test]
fn every_budget_is_copyable_and_has_a_distinct_postcard_binding() -> Result<(), String> {
    for ((body, depth, nodes, strings), expected) in [
        ((11, 2, 13, 14), [11_u8, 2, 13, 14]),
        ((12, 2, 13, 14), [12, 2, 13, 14]),
        ((11, 3, 13, 14), [11, 3, 13, 14]),
        ((11, 2, 14, 14), [11, 2, 14, 14]),
        ((11, 2, 13, 15), [11, 2, 13, 15]),
    ] {
        let limits = DecodeLimits::new(body, depth, nodes, strings)
            .map_err(|error| format!("limits: {error:?}"))?;
        let copied = limits;
        assert_eq!(limits, copied);
        let bytes =
            postcard::to_allocvec(&limits).map_err(|error| format!("postcard: {error:?}"))?;
        assert_eq!(bytes.as_slice(), expected.as_slice());
        let bound = postcard::to_allocvec(&(limits, 99_u8))
            .map_err(|error| format!("postcard tuple: {error:?}"))?;
        let mut expected_bound = expected.to_vec();
        expected_bound.push(99);
        assert_eq!(bound, expected_bound);
    }
    Ok(())
}
