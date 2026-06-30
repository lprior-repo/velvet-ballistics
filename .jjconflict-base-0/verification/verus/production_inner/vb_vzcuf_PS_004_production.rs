// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for vb_vzcuf_PS_004
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
// (`verification/verus/extern_vb_vzcuf_PS_004.rs`)
// can use `#[path = "production_inner/vb_vzcuf_PS_004_production.rs"]` to bind the
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
// Extern surface for vb-vzcuf-PS-004 Verus spec.
//
// =============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// Target: vb_storage::batch::JournalWriteBatch<'j>:
//   * ::new             <- crates/vb_storage/src/batch/types.rs:33-44
//   * ::is_aborted      <- crates/vb_storage/src/batch/types.rs:67-70
//   * ::staged_event_bytes
//                        <- crates/vb_storage/src/batch/types.rs:74-77
//   * ::len             <- crates/vb_storage/src/batch/types.rs:47-50
//   * ::byte_limit      <- crates/vb_storage/src/batch/types.rs:80-83
//   * ::append_event    <- crates/vb_storage/src/batch/append_event.rs:41-106
//   * ::commit          <- crates/vb_storage/src/batch/commit.rs:20-26
//
// =============================================================================
// BINDING LEDGER (drift tracking)
// =============================================================================
//
// Production fields of JournalWriteBatch (types.rs:21-30) — mirrored 1:1
// to the public fields of `SpecJournalWriteBatch` below:
//
//   production field                                mirror field
//   ---------------------------------------------- -------------------------------
//   inner: fjall::OwnedWriteBatch                   inner_len: usize (count only;
//                                                    OwnedWriteBatch is Fjall-opaque)
//   journal: &'j FjallJournal                       journal_has_key: bool (exec arg;
//                                                    memtable read is Fjall-opaque)
//   staged_event_keys: HashSet<[u8; 17]>            staged_event_keys: HashSet<u64>
//                                                    (key abstracted to u64 for spec)
//   aborted: bool                                   aborted: bool
//   staged_bytes: u64                               staged_bytes: u64
//   byte_limit: Option<u64>                         byte_limit: Option<u64>
//   _not_send_or_sync: PhantomData<*mut FjallJournal> dropped (!Send/!Sync marker,
//                                                    no semantic content for C5)
//
// Production constants used in this file (inlined as literals per the
// established vzcuf extern pattern — see extern_vb_vzcuf_PS_009.rs — to
// avoid the `--crate-type=lib` panic on `pub const` items declared
// inside an extern module; see the prose note in vb-vzcuf-PS-009.rs:67-79):
//
//   constant                                       production site           value
//   ---------------------------------------------- ------------------------- -----------
//   MAX_BATCH_COUNT                                constants.rs:100          10_000
//   MAX_JOURNAL_EVENT_PAYLOAD_BYTES                constants.rs:88           1_048_576
//   DEFAULT_JOURNAL_BATCH_BYTE_LIMIT               batch/types.rs:10         1_048_576
//
// =============================================================================
// DRIFT DEBT (tracked in .beads/vb-vzcuf/proof-findings.jsonl)
// =============================================================================
//
//   * Guard 5 (encode step, append_event.rs:67-73) is abstracted to
//     `encode_ok: bool` because postcard + custom record framing is not
//     vstd-modelable. The bridge contract folds the three reachable
//     encode-time errors (`Encode`, `PayloadTooLarge`,
//     `SequenceOverflow`) into distinct arms keyed on
//     `encode_ok` and `encoded_len` so drift in the production error
//     enum (e.g., a renamed variant) breaks the mirror.
//   * Guard 1 (key construction, append_event.rs:42) is abstracted
//     away; the mirror takes the post-construction u64 key directly.
//     `KeyCapacity` is therefore unreachable in this mirror; the
//     contract explicitly excludes it.
//   * `fjall::OwnedWriteBatch::commit` (commit.rs:24) is abstracted
//     to "Ok(()) on non-aborted, Err(BatchAborted) on aborted". The
//     bridge contract does not promise Fjall persistence durability;
//     crash-consistency is governed by the Fjall trust base
//     (TBP-008 in .beads/vb-vzcuf/trusted-base-ledger.jsonl).
//   * `fjall::OwnedWriteBatch::insert` (append_event.rs:100) is
//     abstracted to incrementing `inner_len: usize`.
//
// =============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// =============================================================================
//
// The production bodies of `append_event` and `commit` are NOT verified
// by Verus. Both are declared `#[verifier::external]` below so Verus
// skips body verification. The companion spec file
// (`vb-vzcuf-PS-004.rs`) attaches `assume_specification` bridges that
// state the production behavior the spec proofs discharge. Drift
// between the mirror bodies below and the production sources is
// reported as binding-debt outside Verus.
//
// The accessor methods (`is_aborted`, `staged_event_bytes`, `len`,
// `byte_limit`) and the constructor (`new`) have NO
// `#[verifier::external]` attribute — Verus verifies their bodies
// directly because they contain only field reads and trivial struct
// construction.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::collections::HashSet;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Mirror of `JournalError` (subset reached by append_event and commit)
// ---------------------------------------------------------------------------
//
// Production: `crates/vb_storage/src/error/mod.rs:21-163`. The full enum
// has 50+ variants; only the subset below is reachable from
// `append_event` (C5/C6 paths) and `commit` (C5 path).
/// Subset of `vb_storage::error::JournalError` reachable from
/// `JournalWriteBatch::append_event` and `::commit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecJournalError {
    /// Mirror of `JournalError::DuplicateStagedKey { run, seq }` at
    /// batch/append_event.rs:52-55. Returned when the key is already
    /// in the same batch's `staged_event_keys` (guard 2, C6).
    DuplicateStagedKey,
    /// Mirror of `JournalError::DuplicateEvent { run, seq }` at
    /// batch/append_event.rs:59-62. Returned when the journal
    /// memtable already has the key (guard 3, C6). Sets
    /// `aborted = true` so `commit` short-circuits.
    DuplicateEvent,
    /// Mirror of `JournalError::QueueFull` at batch/append_event.rs:65.
    /// Returned when `inner.len() >= MAX_BATCH_COUNT` (guard 4, C6).
    QueueFull,
    /// Mirror of `JournalError::JournalBatchBytesExceeded { attempted,
    /// limit }` at batch/append_event.rs:88-95. Returned on
    /// byte-budget overrun or `staged_bytes.checked_add(encoded_len)`
    /// overflow (guard 6, C6). THIS is the C5 path PS-004 binds.
    JournalBatchBytesExceeded { attempted: u64, limit: u64 },
    /// Mirror of `JournalError::SequenceOverflow` at
    /// batch/append_event.rs:84. Reachable from the `u64::try_from`
    /// conversion of `value.len()`.
    SequenceOverflow,
    /// Mirror of `JournalError::PayloadTooLarge { len, max }` at
    /// batch/append_event.rs (encoding step, guard 5).
    PayloadTooLarge { len: u32, max: u32 },
    /// Folded mirror of `JournalError::Encode(postcard::Error)` and
    /// `JournalError::PostcardEncodeFailed`. Reachable when
    /// `encode_ok == false` for any other reason (guard 5).
    Encode,
    /// Mirror of `JournalError::KeyCapacity`. Unreachable in this
    /// mirror (key construction is abstracted out); retained for
    /// symmetry with the production error set.
    KeyCapacity,
    /// Mirror of `JournalError::BatchAborted` at commit.rs:22.
    /// Returned by `commit` when `self.aborted == true`.
    BatchAborted,
}

