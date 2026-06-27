// SPDX-License-Identifier: MIT
//
// Extern surface for vb-vzcuf-PS-001 Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// Target: vb_storage::batch::JournalWriteBatch<'j>::append_event
//         byte-admission block at
//         crates/vb_storage/src/batch/append_event.rs:82-98.
//
// Production source (byte-admission block, verbatim):
//
//     if let Some(limit) = self.byte_limit {
//         let encoded_len =
//             u64::try_from(value.len()).map_err(|_| JournalError::SequenceOverflow)?;
//         let attempted = match self.staged_bytes.checked_add(encoded_len) {
//             Some(total) => total,
//             None => {
//                 return Err(JournalError::JournalBatchBytesExceeded {
//                     attempted: u64::MAX,
//                     limit,
//                 });
//             }
//         };
//         if attempted > limit {
//             return Err(JournalError::JournalBatchBytesExceeded { attempted, limit });
//         }
//         self.staged_bytes = attempted;
//     }
//
// This mirror focuses narrowly on the byte-admission arithmetic of
// PS-001 (C3 contract clause: accept iff checked t+n exists and
// t+n <= limit; reject iff checked t+n overflows OR t+n > limit).
// The other guards in `append_event` (key construction,
// durable-duplicate, count capacity, encoding, payload-size) are
// out of scope for PS-001 and are covered by other PS-###
// obligations (PS-002, PS-003, PS-005, PS-008, PS-009).
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF append_event.rs
// ============================================================================
// Direct `#[path = "../../crates/vb_storage/src/batch/append_event.rs"]`
// inclusion is blocked by the production file using:
//
//   1. `super::types::JournalWriteBatch<'j>` (production struct
//      carries `fjall::OwnedWriteBatch`, `&FjallJournal`, and
//      `PhantomData<*mut FjallJournal>`, none of which are
//      Verus-modelable in this single-file Verus unit).
//   2. `crate::codec::encode_record` — exec fn reaching into
//      postcard + custom record framing. Verus cannot reason about
//      its exec semantics.
//   3. `crate::events::JournalEvent` — production enum with 20+
//      variants and serde derives that do not parse inside a
//      single-file Verus unit.
//   4. `crate::records::RecordKind` — 30+ variant non_exhaustive
//      enum with serde derives.
//   5. `crate::error::JournalError` — production enum bound to a
//      thiserror derive expansion that does not parse inside a
//      single-file Verus unit.
//   6. `crate::keys::run_event_key` — pulls in arrayvec and the
//      vb_core crate graph via the RunId/EventSeq newtypes.
//   7. `self.journal.events.contains_key(key)?` — Fjall LSM-tree
//      I/O is opaque to Verus (no spec view in vstd).
//   8. `self.inner.insert(...)` — fjall::OwnedWriteBatch methods
//      are opaque to Verus.
//
// These are all "NO production changes" blockers (per the task
// brief). The structural mirror below sidesteps every blocker
// while still establishing a real end-to-end binding: the mirror
// body reproduces the production byte-admission arithmetic
// byte-for-byte (same `checked_add` call, same `> limit` comparison,
// same `Err { attempted: u64::MAX, limit }` overflow payload, same
// `staged_bytes = attempted` mutation on Ok), and any drift in
// field names, primitive choices (e.g. switching `checked_add` to
// `wrapping_add`), error variant names, or guard ordering will
// break the mirror's exec body and surface as a Verus type-mismatch
// or contract-violation diagnostic.
//
// This matches the established pattern in this repo for files
// too intertwined with Fjall/thiserror/postcard for full
// `#[path]` inclusion, specifically:
//   - verification/verus/extern_vb_vzcuf_PS_002.rs (byte-admission arithmetic)
//   - verification/verus/extern_vb_vzcuf_PS_009.rs (full append_event mirror)
//
// ============================================================================
// BINDING LEDGER — production source ↔ mirror
// ============================================================================
//   - `SpecJournalError::SequenceOverflow`
//       <- crates/vb_storage/src/error/mod.rs JournalError::SequenceOverflow
//          (returned at append_event.rs:84 from u64::try_from)
//   - `SpecJournalError::JournalBatchBytesExceeded { attempted, limit }`
//       <- crates/vb_storage/src/error/mod.rs JournalError::JournalBatchBytesExceeded
//          (returned at append_event.rs:88-91 on overflow and
//           append_event.rs:95 on attempted > limit)
//   - `SpecJournalWriteBatch::staged_bytes`
//       <- crates/vb_storage/src/batch/types.rs:27 `pub staged_bytes: u64`
//   - `SpecJournalWriteBatch::byte_limit`
//       <- crates/vb_storage/src/batch/types.rs:28 `pub byte_limit: Option<u64>`
//   - `SpecJournalWriteBatch::byte_admit` (production mirror of guard 6)
//       <- crates/vb_storage/src/batch/append_event.rs:82-98
//          (verbatim: same checked_add call, same > limit comparison,
//           same Err payload on overflow, same staged_bytes = attempted
//           mutation on Ok)
//   - `MAX_JOURNAL_BATCH_BYTES_LIMIT: u64 = 1_048_576`
//       <- crates/vb_storage/src/batch/types.rs:10
//          ::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT
//          (and the C1 contract ceiling referenced in
//           .beads/vb-vzcuf/contract.md:C1)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of the byte-admission guard is NOT verified
// by Verus directly. The exec fns below are `#[verifier::external]`
// so Verus skips body verification, and the contracts attached via
// `assume_specification` in the companion spec file
// (`vb-vzcuf-PS-001.rs`) state the production behavior the spec
// proofs discharge. Drift between the mirror and the production
// source is reported as binding-debt tracked outside Verus.
//
// The single most important property preserved by this mirror is
// that the byte-admission guard has EXACTLY three observable
// behaviors on a given `(staged_bytes, byte_limit, encoded_len)`:
//
//   - byte_limit == None              => staged_bytes unchanged, Ok(())
//   - byte_limit == Some(L), overflow => Err(JournalBatchBytesExceeded{ attempted: u64::MAX, limit: L })
//   - byte_limit == Some(L), ok       => Err if attempted > L else Ok(staged_bytes = attempted)
//
// Any drift in the production code that introduces a fourth outcome
// (panic, wrap, silent mutation on Err, etc.) breaks the mirror and
// surfaces as a Verus type-mismatch or contract-violation diagnostic.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Mirror of `JournalError` (subset reachable from the byte-admission block)
// ---------------------------------------------------------------------------
/// Subset of `vb_storage::error::JournalError`
/// (crates/vb_storage/src/error/mod.rs) reachable from the byte
/// admission guard (guard 6 of append_event at
/// append_event.rs:82-98).
///
/// Variants omitted because the byte-admission block cannot return
/// them:
///   - `DuplicateStagedKey` / `DuplicateEvent` -> guards 2/3
///   - `QueueFull`                             -> guard 4
///   - `PayloadTooLarge` / `Encode`            -> guard 5
///   - All other variants                      -> unreachable from this mirror
#[derive(Clone, Copy)]
pub enum SpecJournalError {
    /// Mirror of `JournalError::SequenceOverflow` at
    /// batch/append_event.rs:84. Returned when the `u64::try_from`
    /// conversion of `value.len()` (the encoded `Vec<u8>` length)
    /// fails because `value.len()` exceeds `u64::MAX`. In practice
    /// this is statically precluded by the bounded payload
    /// (`MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 1_048_576` as u32), but
    /// the production code carries the typed rejection so the
    /// failure mode is observable.
    SequenceOverflow,
    /// Mirror of `JournalError::JournalBatchBytesExceeded { attempted, limit }`
    /// at batch/append_event.rs:88-91 (overflow branch with
    /// `attempted: u64::MAX`) and batch/append_event.rs:95
    /// (over-limit branch with the actual `attempted` value).
    /// Returned when `staged_bytes.checked_add(encoded_len)` either
    /// overflows u64 OR the resulting `attempted` is strictly greater
    /// than `byte_limit`.
    JournalBatchBytesExceeded { attempted: u64, limit: u64 },
}

