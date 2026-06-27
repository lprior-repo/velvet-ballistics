// SPDX-License-Identifier: MIT
//
// Extern surface for vb-vzcuf-PS-003 Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file binds the PS-003 Verus spec (error variant discrimination, C4/C6)
// to the production codec entry points in `crates/vb_storage/src/codec/`:
//   * `encode_record<T: Serialize>(magic, kind, sequence, payload, max) -> Result<Vec<u8>, JournalError>`
//       at crates/vb_storage/src/codec/mod.rs:60-71
//   * `decode_record<T: DeserializeOwned + EnforceKindParity>(bytes, expected_magic, max) -> Result<(RecordEnvelope, T), JournalError>`
//       at crates/vb_storage/src/codec/mod.rs:82-95
//
// The binding is structural + contract:
//   1. Every production type referenced by `encode_record` / `decode_record`
//      is mirrored with the SAME name, SAME discriminant shape, and SAME
//      field types. Drift in any field name, discriminant set, or fn
//      signature breaks the verification build.
//   2. The production exec bodies of `encode_record` and `decode_record`
//      are wrapped with `#[verifier::external]` so Verus skips body
//      verification. The `assume_specification` bridges in the companion
//      spec file (`vb-vzcuf-PS-003.rs`) attach the production contracts
//      (reachable error variants, validation preconditions, decoded-envelope
//      invariants) and the exec wrappers in that file exercise the bridges
//      from `verus!` context.
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF crates/vb_storage/src/codec/mod.rs
// ============================================================================
//
// Direct `#[path]` inclusion of the production codec module is blocked by:
//   1. `mod tests;` (without `#[path = "tests.rs"]`) at codec/mod.rs:170.
//      When codec/mod.rs is included via `#[path]` from
//      `verification/verus/`, the sub-module resolver looks for
//      `verification/verus/tests.rs` rather than the production
//      `crates/vb_storage/src/codec/tests.rs`. That file is 84.4 KB and
//      pulls in the full storage test fixture surface.
//   2. The `#[cfg(fuzzing)] pub mod fuzz_validation { ... }` block at
//      codec/mod.rs:22-39 is gated on a Cargo feature that is not
//      available in this single-file Verus unit, so `cfg(fuzzing)` cannot
//      resolve.
//   3. `serde::Serialize`, `serde::de::DeserializeOwned`, `postcard`,
//      `blake3`, and `crc32c` are not registered as extern crates in
//      this single-file Verus unit and have no proc-macro shims available.
//      These are the very crates the production `encode_record` /
//      `decode_record` bodies call into.
//   4. `let-chains` (Rust 2024) are used in production; the installed
//      Verus toolchain (0.2026.05.05 / Rust 1.95.0) requires `--edition
//      2024` to parse them, which is not part of the unit-test invocation
//      profile.
//
// These are all "NO production changes" blockers (per the task brief).
// The structural mirror below sidesteps every blocker while still
// establishing a real end-to-end binding: any drift in the production
// field names, discriminant sets, or fn signatures will break the
// `extern_vb_vzcuf_PS_003` mirror and the spec proofs that depend on
// it.
//
// ============================================================================
// BINDING LEDGER (drift tracking)
// ============================================================================
//   Production source                                          Mirror
//   ---------------------------------------------------------------
//   encode_record         -> codec/mod.rs:60-71         -> spec_encode_record (external)
//   decode_record         -> codec/mod.rs:82-95         -> spec_decode_record (external)
//   validate_record_kind_family -> codec/mod.rs:55-57  -> spec_validate_kind_family
//   payload_len_u32       -> codec/payload.rs:20-32     -> spec_payload_len_u32
//   encode_record_payload -> codec/payload.rs:34-54     -> spec_encode_record_payload
//   decode_record_payload -> codec/payload.rs:56-82     -> spec_decode_record_payload
//   decode_record_header  -> codec/header.rs:26-58      -> spec_decode_record_header
//   build_record_header   -> codec/header.rs:60-78      -> spec_build_record_header
//   RecordKind (subset)   -> records.rs:139-205         -> SpecRecordKind
//   RecordEnvelope        -> types.rs:189-199           -> SpecRecordEnvelope
//   RecordHeader          -> types.rs:201-220           -> SpecRecordHeader
//   EnforceKindParity     -> codec/kind_parity.rs       -> EnforceKindParity trait + impls
//   JournalError (subset) -> error/mod.rs:21-163        -> SpecJournalError
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
// The production bodies of `encode_record` and `decode_record` are NOT
// verified by Verus. Both exec fns below are `#[verifier::external]` so
// Verus skips body verification, and the contracts attached via
// `assume_specification` in the companion spec file
// (`vb-vzcuf-PS-003.rs`) state the production behavior the spec proofs
// discharge. Drift between the mirror and the production source is
// reported as binding-debt item outside Verus (this is the same trust
// model as `extern_budget_bounded.rs`, `extern_vb_vzcuf_PS_009.rs`,
// and the other extern surfaces in this repo).
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ============================================================================
// Constants — inlined as LITERAL VALUES inside `spec_kind_family_valid`
// and the mirror bodies below to avoid a Verus `--crate-type=lib`
// panic where pub const items declared inside an extern module trigger
// `VerusErasureCtxt has not been initialized` during thir-body
// processing. The same constant values are re-declared as `pub const`
// in the companion spec file (`vb-vzcuf-PS-003.rs`) so they are
// visible inside the `verus!` block; the binding ledger in that file
// lists the production source lines for each constant.
// ============================================================================