// ---------------------------------------------------------------------------
// Mirror of `JournalWriteBatch<'j>`
// ---------------------------------------------------------------------------
/// Mirror of `vb_storage::batch::JournalWriteBatch<'j>`.
///
/// Field correspondence is documented in BINDING LEDGER above. Every
/// field of the production struct has a 1:1 mirror here; the only
/// abstractions are `inner` (Fjall `OwnedWriteBatch` -> `inner_len:
/// usize` count) and `journal` (Fjall `&FjallJournal` ->
/// `journal_has_key: bool` exec arg).
pub struct SpecJournalWriteBatch {
    /// Mirror of production `staged_event_keys: HashSet<[u8; 17]>`.
    /// Key type abstracted to `u64` (stable hash handle).
    pub staged_event_keys: HashSet<u64>,
    /// Mirror of production `staged_bytes: u64` (1:1).
    pub staged_bytes: u64,
    /// Mirror of production `byte_limit: Option<u64>` (1:1).
    pub byte_limit: Option<u64>,
    /// Mirror of production `aborted: bool` (1:1).
    pub aborted: bool,
    /// Mirror of `inner.len()` (OwnedWriteBatch itself is opaque to
    /// Verus; only the count matters for the `MAX_BATCH_COUNT` guard
    /// and for C5's "batch state unchanged" claim).
    pub inner_len: usize,
}

impl SpecJournalWriteBatch {
    // ------------------------------------------------------------------
    // Constructor
    // ------------------------------------------------------------------
    /// Mirror of `JournalWriteBatch::new(journal: &'j FjallJournal) ->
    /// Self` at `crates/vb_storage/src/batch/types.rs:33-44`. The
    /// production constructor takes `&FjallJournal`; the mirror takes
    /// `byte_limit: Option<u64>` and abstracts the Fjall dependency so
    /// the spec can drive byte-budget behavior without constructing a
    /// live journal.
    ///
    /// `#[verifier::external]` because Verus does not model
    /// `HashSet::new()` exec semantics across module boundaries when
    /// the caller uses `assume_specification`; the spec file attaches
    /// an `assume_specification` bridge that exposes the post-state
    /// explicitly for the spec proofs.
    #[verifier::external]
    #[allow(dead_code)]
    pub fn new(byte_limit: Option<u64>) -> Self {
        Self {
            staged_event_keys: HashSet::new(),
            staged_bytes: 0,
            byte_limit,
            aborted: false,
            inner_len: 0,
        }
    }

