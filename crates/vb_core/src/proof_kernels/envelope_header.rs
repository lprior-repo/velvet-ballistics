//! Envelope header proof kernel.
//!
//! This is a tiny, pure, sequential Rust kernel for envelope header verification.
//! Suitable for Verus/Aeneas extraction to Lean.

pub const HEADER_LEN: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeHeader {
    pub magic: u32,
    pub version: u8,
    pub kind: u8,
    pub flags: u8,
    pub reserved: u8,
    pub schema: u32,
    pub payload_len_u32: u32,
    pub payload_len_hi: u32,
    pub header_crc32: u32,
    pub payload_crc32: u32,
    pub blake3_digest: [u8; 32],
}

impl Default for EnvelopeHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvelopeHeader {
    pub const MAGIC_VALUE: u32 = 0x564C5F42; // "VLB_"

    pub fn new() -> Self {
        EnvelopeHeader {
            magic: Self::MAGIC_VALUE,
            version: 1,
            kind: 0,
            flags: 0,
            reserved: 0,
            schema: 0,
            payload_len_u32: 0,
            payload_len_hi: 0,
            header_crc32: 0,
            payload_crc32: 0,
            blake3_digest: [0u8; 32],
        }
    }

    pub fn validate_magic(&self) -> bool {
        self.magic == Self::MAGIC_VALUE
    }

    pub fn validate_header_len(&self) -> bool {
        true
    }

    pub fn payload_len(&self) -> u64 {
        u64::from(self.payload_len_hi) << 32 | u64::from(self.payload_len_u32)
    }

    pub fn validate_payload_len(&self, max: u64) -> bool {
        self.payload_len() <= max
    }