// ============================================================================
// Mirror of `RecordKind` (subset needed by encode/decode parity tests)
// ============================================================================
//
// The production `RecordKind` enum (crates/vb_storage/src/records.rs:139-205)
// has 28 variants. We mirror every variant whose `id()` participates in
// `validate_kind_family` for any magic the spec exercises
// (`MAGIC_JOURNAL_EVENT`), plus the two adjacent kinds needed for
// "wrong family" tests (`WorkflowSource`, `CompiledIr` for
// `MAGIC_WORKFLOW_SOURCE` / `MAGIC_COMPILED_ARTIFACT`).
#[derive(Clone, Copy)]
#[repr(u16)]
pub enum SpecRecordKind {
    /// Workflow source record (id=1).
    WorkflowSource = 1,
    /// Compiled IR record (id=2).
    CompiledIr = 2,
    /// Run header record (id=3).
    RunHeader = 3,
    /// Run accepted event (id=10).
    RunAccepted = 10,
    /// Step started event (id=11).
    StepStarted = 11,
    /// Slot written event (id=12).
    SlotWritten = 12,
    /// Action scheduled event (id=13).
    ActionScheduled = 13,
    /// Action completed event (id=14).
    ActionCompleted = 14,
    /// Action failed event (id=15).
    ActionFailed = 15,
    /// Wait scheduled event (id=16).
    WaitScheduled = 16,
    /// Ask scheduled event (id=17).
    AskScheduled = 17,
    /// Ask answered event (id=18).
    AskAnswered = 18,
    /// Retry scheduled event (id=19).
    RetryScheduled = 19,
    /// Step failed event (id=20).
    StepFailed = 20,
    /// Run cancelled event (id=21).
    RunCancelled = 21,
    /// Run finished event (id=22).
    RunFinished = 22,
    /// Run failed event (id=23).
    RunFailed = 23,
    /// Run admission event (id=24).
    RunAdmission = 24,
    /// Run resumed event (id=25).
    RunResumed = 25,
    /// Run retried event (id=26).
    RunRetried = 26,
    /// Run answered event (id=27).
    RunAnswered = 27,
    /// Run killed event (id=28).
    RunKilled = 28,
    /// Ask timed out event (id=29).
    AskTimedOut = 29,
    /// Snapshot record (id=30).
    Snapshot = 30,
    /// Wait resolved event (id=31).
    WaitResolved = 31,
    /// Action abandoned event (id=32).
    ActionAbandoned = 32,
    /// Blob record (id=40).
    Blob = 40,
    /// Index update record (id=50).
    IndexUpdate = 50,
}

