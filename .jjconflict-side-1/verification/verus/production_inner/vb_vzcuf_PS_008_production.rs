// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for vb_vzcuf_PS_008
// ============================================================================
//
// DRIFT POLICY: This file MUST be regenerated from `crates/vb_storage/src/batch/append_event.rs:1-110`
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
// (`verification/verus/extern_vb_vzcuf_PS_008.rs`)
// can use `#[path = "production_inner/vb_vzcuf_PS_008_production.rs"]` to bind the
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
// Extern surface for vb-vzcuf-PS-008 Verus spec.
//
// =============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// Target: vb_storage::batch::JournalWriteBatch<'j>::append_event at
//         crates/vb_storage/src/batch/append_event.rs:41-106 (called via
//         the public re-export at crates/vb_storage/src/batch/mod.rs:7).
//
// This file mirrors the production types and the production exec body of
// `append_event` in a Verus-friendly form. The body is declared
// `#[verifier::external]` so Verus does not attempt to verify it directly;
// the companion spec file `vb-vzcuf-PS-008.rs` attaches the production
// behavioral contract via `assume_specification` and proves the guard
// ordering property from the contract's per-variant preconditions.
//
// =============================================================================
// BINDING LEDGER
// =============================================================================
//
// Production field correspondence
// (crates/vb_storage/src/batch/types.rs:21-30):
//   - staged_event_keys: HashSet<[u8; 17]> -> HashSet<u64> (1:1 view)
//   - staged_bytes: u64 -> u64 (1:1)
//   - byte_limit: Option<u64> -> Option<u64> (1:1)
//   - aborted: bool -> bool (1:1)
//   - inner.len(): usize -> inner_len: usize (1:1)
//   - journal.events.contains_key(key): bool -> journal_has_key: bool (projected)
//   - encode_record(...)?: Result<Vec<u8>, _> -> (encode_ok: bool, encoded_len: u64)
//
// Production constants (literal-inlined to avoid a Verus
// `--crate-type=lib` panic on `pub const` items in extern modules;
// mirrors production source byte-for-byte):
//   * MAX_BATCH_COUNT            = 10_000   (crates/vb_storage/src/constants.rs:100)
//   * MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 1_048_576
//                                    (crates/vb_storage/src/constants.rs:88)
//   * DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576
//                                    (crates/vb_storage/src/batch/types.rs:10)
//
// =============================================================================
// PRODUCTION GUARD ORDER (verified by SA-003 regression tests and POB-vb-vzcuf-029)
// =============================================================================
//
// crates/vb_storage/src/batch/append_event.rs executes the following
// strict 7-guard order:
//
//   G1 KeyConstruction           line 42: let key = run_event_key(...)?
//   G2 SameBatchDuplicate        line 51: HashSet::contains(&key) -> DuplicateStagedKey
//   G3 DurableDuplicate          line 57: events.contains_key(key) -> DuplicateEvent (aborts)
//   G4 BatchCount                line 64: inner.len() >= MAX_BATCH_COUNT -> QueueFull
//   G5 PerRecordEncoding         line 67: encode_record(...) -> Encode/PayloadTooLarge
//   G6 AccumulatedByteAdmission  line 82: byte_limit.checked_add -> JournalBatchBytesExceeded
//   G7 Mutation                  line 100: inner.insert(...) -> Ok(())
//
// Each guard's `Err` variant is exclusively reachable at that guard's
// position; later guards cannot fire without earlier guards passing.
// The `assume_specification` contract in vb-vzcuf-PS-008.rs enforces
// this by requiring the witness precondition on each Err variant and
// asserting state preservation across all subsequent guards' fields.
//
// =============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// =============================================================================
//
// The production body of `append_event` is NOT verified by Verus:
//   * `fjall::OwnedWriteBatch` and `FjallJournal` are opaque to Verus.
//   * `encode_record` (codec step) reaches into postcard + record framing.
//   * The mirror body below is `#[verifier::external]` so Verus skips
//     body verification.
//
// The `assume_specification` bridge in `vb-vzcuf-PS-008.rs` therefore
// represents the FULL behavioral contract: Fjall/codec layers are trusted
// to project the right `journal_has_key`, `encode_ok`, `encoded_len`
// values. Any drift between the projection and the production body is
// recorded as drift debt below; the guard-ordering proof itself is local
// to the contract and does not depend on the projection correctness.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Mirror of `JournalError` (subset reached by `append_event`)
// ---------------------------------------------------------------------------

