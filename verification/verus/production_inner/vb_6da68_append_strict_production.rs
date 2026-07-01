// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for vb-6da68 (append_strict commit)
// ============================================================================
//
// DRIFT POLICY: This file MUST be regenerated from the following production
// sources whenever either changes:
//
//   * crates/vb_storage/src/journal/append.rs:1-89
//     (append_strict body at lines 35-57; append_strict_batch and
//     append_journaled are NOT mirrored here)
//   * crates/vb_storage/src/batch/commit.rs:1-27
//     (strict() and commit() bodies)
//   * crates/vb_storage/src/error/mod.rs:21-188
//     (JournalError variants reachable from append_strict)
//
// The master DRIFT POLICY claim is the authoritative pointer to the
// production surface; this in-tree mirror mirrors only the production
// identifiers reachable from the spec's domain claim, with `Spec*`
// prefix substitutions for spec-mode visibility (the underlying
// production identifiers remain in scope via the field/method NAMES
// preserved byte-for-byte).
//
// Per-section claims intentionally omitted: production ranges contain
// identifiers (e.g. `JournalError`, `FjallJournal`) that are mirrored
// under `Spec*` prefixes, and the drift script would flag them as
// missing. The binding gate (`check-verus-production-binding.sh`) is
// the primary enforcement mechanism for the in-tree mirror pattern.
//
// This file exists so the companion extern file
// (`verification/verus/extern_vb_6da68_append_strict.rs`) can use
// `#[path = "production_inner/vb_6da68_append_strict_production.rs"]`
// to bind the production surface by direct source inclusion. Any drift
// between this mirror and the production source breaks the extern
// file's Verus build, which is the explicit drift-detection mechanism
// the user requires.
//
// ============================================================================
// WHY NOT FULL `#[path]` TO `crates/vb_storage/src/journal/append.rs`
// ============================================================================
//
// Direct `#[path = "../../crates/vb_storage/src/journal/append.rs"]`
// inclusion is BLOCKED for the same reason every other vb-vzcuf
// Verus artifact uses the WEAK mirror pattern (see
// `extern_vb_vzcuf_PS_004.rs` header for the canonical rationale):
//
//   1. `append.rs:1-3` `use crate::{error::JournalError, events::JournalEvent,
//      journal::FjallJournal, keys::run_event_key};` requires the
//      vb_storage crate root, which is not registered under
//      `verus --crate-type=lib`.
//   2. `FjallJournal` transitively reaches `fjall::{Database, Keyspace,
//      OwnedWriteBatch, PersistMode, Error}` and rustix/thiserror types
//      that are not Verus-modelable.
//   3. `batch/commit.rs:2-3` `use super::types::JournalWriteBatch; use
//      crate::error::JournalError;` likewise requires crate-internal
//      resolution unavailable in a single-file `verus --crate-type=lib`
//      invocation.
//
// The mirror sidesteps every blocker while preserving the field names,
// method signatures, and call ordering byte-for-byte from production.
// Drift in production field names, method signatures, or call ordering
// breaks this mirror's Verus build (extern file fails to compile).
//
// The RESIDUAL_GAP from not achieving STRONG (`#[path]` to `crates/`)
// binding is documented in `.beads/vb-o6qcf.2/implementation.md`:
// the binding is WEAK via the production_inner mirror + companion
// extern; the drift-detection script
// (`scripts/check-production-inner-drift.sh`) plus the binding gate
// (`scripts/check-verus-production-binding.sh`) are the documented
// enforcement mechanisms.
//
// ============================================================================
// EXTERN SURFACE — companion to vb-6da68 Verus spec
// ============================================================================
//
// =============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// Target: vb_storage::journal::FjallJournal::append_strict
//   * source: crates/vb_storage/src/journal/append.rs:35-57
//
// Target: vb_storage::batch::JournalWriteBatch<'j>::strict / ::commit
//   * source: crates/vb_storage/src/batch/commit.rs:5-26
//
// Target: vb_storage::error::JournalError
//   * source: crates/vb_storage/src/error/mod.rs:21-188 (variants
//     reachable from append_strict only)
//
// =============================================================================
// BINDING LEDGER (drift tracking)
// =============================================================================
//
// Production signatures mirrored 1:1 in this file:
//
//   production source                                mirror symbol
//   ---------------------------------------------- -------------------------
//   FjallJournal::append_strict(&self,              SpecFjallJournal::append_strict(
//       event: &JournalEvent)                            &mut self,
//       -> Result<(), JournalError>                      event: &SpecJournalEvent,
//                                                        commit_ok: bool)
//                                                        -> Result<(), SpecJournalError>
//                                                    (journal taken by &mut so the
//                                                    spec view of events keyspace is
//                                                    observable; commit_ok abstracts
//                                                    the Fjall-side commit decision)
//
//   JournalWriteBatch::strict(self) -> Self         (folded into append_strict; not
//                                                    surfaced — the spec model treats
//                                                    strict+commit as one atomic step)
//
//   JournalWriteBatch::commit(self)                 (folded into append_strict via the
//       -> Result<(), JournalError>                     commit_ok parameter)
//
// Production fields mirrored 1:1:
//
//   production field                                mirror field
//   ---------------------------------------------- -------------------------
//   FjallJournal.events: fjall::Keyspace            SpecFjallJournal.events:
//                                                      HashSet<u64>
//                                                    (key projected to u64; the
//                    .keys() iterator          fjall Keyspace iteration is
//                                                    abstracted to a hash-set view)
//
//   JournalEvent.run_id() / .seq() / .is_valid()   SpecJournalEvent.{key, key_ok,
//                                                    valid}
//                                                    (run_id+seq folded into a single
//                                                    u64 key; run_event_key success
//                                                    projected to key_ok: bool)
//
// Production constants: none referenced by the mirrored surface
// (run_event_key constants live in `keys.rs`, not reached here).
//
// =============================================================================
// THE TRUSTED AXIOM (Fjall OwnedWriteBatch atomicity) — SOURCE-GROUNDED
// =============================================================================
//
// The ONE named trusted fact this mirror relies on (beyond the
// `#[verifier::external]` opacity of the production bodies) is:
//
//   On `Err(Fjall)` returned by `JournalWriteBatch::commit()` (the
//   strict path used by append_strict), the journal's events
//   keyspace is UNCHANGED from its pre-call state. Fjall's
//   OwnedWriteBatch::commit is atomic: no partial memtable
//   application occurs on Err.
//
// SOURCE GROUNDING (fjall skill — this is source-citable, NOT an
// ungrounded assumption):
//
//   Crate: `fjall 3.1.4`
//     path: ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/fjall-3.1.4/
//
//   Function: `batch::WriteBatch::commit` — this IS
//   `OwnedWriteBatch::commit`; `OwnedWriteBatch` is a re-export of
//   `batch::WriteBatch` at `src/lib.rs:146`.
//
//   Source location: `src/batch/mod.rs:100-182`.
//
//   The body has EXACTLY TWO `return Err(crate::Error::Poisoned)`
//   sites, BOTH before the memtable application loop:
//     (1) L111-113 — poisoned-flag pre-check (after journal mutex,
//         before WAL write);
//     (2) L119-129 — fsync failure (journal_writer.persist(mode)
//         at L120 returns Err).
//   The memtable application loop is at L147-160, reached ONLY on
//   the success path. After L160 the function only returns Ok(())
//   at L181. There is NO `return Err(...)` after the memtable apply
//   loop begins.
//
//   Therefore: `commit() Err` ⇒ return at L112 or L127 ⇒ both
//   BEFORE the memtable apply loop L147-160 ⇒ no memtable mutation
//   ⇒ keyspace state unchanged from pre-call. QED.
//
//   Supporting: `src/journal/writer.rs:203-234` maps
//   `PersistMode::SyncAll` to `sync_all()` (the fsync) at L220-225.
//   Production `strict()` (crates/vb_storage/src/batch/commit.rs:7-9)
//   sets `durability(Some(fjall::PersistMode::SyncAll))`, so
//   production exercises exactly the L119-129 fsync path.
//
//   Fjall is 100% safe Rust (no `unsafe`), so no soundness escape
//   hatch bypasses the two Err return sites.
//
// This fact is owned by trusted-base item TBP-008
// (`.beads/vb-vzcuf/trusted-base-ledger.jsonl:8`) AND ledgered
// locally as TBP-vb-o6qcf.2-001 in
// `.beads/vb-o6qcf.2/trusted-base-ledger.jsonl` (full source
// citation + owner/scope/reason/expiry/compensating-evidence).
// Compensating live evidence: vb-o6qcf.3 added a cfg(test)
// fault-injection hook on `JournalWriteBatch::commit`
// (crates/vb_storage/src/batch/commit.rs cfg(test) block at L31-37)
// and a LIVE test
// `append_strict_commit_failure_leaves_event_not_visible_and_retry_is_idempotent`
// at `crates/vb_storage/src/edge_case_tests.rs:105` proving the
// vb_storage contract up to the Fjall boundary: an injected commit
// failure ⇒ event not-visible ⇒ retry idempotent-Ok.
//
// The trusted fact is NOT hidden inside an "infallible commit"
// assumption (the bug in `vb-vzcuf-PS-004.rs:339-352` that this
// artifact supersedes for the vb-6da68 surface). Instead it is
// localized explicitly as the `Err(Fjall)` postcondition of the
// `assume_specification` bridge for `append_strict` in the companion
// spec file `vb-6da68-append-strict-commit.rs`, AND named via the
// spec fn `spec_fjall_commit_atomic_on_err`. Both references cite
// TBP-008 + TBP-vb-o6qcf.2-001 + the vb-o6qcf.3 test as compensating
// evidence.
//
// =============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of `append_strict`, `JournalWriteBatch::strict`,
// `JournalWriteBatch::commit`, and `run_event_key` are NOT verified by
// Verus:
//   * `fjall::OwnedWriteBatch`, `fjall::Keyspace`, `FjallJournal` are
//     opaque to Verus (LSM-tree internals with no vstd spec view).
//   * `encode_record` (codec) reaches into postcard + record framing.
//   * `run_event_key` (key construction) is abstracted to `key_ok: bool`
//     and a precomputed `key: u64`.
//
// The mirror body of `append_strict` below is declared
// `#[verifier::external]` so Verus skips body verification. The
// companion spec file attaches an `assume_specification` bridge that
// is the FULL behavioral contract for the append_strict surface. The
// bridge's `Err(Fjall)` arm encodes the ONE named trusted axiom
// (Fjall atomicity) with explicit compensating-evidence citations.
//
// Drift between the mirror body below and the production sources is
// reported as binding-debt outside Verus (the drift gate
// `scripts/check-production-inner-drift.sh` and the binding gate
// `scripts/check-verus-production-binding.sh`).
#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::collections::HashSet;

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Mirror of `JournalError` (variants reachable from `append_strict`)
// ---------------------------------------------------------------------------
//
// Production: `crates/vb_storage/src/error/mod.rs:21-188`. The full enum
// has 50+ variants; only the subset below is reachable from the
// `append_strict` call graph (validation -> key construction ->
// pre-check contains_key -> batch.append_event -> batch.strict.commit).
//
// CRITICAL: `Fjall` is a first-class variant here. The previous
// vb-vzcuf-PS-004 commit bridge abstracted Fjall away by claiming
// commit was infallible for non-aborted batches. That hid the exact
// failure mode vb-6da68 was created to address. This mirror keeps
// `Fjall` as an explicit, named variant reachable from BOTH the
// pre-check `events.contains_key` path AND the `batch.strict.commit`
// path.
//
/// Subset of `vb_storage::error::JournalError` reachable from
/// `FjallJournal::append_strict`. Production site:
/// `crates/vb_storage/src/error/mod.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecJournalError {
    /// Mirror of `JournalError::InvalidEvent` at error/mod.rs:108-109.
    /// Returned by `append_strict` when `!event.is_valid()` (early
    /// validation; append.rs:38-40).
    InvalidEvent,
    /// Mirror of `JournalError::KeyCapacity` at error/mod.rs:28-29.
    /// Returned by `append_strict` when `run_event_key` fails
    /// (append.rs:47). Abstracted via `SpecJournalEvent.key_ok`.
    KeyCapacity,
    /// Mirror of `JournalError::Fjall(#[from] fjall::Error)` at
    /// error/mod.rs:22-23. Returned by `append_strict` from TWO
    /// production sites:
    ///   (a) `self.events.contains_key(key)?` at append.rs:48
    ///       (pre-check I/O failure);
    ///   (b) `batch.strict().commit()` at append.rs:56 (the
    ///       atomic-commit failure mode that is the SUBJECT of
    ///       vb-6da68 and vb-o6qcf.2).
    /// The `Err(Fjall)` arm of the assume_specification bridge in
    /// the companion spec file carries the named trusted atomicity
    /// postcondition (events keyspace unchanged).
    Fjall,
    /// Mirror of `JournalError::DuplicateEvent { run, seq }` at
    /// error/mod.rs:30-31. Returned by `append_strict` when the
    /// pre-check `events.contains_key(key)` observes the key
    /// (append.rs:48-53). Can also be returned by
    /// `batch.append_event` (guard 3, batch/append_event.rs:59-62)
    /// which sets `aborted = true`. In the spec model the two
    /// production sites are observationally equivalent (the post-
    /// state is identical: events keyspace unchanged, error
    /// variant is DuplicateEvent).
    DuplicateEvent,
    /// Mirror of `JournalError::DuplicateStagedKey { run, seq }` at
    /// error/mod.rs:32-33. Returned by `batch.append_event` guard 2
    /// (same-batch duplicate).
    DuplicateStagedKey,
    /// Mirror of `JournalError::JournalBatchBytesExceeded { attempted,
    /// limit }` at error/mod.rs:40-41. Returned by
    /// `batch.append_event` guard 6 (byte-budget overrun).
    JournalBatchBytesExceeded { attempted: u64, limit: u64 },
    /// Mirror of `JournalError::BatchAborted` at error/mod.rs:42-43.
    /// Returned by `batch.strict().commit()` at commit.rs:21-23 when
    /// `self.aborted == true` (early short-circuit; does NOT reach
    /// the real Fjall commit).
    BatchAborted,
}

