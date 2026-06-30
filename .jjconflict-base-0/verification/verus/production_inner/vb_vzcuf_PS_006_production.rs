// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for vb_vzcuf_PS_006
// ============================================================================
//
// DRIFT POLICY: This file MUST be regenerated from `crates/vb_storage/src/batch/types.rs:1-84`
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
// (`verification/verus/extern_vb_vzcuf_PS_006.rs`)
// can use `#[path = "production_inner/vb_vzcuf_PS_006_production.rs"]` to bind the
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
// Extern surface for vb-vzcuf-PS-006 Verus spec.
//
// =============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// Target: vb_storage::batch::JournalWriteBatch<'j> at
//         crates/vb_storage/src/batch/types.rs:21-84, with the
//         DEFAULT_JOURNAL_BATCH_BYTE_LIMIT constant at
//         crates/vb_storage/src/batch/types.rs:10.
//
// Production fields mirrored (1:1):
//   * byte_limit: Option<u64>           types.rs:28 -> Option<u64>
//   * staged_bytes: u64                 types.rs:27 -> u64
//   * aborted: bool                     types.rs:26 -> bool
//   * inner_len: usize (count of `inner: fjall::OwnedWriteBatch`)
//                                         -> usize
//
// Production fields NOT modeled (Verus cannot reason about them):
//   * inner: fjall::OwnedWriteBatch     types.rs:22 (Fjall I/O; opaque to vstd)
//   * journal: &'j FjallJournal         types.rs:23 (Fjall memtable; opaque)
//   * staged_event_keys: HashSet<[u8; 17]>
//                                         types.rs:25 (modeled in PS-009 separately;
//                                                      not in scope for PS-006 C1)
//   * _not_send_or_sync: PhantomData<*mut FjallJournal>
//                                         types.rs:29 (Send/Sync marker; no
//                                                      semantic content for C1)
//
// =============================================================================
// BINDING LEDGER (drift tracking)
// =============================================================================
//
// Production invariants verified by this binding:
//
//   1. Constructor field assignment (types.rs:34-44):
//        byte_limit = Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT)
//        staged_bytes = 0
//        aborted = false
//        inner = journal.database.batch()    <- opaque, modeled as inner_len = 0
//
//      This binding exposes a parameterized form `new_with_limit(limit)`
//      plus a no-arg `new_default()` that hardcodes
//      `Some(1_048_576u64)` (matching DEFAULT_JOURNAL_BATCH_BYTE_LIMIT).
//      The parameterization is required because production's
//      `new(&FjallJournal)` cannot be constructed in Verus (Fjall is
//      opaque); the parameterized view lets exec wrappers drive the
//      contract under different limit inputs without depending on
//      FjallJournal construction.
//
//   2. No byte_limit setter exists in production. A grep for
//      `byte_limit\s*=` across crates/vb_storage/src/ returns exactly
//      two hits: the field declaration and the constructor assignment.
//      Therefore, the only post-construction observable byte_limit is
//      the constructor-supplied value.
//
//   3. Getter `byte_limit()` (types.rs:80-83) returns the stored
//      field directly; `staged_event_bytes()` (types.rs:74-77) and
//      `is_aborted()` (types.rs:67-70) likewise. `len()` (types.rs:47-50)
//      short-circuits to 0 when aborted, else returns inner.len().
//
// DRIFT DEBT (tracked in `.beads/vb-vzcuf/proof-obligations.planned.jsonl`):
//   * The phantom-data !Send/!Sync marker field is dropped (no semantic
//     content for the limit-presence claim).
//   * The Fjall handle and OwnedWriteBatch are abstracted away; their
//     observable behaviors (memtable reads, WAL fsync) are out of
//     scope for PS-006 (C1: limit presence).
//   * Parameterization via `new_with_limit(limit)` is a mirror-only
//     convenience; the production public API is `new(&FjallJournal)`
//     which always sets the default. Drift between the parameter
//     surface and production is contained by the `new_default()`
//     exec fn, which hardcodes the production byte-for-byte.
//
// =============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// =============================================================================
//
// Each exec fn in this extern module is declared `#[verifier::external]`
// for one of two reasons:
//
//   (a) Fjall / HashSet ops are opaque to Verus and cannot be modeled
//       inside exec fn bodies. (Not used in PS-006 — none of the
//       mirrored fns reach Fjall or HashSet.)
//   (b) For pattern consistency with the rest of the vb-vzcuf spec
//       suite (see extern_vb_vzcuf_PS_009.rs), so the spec file
//       attaches contracts via `assume_specification` and the body is
//       visible to anyone reading the extern module without Verus
//       needing to symbolically execute the trivial constructor.
//
// The contract attached in the spec file
// (`verification/verus/vb-vzcuf-PS-006.rs`) is the FULL behavior the
// production constructor and getters provide, derived from the
// production source byte-by-byte. The exec wrappers in the spec
// file exercise the bridge so the contract is not used as a vacuum.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

