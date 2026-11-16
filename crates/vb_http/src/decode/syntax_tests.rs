use super::{decode_integer_field, DecodeError, DecodeLimits, IntegerField};

fn selected(body: &[u8], field: &str) -> Result<IntegerField, DecodeError> {
    decode_integer_field(body, DecodeLimits::new(4096, 16, 512, 1024)?, field)
}

#[test]
fn exact_integer_sign_and_magnitude_never_round_through_float() {
    for (token, negative, magnitude) in [
        ("0", false, Some(0)),
        ("-0", true, Some(0)),
        ("42", false, Some(42)),
        ("-42", true, Some(42)),
        ("9007199254740993", false, Some(9_007_199_254_740_993)),
        ("9223372036854775807", false, Some(9_223_372_036_854_775_807)),
        ("9223372036854775808", false, Some(9_223_372_036_854_775_808)),
        ("-9223372036854775808", true, Some(9_223_372_036_854_775_808)),
        ("-9223372036854775809", true, Some(9_223_372_036_854_775_809)),
        ("18446744073709551615", false, Some(u64::MAX)),
        ("-18446744073709551615", true, Some(u64::MAX)),
        ("18446744073709551616", false, None),
        ("-18446744073709551616", true, None),
        ("999999999999999999999999999999999999999999", false, None),
    ] {
        let body = format!(r#"{{"total_count":{token}}}"#);
        assert_eq!(
            selected(body.as_bytes(), "total_count"),
            Ok(IntegerField::Integer { negative, magnitude }),
            "{token}"
        );
    }
}

#[test]
fn valid_fraction_and_exponent_tokens_are_selected_invalid_types() {
    for token in [
        "0.0", "-0.0", "1.25", "-12.34", "1e0", "1E+0", "1e-9",
        "-0e+0", "1.25e30", "1E999999999999", "18446744073709551616.0",
    ] {
        let body = format!(r#"{{"total_count":{token}}}"#);
        assert_eq!(
            selected(body.as_bytes(), "total_count"),
            Ok(IntegerField::InvalidType),
            "{token}"
        );
        let ignored = format!(r#"{{"ignored":{token}}}"#);
        assert_eq!(selected(ignored.as_bytes(), "total_count"), Ok(IntegerField::Missing));
    }
}

#[test]
fn malformed_number_grammar_is_not_an_integer_or_a_valid_wrong_type() {
    for token in [
        "+1", ".5", "-.5", "01", "-01", "00.1", "1.", "1e", "1e+", "1e-",
        "1.e1", "1E++2", "1e+-2", "--1", "-", "NaN", "Infinity", "-Infinity",
        "1_000", "0x1", "1/2", "1 2",
    ] {
        let body = format!(r#"{{"total_count":{token}}}"#);
        assert_eq!(selected(body.as_bytes(), "total_count"), Err(DecodeError::InvalidJson), "{token}");
        let ignored = format!(r#"{{"total_count":42,"ignored":{token}}}"#);
        assert_eq!(selected(ignored.as_bytes(), "total_count"), Err(DecodeError::InvalidJson), "{token}");
    }
}

#[test]
fn only_decoded_root_keys_select_the_field_and_unrelated_duplicates_are_valid() {
    for (body, expected) in [
        (br#"{}"#.as_slice(), IntegerField::Missing),
        (br#"{"nested":{"total_count":1}}"#.as_slice(), IntegerField::Missing),
        (br#"{"nested":[{"total_count":1}],"total_count":42}"#.as_slice(),
            IntegerField::Integer { negative: false, magnitude: Some(42) }),
        (br#"{"total_count":42,"nested":{"total_count":1}}"#.as_slice(),
            IntegerField::Integer { negative: false, magnitude: Some(42) }),
        (br#"{"other":1,"other":2,"total_count":42}"#.as_slice(),
            IntegerField::Integer { negative: false, magnitude: Some(42) }),
        (br#"{"Total_Count":42,"total_countx":7}"#.as_slice(), IntegerField::Missing),
        (br#"{"total_count":{"total_count":42}}"#.as_slice(), IntegerField::InvalidType),
        (br#"{"\u0074otal_count":42}"#.as_slice(),
            IntegerField::Integer { negative: false, magnitude: Some(42) }),
    ] {
        assert_eq!(selected(body, "total_count"), Ok(expected), "{body:?}");
    }
}

#[test]
fn target_duplicates_are_detected_after_key_escape_decoding_regardless_of_type() {
    for body in [
        br#"{"total_count":1,"total_count":2}"#.as_slice(),
        br#"{"total_count":null,"total_count":42}"#.as_slice(),
        br#"{"total_count":0,"total_count":{}}"#.as_slice(),
        br#"{"total_count":1.0,"\u0074otal_count":42}"#.as_slice(),
        br#"{"to\u0074al_count":42,"total_count":0}"#.as_slice(),
        br#"{"total_count":18446744073709551616,"total_count":-0}"#.as_slice(),
    ] {
        assert_eq!(selected(body, "total_count"), Ok(IntegerField::Duplicate), "{body:?}");
    }
}

#[test]
fn all_selected_noninteger_json_kinds_are_raw_invalid_types_after_validation() {
    for value in ["null", "true", "false", r#""42""#, "[]", "{}", r#"[0,{"a":true}]"#] {
        let body = format!(r#"{{"total_count":{value}}}"#);
        assert_eq!(selected(body.as_bytes(), "total_count"), Ok(IntegerField::InvalidType), "{value}");
    }
}

#[test]
fn arbitrary_ignored_composites_are_validated_without_selecting_nested_fields() {
    let body = br#"{"extra":[true,false,null,{},[],{"k":"v","n":-12.5e+2}],"total_count":42}"#;
    assert_eq!(
        selected(body, "total_count"),
        Ok(IntegerField::Integer { negative: false, magnitude: Some(42) })
    );
    let missing = br#"{"extra":[true,false,null,{},[],{"total_count":42,"n":1e999}]}"#;
    assert_eq!(selected(missing, "total_count"), Ok(IntegerField::Missing));
}

#[test]
fn root_must_be_one_complete_object_with_only_json_whitespace_outside_it() {
    for body in [
        b"".as_slice(), b" \r\n\t".as_slice(), b"null".as_slice(), b"true".as_slice(),
        b"42".as_slice(), br#""42""#.as_slice(), b"[]".as_slice(), b"[{}]".as_slice(),
        b"{}{}".as_slice(), b"{}null".as_slice(), b"{}0".as_slice(),
        b"{}\0".as_slice(), b"\xef\xbb\xbf{}".as_slice(), b"{}\xef\xbb\xbf".as_slice(),
        b"\xc2\xa0{}".as_slice(), b"{}\xc2\xa0".as_slice(),
    ] {
        assert_eq!(selected(body, "total_count"), Err(DecodeError::InvalidJson), "{body:?}");
    }
    let body = b" \t\r\n { \"\\u0074otal_count\" \t:\r\n 42 , \"z\" : [ ] } \n";
    assert_eq!(
        selected(body, "total_count"),
        Ok(IntegerField::Integer { negative: false, magnitude: Some(42) })
    );
}

#[test]
fn malformed_ignored_tail_rejects_every_previously_selected_outcome() {
    for body in [
        br#"{"total_count":42,"ignored":}"#.as_slice(),
        br#"{"total_count":42,"ignored":[0,]}"#.as_slice(),
        br#"{"total_count":42,"ignored":{"a":1,}}"#.as_slice(),
        br#"{"total_count":false,"ignored":"\q"}"#.as_slice(),
        br#"{"total_count":0,"total_count":1,"ignored":tru}"#.as_slice(),
        br#"{"total_count":42,}"#.as_slice(),
        br#"{"total_count":42 "ignored":0}"#.as_slice(),
        br#"{"total_count":42,"ignored" 0}"#.as_slice(),
        br#"{"total_count":42,"ignored":[false true]}"#.as_slice(),
        br#"{"total_count":42,"ignored":{"a"}}"#.as_slice(),
        br#"{"total_count":42,"ignored":[}"#.as_slice(),
    ] {
        assert_eq!(selected(body, "total_count"), Err(DecodeError::InvalidJson), "{body:?}");
    }
}

#[test]
fn every_proper_prefix_of_a_complete_object_is_rejected() -> Result<(), DecodeError> {
    let body = br#"{"total_count":42,"other":[true,{"s":"ok"}]}"#;
    for cut in 0..body.len() {
        let prefix = body.get(..cut).ok_or(DecodeError::InvalidJson)?;
        assert_eq!(selected(prefix, "total_count"), Err(DecodeError::InvalidJson), "cut {cut}");
    }
    assert_eq!(
        selected(body, "total_count"),
        Ok(IntegerField::Integer { negative: false, magnitude: Some(42) })
    );
    Ok(())
}

#[test]
fn every_json_escape_and_valid_unicode_form_can_identify_a_decoded_key() {
    let cases: &[(&[u8], &str)] = &[
        (br#"{"\"":7}"#, "\""),
        (br#"{"\\":7}"#, "\\"),
        (br#"{"\/":7}"#, "/"),
        (br#"{"\b":7}"#, "\u{0008}"),
        (br#"{"\f":7}"#, "\u{000c}"),
        (br#"{"\n":7}"#, "\n"),
        (br#"{"\r":7}"#, "\r"),
        (br#"{"\t":7}"#, "\t"),
        (br#"{"\u0000":7}"#, "\0"),
        (br#"{"\u0078":7}"#, "x"),
        (br#"{"\u00e9":7}"#, "é"),
        (r#"{"é":7}"#.as_bytes(), "é"),
        (r#"{"€":7}"#.as_bytes(), "€"),
        (br#"{"\ud834\udd1e":7}"#, "\u{1d11e}"),
        (br#"{"\ud800\udc00":7}"#, "\u{10000}"),
        (br#"{"\udbff\udfff":7}"#, "\u{10ffff}"),
        (r#"{"𝄞":7}"#.as_bytes(), "\u{1d11e}"),
    ];
    for &(body, field) in cases {
        assert_eq!(
            selected(body, field),
            Ok(IntegerField::Integer { negative: false, magnitude: Some(7) }),
            "{body:?} {field:?}"
        );
    }
    let ignored = br#"{"total_count":42,"s":"\"\\\/\b\f\n\r\t\u0000\uD834\uDD1E"}"#;
    assert_eq!(
        selected(ignored, "total_count"),
        Ok(IntegerField::Integer { negative: false, magnitude: Some(42) })
    );
}

#[test]
fn malformed_escapes_and_unpaired_surrogates_reject_even_ignored_strings() {
    for escaped in [
        r"\q", r"\x20", r"\0", r"\v", r"\'", r"\U0000",
        r"\u", r"\u0", r"\u000", r"\uZZZZ", r"\u00G0",
        r"\uD800", r"\uDC00", r"\uD800x", r"\uD800\uD800",
        r"\uDC00\uD800", r"\uD800\u0041", r"\uD834\uDD", r"\uDC00\uDC00",
    ] {
        let body = format!(r#"{{"total_count":42,"s":"{escaped}"}}"#);
        assert_eq!(selected(body.as_bytes(), "total_count"), Err(DecodeError::InvalidJson), "{escaped}");
        let key = format!(r#"{{"{escaped}":0,"total_count":42}}"#);
        assert_eq!(selected(key.as_bytes(), "total_count"), Err(DecodeError::InvalidJson), "{escaped}");
    }
}

#[test]
fn every_unescaped_control_byte_is_invalid_inside_a_json_string() {
    for control in 0_u8..=0x1f {
        let mut body = br#"{"total_count":42,"s":""#.to_vec();
        body.push(control);
        body.extend_from_slice(b"\"}");
        assert_eq!(selected(&body, "total_count"), Err(DecodeError::InvalidJson), "{control}");
    }
}

#[test]
fn invalid_utf8_is_rejected_before_any_raw_field_outcome_is_returned() {
    let invalid: &[&[u8]] = &[
        b"\xff", b"\x80", b"\xc0\xaf", b"\xe2\x82", b"\xed\xa0\x80",
        b"\xf4\x90\x80\x80",
    ];
    for &bytes in invalid {
        let mut body = br#"{"total_count":42,"s":""#.to_vec();
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\"}");
        assert_eq!(selected(&body, "total_count"), Err(DecodeError::InvalidJson), "{bytes:?}");
    }
}
