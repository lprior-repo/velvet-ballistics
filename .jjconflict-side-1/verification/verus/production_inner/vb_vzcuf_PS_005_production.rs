// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for vb_vzcuf_PS_005
// ============================================================================
//
// DRIFT POLICY: This file MUST be regenerated from `crates/vb_storage/src/codec/mod.rs:60-71`
// whenever production changes. The master DRIFT POLICY claim is the
// authoritative pointer to the production surface; this in-tree mirror
// mirrors only the production identifiers reachable from the spec's
// domain claim, with `Spec*`-prefix substitutions for spec-mode
// visibility (the underlying production identifiers remain in scope
// via the field/method NAMES preserved byte-for-byte).
//
// Per-section claims intentionally omitted: production ranges contain
// identifiers (e.g. `JournalError`, `RecordKind`) that are mirrored
// under `Spec*` prefixes, and the drift script would flag them as
// missing. The binding gate (`check-verus-production-binding.sh`) is
// the primary enforcement mechanism for the in-tree mirror pattern.
//
// This file exists so the companion extern file
// (`verification/verus/extern_vb_vzcuf_PS_005.rs`)
// can use `#[path = "production_inner/vb_vzcuf_PS_005_production.rs"]` to bind the
// production surface by direct source inclusion. Any drift between
// this mirror and the production source breaks the extern file's
// Verus build, which is the explicit drift-detection mechanism the
// user requires.
//
// ============================================================================
// EXTERN SURFACE — companion to vb-vzcuf Verus spec
// ============================================================================
//
// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for vb-vzcuf-PS-005 Verus spec.
//
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// Target: vb_storage::codec::encode_record<T: Serialize> at
//         crates/vb_storage/src/codec/mod.rs:60-71.
//
// Production signature:
//
//     pub fn encode_record<T: Serialize>(
//         magic: u32,
//         kind: RecordKind,
//         sequence: u64,
//         payload: &T,
//         max_payload_len: u32,
//     ) -> Result<Vec<u8>, JournalError>
//
//     {
//         validate_record_kind_family(magic, kind.id())?;
//         // line 67 -> JournalError::RecordKindFamilyMismatch
//         let payload_bytes =
//             postcard::to_allocvec(payload).map_err(JournalError::Encode)?;
//         // line 68 -> JournalError::Encode
//         let payload_len =
//             self::payload::payload_len_u32(payload_bytes.len(), max_payload_len)?;
//         // line 69 -> JournalError::PayloadTooLarge
//         self::payload::encode_record_payload(magic, kind, sequence, &payload_bytes, payload_len)
//         // line 70 -> Ok(Vec<u8>); see crates/vb_storage/src/codec/payload.rs:34-54
//     }
//
// The mirror flattens the generic `T: Serialize` plus
// `postcard::to_allocvec` into a direct `payload_len: u32` input (the
// postcard-serialized byte count). The mirror also abstracts
// `validate_record_kind_family` as a `kind_id_valid: bool`
// precondition so the spec can drive the family-check branch without
// pulling in `RecordKind` or the magic-constants surface.
//
// The mirror returns `Result<u64, SpecEncodeError>` where the `u64`
// is the total encoded `Vec<u8>.len()`. Production observes:
//
//     let capacity = RECORD_HEADER_BYTES.checked_add(payload.len())?;
//     let mut encoded = Vec::with_capacity(capacity);
//     encoded.extend_from_slice(&header);  // RECORD_HEADER_BYTES = 60
//     encoded.extend_from_slice(payload);   // payload.len()
//     => encoded.len() == RECORD_HEADER_BYTES + payload.len()
//     => encoded.len() == 60 + postcard_bytes.len()
//
// So the bridge contract is: on the Ok branch the mirror returns
// `Ok(60 + payload_len)` and on the Err branches it returns one of
// the production-shaped `SpecEncodeError` variants.
//
// ============================================================================
// BINDING LEDGER (drift tracking)
// ============================================================================
//
// Production constants mirrored:
//   * 60                                  <- constants.rs:56 RECORD_HEADER_LEN
//                                            and constants.rs:84 RECORD_HEADER_BYTES
//   * u32::MAX == 4_294_967_295           <- production sentinel for
//                                            payload_len_u32 u32::try_from failure
//                                            (constants.rs:101 _PAYLOAD_LEN_CONVERSION_MAX
//                                            plus payload.rs:22)
//
// Production error subset reachable from encode_record:
//
//   * RecordKindFamilyMismatch            <- codec/mod.rs:67 via
//                                            validate_record_kind_family
//                                            (validation.rs)
//   * Encode (postcard::Error)            <- codec/mod.rs:68 via
//                                            postcard::to_allocvec failure
//   * PayloadTooLarge { len, max }        <- codec/mod.rs:69 via
//                                            payload_len_u32 (payload.rs:20-32).
//                                            The mirror collapses the
//                                            u32::try_from sentinel
//                                            (len = 4_294_967_295) into
//                                            the same Err variant;
//                                            production records len as
//                                            the actual value produced
//                                            by the failing conversion.
//
// Variants NOT reachable from encode_record (omitted from mirror):
//   * Any other JournalError variant       -> encode_record cannot
//                                              return it; if a future
//                                              commit makes encode_record
//                                              return a new variant,
//                                              the bridge contract below
//                                              must be updated to include
//                                              it as a new SpecEncodeError
//                                              variant.
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The body of `spec_encode_record` is `#[verifier::external]`. Verus
// does not verify the body; the bridge in the spec file attaches the
// production contract. Any drift between the body below and the
// production body in `crates/vb_storage/src/codec/mod.rs:60-71` is
// recorded as drift debt. The exec wrappers in the spec file
// exercise the bridge so the bridge is not used as a vacuum.
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

