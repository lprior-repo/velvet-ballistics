// SPDX-License-Identifier: MIT
//
// Extern surface for vb-vzcuf-PS-007 Verus spec.
//
// =============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// This file binds `verification/verus/vb-vzcuf-PS-007.rs` to the
// production `max_journal_batch_bytes` bridge between vb_core (policy)
// and vb_storage (batch limit). The binding is structural +
// contractual: production types and accessors are mirrored with the
// SAME field names and signatures, and each mirrored exec fn is
// declared `#[verifier::external]` with a body that mirrors the
// production source byte-for-byte. The `assume_specification` bridges
// in the companion spec file state the production behavior.
//
// =============================================================================
// BINDING LEDGER — production ↔ mirror ↔ spec
// =============================================================================
//
// Core policy surface (vb_core):
//   - ResourceContract.max_journal_batch_bytes: u32
//       crates/vb_core/src/workflow/mod.rs:225
//       ResourceContract::DEFAULT.max_journal_batch_bytes = 1_048_576
//       crates/vb_core/src/workflow/mod.rs:249
//       -> mirrored as SpecResourceContract::default_max_journal_batch_bytes
//
//   - BoundednessPolicy.absolute_max_journal_batch_bytes: u32
//       crates/vb_core/src/budget.rs:366
//       BoundednessPolicy::DEFAULT.absolute_max_journal_batch_bytes = 1_048_576
//       crates/vb_core/src/budget.rs:391
//       -> mirrored as SpecBoundednessPolicy::default_absolute_max_journal_batch_bytes
//
//   - MAX_JOURNAL_BATCH_BYTES (hard cap): u32 = 16_777_216
//       crates/vb_core/src/limits.rs:130
//       Used at crates/vb_core/src/engine/validate.rs:98-102 to
//       reject contracts exceeding the hard cap, and at
//       crates/vb_core/src/validation/resource.rs:33 via
//       `validate_nonzero_u32`.
//       -> mirrored as spec_max_journal_batch_bytes_hard_cap()
//
// Storage batch surface (vb_storage):
//   - DEFAULT_JOURNAL_BATCH_BYTE_LIMIT: u64 = 1_048_576
//       crates/vb_storage/src/batch/types.rs:10
//       -> mirrored as spec_default_journal_batch_byte_limit()
//
//   - JournalWriteBatch::new(journal: &'j FjallJournal) -> Self
//       crates/vb_storage/src/batch/types.rs:33-44
//       Production body sets `byte_limit: Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT)`
//       (line 41), `staged_event_keys: HashSet::new()` (line 38),
//       `aborted: false` (line 39), `staged_bytes: 0` (line 40).
//       -> mirrored as SpecJournalWriteBatchByDefault::new()
//
//   - JournalWriteBatch::byte_limit(&self) -> Option<u64>
//       crates/vb_storage/src/batch/types.rs:80-83
//       Production body returns `self.byte_limit` unchanged.
//       -> mirrored as SpecJournalWriteBatchByDefault::byte_limit
//
//   - JournalWriteBatch::staged_event_bytes(&self) -> u64
//       crates/vb_storage/src/batch/types.rs:74-77
//       Production body returns `self.staged_bytes` unchanged.
//       -> mirrored as SpecJournalWriteBatchByDefault::staged_event_bytes
//
//   - JournalWriteBatch::is_aborted(&self) -> bool
//       crates/vb_storage/src/batch/types.rs:67-70
//       Production body returns `self.aborted` unchanged.
//       -> mirrored as SpecJournalWriteBatchByDefault::is_aborted
//
// The constant `1_048_576` (1 MiB) appears at the three production
// sites `workflow/mod.rs:249`, `budget.rs:391`, and `batch/types.rs:10`.
// The bridge invariant is that the core policy default equals the
// storage default equals the max payload size — a single shared 1 MiB
// byte-policy value across the core/storage boundary.
//
// =============================================================================
// TRUST BOUNDARY
// =============================================================================
// Every `#[verifier::external]` mirror body is NOT verified by Verus.
// The contracts attached via `assume_specification` in the companion
// spec file `vb-vzcuf-PS-007.rs` are the truth source. Drift between
// the mirror body and the production source is reported as
// binding-debt outside Verus. Specifically:
//   * `BoundednessPolicy::DEFAULT` and `ResourceContract::DEFAULT`
//     literals in production `const` items trigger Verus's
//     `VerusErasureCtxt has not been initialized` panic when included
//     via `#[path]`. The mirror uses regular `fn` methods returning
//     the literal values instead.
//   * `JournalWriteBatch::new` requires a `&'j FjallJournal` argument
//     which is opaque to Verus. The mirror abstracts this away (the
//     journal handle has no role in the byte-policy bridge).
//   * Each mirrored `#[verifier::external]` body is opaque to Verus
//     and trusted to mirror the production body byte-for-byte.
#![forbid(unsafe_code)]
#![allow(dead_code)]

