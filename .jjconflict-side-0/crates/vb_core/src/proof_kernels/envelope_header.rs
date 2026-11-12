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
mod tests;