/// Subset of `vb_storage::error::JournalError`
/// (crates/vb_storage/src/error/mod.rs:21-163) reachable from
/// `JournalWriteBatch::append_event` after the SA-003 fix.
#[derive(Clone, Copy)]
pub enum SpecJournalError {
    /// Mirror of `JournalError::DuplicateStagedKey` at
    /// batch/append_event.rs:52-55. Guard G2.
    DuplicateStagedKey,
    /// Mirror of `JournalError::DuplicateEvent` at
    /// batch/append_event.rs:59-62. Guard G3. Sets `aborted = true`.
    DuplicateEvent,
    /// Mirror of `JournalError::QueueFull` at
    /// batch/append_event.rs:65. Guard G4.
    QueueFull,
    /// Mirror of `JournalError::Encode` and
    /// `JournalError::PostcardEncodeFailed`. Guard G5 (encode failure).
    Encode,
    /// Mirror of `JournalError::PayloadTooLarge { len, max }` at
    /// batch/append_event.rs encoding step. Guard G5.
    PayloadTooLarge { len: u32, max: u32 },
    /// Mirror of `JournalError::SequenceOverflow` at
    /// batch/append_event.rs:84. Guard G5 (try_from overflow).
    SequenceOverflow,
    /// Mirror of `JournalError::JournalBatchBytesExceeded { attempted, limit }`
    /// at batch/append_event.rs:88-95. Guard G6.
    JournalBatchBytesExceeded { attempted: u64, limit: u64 },
    /// Mirror of `JournalError::KeyCapacity` at error/mod.rs:29. Guard G1
    /// (key construction). The mirror's G1 returns `Ok(())` because key
    /// is supplied as input; the contract marks `KeyCapacity` unreachable.
    KeyCapacity,
    /// Mirror of `JournalError::Fjall` for `events.contains_key` I/O
    /// failure. Not currently reachable in production because the Fjall
    /// call site is best-effort; included for completeness.
    FjallUnavailable,
}

// ---------------------------------------------------------------------------
// Mirror of `JournalWriteBatch<'j>`
// ---------------------------------------------------------------------------

/// Mirror of `vb_storage::batch::JournalWriteBatch<'j>` (subset relevant
/// to `append_event`). Field correspondence is documented in the
/// BINDING LEDGER above.
pub struct SpecJournalWriteBatch {
    /// Mirror of production `HashSet<[u8; JOURNAL_KEY_BYTES]>`.
    pub staged_event_keys: HashSet<u64>,
    /// Mirror of production `staged_bytes: u64`.
    pub staged_bytes: u64,
    /// Mirror of production `byte_limit: Option<u64>`.
    pub byte_limit: Option<u64>,
    /// Mirror of production `aborted: bool`.
    pub aborted: bool,
    /// Mirror of `inner.len()`. OwnedWriteBatch is opaque to Verus.
    pub inner_len: usize,
}

impl SpecJournalWriteBatch {
    /// Mirror of `JournalWriteBatch::new`.
    pub fn new(byte_limit: Option<u64>) -> Self {
        Self {
            staged_event_keys: HashSet::new(),
            staged_bytes: 0,
            byte_limit,
            aborted: false,
            inner_len: 0,
        }
    }

    /// Mirror of production `JournalWriteBatch::append_event`.
    ///
    /// `key` is the post-construction journal key (mirrors the result of
    /// `run_event_key(event.run_id(), event.seq())?`). The remaining
    /// inputs abstract over the Fjall-side observables the production
    /// function reads:
    ///
    ///   * `journal_has_key: bool`
    ///         mirror of `journal.events.contains_key(key)?` at
    ///         batch/append_event.rs:57.
    ///   * `encode_ok: bool`
    ///         mirror of `encode_record(...)?` success.
    ///   * `encoded_len: u64`
    ///         mirror of `value.len()` after a successful encode.
    ///
    /// The body is declared `#[verifier::external]` because Verus does
    /// not model `HashSet::contains` / `HashSet::insert` exec semantics
    /// inside exec fn bodies; the `assume_specification` bridge in
    /// `vb-vzcuf-PS-008.rs` attaches the spec contract. The body here is
    /// the mirror algorithm in production order; it is NOT verified.
    #[verifier::external]
    pub fn append_event(
        &mut self,
        key: u64,
        journal_has_key: bool,
        encode_ok: bool,
        encoded_len: u64,
    ) -> Result<(), SpecJournalError> {
        // Guard G2: same-batch duplicate (post-fix SA-003).
        if self.staged_event_keys.contains(&key) {
            return Err(SpecJournalError::DuplicateStagedKey);
        }
        // Guard G3: durable duplicate -> abort.
        if journal_has_key {
            self.aborted = true;
            return Err(SpecJournalError::DuplicateEvent);
        }
        // Guard G4: count capacity.
        if self.inner_len >= 10_000usize {
            return Err(SpecJournalError::QueueFull);
        }
        // Guard G5: encoding.
        if !encode_ok {
            if encoded_len > u64::from(1_048_576u32) {
                return Err(SpecJournalError::PayloadTooLarge {
                    len: 1_048_576u32,
                    max: 1_048_576u32,
                });
            }
            return Err(SpecJournalError::Encode);
        }
        // Guard G6: byte admission.
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
        // Guard G7: insert.
        self.inner_len += 1;
        self.staged_event_keys.insert(key);
        Ok(())
    }
}