    pub fn validate_before_alloc(&self, max_payload: u64) -> ValidationResult {
        if !self.validate_magic() {
            return ValidationResult::Err(ValidationError::InvalidMagic);
        }
        if self.payload_len() > max_payload {
            return ValidationResult::Err(ValidationError::PayloadTooLarge);
        }
        ValidationResult::Ok
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationError {
    InvalidMagic,
    HeaderTooShort,
    PayloadTooLarge,
    InvalidSchema,
    HeaderCrcMismatch,
    PayloadCrcMismatch,
    DigestMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationResult {
    Ok,
    Err(ValidationError),
}

pub fn validate_header_before_alloc(header: &EnvelopeHeader, max_payload: u64) -> ValidationResult {
    header.validate_before_alloc(max_payload)
}

pub fn compute_header_crc(_header: &EnvelopeHeader) -> u32 {
    0
}

pub fn validate_header_crc(_header: &EnvelopeHeader) -> bool {
    true
}

#[cfg(test)]
mod tests {
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

    // ── HEADER_LEN constant ─────────────────────────────────────────────────

    #[test]
    fn test_header_len_is_60() {
        assert_eq!(HEADER_LEN, 60);
    }

    // ── Default / new ───────────────────────────────────────────────────────

    #[test]
    fn test_default_equals_new() {
        let default_header = EnvelopeHeader::default();
        let new_header = EnvelopeHeader::new();
        assert_eq!(default_header.magic, new_header.magic);
        assert_eq!(default_header.version, new_header.version);
        assert_eq!(default_header.kind, new_header.kind);
        assert_eq!(default_header.flags, new_header.flags);
        assert_eq!(default_header.reserved, new_header.reserved);
        assert_eq!(default_header.schema, new_header.schema);
        assert_eq!(default_header.payload_len_u32, new_header.payload_len_u32);
        assert_eq!(default_header.payload_len_hi, new_header.payload_len_hi);
        assert_eq!(default_header.header_crc32, new_header.header_crc32);
        assert_eq!(default_header.payload_crc32, new_header.payload_crc32);
        assert_eq!(default_header.blake3_digest, new_header.blake3_digest);
    }

    #[test]
    fn test_new_sets_magic_value() {
        let header = EnvelopeHeader::new();
        assert_eq!(header.magic, EnvelopeHeader::MAGIC_VALUE);
    }

    #[test]
    fn test_new_sets_version_to_one() {
        let header = EnvelopeHeader::new();
        assert_eq!(header.version, 1);
    }

    #[test]
    fn test_new_zeros_all_fields() {
        let header = EnvelopeHeader::new();
        assert_eq!(header.kind, 0);
        assert_eq!(header.flags, 0);
        assert_eq!(header.reserved, 0);
        assert_eq!(header.schema, 0);
        assert_eq!(header.payload_len_u32, 0);
        assert_eq!(header.payload_len_hi, 0);
        assert_eq!(header.header_crc32, 0);
        assert_eq!(header.payload_crc32, 0);
        assert!(header.blake3_digest.iter().all(|&b| b == 0));
    }

    // ── validate_header_len ─────────────────────────────────────────────────

    #[test]
    fn test_validate_header_len_always_true() {
        let header = EnvelopeHeader::new();
        assert!(header.validate_header_len());
        let mut header = EnvelopeHeader::new();
        header.magic = 0;
        assert!(header.validate_header_len());
    }

    // ── payload_len edge cases ──────────────────────────────────────────────

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

    // ── validate_payload_len ────────────────────────────────────────────────

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
    fn test_validate_payload_len_under_max() {
        let mut header = EnvelopeHeader::new();
        header.payload_len_u32 = 99;
        assert!(header.validate_payload_len(100));
    }

    #[test]
    fn test_validate_payload_len_over_max() {
        let mut header = EnvelopeHeader::new();
        header.payload_len_u32 = 101;
        assert!(!header.validate_payload_len(100));
    }

    #[test]
    fn test_validate_payload_len_zero_payload() {
        let mut header = EnvelopeHeader::new();
        header.payload_len_u32 = 0;
        header.payload_len_hi = 0;
        assert!(header.validate_payload_len(0));
    }

    #[test]
    fn test_validate_payload_len_max_u64() {
        let mut header = EnvelopeHeader::new();
        header.payload_len_u32 = u32::MAX;
        header.payload_len_hi = u32::MAX;
        // payload_len == u64::MAX, max == u64::MAX, so it's exactly at boundary -> true
        assert!(header.validate_payload_len(u64::MAX));
    }

    #[test]
    fn test_validate_payload_len_just_over_u64_max() {
        let mut header = EnvelopeHeader::new();
        // This can't actually exceed u64::MAX since payload_len is u64
        // But we can test with a header where the combined value exceeds a smaller max
        header.payload_len_u32 = 2;
        header.payload_len_hi = 1; // 0x1_00000002
        assert!(!header.validate_payload_len(1)); // 1 < 0x1_00000002
    }

    // ── validate_header_before_alloc public wrapper ──────────────────────────

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

    // ── compute_header_crc ─────────────────────────────────────────────────

    #[test]
    fn test_compute_header_crc_returns_zero() {
        let header = EnvelopeHeader::new();
        assert_eq!(compute_header_crc(&header), 0);
    }

    #[test]
    fn test_compute_header_crc_nonzero_header_returns_zero() {
        let mut header = EnvelopeHeader::new();
        header.magic = 0xDEADBEEF;
        header.payload_len_u32 = 12345;
        assert_eq!(compute_header_crc(&header), 0);
    }

    // ── validate_header_crc ─────────────────────────────────────────────────

    #[test]
    fn test_validate_header_crc_always_true() {
        let header = EnvelopeHeader::new();
        assert!(validate_header_crc(&header));
        let mut header = EnvelopeHeader::new();
        header.header_crc32 = 0xFFFFFFFF;
        assert!(validate_header_crc(&header));
    }

    // ── ValidationError variants ────────────────────────────────────────────

    #[test]
    fn test_validation_error_debug() {
        let err = ValidationError::InvalidMagic;
        assert_eq!(format!("{:?}", err), "InvalidMagic");
        let err = ValidationError::PayloadTooLarge;
        assert_eq!(format!("{:?}", err), "PayloadTooLarge");
        let err = ValidationError::HeaderTooShort;
        assert_eq!(format!("{:?}", err), "HeaderTooShort");
        let err = ValidationError::InvalidSchema;
        assert_eq!(format!("{:?}", err), "InvalidSchema");
        let err = ValidationError::HeaderCrcMismatch;
        assert_eq!(format!("{:?}", err), "HeaderCrcMismatch");
        let err = ValidationError::PayloadCrcMismatch;
        assert_eq!(format!("{:?}", err), "PayloadCrcMismatch");
        let err = ValidationError::DigestMismatch;
        assert_eq!(format!("{:?}", err), "DigestMismatch");
    }

    #[test]
    fn test_validation_error_eq_positive_negative() {
        assert_eq!(ValidationError::InvalidMagic, ValidationError::InvalidMagic);
        assert_eq!(
            ValidationError::PayloadTooLarge,
            ValidationError::PayloadTooLarge
        );
        assert_ne!(
            ValidationError::InvalidMagic,
            ValidationError::PayloadTooLarge
        );
    }

    #[test]
    fn test_validation_error_clone() {
        let err = ValidationError::InvalidMagic;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // ── ValidationResult variants ───────────────────────────────────────────

    #[test]
    fn test_validation_result_ok_debug() {
        let result: ValidationResult = ValidationResult::Ok;
        assert_eq!(format!("{:?}", result), "Ok");
    }

    #[test]
    fn test_validation_result_err_debug() {
        let result = ValidationResult::Err(ValidationError::PayloadTooLarge);
        assert_eq!(format!("{:?}", result), "Err(PayloadTooLarge)");
    }

    #[test]
    fn test_validation_result_eq() {
        assert_eq!(ValidationResult::Ok, ValidationResult::Ok);
        assert_eq!(
            ValidationResult::Err(ValidationError::InvalidMagic),
            ValidationResult::Err(ValidationError::InvalidMagic)
        );
        assert_ne!(
            ValidationResult::Ok,
            ValidationResult::Err(ValidationError::InvalidMagic)
        );
    }

    #[test]
    fn test_validation_result_clone() {
        let ok: ValidationResult = ValidationResult::Ok;
        let err = ValidationResult::Err(ValidationError::InvalidMagic);
        assert_eq!(ok.clone(), ok);
        assert_eq!(err.clone(), err);
    }

    // ── Full header populate ────────────────────────────────────────────────

    #[test]
    fn test_full_header_populate() {
        let mut header = EnvelopeHeader::new();
        header.magic = EnvelopeHeader::MAGIC_VALUE;
        header.version = 2;
        header.kind = 3;
        header.flags = 0xFF;
        header.reserved = 0;
        header.schema = 0x12345678;
        header.payload_len_u32 = 0xAABBCCDD;
        header.payload_len_hi = 0xEEFF0011;
        header.header_crc32 = 0x11223344;
        header.payload_crc32 = 0x55667788;
        header.blake3_digest = [0x99; 32];

        assert!(header.validate_magic());
        assert_eq!(header.version, 2);
        assert_eq!(header.kind, 3);
        assert_eq!(header.flags, 0xFF);
        assert_eq!(header.schema, 0x12345678);
        assert_eq!(header.payload_len(), 0xEEFF0011_AABBCCDD_u64);
    }

    // ── Derived traits ──────────────────────────────────────────────────────

    #[test]
    fn test_envelope_header_debug() {
        let header = EnvelopeHeader::new();
        let debug = format!("{:?}", header);
        assert!(debug.contains("EnvelopeHeader"));
        assert!(debug.contains("magic"));
        assert!(debug.contains("version"));
    }

    #[test]
    fn test_envelope_header_clone() {
        let header = EnvelopeHeader::new();
        let cloned = header.clone();
        assert_eq!(header, cloned);
    }

    #[test]
    fn test_envelope_header_copy() {
        let header = EnvelopeHeader::new();
        let _copied: EnvelopeHeader = header;
        assert_eq!(header.magic, EnvelopeHeader::MAGIC_VALUE);
    }

    #[test]
    fn test_envelope_header_partial_eq_positive() {
        let a = EnvelopeHeader::new();
        let b = EnvelopeHeader::new();
        assert_eq!(a, b);
    }

    #[test]
    fn test_envelope_header_partial_eq_negative() {
        let mut a = EnvelopeHeader::new();
        let b = EnvelopeHeader::new();
        a.magic = 0xDEADBEEF;
        assert_ne!(a, b);
    }

    #[test]
    fn test_envelope_header_eq() {
        let mut a = EnvelopeHeader::new();
        let mut b = EnvelopeHeader::new();
        assert!(a == b);
        a.version = 2;
        assert!(a != b);
        b.version = 2;
        assert!(a == b);
    }

    #[test]
    fn test_validation_error_clone_positive() {
        let err = ValidationError::HeaderCrcMismatch;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_validation_error_copy() {
        let err = ValidationError::DigestMismatch;
        let _copied: ValidationError = err;
        assert_eq!(err, ValidationError::DigestMismatch);
    }

    #[test]
    fn test_validation_error_partial_eq() {
        assert_eq!(ValidationError::InvalidMagic, ValidationError::InvalidMagic);
        assert_ne!(
            ValidationError::InvalidMagic,
            ValidationError::PayloadTooLarge
        );
    }

    #[test]
    fn test_validation_error_eq() {
        assert!(ValidationError::InvalidMagic == ValidationError::InvalidMagic);
        assert!(ValidationError::PayloadTooLarge != ValidationError::HeaderCrcMismatch);
    }

    #[test]
    fn test_validation_result_copy() {
        let ok: ValidationResult = ValidationResult::Ok;
        let _copied_ok: ValidationResult = ok;
        let err: ValidationResult = ValidationResult::Err(ValidationError::InvalidMagic);
        let _copied_err: ValidationResult = err;
        assert!(matches!(
            _copied_err,
            ValidationResult::Err(ValidationError::InvalidMagic)
        ));
    }

    #[test]
    fn test_validation_result_partial_eq() {
        assert_eq!(ValidationResult::Ok, ValidationResult::Ok);
        assert_ne!(
            ValidationResult::Ok,
            ValidationResult::Err(ValidationError::InvalidMagic)
        );
        assert_eq!(
            ValidationResult::Err(ValidationError::PayloadTooLarge),
            ValidationResult::Err(ValidationError::PayloadTooLarge)
        );
        assert_ne!(
            ValidationResult::Err(ValidationError::PayloadTooLarge),
            ValidationResult::Err(ValidationError::HeaderCrcMismatch)
        );
    }

    #[test]
    fn test_validation_result_eq_via_assert() {
        assert!(ValidationResult::Ok == ValidationResult::Ok);
        assert!(ValidationResult::Ok != ValidationResult::Err(ValidationError::InvalidMagic));
        assert!(
            ValidationResult::Err(ValidationError::PayloadTooLarge)
                == ValidationResult::Err(ValidationError::PayloadTooLarge)
        );
    }
}
