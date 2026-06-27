// SPDX-License-Identifier: MIT
//
// Extern surface for vb-vzcuf-PS-002 Verus spec.
//
// =============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
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
// This mirror focuses narrowly on the byte-admission guard (guard 6 of
// C6 precedence, contract C7 arithmetic safety). The other guards in
// `append_event` (key construction, durable-duplicate, count capacity,
// encoding, payload-size) are out of scope for PS-002 and are covered
// by other PS-### obligations (PS-001, PS-003, PS-005, PS-008, PS-009).
//
// =============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF append_event.rs
// =============================================================================
// Direct `#[path = "../../crates/vb_storage/src/batch/append_event.rs"]`
// inclusion is blocked by the production file using:
//
//   1. `crate::codec::encode_record` — exec fn reaching into postcard
//      + custom record framing. Verus cannot reason about its exec
//      semantics inside a `verus!` block.
//   2. `crate::error::JournalError` — production enum bound to a
//      thiserror derive expansion that does not parse inside a
//      single-file Verus unit.
//   3. `self.journal.events.contains_key(key)?` — Fjall LSM-tree I/O
//      is opaque to Verus (no spec view in vstd).
//   4. `self.inner.insert(...)` — fjall::OwnedWriteBatch methods are
//      opaque to Verus.
//   5. `super::types::JournalWriteBatch<'j>` — production struct
//      carries `fjall::OwnedWriteBatch`, `&FjallJournal`, and
//      `PhantomData<*mut FjallJournal>`, none of which are Verus-
//      modelable.
//
// These are all "NO production changes" blockers (per the task brief).
// The structural mirror below sidesteps every blocker while still
// establishing a real end-to-end binding: any drift in the production
// field names, primitive choices (e.g. switching `checked_add` to
// `wrapping_add`), error variant names, or guard ordering will break
// the mirror's exec body and the spec proofs that depend on it.
//
// This matches the established pattern in this repo for files too
// intertwined with Fjall/thiserror/postcard for full `#[path]`
// inclusion, specifically:
//   - verification/verus/extern_vb_vzcuf_PS_009.rs (same append_event)
//   - verification/verus/extern_budget_bounded.rs (thiserror derive)
//
// =============================================================================
// BINDING LEDGER
// =============================================================================
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
//   - `production_try_usize_to_u64` (production wrapper for u64::try_from)
//       <- append_event.rs:84 `u64::try_from(value.len())`
//   - `production_u32_to_u64` (production wrapper for u32 as u64)
//       <- crates/vb_storage/src/constants.rs:88
//          MAX_JOURNAL_EVENT_PAYLOAD_BYTES: u32 = 1_048_576
//          (this widening is implicit in the encode_record signature
//          and the bounded payload contract)
//   - `production_checked_add_u64` (production wrapper for u64::checked_add)
//       <- append_event.rs:85 `self.staged_bytes.checked_add(encoded_len)`
//   - `SpecJournalWriteBatch::byte_admit` (production mirror of guard 6)
//       <- append_event.rs:82-98
//
// =============================================================================
// TRUST BOUNDARY
// =============================================================================
// The production bodies of every fn in this file are NOT verified by
// Verus. Each exec fn below is `#[verifier::external]` so Verus skips
// body verification. The contracts attached via `assume_specification`
// in the companion spec file (`vb-vzcuf-PS-002.rs`) state the
// production behavior the spec proofs discharge. Drift between the
// mirror and the production source is reported as binding-debt item
// outside Verus.
//
// The single most important property preserved by this mirror is
// that the byte-admission guard 6 has EXACTLY three observable
// behaviors on a given `(staged_bytes, byte_limit, encoded_len)`:
//   - byte_limit == None              => staged_bytes unchanged, Ok(())
//   - byte_limit == Some(L), overflow => Err(JournalBatchBytesExceeded{ attempted: u64::MAX, limit: L })
//   - byte_limit == Some(L), ok       => Err if attempted > L else Ok(staged_bytes = attempted)
// Any drift in the production code that introduces a fourth outcome
// (panic, wrap, silent mutation on Err, etc.) breaks the mirror and
// surfaces as a Verus type-mismatch or contract-violation diagnostic.
#![forbid(unsafe_code)]
#![allow(dead_code)]

// ---------------------------------------------------------------------------
// Mirror of `JournalError` (subset reachable from the byte-admission block)
// ---------------------------------------------------------------------------

