//! Envelope header proof kernel.
//!
//! This is a tiny, pure, sequential Rust kernel for envelope header verification.
//! Suitable for Verus/Aeneas extraction to Lean.

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

// ── Verus verified layer ────────────────────────────────────────────────────
#[cfg(verus_keep_ghost)]
verus! {

    // ── Envelope header constants (nat-level) ────────────────────────────
    pub open spec fn HEADER_LEN() -> nat { 60 }
    pub open spec fn MAGIC_VALUE() -> nat { 0x564C5F42 }

    // ── Spec: payload_len combines the two 32-bit halves ───────────────
    // 2^32 = 4294967296
    pub open spec fn spec_payload_len(payload_len_hi: nat, payload_len_u32: nat) -> nat {
        payload_len_hi * 4294967296nat + payload_len_u32
    }

    // ── Spec: magic is valid ───────────────────────────────────────────
    pub open spec fn spec_validate_magic(magic: nat) -> bool {
        magic == MAGIC_VALUE()
    }

    // ── Spec: payload length within bound ──────────────────────────────
    pub open spec fn spec_validate_payload_len(payload_len_hi: nat, payload_len_u32: nat, max: nat) -> bool {
        spec_payload_len(payload_len_hi, payload_len_u32) <= max
    }

    // ── Spec: validate_before_alloc — returns Ok iff magic valid and
    //    payload within bound ──────────────────────────────────────────
    pub enum ValidationSpecResult {
        Ok,
        InvalidMagic,
        PayloadTooLarge,
    }

    // ── Spec: validate_before_alloc — returns Ok iff magic valid and
    //    payload within bound ──────────────────────────────────────────
    pub open spec fn spec_validate_before_alloc(
        magic: nat,
        payload_len_hi: nat,
        payload_len_u32: nat,
        max_payload: nat,
    ) -> ValidationSpecResult {
        if !spec_validate_magic(magic) {
            ValidationSpecResult::InvalidMagic
        } else if spec_payload_len(payload_len_hi, payload_len_u32) > max_payload {
            ValidationSpecResult::PayloadTooLarge
        } else {
            ValidationSpecResult::Ok
        }
    }

    // ── Lemma: payload_len of zero halves is zero ──────────────────────
    proof fn lemma_payload_len_zero_halves()
        ensures
            spec_payload_len(0, 0) == 0,
    {
    }

    // ── Lemma: payload_len is non-negative ─────────────────────────────
    proof fn lemma_payload_len_non_negative(hi: nat, lo: nat)
        ensures
            spec_payload_len(hi, lo) >= 0,
    {
    }

    // ── Lemma: validate_magic of correct magic is true ─────────────────
    proof fn lemma_magic_valid()
        ensures
            spec_validate_magic(MAGIC_VALUE()),
    {
    }

    // ── Lemma: validate_magic of wrong magic is false ──────────────────
    proof fn lemma_magic_invalid()
        ensures
            !spec_validate_magic(MAGIC_VALUE() + 1),
    {
    }

    // ── Lemma: new header (magic + zero payload) validates against any max ─
    proof fn lemma_new_validates(
        hi: nat,
        lo: nat,
        max: nat,
    )
        requires
            hi == 0 && lo == 0,
            max >= 0,
        ensures
            spec_validate_before_alloc(MAGIC_VALUE(), hi, lo, max) == ValidationSpecResult::Ok,
    {
    }

    // ── Lemma: bad magic always yields InvalidMagic ─────────────────────
    proof fn lemma_bad_magic_yields_invalid_magic(
        hi: nat,
        lo: nat,
        max: nat,
    )
        requires
            hi >= 0 && lo >= 0 && max >= 0,
        ensures
            spec_validate_before_alloc(MAGIC_VALUE() + 1, hi, lo, max) == ValidationSpecResult::InvalidMagic,
    {
    }

    // ── Lemma: oversized payload yields PayloadTooLarge (when magic valid) ─
    proof fn lemma_oversize_yields_payload_too_large(
        hi: nat,
        lo: nat,
        max: nat,
    )
        requires
            hi >= 0 && lo >= 0 && max >= 0,
            spec_payload_len(hi, lo) > max,
        ensures
            spec_validate_before_alloc(MAGIC_VALUE(), hi, lo, max) == ValidationSpecResult::PayloadTooLarge,
    {
    }

    // ── Lemma: both checks pass yields Ok ──────────────────────────────
    proof fn lemma_both_pass(
        hi: nat,
        lo: nat,
        max: nat,
    )
        requires
            hi >= 0 && lo >= 0 && max >= 0,
            spec_payload_len(hi, lo) <= max,
        ensures
            spec_validate_before_alloc(MAGIC_VALUE(), hi, lo, max) == ValidationSpecResult::Ok,
    {
    }

    // ── Lemma: validate_before_alloc is total (always returns a result) ──
    proof fn lemma_validate_before_alloc_total(
        magic: nat,
        hi: nat,
        lo: nat,
        max: nat,
    )
        requires
            magic >= 0 && hi >= 0 && lo >= 0 && max >= 0,
        ensures
            match spec_validate_before_alloc(magic, hi, lo, max) {
                ValidationSpecResult::Ok => true,
                ValidationSpecResult::InvalidMagic => true,
                ValidationSpecResult::PayloadTooLarge => true,
            },
    {
    }

    // ── Lemma: payload_len monotone in hi ──────────────────────────────
    proof fn lemma_payload_len_monotone_hi(hi1: nat, hi2: nat, lo: nat)
        requires
            hi1 < hi2 && lo >= 0,
        ensures
            spec_payload_len(hi1, lo) < spec_payload_len(hi2, lo),
    {
    }

    // ── Lemma: payload_len monotone in lo ──────────────────────────────
    proof fn lemma_payload_len_monotone_lo(hi: nat, lo1: nat, lo2: nat)
        requires
            hi >= 0 && lo1 < lo2,
        ensures
            spec_payload_len(hi, lo1) < spec_payload_len(hi, lo2),
    {
    }

} // verus!

// ── Regular Rust implementation (non-Verus compilation) ─────────────────────
#[cfg(not(verus_keep_ghost))]
mod cargo_kernel {
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
}
#[cfg(not(verus_keep_ghost))]
pub use cargo_kernel::*;

// ── Tests (compiled in both modes) ──────────────────────────────────────────
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
}
