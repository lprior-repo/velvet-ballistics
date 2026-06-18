#![forbid(unsafe_code)]
//! Internal fuzz-harness accessors; unavailable in normal Cargo builds.

use crate::{JournalError, records::CompiledIrRecord};

use super::types::AcceptedArtifact;

pub(crate) fn validate_compiled_ir_record(record: &CompiledIrRecord) -> Result<(), JournalError> {
    super::record::validate_compiled_ir_record(record)
}

pub(crate) fn decode_accepted_artifact_envelope(
    bytes: &[u8],
) -> Result<AcceptedArtifact, JournalError> {
    super::record::decode_accepted_artifact_envelope(bytes)
}

pub(crate) fn reject_oversized_compiled_ir_value(len: usize) -> Result<(), JournalError> {
    super::bytes::reject_oversized_compiled_ir_value(len)
}