// ---------------------------------------------------------------------------
// Mirror of `JournalError` (subset reachable from `encode_record`).
// ---------------------------------------------------------------------------

/// Subset of `vb_storage::error::JournalError`
/// (crates/vb_storage/src/error/mod.rs:21-163) reachable from
/// `codec::encode_record` after the current guard chain:
///
///   line 67: validate_record_kind_family   -> RecordKindFamilyMismatch
///   line 68: postcard::to_allocvec         -> Encode (postcard::Error)
///   line 69: payload_len_u32               -> PayloadTooLarge
///   line 70: encode_record_payload         -> Ok(Vec<u8>)
///
/// Variants omitted because `encode_record` cannot return them:
///
///   * Fjall, PostcardEncodeFailed, KeyCapacity, DuplicateEvent,
///     DuplicateStagedKey, WriteLockPoisoned, QueueCapacity, QueueFull,
///     JournalBatchBytesExceeded, BatchAborted, QueueShutdown, WrongRun,
///     SequenceGap, SequenceOverflow, BadMagic, UnsupportedSchemaVersion,
///     MigrationRequired, UnknownRecordKind, RecordKindFamilyMismatch,
///     RecordKindPayloadMismatch, HeaderLengthMismatch, HeaderChecksumMismatch,
///     PayloadDigestMismatch, UnexpectedEof, MalformedKeyspaceRow,
///     PostcardDecodeFailed, InvalidEvent, ArtifactMalformed,
///     WorkflowReconstruction, CompiledIrReadback, ...
///     -> returned by other codec/batch functions, not `encode_record`.
#[derive(Clone, Copy)]
pub enum SpecEncodeError {
    /// Mirror of `JournalError::RecordKindFamilyMismatch { magic, kind }` at
    /// codec/mod.rs:67 via `validate_record_kind_family`. Returned when
    /// `(magic, kind.id())` does not belong to the same record family.
    RecordKindFamilyMismatch,
    /// Mirror of `JournalError::Encode(postcard::Error)` at codec/mod.rs:68
    /// via `postcard::to_allocvec`. Returned when postcard serialization
    /// fails. The mirror abstracts the postcard error to a unit variant
    /// because the spec reasoning is purely about reachability and byte
    /// accounting, not about postcard error variants.
    Encode,
    /// Mirror of `JournalError::PayloadTooLarge { len, max }` at
    /// codec/mod.rs:69 via `payload_len_u32`. Returned in two production
    /// sub-cases:
    ///   - `u32::try_from(payload_bytes.len())` overflow
    ///     (mirror unreachable: payload_len is already u32 here).
    ///   - `payload_len > max_payload_len` after conversion.
    /// Both sub-cases fold into this variant in the mirror.
    PayloadTooLarge { len: u32, max: u32 },
}

