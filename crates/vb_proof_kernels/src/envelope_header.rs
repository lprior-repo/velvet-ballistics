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

    /// Compute CRC-32C over this header's bytes.
    /// The CRC field is set to zero during computation.
    pub fn compute_crc(&self) -> u32 {
        let mut header = *self;
        header.header_crc32 = 0;
        compute_header_crc(&header)
    }

    /// Validate that this header's stored CRC matches the computed CRC.
    pub fn validate_crc(&self) -> bool {
        validate_header_crc(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub enum ValidationResult {
    Ok,
    Err(ValidationError),
}

pub fn validate_header_before_alloc(header: &EnvelopeHeader, max_payload: u64) -> ValidationResult {
    header.validate_before_alloc(max_payload)
}

/// CRC-32C lookup table for fast computation.
const CRC32C_TABLE: [u32; 256] = make_crc32c_table();

/// Constructs the CRC-32C lookup table at compile time.
const fn make_crc32c_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut c = i as u32;
        let mut j = 0usize;
        while j < 8 {
            if c & 1 != 0 {
                c = 0x82F63B78 ^ (c >> 1);
            } else {
                c = c >> 1;
            }
            j += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

/// Update CRC-32C state with a single byte.
#[inline]
const fn crc32c_update(crc: u32, byte: u8) -> u32 {
    let index = ((crc ^ byte as u32) & 0xFF) as usize;
    CRC32C_TABLE[index] ^ (crc >> 8)
}

/// Compute CRC-32C (Castagnoli) over a byte slice.
fn crc32c_slice(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFF;
    let mut i = 0usize;
    while i < data.len() {
        crc = crc32c_update(crc, data[i]);
        i += 1;
    }
    crc ^ 0xFFFFFFFF
}

/// Compute CRC-32C (Castagnoli) over the header fields before the CRC field.
///
/// The CRC field itself (bytes 20..24 in the header) is excluded from computation
/// and should be set to zero when computing. Uses the Castagnoli polynomial 0x82F63B78.
///
/// Layout: magic(4) + version(1) + kind(1) + flags(1) + reserved(1) +
///         schema(4) + payload_len_u32(4) + payload_len_hi(4) = 20 bytes
///         then header_crc32(4) + payload_crc32(4) + blake3_digest(32) = 40 bytes
/// CRC is over the first 20 bytes (excluding header_crc32 field).
pub fn compute_header_crc(header: &EnvelopeHeader) -> u32 {
    // Build header bytes for CRC computation, excluding the 4-byte CRC field at offset 20.
    // All multi-byte fields are in little-endian byte order.
    let mut buf = [0u8; 20];
    // Magic (4 bytes, offset 0)
    buf[0..4].copy_from_slice(&header.magic.to_le_bytes());
    // Version (1 byte, offset 4)
    buf[4] = header.version;
    // Kind (1 byte, offset 5)
    buf[5] = header.kind;
    // Flags (1 byte, offset 6)
    buf[6] = header.flags;
    // Reserved (1 byte, offset 7)
    buf[7] = header.reserved;
    // Schema (4 bytes, offset 8)
    buf[8..12].copy_from_slice(&header.schema.to_le_bytes());
    // Payload len lo (4 bytes, offset 12)
    buf[12..16].copy_from_slice(&header.payload_len_u32.to_le_bytes());
    // Payload len hi (4 bytes, offset 16)
    buf[16..20].copy_from_slice(&header.payload_len_hi.to_le_bytes());
    crc32c_slice(&buf)
}

/// Validate that the stored header CRC matches the computed CRC.
/// Returns true if CRC is valid, false otherwise.
pub fn validate_header_crc(header: &EnvelopeHeader) -> bool {
    let computed = compute_header_crc(header);
    computed == header.header_crc32
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

    #[test]
    fn test_compute_header_crc_not_zero() {
        let header = EnvelopeHeader::new();
        let crc = compute_header_crc(&header);
        // CRC should not be zero for a non-empty header
        assert_ne!(crc, 0, "CRC should not be zero for a valid header");
    }

    #[test]
    fn test_validate_header_crc_with_correct_crc() {
        let header = EnvelopeHeader::new();
        let crc = compute_header_crc(&header);
        let mut header_with_crc = header;
        header_with_crc.header_crc32 = crc;
        assert!(validate_header_crc(&header_with_crc));
    }

    #[test]
    fn test_validate_header_crc_with_incorrect_crc() {
        let header = EnvelopeHeader::new();
        let mut header_with_wrong_crc = header;
        header_with_wrong_crc.header_crc32 = 0xDEADBEEF;
        assert!(!validate_header_crc(&header_with_wrong_crc));
    }

    #[test]
    fn test_envelope_header_compute_crc() {
        let header = EnvelopeHeader::new();
        let crc = header.compute_crc();
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_envelope_header_validate_crc_with_correct_crc() {
        let header = EnvelopeHeader::new();
        let crc = header.compute_crc();
        let mut header_with_crc = header;
        header_with_crc.header_crc32 = crc;
        assert!(header_with_crc.validate_crc());
    }

    #[test]
    fn test_envelope_header_validate_crc_with_incorrect_crc() {
        let header = EnvelopeHeader::new();
        let mut header_with_wrong_crc = header;
        header_with_wrong_crc.header_crc32 = 0xDEADBEEF;
        assert!(!header_with_wrong_crc.validate_crc());
    }

    #[test]
    fn test_crc_changes_with_header_content() {
        let header1 = EnvelopeHeader::new();
        let mut header2 = EnvelopeHeader::new();
        header2.magic = 0xFFFFFFFF;
        let crc1 = compute_header_crc(&header1);
        let crc2 = compute_header_crc(&header2);
        assert_ne!(crc1, crc2, "CRC should differ when header content changes");
    }
}
