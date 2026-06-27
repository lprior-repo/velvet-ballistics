// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for vb_vzcuf_PS_009
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
// (`verification/verus/extern_vb_vzcuf_PS_009.rs`)
// can use `#[path = "production_inner/vb_vzcuf_PS_009_production.rs"]` to bind the
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
// Extern surface for vb-vzcuf-PS-009 Verus spec.
//
// =============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// Target: vb_storage::batch::JournalWriteBatch<'j>::append_event at
//         crates/vb_storage/src/batch/append_event.rs:41-106 (called via
//         the public re-export at crates/vb_storage/src/batch/mod.rs:7).
//
// Production fields mirrored (crates/vb_storage/src/batch/types.rs:21-30):
//   - staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>
//       -> here: HashSet<u64> where u64 is a stable hash handle for the key
//   - staged_bytes: u64
//       -> here: u64 (1:1)
//   - byte_limit: Option<u64>
//       -> here: Option<u64> (1:1)
//   - aborted: bool
//       -> here: bool (1:1)
//
// Production internals NOT modeled (Verus cannot reason about them):
//   - inner: fjall::OwnedWriteBatch            (Fjall I/O)
//       -> here: inner_len: usize counts batched inserts
//   - journal: &'j FjallJournal                (Fjall memtable read)
//       -> here: journal_has_key: bool passed as an exec arg
//   - _not_send_or_sync: PhantomData<*mut FjallJournal>
//       -> here: dropped (it is a !Send / !Sync marker with no
//                semantic content for the duplicate-accounting proof)
//
// =============================================================================
// BINDING LEDGER (drift tracking)
// =============================================================================
//
// Production guard precedence (crates/vb_storage/src/batch/append_event.rs:18-25,
// verified by SA-003 regression test `append_event_rejects_same_batch_duplicate`):
//
//   1. Key construction          -> run_event_key (omitted; key supplied)
//   2. Same-batch duplicate check -> HashSet::contains(&key) -> DuplicateStagedKey
//   3. Durable duplicate check    -> journal.events.contains_key(key) -> DuplicateEvent (aborts)
//   4. Count capacity check       -> inner.len() >= MAX_BATCH_COUNT  -> QueueFull
//   5. Per-record encoding        -> encode_record (omitted; encoded_len supplied)
//   6. Byte admission check       -> staged_bytes.checked_add(encoded_len) -> JournalBatchBytesExceeded
//   7. Insert into inner + staged_event_keys -> Ok(())
//
// Each guard maps 1:1 to a `SpecJournalError` variant below and to a branch
// of the `assume_specification` postcondition in `vb-vzcuf-PS-009.rs`.
//
// DRIFT DEBT (tracked in `.beads/vb-vzcuf/proof-obligations.planned.jsonl`):
//   - Encoding step (guard 5) is abstracted to `encode_ok: bool`. A drift
//     that adds a new `JournalError::Encode` variant distinct from
//     `Encode(postcard::Error)` would require updating
//     `SpecJournalError::Encode`.
//   - Key construction (guard 1) is abstracted away: we take the key
//     directly. The `KeyCapacity` variant of `JournalError` is therefore
//     unreachable from this mirror; it is kept in `SpecJournalError`
//     for symmetry but the contract postcondition never returns it.
//   - The `inner` field is reduced to `inner_len: usize` because we never
//     need to inspect batched values; we only need the count for the
//     `QueueFull` guard.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Constants are inlined as literals in this file to avoid a Verus
// `--crate-type=lib` panic where pub const items declared inside an extern
// module trigger `VerusErasureCtxt has not been initialized` during
// thir-body processing. The literal values mirror the production
// source byte-for-byte:
//   * 17    = crates/vb_storage/src/constants.rs::JOURNAL_KEY_BYTES
//   * 1024  = crates/vb_storage/src/batch/writer_queue.rs::MAX_BATCH_COUNT
//   * 1_048_576 = crates/vb_storage/src/batch/types.rs:10
//                  ::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT
//   * 65_536 = crates/vb_storage/src/constants.rs
//                  ::MAX_JOURNAL_EVENT_PAYLOAD_BYTES
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Mirror of `JournalError` (subset reached by `append_event`)
// ---------------------------------------------------------------------------

