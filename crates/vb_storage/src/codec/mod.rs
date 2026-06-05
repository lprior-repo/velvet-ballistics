#![forbid(unsafe_code)]
//! Record encoding and decoding functions.

#[cfg(test)]
use crate::error::JournalError;

pub(crate) mod envelope;
pub(crate) mod header;
pub(crate) mod payload;
mod record;
mod replay;
mod semantic;
pub(crate) mod validation;

#[cfg(fuzzing)]
#[doc(hidden)]
pub mod fuzz_validation {
    //! Internal fuzz-harness accessors; unavailable in normal Cargo builds.

    use crate::JournalError;

    pub const fn is_known_record_kind(kind: u16) -> bool {
        super::validation::is_known_record_kind(kind)
    }

    pub fn validate_known_kind(kind: u16) -> Result<(), JournalError> {
        super::validation::validate_known_kind(kind)
    }

    pub fn validate_kind_family(magic: u32, kind: u16) -> Result<(), JournalError> {
        super::validation::validate_kind_family(magic, kind)
    }

    pub fn reject_trailing_bytes(
        declared_end: usize,
        actual_len: usize,
    ) -> Result<(), JournalError> {
        super::payload::reject_trailing_bytes(declared_end, actual_len)
    }
}

pub use self::envelope::decode_envelope_only;
pub use self::header::{decode_record_header, encode_record_header};
pub use self::payload::verify_digest_match;
pub use self::record::{
    decode_journal_event, decode_record, decode_validated_journal_record,
    encode_journal_event_record, encode_record,
};
pub(crate) use self::replay::{next_seq, validate_replayed_event};
pub use self::semantic::{
    ExactJournalKindParity, JournalKindCompatibility, JournalSemanticDecodeDecision,
    ValidatedJournalRecord, classify_journal_kind_compatibility, classify_journal_semantic_decode,
    classify_record_kind_family, is_journal_record_kind, is_known_record_kind,
    journal_kinds_are_exact_match, validate_journal_event_semantics, validate_known_record_kind,
    validate_record_kind_family,
};
pub use self::validation::RecordKindFamilyDecision;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod trailing_bytes_proptests;

// vb-b8i8f: flux_validation requires flux_rs crate (not in workspace).
// Keep as artifact; re-enable when flux_rs dependency is added.
// #[cfg(feature = "flux")]
// pub mod flux_validation;