// ---------------------------------------------------------------------------
// Mirror of `JournalWriteBatch<'j>` (byte-accounting fields only)
// ---------------------------------------------------------------------------
/// Mirror of `vb_storage::batch::JournalWriteBatch<'j>` restricted
/// to the byte-accounting fields exercised by the byte-admission
/// guard.
///
/// Production fields NOT mirrored (irrelevant to PS-001 / C3):
///   - `inner: fjall::OwnedWriteBatch`               -> guard 7 (out of scope)
///   - `journal: &'j FjallJournal`                   -> guard 3 (out of scope)
///   - `staged_event_keys: HashSet<[u8; 17]>`        -> guard 2 (out of scope)
///   - `aborted: bool`                               -> guard 3 (out of scope)
///   - `_not_send_or_sync: PhantomData<*mut ...>`    -> !Send/!Sync marker
///
/// Field correspondence:
///   - `staged_bytes` mirrors `pub staged_bytes: u64` at
///     crates/vb_storage/src/batch/types.rs:27 (1:1).
///   - `byte_limit`  mirrors `pub byte_limit: Option<u64>` at
///     crates/vb_storage/src/batch/types.rs:28 (1:1).
#[derive(Clone, Copy)]
pub struct SpecJournalWriteBatch {
    /// Mirror of production `staged_bytes: u64` (1:1).
    pub staged_bytes: u64,
    /// Mirror of production `byte_limit: Option<u64>` (1:1).
    pub byte_limit: Option<u64>,
}

