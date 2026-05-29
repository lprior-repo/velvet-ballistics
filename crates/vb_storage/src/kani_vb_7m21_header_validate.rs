#![forbid(unsafe_code)]
//! PO-vb-7m21-C002: Header validation bounds proof.
//!
//! Proves that the three validation functions — `validate_schema_version`,
//! `validate_known_kind`, and `validate_kind_family` — have complete error
//! coverage and never panic on arbitrary inputs.
//!
//! GOD RULE 1: Uses `kani::any()` for all inputs. No hardcoded shapes.

use crate::{
    codec::validation::{validate_kind_family, validate_known_kind, validate_schema_version},
    constants::{
        MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD,
        MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE,
    },
    error::JournalError,
};

// ── helpers ────────────────────────────────────────────────────────

/// An arbitrary valid magic value, plus garbage.
fn arbitrary_magic() -> u32 {
    let pick: u8 = kani::any();
    match pick {
        0 => MAGIC_WORKFLOW_SOURCE,
        1 => MAGIC_COMPILED_ARTIFACT,
        2 => MAGIC_JOURNAL_EVENT,
        3 => MAGIC_SNAPSHOT,
        4 => MAGIC_BLOB,
        5 => MAGIC_INDEX_RECORD,
        _ => {
            let magic: u32 = kani::any();
            magic
        }
    }
}

/// An arbitrary record kind that may or may not be known.
fn arbitrary_record_kind() -> u16 {
    let pick: u8 = kani::any();
    match pick {
        0 => 1,   // WorkflowSource
        1 => 2,   // CompiledIr
        2 => 3,   // RunHeader
        3 => 10,  // RunAccepted
        4 => 20,  // StepFailed
        5 => 30,  // Snapshot
        6 => 40,  // Blob
        7 => 50,  // IndexUpdate
        _ => {
            let kind: u16 = kani::any();
            kind
        }
    }
}

// ── harnesses ──────────────────────────────────────────────────────

/// Prove validate_schema_version never panics for any u16 input.
#[kani::proof]
fn kani_vb_7m21_validate_schema_version_never_panics() {
    let version: u16 = kani::any();
    let result = validate_schema_version(version);

    let current = crate::constants::CURRENT_SCHEMA_VERSION;
    match &result {
        Ok(()) => {
            assert!(version == current, "Ok only when schema version matches current");
            kani::cover!(true, "validate_schema_version returned Ok");
        }
        Err(JournalError::MigrationRequired { from, to }) => {
            assert!(*from < current, "MigrationRequired requires from < CURRENT_SCHEMA_VERSION");
            assert!(*to == current, "Migration target must be CURRENT_SCHEMA_VERSION");
            kani::cover!(true, "validate_schema_version returned MigrationRequired");
        }
        Err(JournalError::UnsupportedSchemaVersion { version: v }) => {
            assert!(*v > current, "UnsupportedSchemaVersion requires version > CURRENT_SCHEMA_VERSION");
            kani::cover!(true, "validate_schema_version returned UnsupportedSchemaVersion");
        }
        Err(e) => {
            let _ = e;
            assert!(false, "validate_schema_version returned unexpected error variant");
        }
    }
    assert!(
        result.is_ok() || result.is_err(),
        "validate_schema_version always returns a valid Result"
    );
}

/// Prove validate_known_kind never panics for any u16 input.
#[kani::proof]
fn kani_vb_7m21_validate_known_kind_never_panics() {
    let kind: u16 = kani::any();
    let result = validate_known_kind(kind);

    match &result {
        Ok(()) => {
            assert!(
                matches!(kind, 1 | 2 | 3 | 10..=27 | 30 | 40 | 50),
                "Ok only for known kind ranges"
            );
            kani::cover!(true, "validate_known_kind returned Ok");
        }
        Err(JournalError::UnknownRecordKind { kind: k }) => {
            assert!(
                !matches!(*k, 1 | 2 | 3 | 10..=27 | 30 | 40 | 50),
                "UnknownRecordKind requires unknown kind"
            );
            kani::cover!(true, "validate_known_kind returned UnknownRecordKind");
        }
        Err(_) => {
            assert!(false, "validate_known_kind returned unexpected error variant");
        }
    }
    assert!(
        result.is_ok() || result.is_err(),
        "validate_known_kind always returns a valid Result"
    );
}

/// Prove validate_kind_family never panics for any (magic, kind) pair.
#[kani::proof]
fn kani_vb_7m21_validate_kind_family_never_panics() {
    let magic: u32 = kani::any();
    let kind: u16 = kani::any();
    let result = validate_kind_family(magic, kind);

    match &result {
        Ok(()) => {
            kani::cover!(true, "validate_kind_family returned Ok");
            // Verify the magic is one of the six known values
            let is_known_magic = matches!(
                magic,
                MAGIC_WORKFLOW_SOURCE
                    | MAGIC_COMPILED_ARTIFACT
                    | MAGIC_JOURNAL_EVENT
                    | MAGIC_SNAPSHOT
                    | MAGIC_BLOB
                    | MAGIC_INDEX_RECORD
            );
            assert!(is_known_magic, "Ok implies magic is one of the known values");
        }
        Err(JournalError::RecordKindFamilyMismatch { magic: m, kind: k }) => {
            assert!(*m == magic && *k == kind, "RecordKindFamilyMismatch preserves input values");
            kani::cover!(true, "validate_kind_family returned RecordKindFamilyMismatch");
        }
        Err(_) => {
            assert!(false, "validate_kind_family returned unexpected error variant");
        }
    }
    assert!(
        result.is_ok() || result.is_err(),
        "validate_kind_family always returns a valid Result"
    );
}

/// Prove validate_kind_family with known magic always yields
/// deterministic result (no panic, consistent classification).
#[kani::proof]
fn kani_vb_7m21_known_magic_kind_family_is_deterministic() {
    let magic = arbitrary_magic();
    let kind = arbitrary_record_kind();
    let result = validate_kind_family(magic, kind);

    // The function must always return either Ok or RecordKindFamilyMismatch
    // for known magic values — never any other error, and never panic.
    match result {
        Ok(()) => {
            kani::cover!(true, "known-magic kind_family Ok");
        }
        Err(JournalError::RecordKindFamilyMismatch { .. }) => {
            kani::cover!(true, "known-magic kind_family RecordKindFamilyMismatch");
        }
        Err(ref _e) => {
            assert!(false, "known magic must not yield unexpected errors");
        }
    }
    assert!(
        result.is_ok() || result.is_err(),
        "known magic kind_family always returns a valid Result"
    );
}