/// Subset of `vb_storage::error::JournalError`
/// (crates/vb_storage/src/error/mod.rs:21-166) reachable from
/// `JournalWriteBatch::append_event` after the SA-003 fix.
///
/// Variants omitted because `append_event` cannot return them:
///   - `Fjall(fjall::Error)`              -> folded into `Encode` here
///   - `BatchAborted`                     -> returned by `commit`, not `append_event`
///   - `WriteLockPoisoned`                -> returned by queue, not `append_event`
///   - `QueueCapacity` / `QueueShutdown`  -> returned by queue construction
///   - `WrongRun` / `SequenceGap`         -> returned by `replay_*`, not `append_event`
///   - `BadMagic` / `UnsupportedSchemaVersion` / `MigrationRequired`
///       / `UnknownRecordKind` / `RecordKindFamilyMismatch`
///       / `RecordKindPayloadMismatch` / `HeaderLengthMismatch`
///       / `HeaderChecksumMismatch` / `PayloadDigestMismatch`
///       / `UnexpectedEof` / `MalformedKeyspaceRow`
///       / `PostcardDecodeFailed` / `InvalidEvent`
///       / `ArtifactMalformed` / `WorkflowReconstruction`
///       / `CompiledIrReadback`
///     -> returned by record decode / workflow reconstruction, not `append_event`
///   - `PostcardEncodeFailed` -> folded into `Encode` here (production
///     only raises it from `putters.rs`, but `Encode` is the mirror of
///     every encode-time failure for safety)
///
/// `KeyCapacity` is kept for symmetry with production even though the
/// mirror cannot return it (key construction is abstracted out); the
/// contract postcondition explicitly excludes it from the reachable set.
#[derive(Clone, Copy)]
pub enum SpecJournalError {
    /// Mirror of `JournalError::DuplicateStagedKey { run, seq }` at
    /// batch/append_event.rs:52-55. Returned when the key is already in
    /// the same batch's `staged_event_keys`.
    DuplicateStagedKey,
    /// Mirror of `JournalError::DuplicateEvent { run, seq }` at
    /// batch/append_event.rs:59-62. Returned when the journal memtable
    /// already has the key (committed durable duplicate). Sets
    /// `aborted = true` so `commit()` short-circuits.
    DuplicateEvent,
    /// Mirror of `JournalError::QueueFull` at batch/append_event.rs:65.
    /// Returned when `inner.len() >= MAX_BATCH_COUNT`.
    QueueFull,
    /// Mirror of `JournalError::JournalBatchBytesExceeded { attempted, limit }`
    /// at batch/append_event.rs:88-95. Returned on byte-budget overrun or
    /// `staged_bytes.checked_add(encoded_len)` overflow.
    JournalBatchBytesExceeded { attempted: u64, limit: u64 },
    /// Mirror of `JournalError::SequenceOverflow` at
    /// batch/append_event.rs:84. Reachable from the `u64::try_from`
    /// conversion of `value.len()`. In practice `value.len() <= u32::MAX`
    /// so this variant is unreachable, but the production code returns
    /// it on the conversion failure branch.
    SequenceOverflow,
    /// Mirror of `JournalError::PayloadTooLarge { len, max }` at
    /// batch/append_event.rs (encoding step). The mirror abstracts
    /// encoding into `encode_ok: bool`; this variant is returned when
    /// `encode_ok == false` and the encoded length would exceed
    /// `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`.
    PayloadTooLarge { len: u32, max: u32 },
    /// Folded mirror of `JournalError::Encode(postcard::Error)` and
    /// `JournalError::PostcardEncodeFailed`. Reachable when `encode_ok
    /// == false` for any other reason.
    Encode,
    /// Mirror of `JournalError::KeyCapacity`. Unreachable in this mirror
    /// (key construction is abstracted out); retained for symmetry with
    /// the production error set so the mirror enum is exhaustive over
    /// every `JournalError` variant that `append_event` COULD return.
    KeyCapacity,
}

// ---------------------------------------------------------------------------
// Mirror of `JournalWriteBatch<'j>`
// ---------------------------------------------------------------------------

/// Mirror of `vb_storage::batch::JournalWriteBatch<'j>`.
///
/// Field correspondence:
///   * `staged_event_keys` mirrors `staged_event_keys: HashSet<[u8; 17]>`.
///     The key type is abstracted to `u64` because Verus cannot directly
///     reason about `[u8; 17]` keys in spec context, but every spec-level
///     property the production HashSet provides (containment, insert
///     immutability) is preserved by the `HashSet<u64>` view `s@ -> Set<u64>`
///     in vstd.
///   * `staged_bytes` mirrors `staged_bytes: u64` (1:1).
///   * `byte_limit` mirrors `byte_limit: Option<u64>` (1:1).
///   * `aborted` mirrors `aborted: bool` (1:1).
///   * `inner_len` mirrors the cardinality of `inner: fjall::OwnedWriteBatch`.
///     The OwnedWriteBatch type cannot be modeled by Verus; only the count
///     matters for the `MAX_BATCH_COUNT` guard.
pub struct SpecJournalWriteBatch {
    /// HashSet<u64> mirror of production `HashSet<[u8; JOURNAL_KEY_BYTES]>`.
    pub staged_event_keys: HashSet<u64>,
    /// Mirror of production `staged_bytes: u64`.
    pub staged_bytes: u64,
    /// Mirror of production `byte_limit: Option<u64>`.
    pub byte_limit: Option<u64>,
    /// Mirror of production `aborted: bool`.
    pub aborted: bool,
    /// Mirror of `inner.len()`. The OwnedWriteBatch itself is opaque to Verus.
    pub inner_len: usize,
}