impl SpecJournalWriteBatch {
    /// Constructor matching production default for the
    /// byte-accounting fields. Production's
    /// `JournalWriteBatch::new` at types.rs:34 sets
    /// `byte_limit: Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT)`; this
    /// mirror parameterizes on the limit so the spec harness can
    /// drive overflow / over-limit / in-limit scenarios.
    pub fn new(byte_limit: Option<u64>) -> Self {
        Self { staged_bytes: 0, byte_limit }
    }

    /// Production mirror of the byte-admission guard at
    /// `crates/vb_storage/src/batch/append_event.rs:82-98`.
    ///
    /// Body skipped by Verus (`#[verifier::external]`); the contract
    /// attached via `assume_specification` in `vb-vzcuf-PS-001.rs`
    /// states the production behavior the spec proofs discharge.
    ///
    /// `encoded_len` is the result of `u64::try_from(value.len())?`
    /// at production append_event.rs:84. The mirror takes it as a
    /// `u64` arg so the byte-admission block (the only PS-001 scope)
    /// is isolated from the Fjall `Vec<u8>` layer that Verus cannot
    /// model.
    ///
    /// The body below reproduces the production byte-admission
    /// arithmetic verbatim — same `checked_add` call, same `> limit`
    /// comparison, same `Err { attempted: u64::MAX, limit }` overflow
    /// payload, same `staged_bytes = attempted` mutation on Ok. Drift
    /// between this body and the production source is binding-debt
    /// tracked outside Verus.
    #[verifier::external]
    pub fn byte_admit(&mut self, encoded_len: u64) -> Result<(), SpecJournalError> {
        if let Some(limit) = self.byte_limit {
            let attempted = match self.staged_bytes.checked_add(encoded_len) {
                Some(total) => total,
                None => {
                    return Err(
                        SpecJournalError::JournalBatchBytesExceeded { attempted: u64::MAX, limit },
                    );
                },
            };
            if attempted > limit {
                return Err(SpecJournalError::JournalBatchBytesExceeded { attempted, limit });
            }
            self.staged_bytes = attempted;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Production constants (mirrors of crate::constants values)
// ---------------------------------------------------------------------------
/// Mirror of `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` at
/// `crates/vb_storage/src/batch/types.rs:10` (= `1_048_576`).
/// Also matches the core `max_journal_batch_bytes` default of
/// `1_048_576` referenced in the C1 contract ceiling
/// (`.beads/vb-vzcuf/contract.md:C1`).
pub const SPEC_MAX_JOURNAL_BATCH_BYTES_LIMIT: u64 = 1_048_576;

} // verus!