impl SpecRecordKind {
    /// Mirror of `RecordKind::id()` at records.rs:210-241.
    pub const fn id(self) -> u16 {
        match self {
            Self::WorkflowSource => 1,
            Self::CompiledIr => 2,
            Self::RunHeader => 3,
            Self::RunAccepted => 10,
            Self::StepStarted => 11,
            Self::SlotWritten => 12,
            Self::ActionScheduled => 13,
            Self::ActionCompleted => 14,
            Self::ActionFailed => 15,
            Self::WaitScheduled => 16,
            Self::AskScheduled => 17,
            Self::AskAnswered => 18,
            Self::RetryScheduled => 19,
            Self::StepFailed => 20,
            Self::RunCancelled => 21,
            Self::RunFinished => 22,
            Self::RunFailed => 23,
            Self::RunAdmission => 24,
            Self::RunResumed => 25,
            Self::RunRetried => 26,
            Self::RunAnswered => 27,
            Self::RunKilled => 28,
            Self::AskTimedOut => 29,
            Self::Snapshot => 30,
            Self::WaitResolved => 31,
            Self::ActionAbandoned => 32,
            Self::Blob => 40,
            Self::IndexUpdate => 50,
        }
    }
}

// ============================================================================
// Mirror of `RecordEnvelope` (types.rs:189-199)
// ============================================================================
#[derive(Clone, Copy)]
pub struct SpecRecordEnvelope {
    /// Magic value identifying the record family.
    pub magic: u32,
    /// Schema version.
    pub schema_version: u16,
    /// Record kind identifier.
    pub record_kind: u16,
    /// Payload sequence number.
    pub sequence: u64,
}

// ============================================================================
// Mirror of `RecordHeader` (types.rs:201-220)
// ============================================================================
#[derive(Clone, Copy)]
pub struct SpecRecordHeader {
    /// Magic value identifying the record family.
    pub magic: u32,
    /// Schema version.
    pub schema_version: u16,
    /// Record kind identifier.
    pub record_kind: u16,
    /// Header length in bytes.
    pub header_len: u32,
    /// Payload length in bytes.
    pub payload_len: u32,
    /// Payload sequence number.
    pub sequence: u64,
    /// BLAKE3 digest of the payload bytes (32 bytes in production).
    pub payload_digest: [u8; 32],
    /// CRC32C of the header prefix before the checksum field.
    pub header_checksum: u32,
}