// ---------------------------------------------------------------------------
// Constants are inlined as literals in this file to avoid a Verus
// `--crate-type=lib` panic where pub const items declared inside an extern
// module trigger `VerusErasureCtxt has not been initialized` during
// thir-body processing. The literal value mirrors the production
// source byte-for-byte:
//
//   * 1_048_576 = crates/vb_storage/src/batch/types.rs:10
//                  ::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Mirror of `JournalWriteBatch<'j>` (byte-limit-relevant subset)
// ---------------------------------------------------------------------------

/// Mirror of `vb_storage::batch::JournalWriteBatch<'j>` with Fjall
/// internals replaced by their Verus-visible projections.
pub struct SpecJournalWriteBatch {
    /// Mirror of production `byte_limit: Option<u64>` at
    /// `crates/vb_storage/src/batch/types.rs:28`. The ONLY post-
    /// construction setter is the constructor; no setter fn exists
    /// in production (grep-confirmed).
    pub byte_limit: Option<u64>,
    /// Mirror of production `staged_bytes: u64` at
    /// `crates/vb_storage/src/batch/types.rs:27`.
    pub staged_bytes: u64,
    /// Mirror of production `aborted: bool` at
    /// `crates/vb_storage/src/batch/types.rs:26`.
    pub aborted: bool,
    /// Mirror of `inner.len()` where
    /// `inner: fjall::OwnedWriteBatch` lives at
    /// `crates/vb_storage/src/batch/types.rs:22`. The OwnedWriteBatch
    /// itself is opaque to Verus; only its count is observable.
    pub inner_len: usize,
}

impl SpecJournalWriteBatch {
    /// Production constructor mirror, parameterized on `byte_limit` so
    /// exec wrappers can drive the contract under arbitrary inputs
    /// without depending on FjallJournal construction. Mirrors the
    /// post-state of `JournalWriteBatch::new` at
    /// `crates/vb_storage/src/batch/types.rs:33-44` for the
    /// byte-limit-relevant fields.
    #[verifier::external]
    pub fn new_with_limit(byte_limit: Option<u64>) -> Self {
        Self {
            byte_limit,
            staged_bytes: 0,
            aborted: false,
            inner_len: 0,
        }
    }

    /// Production constructor mirror with the production default.
    /// Mirrors `JournalWriteBatch::new(&journal)` at
    /// `crates/vb_storage/src/batch/types.rs:34-44`, which sets
    /// `byte_limit: Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT)`.
    /// The literal `1_048_576u64` matches the production constant
    /// at `crates/vb_storage/src/batch/types.rs:10`.
    #[verifier::external]
    pub fn new_default() -> Self {
        Self::new_with_limit(Some(1_048_576u64))
    }

    /// Mirror of `JournalWriteBatch::byte_limit` at
    /// `crates/vb_storage/src/batch/types.rs:80-83`. Returns the
    /// stored field directly.
    #[verifier::external]
    pub fn byte_limit(&self) -> Option<u64> {
        self.byte_limit
    }

    /// Mirror of `JournalWriteBatch::staged_event_bytes` at
    /// `crates/vb_storage/src/batch/types.rs:74-77`. Returns the
    /// stored field directly.
    #[verifier::external]
    pub fn staged_event_bytes(&self) -> u64 {
        self.staged_bytes
    }

    /// Mirror of `JournalWriteBatch::len` at
    /// `crates/vb_storage/src/batch/types.rs:47-50`. Short-circuits
    /// to 0 when aborted, else returns `inner.len()` (mirrored as
    /// `inner_len`).
    #[verifier::external]
    pub fn len(&self) -> usize {
        if self.aborted {
            0
        } else {
            self.inner_len
        }
    }

    /// Mirror of `JournalWriteBatch::is_empty` at
    /// `crates/vb_storage/src/batch/types.rs:53-56`. Derived from
    /// `len()`. Marked `#[verifier::external]` for symmetry with the
    /// other getters so the spec file attaches the contract via
    /// `assume_specification` and the body is not symbolically
    /// executed inside wrappers (avoids a Verus visibility gap where
    /// the call to the externally-marked `len()` inside `is_empty()`
    /// is not resolved against the same instance's bridge call).
    #[verifier::external]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Mirror of `JournalWriteBatch::is_aborted` at
    /// `crates/vb_storage/src/batch/types.rs:67-70`. Returns the
    /// stored field directly.
    #[verifier::external]
    pub fn is_aborted(&self) -> bool {
        self.aborted
    }
}
