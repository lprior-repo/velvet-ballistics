use super::*;

#[test]
fn test_valid_magic() {
    let header = EnvelopeHeader::new();
    assert!(header.validate_magic());
}

#[test]
fn test_invalid_magic() {
    let mut header = EnvelopeHeader::new();
    header.magic = 0xDEADBEEF;
    assert!(!header.validate_magic());
}

#[test]
fn test_payload_len_combine() {
    let mut header = EnvelopeHeader::new();
    header.payload_len_u32 = 0x12345678;
    header.payload_len_hi = 0x9ABCDEF0;
    assert_eq!(header.payload_len(), 0x9ABCDEF012345678);
}

#[test]
fn test_validate_before_alloc_rejects_bad_magic() {
    let mut header = EnvelopeHeader::new();
    header.magic = 0xDEAD;
    let result = header.validate_before_alloc(1024 * 1024);
    assert!(matches!(
        result,
        ValidationResult::Err(ValidationError::InvalidMagic)
    ));
}

#[test]
fn test_validate_before_alloc_rejects_oversize() {
    let mut header = EnvelopeHeader::new();
    header.payload_len_u32 = 0xFFFFFFFF;
    header.payload_len_hi = 0xFFFFFFFF;
    let result = header.validate_before_alloc(1024 * 1024);
    assert!(matches!(
        result,
        ValidationResult::Err(ValidationError::PayloadTooLarge)
    ));
}

#[test]
fn test_validate_before_alloc_accepts_valid() {
    let header = EnvelopeHeader::new();
    let result = header.validate_before_alloc(1024 * 1024);
    assert!(matches!(result, ValidationResult::Ok));
}

#[test]
fn test_header_len_is_60() {
    assert_eq!(HEADER_LEN, 60);
}

#[test]
fn test_default_equals_new() {
    let default_header = EnvelopeHeader::default();
    let new_header = EnvelopeHeader::new();
    assert_eq!(default_header.magic, new_header.magic);
    assert_eq!(default_header.version, new_header.version);
}

#[test]
fn test_new_sets_magic_value() {
    let header = EnvelopeHeader::new();
    assert_eq!(header.magic, EnvelopeHeader::MAGIC_VALUE);
}

#[test]
fn test_payload_len_zero() {
    let header = EnvelopeHeader::new();
    assert_eq!(header.payload_len(), 0);
}

#[test]
fn test_payload_len_hi_only() {
    let mut header = EnvelopeHeader::new();
    header.payload_len_hi = 1;
    assert_eq!(header.payload_len(), 0x1_00000000_u64);
}

#[test]
fn test_payload_len_u32_only() {
    let mut header = EnvelopeHeader::new();
    header.payload_len_u32 = 42;
    assert_eq!(header.payload_len(), 42);
}

#[test]
fn test_validate_payload_len_zero_max() {
    let header = EnvelopeHeader::new();
    assert!(header.validate_payload_len(0));
}

#[test]
fn test_validate_payload_len_exact_max() {
    let mut header = EnvelopeHeader::new();
    header.payload_len_u32 = 100;
    assert!(header.validate_payload_len(100));
}

#[test]
fn test_validate_payload_len_over_max() {
    let mut header = EnvelopeHeader::new();
    header.payload_len_u32 = 101;
    assert!(!header.validate_payload_len(100));
}

#[test]
fn test_validate_header_before_alloc_accepts_valid_header() {
    let header = EnvelopeHeader::new();
    let result = validate_header_before_alloc(&header, 1024);
    assert!(matches!(result, ValidationResult::Ok));
}

#[test]
fn test_validate_header_before_alloc_rejects_bad_magic() {
    let mut header = EnvelopeHeader::new();
    header.magic = 0xBAD;
    let result = validate_header_before_alloc(&header, 1024);
    assert!(matches!(
        result,
        ValidationResult::Err(ValidationError::InvalidMagic)
    ));
}

#[test]
fn test_validate_header_before_alloc_rejects_oversize() {
    let mut header = EnvelopeHeader::new();
    header.payload_len_u32 = u32::MAX;
    header.payload_len_hi = u32::MAX;
    let result = validate_header_before_alloc(&header, 1024);
    assert!(matches!(
        result,
        ValidationResult::Err(ValidationError::PayloadTooLarge)
    ));
}

#[test]
fn test_compute_header_crc_returns_zero() {
    let header = EnvelopeHeader::new();
    assert_eq!(compute_header_crc(&header), 0);
}

#[test]
fn test_validate_header_crc_always_true() {
    let header = EnvelopeHeader::new();
    assert!(validate_header_crc(&header));
}

#[test]
fn test_envelope_header_clone() {
    let header = EnvelopeHeader::new();
    let cloned = header;
    assert_eq!(header, cloned);
}

#[test]
fn test_envelope_header_copy() {
    let header = EnvelopeHeader::new();
    let _copied: EnvelopeHeader = header;
}

#[test]
fn test_envelope_header_eq() {
    let mut a = EnvelopeHeader::new();
    let b = EnvelopeHeader::new();
    assert!(a == b);
    a.version = 2;
    assert!(a != b);
}