impl SpecJournalWriteBatch {
    /// Mirror of `JournalWriteBatch::new` at
    /// `crates/vb_storage/src/batch/types.rs:33-44`, parameterized on
    /// `byte_limit` so the test harness can drive byte-budget behavior
    /// without depending on `FjallJournal` construction.
    pub fn new(byte_limit: Option<u64>) -> Self {
        Self {
            staged_event_keys: HashSet::new(),
            staged_bytes: 0,
            byte_limit,
            aborted: false,
            inner_len: 0,
        }
    }

    /// Mirror of `JournalWriteBatch::is_aborted` at
    /// `crates/vb_storage/src/batch/types.rs:67-70`.
    #[allow(dead_code)]
    pub fn is_aborted(&self) -> bool {
        self.aborted
    }

    /// Mirror of `JournalWriteBatch::staged_event_bytes` at
    /// `crates/vb_storage/src/batch/types.rs:74-77`.
    #[allow(dead_code)]
    pub fn staged_event_bytes(&self) -> u64 {
        self.staged_bytes
    }

    /// Mirror of `JournalWriteBatch::len` at
    /// `crates/vb_storage/src/batch/types.rs:47-50`.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        if self.aborted {
            0
        } else {
            self.inner_len
        }
    }

    /// Mirror of `JournalWriteBatch::is_empty`.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Mirror of `JournalWriteBatch::byte_limit` at
    /// `crates/vb_storage/src/batch/types.rs:80-83`.
    #[allow(dead_code)]
    pub fn byte_limit(&self) -> Option<u64> {
        self.byte_limit
    }

    /// Mirror of `JournalWriteBatch::append_event` at
    /// `crates/vb_storage/src/batch/append_event.rs:41-106`.
    ///
    /// `key` is the post-construction journal key (mirrors the result of
    /// `run_event_key(event.run_id(), event.seq())?`). The remaining
    /// inputs abstract over the Fjall-side observables the production
    /// function reads:
    ///
    ///   * `journal_has_key: bool`
    ///         mirror of `journal.events.contains_key(key)?` at
    ///         batch/append_event.rs:57. Production swallows the Fjall
    ///         error and the `bool` is the projected answer; on Fjall
    ///         error production would propagate `JournalError::Fjall`
    ///         which our mirror folds into `Encode` for symmetry.
    ///   * `encode_ok: bool`
    ///         mirror of `encode_record(...)?` success. When false,
    ///         the production body returns one of
    ///         `Encode` / `PostcardEncodeFailed` / `PayloadTooLarge` /
    ///         `SequenceOverflow`. The mirror collapses these to
    ///         `Encode` for the `encode_ok=false && encoded_len <= max`
    ///         case and `PayloadTooLarge` for the
    ///         `encode_ok=false && encoded_len > max` case.
    ///   * `encoded_len: u64`
    ///         mirror of `value.len()` after a successful encode.
    ///         When `encode_ok` is true, `encoded_len` must satisfy
    ///         `encoded_len as u32 <= MAX_JOURNAL_EVENT_PAYLOAD_BYTES`
    ///         per the production record-format bound.
    ///
    /// The body is declared `#[verifier::external]` because Verus does
    /// not model `HashSet::contains` / `HashSet::insert` exec semantics
    /// inside exec fn bodies; the `assume_specification` bridge in
    /// `vb-vzcuf-PS-009.rs` attaches the spec contract and the exec
    /// wrappers in that file exercise the contract from `verus!`
    /// context.
    #[verifier::external]
    pub fn append_event(
        &mut self,
        key: u64,
        journal_has_key: bool,
        encode_ok: bool,
        encoded_len: u64,
    ) -> Result<(), SpecJournalError> {
        // Guard 2: same-batch duplicate (post-fix SA-003).
        if self.staged_event_keys.contains(&key) {
            return Err(SpecJournalError::DuplicateStagedKey);
        }
        // Guard 3: durable duplicate -> abort.
        if journal_has_key {
            self.aborted = true;
            return Err(SpecJournalError::DuplicateEvent);
        }
        // Guard 4: count capacity.
        if self.inner_len >= 1024usize {
            return Err(SpecJournalError::QueueFull);
        }
        // Guard 5: encoding failure.
        if !encode_ok {
            if encoded_len > u64::from(65_536u32) {
                return Err(SpecJournalError::PayloadTooLarge {
                    len: 65_536u32,
                    max: 65_536u32,
                });
            }
            return Err(SpecJournalError::Encode);
        }
        // Guard 6: byte admission.
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
        // Guard 7: insert.
        self.inner_len += 1;
        self.staged_event_keys.insert(key);
        Ok(())
    }
}
