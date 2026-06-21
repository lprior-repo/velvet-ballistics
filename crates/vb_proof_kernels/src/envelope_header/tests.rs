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
fn test_compute_header_crc_is_deterministic() {
    // compute_header_crc must be deterministic: same header → same CRC.
    // This is enforceable even on the current stub (which returns 0);
    // a regression to a non-deterministic impl (e.g., random) would break this.
    let header = EnvelopeHeader::new();
    let crc1 = compute_header_crc(&header);
    let crc2 = compute_header_crc(&header);
    assert_eq!(
        crc1, crc2,
        "compute_header_crc must be deterministic for the same header"
    );
}

#[test]
fn test_validate_header_crc_accepts_default_header() {
    // validate_header_crc must accept a header whose CRC matches the
    // compute_header_crc output. For a valid (unmodified) header the
    // CRC contract requires validation to succeed.
    let header = EnvelopeHeader::new();
    let crc = compute_header_crc(&header);
    let valid = validate_header_crc(&header);
    if crc == 0 {
        // Current stub impl: validate is constant true. Document that.
        assert!(
            valid,
            "validate_header_crc accepts default header (stub contract)"
        );
    } else {
        // Real impl: CRC of a default header must validate as true.
        assert!(
            valid,
            "validate_header_crc must accept a header whose CRC matches compute_header_crc"
        );
    }
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
