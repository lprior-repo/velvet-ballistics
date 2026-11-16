mod error;
mod limits;
mod number;
mod parser;
mod string;

pub use error::DecodeError;
pub use limits::DecodeLimits;

/// A root object's selected field after complete bounded JSON validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegerField {
    Missing,
    Duplicate,
    InvalidType,
    Integer {
        negative: bool,
        magnitude: Option<u64>,
    },
}

/// Preserve integer sign and overflow without a floating-point conversion.
/// The caller applies its field-specific type/range policy after validation.
pub(crate) fn decode_integer_field(
    body: &[u8],
    limits: DecodeLimits,
    field: &str,
) -> Result<IntegerField, DecodeError> {
    if body.len() > limits.body_bytes {
        return Err(DecodeError::BodyLimit);
    }
    if std::str::from_utf8(body).is_err() {
        return Err(DecodeError::InvalidJson);
    }
    parser::Parser::new(body, limits, field).run()
}

#[cfg(test)]
#[path = "decode/syntax_tests.rs"]
mod syntax_tests;

#[cfg(test)]
#[path = "decode/boundary_tests.rs"]
mod boundary_tests;