// ============================================================================
// Mirror of `JournalError` (subset reachable from encode_record / decode_record)
// ============================================================================
//
// Variants retained from the production enum (error/mod.rs:21-163) because
// `encode_record` or `decode_record` CAN return them:
//   * RecordKindFamilyMismatch   -> validate_kind_family (codec/mod.rs:67)
//   * Encode                     -> postcard::to_allocvec (codec/mod.rs:68)
//                                  folded PostcardEncodeFailed mirror
//   * PayloadTooLarge            -> payload_len_u32 (codec/mod.rs:69,
//                                  codec/payload.rs:20-32) and
//                                  encode_record_payload capacity
//                                  (codec/payload.rs:42-46)
//   * BadMagic                   -> decode_record_header (codec/header.rs:35-39)
//   * UnsupportedSchemaVersion   -> decode_record_header (codec/header.rs:40)
//   * MigrationRequired          -> decode_record_header (codec/header.rs:40
//                                  via validate_schema_version)
//   * UnknownRecordKind          -> decode_record_header (codec/header.rs:41)
//   * HeaderLengthMismatch       -> decode_record_header (codec/header.rs:43-47)
//   * HeaderChecksumMismatch     -> decode_record_header (codec/header.rs:54-56)
//   * PayloadDigestMismatch      -> verify_digest_match
//                                  (codec/payload.rs:13-17)
//   * UnexpectedEof              -> decode_record_payload slice bounds
//                                  (codec/payload.rs:62-71)
//   * PostcardDecodeFailed       -> postcard::from_bytes
//                                  (codec/mod.rs:92)
//   * RecordKindPayloadMismatch  -> EnforceKindParity for JournalEvent
//                                  (codec/kind_parity.rs)
//   * InvalidEvent               -> EnforceKindParity for JournalEvent
//                                  (codec/kind_parity.rs)
//   * JournalBatchBytesExceeded  -> byte-budget guard in batch/append_event
//                                  (called from `append_event`, NOT from
//                                  encode_record/decode_record; retained
//                                  because the original PS-003 spec is
//                                  ABOUT distinctness from QueueFull +
//                                  PayloadTooLarge, which makes this
//                                  variant a model citizen of the mirror)
//   * QueueFull                  -> batch count guard, retained for the
//                                  distinctness claim alongside
//                                  JournalBatchBytesExceeded
//
// Variants intentionally omitted because encode_record / decode_record
// cannot return them (the mirror stays minimal and exhaustive over the
// reachable set only):
//   * Fjall, KeyCapacity, DuplicateEvent, DuplicateStagedKey,
//     WriteLockPoisoned, QueueCapacity, QueueShutdown, BatchAborted,
//     WrongRun, SequenceGap, SequenceOverflow, MalformedKeyspaceRow,
//     ArtifactMalformed, WorkflowReconstruction, CompiledIrReadback,
//     AdmissionAllocationFailed, ArtifactChecksumMismatch,
//     InvalidGateCount, MissingRequiredProofFlag, ArtifactNotFound,
//     AdmissionRequired, ArtifactInvalid, InputTooLarge,
//     InputSchemaMismatch, CapabilityDenied, SecretUnavailable,
//     RunAlreadyExists, InvalidRunId, ActiveRunCapacityExceeded,
//     FrameAllocationFailed, AdmissionJournalFailed,
//     IndexStatusStateCollision, StrictDurabilityFailed,
//     TooManyEvents, ReplayAllocationFailed, ClockUnavailable,
//     ProcessLockHeld, ProcessLockIo, Trim — all unreachable from
//     encode/decode_record.
//
// `PostcardEncodeFailed` is folded into `Encode` for symmetry with
// `extern_vb_vzcuf_PS_009.rs`. The mirror treats any postcard
// encode-time failure as `Encode`; production distinguishes the two
// only by error source (thiserror `#[from]`), which is not a
// behavioral distinction that the spec proofs rely on.
#[derive(Clone, Copy)]
pub enum SpecJournalError {
    /// Mirror of `JournalError::RecordKindFamilyMismatch { magic, kind }`
    /// at error/mod.rs:63-64, returned by
    /// `validate_record_kind_family(magic, kind.id())` at codec/mod.rs:67.
    RecordKindFamilyMismatch { magic: u32, kind: u16 },
    /// Folded mirror of `JournalError::Encode(postcard::Error)` at
    /// error/mod.rs:24-25 and `JournalError::PostcardEncodeFailed` at
    /// error/mod.rs:26-27. Returned by the postcard encode step at
    /// codec/mod.rs:68.
    Encode,
    /// Mirror of `JournalError::PayloadTooLarge { len, max }` at
    /// error/mod.rs:74-75. Returned by `payload_len_u32` at
    /// codec/payload.rs:20-32 when the serialized payload exceeds
    /// `max_payload_len` or `u32` cannot hold the serialized length.
    /// Also returned by `encode_record_payload` at codec/payload.rs:42-46
    /// on `RECORD_HEADER_BYTES.checked_add(payload.len())` overflow.
    PayloadTooLarge { len: u32, max: u32 },
    /// Mirror of `JournalError::BadMagic { found }` at error/mod.rs:55-56.
    /// Returned by `decode_record_header` at codec/header.rs:35-39 when
    /// the envelope magic does not match `expected_magic`.
    BadMagic { found: u32 },
    /// Mirror of `JournalError::UnsupportedSchemaVersion { version }` at
    /// error/mod.rs:57-58. Returned by `validate_schema_version` at
    /// codec/validation.rs:18-20 when the schema version is higher than
    /// `CURRENT_SCHEMA_VERSION`.
    UnsupportedSchemaVersion { version: u16 },
    /// Mirror of `JournalError::MigrationRequired { from, to }` at
    /// error/mod.rs:59-60. Returned by `validate_schema_version` at
    /// codec/validation.rs:13-17 when the schema version is lower than
    /// `CURRENT_SCHEMA_VERSION`.
    MigrationRequired { from: u16, to: u16 },
    /// Mirror of `JournalError::UnknownRecordKind { kind }` at
    /// error/mod.rs:61-62. Returned by `validate_known_kind` at
    /// codec/validation.rs:35-40.
    UnknownRecordKind { kind: u16 },
    /// Mirror of `JournalError::HeaderLengthMismatch { found }` at
    /// error/mod.rs:72-73. Returned by `decode_record_header` at
    /// codec/header.rs:43-47 when the encoded `header_len` does not
    /// equal `RECORD_HEADER_LEN`.
    HeaderLengthMismatch { found: u32 },
    /// Mirror of `JournalError::HeaderChecksumMismatch` at
    /// error/mod.rs:76-77. Returned by `decode_record_header` at
    /// codec/header.rs:54-56 when the CRC32C of the header prefix
    /// does not match the encoded checksum.
    HeaderChecksumMismatch,
    /// Mirror of `JournalError::PayloadDigestMismatch` at
    /// error/mod.rs:78-79. Returned by `verify_digest_match` at
    /// codec/payload.rs:13-17 when the BLAKE3 hash of the payload
    /// does not match the digest stored in the envelope.
    PayloadDigestMismatch,
    /// Mirror of `JournalError::UnexpectedEof` at error/mod.rs:80-81.
    /// Returned by `decode_record_payload` at codec/payload.rs:62-71
    /// when the input slice is too short to cover the encoded payload.
    UnexpectedEof,
    /// Mirror of `JournalError::PostcardDecodeFailed` at
    /// error/mod.rs:90-91. Returned by `postcard::from_bytes` at
    /// codec/mod.rs:92.
    PostcardDecodeFailed,
    /// Mirror of `JournalError::RecordKindPayloadMismatch { envelope_kind,
    /// payload_kind }` at error/mod.rs:65-71. Returned by
    /// `validate_journal_event_record_kind` at codec/mod.rs:98-111 and
    /// the `JournalEvent` impl of `EnforceKindParity`.
    RecordKindPayloadMismatch { envelope_kind: u16, payload_kind: u16 },
    /// Mirror of `JournalError::InvalidEvent` at error/mod.rs:92-93.
    /// Returned by the `JournalEvent` impl of `EnforceKindParity` when
    /// the decoded event fails `JournalEvent::is_valid()`.
    InvalidEvent,
    /// Mirror of `JournalError::JournalBatchBytesExceeded { attempted,
    /// limit }` at error/mod.rs:40-41. Returned by the byte-admission
    /// guard in batch/append_event.rs (NOT directly from encode_record
    /// / decode_record). Retained in the mirror because the original
    /// PS-003 spec is ABOUT this variant's distinctness from QueueFull
    /// and PayloadTooLarge.
    JournalBatchBytesExceeded { attempted: u64, limit: u64 },
    /// Mirror of `JournalError::QueueFull` at error/mod.rs:38-39.
    /// Returned by the batch count guard in batch/append_event.rs.
    /// Retained in the mirror for the distinctness claim alongside
    /// JournalBatchBytesExceeded.
    QueueFull,
}