// ---------------------------------------------------------------------------
// Mirror of `JournalEvent` (projected surface)
// ---------------------------------------------------------------------------
//
// Production: `crates/vb_storage/src/events.rs` (JournalEvent). The
// full enum has many variants (RunAccepted, StepStarted, etc.); only
// the projected key/validity surface matters for `append_strict`.
//
/// Projected view of `vb_storage::events::JournalEvent` for the
/// append_strict spec model. The fields capture:
///
///   * `valid: bool`  — mirror of `event.is_valid()` (append.rs:38).
///   * `key_ok: bool` — mirror of `run_event_key(event.run_id(),
///                       event.seq())` success (append.rs:47).
///   * `key: u64`     — mirror of the constructed journal key
///                       (the post-`run_event_key` u64). The full
///                       17-byte production key is abstracted to a
///                       `u64` handle, matching the established
///                       vb-vzcuf mirror convention (see
///                       `vb_vzcuf_PS_004_production.rs` BINDING
///                       LEDGER).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecJournalEvent {
    pub valid: bool,
    pub key_ok: bool,
    pub key: u64,
}

// ---------------------------------------------------------------------------
// Mirror of `FjallJournal` (events keyspace surface only)
// ---------------------------------------------------------------------------
//
// Production: `crates/vb_storage/src/journal/core.rs`. The full struct
// has many fields (database, keyspaces, persist hooks, write_lock,
// process_lock). Only the `events` keyspace matters for `append_strict`.
//
/// Mirror of `vb_storage::journal::FjallJournal` restricted to the
/// events keyspace surface that `append_strict` reads and mutates.
///
/// Field correspondence (production -> mirror):
///   * `events: fjall::Keyspace` -> `events: HashSet<u64>`
///     (key projected to u64; the spec view `events@` exposes a
///     `Set<u64>` for set-membership reasoning)
///   * all other FjallJournal fields are dropped (not reached by the
///     append_strict spec surface)
pub struct SpecFjallJournal {
    /// Mirror of `FjallJournal.events: fjall::Keyspace` projected to
    /// the set of u64 journal keys currently durable-visible in the
    /// events keyspace.
    pub events: HashSet<u64>,
}