// ---------------------------------------------------------------------------
// Mirror: vb_core ResourceContract (byte-policy subset)
// ---------------------------------------------------------------------------

/// Mirror of `vb_core::workflow::ResourceContract` (subset).
///
/// Production struct definition at
/// `crates/vb_core/src/workflow/mod.rs:217-228`. Only the
/// `max_journal_batch_bytes: u32` field is mirrored because PS-007's
/// domain claim is exclusively about this single byte-policy field.
/// The remaining fields (max_steps, max_blob_bytes, etc.) are out of
/// scope for the core/storage byte-policy bridge.
pub struct SpecResourceContract {
    /// Mirror of `ResourceContract::max_journal_batch_bytes: u32`
    /// at `crates/vb_core/src/workflow/mod.rs:225`.
    pub max_journal_batch_bytes: u32,
}

impl SpecResourceContract {
    /// Mirror of `ResourceContract::DEFAULT.max_journal_batch_bytes
    /// == 1_048_576` at `crates/vb_core/src/workflow/mod.rs:249`.
    ///
    /// The full `ResourceContract::DEFAULT` struct literal spans
    /// lines 232-251 of `crates/vb_core/src/workflow/mod.rs`; the
    /// byte-policy field is the one relevant to PS-007. The body
    /// mirrors the literal value at line 249 byte-for-byte.
    #[verifier::external]
    pub fn default_max_journal_batch_bytes() -> u32 {
        1_048_576u32
    }
}

// ---------------------------------------------------------------------------
// Mirror: vb_core BoundednessPolicy (byte-policy subset)
// ---------------------------------------------------------------------------

/// Mirror of `vb_core::budget::BoundednessPolicy` (subset).
///
/// Production struct definition at `crates/vb_core/src/budget.rs:
/// 339-375`. Only the `absolute_max_journal_batch_bytes: u32` field
/// is mirrored because PS-007's bridge also asserts the absolute
/// hard cap from policy matches the storage default.
pub struct SpecBoundednessPolicy {
    /// Mirror of `BoundednessPolicy::absolute_max_journal_batch_bytes
    /// : u32` at `crates/vb_core/src/budget.rs:366`.
    pub absolute_max_journal_batch_bytes: u32,
}

impl SpecBoundednessPolicy {
    /// Mirror of `BoundednessPolicy::DEFAULT
    /// .absolute_max_journal_batch_bytes == 1_048_576` at
    /// `crates/vb_core/src/budget.rs:391`.
    ///
    /// The full `BoundednessPolicy::DEFAULT` struct literal spans
    /// lines 379-... of `crates/vb_core/src/budget.rs`; the
    /// byte-policy field is the one relevant to PS-007. The body
    /// mirrors the literal value at line 391 byte-for-byte.
    #[verifier::external]
    pub fn default_absolute_max_journal_batch_bytes() -> u32 {
        1_048_576u32
    }
}

// ---------------------------------------------------------------------------
// Mirror: vb_core hard cap constant
// ---------------------------------------------------------------------------

/// Mirror of `MAX_JOURNAL_BATCH_BYTES: u32 = 16_777_216` at
/// `crates/vb_core/src/limits.rs:130`.
///
/// Used by `validate_resource_contract` at
/// `crates/vb_core/src/engine/validate.rs:98-102` to reject
/// contracts exceeding the hard cap. Body mirrors the literal
/// value at line 130 byte-for-byte.
#[verifier::external]
pub fn spec_max_journal_batch_bytes_hard_cap() -> u32 {
    16_777_216u32
}