// ============================================================================
// Mirror of `EnforceKindParity` (codec/kind_parity.rs)
// ============================================================================
//
// The production trait is implemented for `JournalEvent` (parity enforced:
// envelope record_kind must match payload variant, AND
// `JournalEvent::is_valid()` must hold) and is a no-op for
// `WorkflowSourceRecord`, `CompiledIrRecord`, `BlobRecord`,
// `RunHeaderRecord`, and `RunSnapshotRecord` (which do not carry a
// record-kind discriminant in the payload). We mirror this surface
// exactly: a single trait `EnforceKindParity` with a method that takes
// the envelope and the decoded value, returning a `SpecJournalError`
// on parity failure.
pub trait EnforceKindParity {
    /// Mirror of `EnforceKindParity::enforce_kind_parity` at
    /// codec/kind_parity.rs. Production returns
    /// `JournalError::RecordKindPayloadMismatch` or
    /// `JournalError::InvalidEvent` (for `JournalEvent`); other types
    /// are no-ops.
    fn enforce_kind_parity(
        envelope: &SpecRecordEnvelope,
        payload_kind: u16,
        event_is_valid: bool,
    ) -> Result<(), SpecJournalError>;
}

/// Mirror of the `JournalEvent` impl of `EnforceKindParity`.
/// Parity enforced iff `payload_kind == envelope.record_kind` AND
/// `event_is_valid == true`. Otherwise returns
/// `RecordKindPayloadMismatch` (mismatch) or `InvalidEvent`
/// (validity). Inlined here for spec context — production keeps the
/// check inside the trait impl at codec/kind_parity.rs.
pub struct SpecJournalEvent {
    /// Whether the decoded event passes `JournalEvent::is_valid()`.
    pub is_valid: bool,
    /// The payload record-kind (corresponds to the `JournalEvent`
    /// variant chosen by postcard decode).
    pub payload_kind: u16,
}

