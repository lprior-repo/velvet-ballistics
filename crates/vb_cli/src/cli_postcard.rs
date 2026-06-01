//! CLI Postcard Module
//!
//! This module provides binary Postcard output with bounded allocation.
//! Postcard payloads are validated against header before decode.
//!
//! ## Contract Clauses
//! - INV-005: Postcard payloads respect bounded allocation (header_len + payload_len validated before decode)
//! - POST-007: Postcard output validates magic + header length before payload decode

#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "cli_postcard/codec.rs"]
mod codec;
#[path = "cli_postcard/error.rs"]
mod error;
#[path = "cli_postcard/types.rs"]
mod types;
#[path = "cli_postcard/validation.rs"]
mod validation;

pub(crate) use error::PostcardError;
#[cfg(test)]
pub(crate) use types::MAX_PAYLOAD_U32;
pub(crate) use types::{
    CLI_MAGIC, CLI_POSTCARD_KIND, CLI_SCHEMA_VERSION, CliPostcardContentType, CliPostcardPayload,
    HEADER_SIZE, HEADER_SIZE_U32, MAX_PAYLOAD, PostcardHeader,
};

#[allow(unused_imports)]
pub(crate) use codec::{decode_cli_payload, decode_postcard_json, encode_postcard};
pub(crate) use validation::{decode_postcard, payload_digest, validate_cli_payload};

fn read_array<const N: usize>(data: &[u8], start: usize) -> Result<[u8; N], PostcardError> {
    let end = start.checked_add(N).ok_or(PostcardError::DecodeFailed)?;
    let bytes = data.get(start..end).ok_or(PostcardError::DecodeFailed)?;
    <[u8; N]>::try_from(bytes).map_err(|_| PostcardError::DecodeFailed)
}

#[cfg(test)]
#[path = "cli_postcard/tests.rs"]
mod tests;
