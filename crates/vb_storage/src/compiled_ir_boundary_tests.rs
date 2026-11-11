use super::*;

#[test]
fn trailing_byte_rejected() -> TestResult {
    let mut bytes = postcard::to_allocvec(&legacy(0)?.to_parts())?;
    bytes.push(0);
    assert_malformed(decode_compiled_ir(&bytes, None))
}

#[test]
fn concatenated_payloads_rejected() -> TestResult {
    let original = postcard::to_allocvec(&legacy(0)?.to_parts())?;
    let mut bytes = original.clone();
    bytes.extend_from_slice(&original);
    assert_malformed(decode_compiled_ir(&bytes, None))
}

#[test]
fn v2_trailing_and_concatenated_rejected() -> TestResult {
    let original = wire_v2(&bound_ir(&["key"])?)?;
    let mut trailing = original.clone();
    trailing.push(0);
    assert_malformed(decode_compiled_ir(&trailing, None))?;
    let mut concatenated = original.clone();
    concatenated.extend_from_slice(&original);
    assert_malformed(decode_compiled_ir(&concatenated, None))
}

#[test]
fn byte_cap_exactly_at_limit_reaches_decoding() -> TestResult {
    let mut bytes = postcard::to_allocvec(&legacy(0)?.to_parts())?;
    let limit = usize::try_from(MAX_COMPILED_IR_BYTES)?;
    bytes.resize(limit, 0);
    assert_eq!(bytes.len(), limit);
    assert_malformed(decode_compiled_ir(&bytes, None))
}

#[test]
fn byte_cap_exceeded() -> TestResult {
    let length = MAX_COMPILED_IR_BYTES
        .checked_add(1)
        .ok_or("byte length overflow")?;
    let bytes = vec![0; usize::try_from(length)?];
    assert_too_large(decode_compiled_ir(&bytes, None), length)
}

#[test]
fn byte_cap_precedes_version_decode() -> TestResult {
    let length = MAX_COMPILED_IR_BYTES
        .checked_add(1)
        .ok_or("byte length overflow")?;
    let bytes = vec![0; usize::try_from(length)?];
    assert_too_large(
        decode_compiled_ir(&bytes, Some(SchemaVersion::new(3, 0, 0))),
        length,
    )
}

#[test]
fn corrupt_v2_payload() -> TestResult {
    let mut ir = bound_ir(&[])?;
    ir.wire_version = 3;
    assert_malformed(decode_compiled_ir(&wire_v2(&ir)?, None))
}

#[test]
fn missing_and_truncated_payloads() -> TestResult {
    assert_eof(&[])?;
    assert_eof(&[0x01, 0xff, 0x56, 0x42])
}

#[test]
fn encode_preflights_wire_size_before_digest() -> TestResult {
    let count = MAX_COMPILED_IR_BYTES
        .checked_add(1)
        .ok_or("byte length overflow")?;
    let mut value = parts(0);
    value.name = "x".repeat(usize::try_from(count)?).into_boxed_str();
    let expected = u32::try_from(postcard::to_allocvec(&value)?.len())?;
    let workflow = CompiledWorkflow::try_from_parts(value)?;
    assert_too_large(encode_compiled_ir(&workflow), expected)
}