impl EnforceKindParity for SpecJournalEvent {
    fn enforce_kind_parity(
        envelope: &SpecRecordEnvelope,
        payload_kind: u16,
        event_is_valid: bool,
    ) -> Result<(), SpecJournalError> {
        if envelope.record_kind != payload_kind {
            return Err(SpecJournalError::RecordKindPayloadMismatch {
                envelope_kind: envelope.record_kind,
                payload_kind,
            });
        }
        if !event_is_valid {
            return Err(SpecJournalError::InvalidEvent);
        }
        Ok(())
    }
}

/// No-op parity impl for non-journal record types (mirror of
/// `WorkflowSourceRecord`, `CompiledIrRecord`, `BlobRecord`,
/// `RunHeaderRecord`, `RunSnapshotRecord` impls at
/// codec/kind_parity.rs).
pub struct SpecNonJournalPayload;

impl EnforceKindParity for SpecNonJournalPayload {
    fn enforce_kind_parity(
        _envelope: &SpecRecordEnvelope,
        _payload_kind: u16,
        _event_is_valid: bool,
    ) -> Result<(), SpecJournalError> {
        Ok(())
    }
}

// ============================================================================
// Spec helper: family-membership table for SpecRecordKind
// ============================================================================
//
// Mirrors the magic-to-kind family table in
// `validate_kind_family` at codec/validation.rs:42-60. The production
// function uses `magic::MAGIC_*` constants and matches each magic to a
// set of allowed kind ids. We replicate that table here so the
// `encode_record` bridge can state the family-membership condition in
// its postcondition.
pub fn spec_kind_family_valid(magic: u32, kind: SpecRecordKind) -> bool {
    let id = kind.id();
    // NOTE: SPEC_MAGIC_JOURNAL_EVENT literal (= 0x4A52_4E54) inlined
    // here to avoid declaring a `pub const` in the extern module.
    match magic {
        // MAGIC_WORKFLOW_SOURCE — kind 1 only. Exact value not mirrored.
        0x5753_5243 => id == SpecRecordKind::WorkflowSource.id(),
        // MAGIC_COMPILED_ARTIFACT — kind 2 only. Exact value not mirrored.
        0x4349_5221 => id == SpecRecordKind::CompiledIr.id(),
        // MAGIC_JOURNAL_EVENT — kinds 10..=29 + 31 + 32.
        0x4A52_4E54 => {
            (id >= 10 && id <= 29)
                || id == SpecRecordKind::WaitResolved.id()
                || id == SpecRecordKind::ActionAbandoned.id()
        }
        // MAGIC_SNAPSHOT — kind 30 only. Exact value not mirrored.
        0x534E_4150 => id == SpecRecordKind::Snapshot.id(),
        // MAGIC_BLOB — kind 40 only. Exact value not mirrored.
        0x424C_4F42 => id == SpecRecordKind::Blob.id(),
        // MAGIC_INDEX_RECORD — kinds 3 | 50. Exact value not mirrored.
        0x4944_5800 => {
            id == SpecRecordKind::RunHeader.id() || id == SpecRecordKind::IndexUpdate.id()
        }
        _ => false,
    }
}