// ---------------------------------------------------------------------------
// Mirror: vb_storage DEFAULT_JOURNAL_BATCH_BYTE_LIMIT
// ---------------------------------------------------------------------------

/// Mirror of `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT: u64 = 1_048_576`
/// at `crates/vb_storage/src/batch/types.rs:10`.
///
/// Body returns the literal value at line 10 byte-for-byte.
#[verifier::external]
pub fn spec_default_journal_batch_byte_limit() -> u64 {
    1_048_576u64
}

// ---------------------------------------------------------------------------
// Mirror: vb_storage JournalWriteBatch (byte-policy subset)
// ---------------------------------------------------------------------------

/// Mirror of `vb_storage::batch::JournalWriteBatch<'j>` (byte-policy
/// subset).
///
/// Production struct definition at
/// `crates/vb_storage/src/batch/types.rs:21-30`.
///
/// Fields mirrored:
///   * `byte_limit: Option<u64>`     — production line 28
///   * `staged_bytes: u64`           — production line 27
///   * `aborted: bool`               — production line 26
///
/// Fields NOT mirrored (out of PS-007 scope):
///   * `inner: fjall::OwnedWriteBatch`     — line 22, opaque to Verus
///   * `journal: &'j FjallJournal`         — line 23, opaque to Verus
///   * `staged_event_keys: HashSet<[u8; 17]>` — line 25, covered by
///     PS-009 (duplicate accounting); out of scope for PS-007 (byte
///     policy bridge).
///   * `_not_send_or_sync: PhantomData<*mut FjallJournal>` — line 29,
///     a `!Send + !Sync` marker with no semantic content for the
///     byte-policy bridge.
pub struct SpecJournalWriteBatchByDefault {
    /// Mirror of production `byte_limit: Option<u64>` at types.rs:28.
    pub byte_limit: Option<u64>,
    /// Mirror of production `staged_bytes: u64` at types.rs:27.
    pub staged_bytes: u64,
    /// Mirror of production `aborted: bool` at types.rs:26.
    pub aborted: bool,
}

impl SpecJournalWriteBatchByDefault {
    /// Mirror of `JournalWriteBatch::new(journal: &'j FjallJournal)`
    /// at `crates/vb_storage/src/batch/types.rs:33-44`.
    ///
    /// The production constructor signature takes `&'j FjallJournal`
    /// which is opaque to Verus. The mirror abstracts the journal
    /// input away — it has no semantic role in PS-007 (the byte limit
    /// is the same regardless of the journal handle) — and returns a
    /// freshly-constructed batch whose `byte_limit` is
    /// `Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT)`.
    ///
    /// Body mirrors the relevant subset of the production body:
    ///   byte_limit:    Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT),  // line 41
    ///   staged_bytes:  0,                                       // line 39
    ///   aborted:       false,                                   // line 40
    /// The production `inner` / `journal` / `staged_event_keys` /
    /// `_not_send_or_sync` fields are dropped (see struct doc).
    #[verifier::external]
    pub fn new() -> Self {
        Self {
            byte_limit: Some(1_048_576u64),
            staged_bytes: 0u64,
            aborted: false,
        }
    }

    /// Mirror of `JournalWriteBatch::byte_limit(&self) -> Option<u64>`
    /// at `crates/vb_storage/src/batch/types.rs:80-83`.
    ///
    /// Production body returns `self.byte_limit` unchanged.
    #[verifier::external]
    pub fn byte_limit(&self) -> Option<u64> {
        self.byte_limit
    }

    /// Mirror of `JournalWriteBatch::staged_event_bytes(&self) -> u64`
    /// at `crates/vb_storage/src/batch/types.rs:74-77`.
    ///
    /// Production body returns `self.staged_bytes` unchanged.
    #[verifier::external]
    pub fn staged_event_bytes(&self) -> u64 {
        self.staged_bytes
    }

    /// Mirror of `JournalWriteBatch::is_aborted(&self) -> bool`
    /// at `crates/vb_storage/src/batch/types.rs:67-70`.
    ///
    /// Production body returns `self.aborted` unchanged.
    #[verifier::external]
    pub fn is_aborted(&self) -> bool {
        self.aborted
    }
}
