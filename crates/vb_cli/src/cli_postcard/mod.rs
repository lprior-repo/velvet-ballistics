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

mod codec;
mod error;
mod types;
mod validation;

pub(crate) use error::PostcardError;
pub(crate) use types::{
    CliPostcardContentType, CliPostcardPayload, PostcardHeader, CLI_MAGIC, CLI_POSTCARD_KIND,
    CLI_SCHEMA_VERSION, HEADER_SIZE, HEADER_SIZE_U32, MAX_PAYLOAD, MAX_PAYLOAD_U32,
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
#[path = "tests.rs"]
mod tests;