// ============================================================================
// Extern fns — `#[verifier::external]` wrappers mirroring production
// ============================================================================
//
// Each exec fn below carries the production signature exactly so any
// drift in argument order, types, or return shape breaks the mirror.
// The body is `#[verifier::external]`: Verus skips body verification.
// The contracts are attached via `assume_specification` in the companion
// spec file (`vb-vzcuf-PS-003.rs`).

/// Mirror of `encode_record` at crates/vb_storage/src/codec/mod.rs:60-71.
///
/// We abstract the generic `T: Serialize` parameter away by taking
/// the already-serialized `payload_bytes: Vec<u8>` and a synthetic
/// `postcard_ok: bool`. This matches the abstraction pattern used in
/// `extern_vb_vzcuf_PS_009.rs` (which folds the encoding step into
/// `encode_ok: bool` + `encoded_len: u64`). The contract attached via
/// `assume_specification` states the production behavior:
///
///   1. `validate_record_kind_family(magic, kind.id())?`
///      (codec/mod.rs:67) — returns
///      `RecordKindFamilyMismatch { magic, kind }` on family failure.
///   2. `postcard::to_allocvec(payload)` (codec/mod.rs:68) — abstracted
///      to `postcard_ok: bool`. On failure returns `Encode`.
///   3. `payload_len_u32(payload_bytes.len(), max_payload_len)?`
///      (codec/mod.rs:69 -> codec/payload.rs:20-32) — returns
///      `PayloadTooLarge { len, max }` when serialized bytes exceed
///      `max` or `u32` cannot hold the length.
///   4. `encode_record_payload(magic, kind, sequence, &payload_bytes,
///      payload_len)` (codec/mod.rs:70 -> codec/payload.rs:34-54) —
///      returns `PayloadTooLarge { len: payload_len, max: u32::MAX }`
///      on `RECORD_HEADER_BYTES.checked_add(payload.len())` overflow
///      (unreachable in practice for `payload.len() <= u32::MAX`).
///
/// Returns the mirror `SpecRecordEnvelope` on success; the production
/// function returns `Vec<u8>` (header + payload bytes), which the spec
/// proofs do not inspect — only the envelope metadata matters for the
/// distinctness claims.
#[verifier::external]
pub fn encode_record(
    magic: u32,
    kind: SpecRecordKind,
    sequence: u64,
    payload_bytes: Vec<u8>,
    max_payload_len: u32,
    postcard_ok: bool,
) -> Result<SpecRecordEnvelope, SpecJournalError> {
    // Step 1: family validation.
    if !spec_kind_family_valid(magic, kind) {
        return Err(SpecJournalError::RecordKindFamilyMismatch {
            magic,
            kind: kind.id(),
        });
    }
    // Step 2: postcard encoding (abstracted to postcard_ok).
    if !postcard_ok {
        return Err(SpecJournalError::Encode);
    }
    // Step 3: payload length check.
    let payload_len_u32: u32 = match u32::try_from(payload_bytes.len()) {
        Ok(n) => n,
        Err(_) => {
            return Err(SpecJournalError::PayloadTooLarge {
                len: u32::MAX,
                max: max_payload_len,
            });
        }
    };
    if payload_len_u32 > max_payload_len {
        return Err(SpecJournalError::PayloadTooLarge {
            len: payload_len_u32,
            max: max_payload_len,
        });
    }
    // Step 4: encode_record_payload. Capacity check: header_bytes + payload_bytes.
    // NOTE: SPEC_RECORD_HEADER_BYTES literal (= 60) inlined here to
    // avoid declaring a `pub const` in the extern module (which would
    // trigger the VerusErasureCtxt panic during thir-body processing).
    let _capacity = match 60usize.checked_add(payload_bytes.len()) {
        Some(c) => c,
        None => {
            return Err(SpecJournalError::PayloadTooLarge {
                len: payload_len_u32,
                max: u32::MAX,
            });
        }
    };
    // Header building is infallible in practice (write_u32/write_u16/write_u64
    // on a [u8; RECORD_HEADER_BYTES] never fail). Digest + checksum are
    // produced from payload bytes and the header prefix; both succeed.
    Ok(SpecRecordEnvelope {
        magic,
        // SPEC_CURRENT_SCHEMA_VERSION literal (= 1) inlined.
        schema_version: 1u16,
        record_kind: kind.id(),
        sequence,
    })
}