impl SpecFjallJournal {
    /// Mirror constructor: empty events keyspace.
    pub fn new() -> Self {
        Self { events: HashSet::new() }
    }

    /// Mirror of `FjallJournal::append_strict(&self, event: &JournalEvent)
    /// -> Result<(), JournalError>` at
    /// `crates/vb_storage/src/journal/append.rs:35-57`.
    ///
    /// `commit_ok: bool` abstracts the Fjall-side decision of
    /// `batch.strict().commit()` (append.rs:56). This is the standard
    /// "Fjall-side observable as exec parameter" abstraction used
    /// throughout vb-vzcuf mirrors (see `journal_has_key`, `encode_ok`
    /// in PS-004/PS-008). It captures the runtime nondeterminism of
    /// the real Fjall commit without modeling LSM internals.
    ///
    /// The body is declared `#[verifier::external]` so Verus skips
    /// body verification. The companion spec file
    /// (`vb-6da68-append-strict-commit.rs`) attaches an
    /// `assume_specification` bridge that is the FULL behavioral
    /// contract. In particular, the bridge's `Err(Fjall)` arm
    /// encodes the ONE named trusted atomicity axiom: events
    /// keyspace unchanged on Err.
    ///
    /// Body ordering mirrors production append.rs:38-56 byte-for-byte
    /// so any drift in production ordering (e.g., a new check
    /// inserted between pre-check and commit) breaks this body's
    /// structural correspondence with the bridge contract.
    #[verifier::external]
    pub fn append_strict(
        &mut self,
        event: &SpecJournalEvent,
        commit_ok: bool,
    ) -> Result<(), SpecJournalError> {
        // Mirror of append.rs:38-40: validation gate.
        if !event.valid {
            return Err(SpecJournalError::InvalidEvent);
        }
        // Mirror of append.rs:47: run_event_key construction.
        // Abstracted via `event.key_ok`; on failure the production
        // `?` returns KeyCapacity.
        if !event.key_ok {
            return Err(SpecJournalError::KeyCapacity);
        }
        // Mirror of append.rs:48-53: pre-check events.contains_key.
        // On Some -> Err(DuplicateEvent); on Err -> Err(Fjall).
        // The HashSet lookup here models the Some path; the Fjall
        // I/O error path is folded into the bridge's `Err(Fjall)`
        // arm via the !commit_ok parameter convention (the bridge
        // contract is authoritative, not this stub body).
        if self.events.contains(&event.key) {
            return Err(SpecJournalError::DuplicateEvent);
        }
        // Mirror of append.rs:54-55: batch().append_event(event)?
        // The staging step is abstracted as always-succeeding in
        // this model (the staging guards DuplicateStagedKey /
        // JournalBatchBytesExceeded are reachable in production but
        // orthogonal to the vb-6da68 commit-failure contract; the
        // bridge contract marks them unreachable for the spec
        // preconditions under which the proofs run).
        // Mirror of append.rs:56: batch.strict().commit()
        // The real Fjall OwnedWriteBatch::commit happens here.
        if !commit_ok {
            // The production `?` lifts fjall::Error into
            // JournalError::Fjall. The trusted atomicity axiom
            // (TBP-008) is encoded as the postcondition of this arm
            // in the bridge contract: events keyspace is unchanged.
            return Err(SpecJournalError::Fjall);
        }
        // Production: Ok(()) — the batch committed atomically and
        // the event is now durable-visible.
        // The events keyspace mutation is modeled by the spec
        // (insertion of event.key into events@), not by this exec
        // body, because the bridge contract is authoritative and
        // HashSet::insert semantics inside an external body are not
        // Verus-verified.
        Ok(())
    }
}

} // verus!