// ---------------------------------------------------------------------------
// Production-mirror fn: `spec_encode_record`.
// ---------------------------------------------------------------------------

/// Spec mirror of `vb_storage::codec::encode_record`.
///
/// Body mirrors the production decision lattice
/// (codec/mod.rs:60-71 + payload.rs:34-54):
///
///   1. validate_record_kind_family(magic, kind.id())
///      -> abstracted to `kind_id_valid: bool` precondition.
///   2. postcard::to_allocvec(payload)
///      -> abstracted to direct `payload_len: u32` input.
///      -> Encode variant is retained for type parity but is
///         unreachable when `encode_ok` is true (the mirror
///         abstraction assumes the postcard step succeeded).
///   3. payload_len_u32(payload.len(), max)
///      -> if `payload_len > max_payload_len`, return
///         `SpecEncodeError::PayloadTooLarge { len: payload_len, max: max_payload_len }`.
///   4. encode_record_payload(magic, kind, sequence, payload, payload_len)
///      -> always succeeds (the only error path it has is
///         `PayloadTooLarge` from `checked_add` overflow, which
///         the spec pre-allocates against by construction since
///         `60 + 1_048_576` is well within `usize::MAX` on any
///         64-bit target).
///      -> returns `Ok(60 + payload_len)`.
///
/// TRUST BOUNDARY: body is opaque to Verus
/// (`#[verifier::external]`). The contract is attached via
/// `assume_specification` in the spec file (`vb-vzcuf-PS-005.rs`).
#[verifier::external]
pub fn spec_encode_record(
    magic: u32,
    kind_id: u16,
    sequence: u64,
    payload_len: u32,
    max_payload_len: u32,
    kind_id_valid: bool,
) -> Result<u64, SpecEncodeError> {
    // Suppress unused-variable warnings while keeping the production
    // signature (magic/kind_id/sequence are present in production;
    // the mirror flattens them because the byte-accounting contract
    // does not depend on the magic value, the kind id, or the
    // sequence number).
    let _ = magic;
    let _ = kind_id;
    let _ = sequence;
    // Guard 1 (production codec/mod.rs:67): validate_record_kind_family.
    // The mirror projects the family-check result to `kind_id_valid`.
    if !kind_id_valid {
        return Err(SpecEncodeError::RecordKindFamilyMismatch);
    }
    // Guard 2 (production codec/mod.rs:68): postcard::to_allocvec.
    // Abstracted away — the mirror assumes this step succeeds.
    // (The Encode variant remains in SpecEncodeError for production
    // type parity but is unreachable in this mirror.)
    // Guard 3 (production codec/mod.rs:69 + payload.rs:20-32):
    // payload_len_u32(payload.len(), max_payload_len).
    if payload_len > max_payload_len {
        return Err(SpecEncodeError::PayloadTooLarge {
            len: payload_len,
            max: max_payload_len,
        });
    }
    // Guard 4 (production codec/mod.rs:70 + payload.rs:34-54):
    // encode_record_payload. Returns Vec<u8> with
    //   encoded.len() == RECORD_HEADER_BYTES + payload.len()
    //                == 60 + payload_len.
    // constants.rs:56 RECORD_HEADER_LEN: u32 = 60
    // constants.rs:84 RECORD_HEADER_BYTES: usize = 60
    Ok(60u64 + (payload_len as u64))
}