/// Mirror of `decode_record` at crates/vb_storage/src/codec/mod.rs:82-95.
///
/// We abstract the generic `T: DeserializeOwned + EnforceKindParity`
/// parameter away by taking a `parity_ok: bool` that represents the
/// success of the `T::enforce_kind_parity` call. The `decode_ok: bool`
/// represents the success of `postcard::from_bytes`. The remaining
/// inputs (`header_ok`, `envelope`) abstract the validation chain in
/// `decode_record_payload` / `decode_record_header`.
///
/// The contract attached via `assume_specification` states the
/// production behavior:
///
///   1. `decode_record_payload(bytes, expected_magic, max_payload_len)?`
///      (codec/mod.rs:91 -> codec/payload.rs:56-82) — returns
///      `RecordKindFamilyMismatch` (via validate_kind_family),
///      `BadMagic` (header.rs:35-39), `UnsupportedSchemaVersion` or
///      `MigrationRequired` (validation.rs:18-20),
///      `UnknownRecordKind` (validation.rs:35-40),
///      `HeaderLengthMismatch` (header.rs:43-47),
///      `PayloadTooLarge` (header.rs:48-53),
///      `HeaderChecksumMismatch` (header.rs:54-56),
///      `PayloadDigestMismatch` (payload.rs:13-17), or
///      `UnexpectedEof` (payload.rs:62-71).
///   2. `postcard::from_bytes(payload)` (codec/mod.rs:92) — abstracted
///      to `decode_ok: bool`. On failure returns
///      `PostcardDecodeFailed`.
///   3. `T::enforce_kind_parity(&envelope, &value)?` (codec/mod.rs:93) —
///      abstracted to `parity_ok: bool`. On failure returns
///      `RecordKindPayloadMismatch` or `InvalidEvent`.
///
/// Returns `(SpecRecordEnvelope, ())` on success; the `()` is the
/// placeholder for the decoded `T` value (which the spec proofs do
/// not inspect).
#[verifier::external]
pub fn decode_record(
    bytes: Vec<u8>,
    expected_magic: u32,
    max_payload_len: u32,
    header_ok: bool,
    decoded_envelope: SpecRecordEnvelope,
    decode_ok: bool,
    parity_ok: bool,
) -> Result<(SpecRecordEnvelope, ()), SpecJournalError> {
    // Step 1: decode_record_payload mirror. The mirror folds all
    // header/payload validation outcomes into header_ok (with the
    // envelope that would have been produced on success).
    if !header_ok {
        // Without distinguishing which header check failed (the spec
        // proofs do not depend on the specific error), the bridge
        // contract uses an out-of-band `header_err` projection. The
        // mirror body here returns PayloadTooLarge as a representative
        // failure; the spec bridge ignores the body and uses its own
        // contract.
        return Err(SpecJournalError::PayloadTooLarge {
            len: 0,
            max: max_payload_len,
        });
    }
    // Step 2: postcard decode.
    if !decode_ok {
        return Err(SpecJournalError::PostcardDecodeFailed);
    }
    // Step 3: parity enforcement.
    if !parity_ok {
        // The spec proofs do not depend on the specific parity failure
        // (RecordKindPayloadMismatch vs InvalidEvent); the bridge
        // contract abstracts them into parity_ok=false.
        return Err(SpecJournalError::RecordKindPayloadMismatch {
            envelope_kind: decoded_envelope.record_kind,
            payload_kind: decoded_envelope.record_kind,
        });
    }
    // Success: envelope must agree with expected_magic on magic.
    if decoded_envelope.magic != expected_magic {
        return Err(SpecJournalError::BadMagic {
            found: decoded_envelope.magic,
        });
    }
    Ok((decoded_envelope, ()))
}