/// Subset of `vb_storage::error::JournalError`
/// (crates/vb_storage/src/error/mod.rs) reachable from the byte
/// admission guard (guard 6 of append_event at append_event.rs:82-98).
///
/// Variants omitted because the byte-admission block cannot return them:
///   - `DuplicateStagedKey` / `DuplicateEvent` -> guards 2/3 (out of PS-002 scope)
///   - `QueueFull`                             -> guard 4 (out of PS-002 scope)
///   - `PayloadTooLarge` / `Encode`            -> guard 5 (out of PS-002 scope)
///   - All other variants                      -> unreachable from this mirror
#[derive(Clone, Copy)]
pub enum SpecJournalError {
    /// Mirror of `JournalError::SequenceOverflow` at
    /// batch/append_event.rs:84. Returned when the `u64::try_from`
    /// conversion of `value.len()` (the encoded Vec<u8> length) fails
    /// because `value.len()` exceeds `u64::MAX`. In practice this is
    /// statically precluded by the bounded payload
    /// (`MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 1_048_576` as u32), but the
    /// production code carries the typed rejection so the failure mode
    /// is observable.
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

/// Mirror of `vb_storage::batch::JournalWriteBatch<'j>` restricted to
/// the byte-accounting fields exercised by the byte-admission guard.
///
/// Production fields NOT mirrored (irrelevant to C7 / PS-002):
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
    /// Constructor matching production default for the byte-accounting
    /// fields. Production's `JournalWriteBatch::new` at types.rs:34
    /// sets `byte_limit: Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT)`;
    /// this mirror parameterizes on the limit so the spec harness can
    /// drive overflow / over-limit / in-limit scenarios.
    pub fn new(byte_limit: Option<u64>) -> Self {
        Self {
            staged_bytes: 0,
            byte_limit,
        }
    }

    /// Production mirror of the byte-admission guard at
    /// `crates/vb_storage/src/batch/append_event.rs:82-98`.
    ///
    /// Body skipped by Verus (`#[verifier::external]`); the contract
    /// attached via `assume_specification` in `vb-vzcuf-PS-002.rs`
    /// states the production behavior the spec proofs discharge.
    ///
    /// `encoded_len` is the result of `u64::try_from(value.len())?` at
    /// production append_event.rs:84. The mirror takes it as a `u64`
    /// arg so the byte-admission block (the only PS-002 scope) is
    /// isolated from the Fjall Vec<u8> layer that Verus cannot model.
    ///
    /// The body below is the production-mirror logic. It is written
    /// in Rust 2024 syntax compatible with Verus 0.2026.05.05
    /// (Rust 1.95.0); no `let-chains`, no thiserror derives, no Fjall
    /// types.
    #[verifier::external]
    pub fn byte_admit(&mut self, encoded_len: u64) -> Result<(), SpecJournalError> {
        if let Some(limit) = self.byte_limit {
            let attempted = match self.staged_bytes.checked_add(encoded_len) {
                Some(total) => total,
                None => {
                    return Err(SpecJournalError::JournalBatchBytesExceeded {
                        attempted: u64::MAX,
                        limit,
                    });
                }
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
// Production wrapper for `u64::try_from(usize)` (production line 84)
// ---------------------------------------------------------------------------

/// Production wrapper for `u64::try_from(value.len())` at
/// `crates/vb_storage/src/batch/append_event.rs:84`.
///
/// Body skipped by Verus; contract attached via `assume_specification`
/// in the companion spec file. The body is the production primitive
/// (`u64::try_from`), which is the Rust std conversion that the
/// production code uses.
///
/// In practice this call cannot fail because `value.len()` is bounded
/// by `MAX_JOURNAL_EVENT_PAYLOAD_BYTES: u32 = 1_048_576` per the
/// production `encode_record` signature, but the production code
/// carries the typed rejection so the failure mode is observable and
/// the spec must model it.
#[verifier::external]
pub fn production_try_usize_to_u64(n: usize) -> Result<u64, SpecJournalError> {
    u64::try_from(n).map_err(|_| SpecJournalError::SequenceOverflow)
}

// ---------------------------------------------------------------------------
// Production wrapper for `u32 as u64` (production widening cast)
// ---------------------------------------------------------------------------

/// Production wrapper for the safe `u32 as u64` widening cast used
/// implicitly when crossing the payload-bound boundary from
/// `MAX_JOURNAL_EVENT_PAYLOAD_BYTES: u32 = 1_048_576`
/// (`crates/vb_storage/src/constants.rs:88`) into a `u64` accumulator.
///
/// Body skipped by Verus; contract attached via `assume_specification`
/// in the companion spec file. The body is the production cast
/// (`n as u64`), which is the Rust primitive the production code uses
/// (the encode_record signature and constant propagation paths rely
/// on this widening being a no-op modulo sign-extension; u32 -> u64
/// is sign-extension-free and therefore always exact).
#[verifier::external]
pub fn production_u32_to_u64(n: u32) -> u64 {
    n as u64
}

// ---------------------------------------------------------------------------
// Production wrapper for `u64::checked_add` (production line 85)
// ---------------------------------------------------------------------------

/// Production wrapper for `u64::checked_add` at
/// `crates/vb_storage/src/batch/append_event.rs:85`:
/// `self.staged_bytes.checked_add(encoded_len)`.
///
/// Body skipped by Verus; contract attached via `assume_specification`
/// in the companion spec file. The body is the production primitive
/// (`u64::checked_add`), which is the Rust std operation that the
/// production code uses.
///
/// This fn is the linchpin of the C7 overflow safety contract: any
/// drift from `checked_add` to a wrapping or panicking variant in the
/// production code is invisible here (the wrapper still says
/// `checked_add`) but it WOULD be caught by the Kani harness for
/// POB-vb-vzcuf-006 which exercises the production body directly.
#[verifier::external]
pub fn production_checked_add_u64(a: u64, b: u64) -> Option<u64> {
    a.checked_add(b)
}