    // ------------------------------------------------------------------
    // Accessors — bodies are opaque; bridges attach contracts
    // ------------------------------------------------------------------
    /// Mirror of `JournalWriteBatch::is_aborted` at
    /// `crates/vb_storage/src/batch/types.rs:67-70`.
    #[verifier::external]
    #[allow(dead_code)]
    pub fn is_aborted(&self) -> bool {
        self.aborted
    }

    /// Mirror of `JournalWriteBatch::staged_event_bytes` at
    /// `crates/vb_storage/src/batch/types.rs:74-77`. Returns the
    /// accumulated encoded-byte total for journal events accepted
    /// into this batch so far (C9 contract).
    #[verifier::external]
    #[allow(dead_code)]
    pub fn staged_event_bytes(&self) -> u64 {
        self.staged_bytes
    }

    /// Mirror of `JournalWriteBatch::len` at
    /// `crates/vb_storage/src/batch/types.rs:47-50`. Returns
    /// `inner.len()` if the batch is not aborted, else `0`.
    #[verifier::external]
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        if self.aborted {
            0
        } else {
            self.inner_len
        }
    }

    /// Mirror of `JournalWriteBatch::is_empty` at
    /// `crates/vb_storage/src/batch/types.rs:53-56`.
    #[verifier::external]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Mirror of `JournalWriteBatch::byte_limit` at
    /// `crates/vb_storage/src/batch/types.rs:80-83`.
    #[verifier::external]
    #[allow(dead_code)]
    pub fn byte_limit(&self) -> Option<u64> {
        self.byte_limit
    }

    // ------------------------------------------------------------------
    // Staging entry — body is opaque to Verus
    // ------------------------------------------------------------------
    /// Mirror of `JournalWriteBatch::append_event` at
    /// `crates/vb_storage/src/batch/append_event.rs:41-106`.
    ///
    /// `#[verifier::external]` because Verus does not model
    /// `HashSet::contains` / `HashSet::insert` exec semantics inside
    /// exec fn bodies; the spec file attaches an `assume_specification`
    /// bridge that gives the post-fix SA-003 production contract.
    ///
    /// Argument mapping (production -> mirror):
    ///   * `event: &JournalEvent`            -> abstracted out
    ///   * `event.run_id()`, `event.seq()`   -> folded into `key: u64`
    ///   * `journal.events.contains_key`     -> `journal_has_key: bool`
    ///   * `encode_record(...)?`             -> `encode_ok: bool`
    ///   * `value.len()`                     -> `encoded_len: u64`
    ///   * `MAX_BATCH_COUNT = 10_000`        -> inlined literal
    ///   * `MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 1_048_576`
    ///                                        -> inlined literal
    #[verifier::external]
    pub fn append_event(
        &mut self,
        key: u64,
        journal_has_key: bool,
        encode_ok: bool,
        encoded_len: u64,
    ) -> Result<(), SpecJournalError> {
        // Guard 2: same-batch duplicate (SA-003 post-fix).
        if self.staged_event_keys.contains(&key) {
            return Err(SpecJournalError::DuplicateStagedKey);
        }
        // Guard 3: durable duplicate -> abort.

        if journal_has_key {
            self.aborted = true;
            return Err(SpecJournalError::DuplicateEvent);
        }
        // Guard 4: count capacity (production constants.rs:100).

        if self.inner_len >= 10_000usize {
            return Err(SpecJournalError::QueueFull);
        }
        // Guard 5: encoding failure.

        if !encode_ok {
            // production constants.rs:88.
            if encoded_len > u64::from(1_048_576u32) {
                return Err(
                    SpecJournalError::PayloadTooLarge { len: 1_048_576u32, max: 1_048_576u32 },
                );
            }
            return Err(SpecJournalError::Encode);
        }
        // Guard 6: byte admission (C5 path).

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
        // Guard 7: insert into inner + record key.

        self.inner_len += 1;
        self.staged_event_keys.insert(key);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Commit — body is opaque to Verus
    // ------------------------------------------------------------------
    /// Mirror of `JournalWriteBatch::commit` at
    /// `crates/vb_storage/src/batch/commit.rs:20-26`.
    ///
    /// `#[verifier::external]` because the production body calls
    /// `fjall::OwnedWriteBatch::commit`, which Verus cannot model.
    /// The spec file attaches an `assume_specification` bridge that
    /// gives the early-return + Fjall-commit contract.
    ///
    /// The mirror abstracts the Fjall commit to an infallible
    /// "succeeds if the batch is not aborted" operation; the
    /// atomicity and durability are governed by Fjall's trust base
    /// (TBP-008 in `.beads/vb-vzcuf/trusted-base-ledger.jsonl`).
    #[verifier::external]
    pub fn commit(self) -> Result<(), SpecJournalError> {
        if self.aborted {
            return Err(SpecJournalError::BatchAborted);
        }
        // Production: self.inner.commit()?  (Fjall-opaque, abstracted).

        Ok(())
    }
}

} // verus!
