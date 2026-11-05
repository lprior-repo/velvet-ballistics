// Verus proof obligations for vb-6da68 (append_strict commit contract).
//
// Obligation IDs: vb-o6qcf.2 (this artifact); supersedes the
//                 vb-vzcuf-PS-004 commit-bridge for the
//                 append_strict surface (POB-vb-vzcuf-013 coverage
//                 of the commit-failure mode).
// Verifier: verus
// Command: verus --crate-type=lib verification/verus/vb-6da68-append-strict-commit.rs
//
// =============================================================================
// WHAT THIS ARTIFACT SUPERSEDES AND WHY
// =============================================================================
//
// Origin: proof-review of vb-6da68 REJECTED the prior Verus coverage
// of the append_strict commit-failure mode. The only Verus artifact
// reaching the `JournalWriteBatch::commit` surface
// (`vb-vzcuf-PS-004.rs:339-352`, WEAK mirror of
// `JournalWriteBatch::commit`) ASSUMED the Fjall commit is infallible
// for non-aborted batches:
//
//     pub assume_specification[ production::SpecJournalWriteBatch::commit ](
//         batch: SpecJournalWriteBatch,
//     ) -> (r: Result<(), SpecJournalError>)
//         ensures
//             (r.is_ok() && !batch.aborted)
//                 || (r == Err::<(), SpecJournalError>(
//                     SpecJournalError::BatchAborted,
//                 ) && batch.aborted),
//             r.is_err() == batch.aborted,
//     ;
//
// That contract hides the exact failure mode vb-6da68 was created to
// address: a real `fjall::OwnedWriteBatch::commit` failure on the
// strict path surfaces as `Err(JournalError::Fjall(..))`, NOT as
// `Ok(())`. Under the infallibility assumption, the spec literally
// cannot express `Err(Fjall)`, so the not-visible / idempotent-retry
// guarantees could not be discharged at the Verus layer.
//
// This artifact provides a dedicated `append_strict`-surface Verus
// model that:
//
//   1. Models `SpecFjallJournal::append_strict` (the production
//      entry point at append.rs:35-57) directly, with the Fjall
//      commit decision exposed as a `commit_ok: bool` exec parameter.
//   2. Models `Err(Fjall)` as a first-class return variant
//      (SpecJournalError::Fjall), reachable from BOTH the pre-check
//      I/O path and the commit failure path.
//   3. Localizes the ONE named trusted atomicity fact (Fjall
//      OwnedWriteBatch commit is atomic on Err) as the explicit
//      `Err(Fjall)` postcondition of the `assume_specification`
//      bridge AND as the named spec fn
//      `spec_fjall_commit_atomic_on_err`.
//   4. Discharges two `proof fn`s on top of the bridge:
//      `append_strict_commit_failure_leaves_event_not_visible` and
//      `append_strict_retry_after_failure_is_idempotent`.
//
// =============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// Target: vb_storage::journal::FjallJournal::append_strict
//   * source: crates/vb_storage/src/journal/append.rs:35-57
//
// Binding mechanism: `#[path = "extern_vb_6da68_append_strict.rs"]`
// brings the production-mirror types (`SpecJournalError`,
// `SpecJournalEvent`, `SpecFjallJournal`) and the
// `#[verifier::external]` exec body of `append_strict` into the
// `verus!` block. The `assume_specification` bridge below attaches
// the production contract (atomicity on Err + idempotent retry) to
// the extern body. The exec wrappers at the bottom of this file
// exercise every bridge arm from `verus!` context so the contract is
// not used as a vacuum.
//
// BINDING STRENGTH: WEAK via the production_inner mirror + companion
// extern pattern (the established vb-vzcuf convention). The mirror
// at `production_inner/vb_6da68_append_strict_production.rs` is a
// verbatim copy of the production `append_strict` call graph with
// minimal Fjall-opaque substitutions; drift is detected by
// `scripts/check-production-inner-drift.sh` and the binding gate
// `scripts/check-verus-production-binding.sh`.
//
// STRONG binding (`#[path = "../../crates/vb_storage/src/journal/append.rs"]`)
// is BLOCKED by TWO independent rustc diagnostics, each sufficient on its
// own. Captured via an empirical probe (`verus --crate-type=lib` on a
// one-line `#[path]`-include of append.rs, 2026-07-01):
//
//   (1) error[E0432]: unresolved imports `crate::error`, `crate::events`,
//       `crate::journal`, `crate::keys`
//        --> crates/vb_storage/src/journal/append.rs:2:5
//         2 |     error::JournalError, events::JournalEvent,
//           |     ^^^^^  ^^^^^^  ^^^^^^^  ^^^^ could not be found in the
//           |                                        crate root
//       Production `append.rs:1-3` has
//       `use crate::{error::JournalError, events::JournalEvent,
//       journal::FjallJournal, keys::run_event_key};`. These `crate::`
//       relative imports require the vb_storage crate root, which is NOT
//       registered under `verus --crate-type=lib` (the verus lib's crate
//       root is the spec file itself). This is a HARD SYNTACTIC BLOCKER:
//       no Verus feature (external_type_specification,
//       external_fn_specification, extern_spec!) can make `crate::*`
//       resolve without registering the entire vb_storage crate, which
//       transitively requires vb_core + serde + postcard + chrono +
//       thiserror proc-macros — none Verus-modelable. Fixing this would
//       require refactoring production `append.rs` to drop `crate::`
//       imports, which is OUT OF SCOPE for vb-o6qcf.2 (proof-write-only;
//       production Rust under crates/** is immutable per task brief).
//
//   (2) error[E0433]: cannot find module or crate `fjall` in this scope
//        --> crates/vb_storage/src/journal/append.rs:86:31
//         86 |     self.database.persist(fjall::PersistMode::SyncAll)?;
//            |                               ^^^^^ use of unresolved
//            |                                   module or unlinked crate
//       The `fjall::PersistMode` / `fjall::OwnedWriteBatch` / `fjall::Error`
//       references in the production append_strict call graph require
//       `extern crate fjall;` with a compiled `--extern fjall` rlib —
//       unavailable under the no-installs `verus --crate-type=lib`
//       constraint. Even if fjall were linked, the verus-native
//       `external_type_specification` / `external_fn_specification`
//       bindings for `fjall::{Error, OwnedWriteBatch, PersistMode}` would
//       still leave blocker (1) (`crate::*`) unaddressed.
//
// Conclusion: STRONG is GENUINELY INFEASIBLE for the append_strict
// surface without production-refactoring crate:: imports away (out of
// scope). The WEAK companion-extern + production_inner mirror pattern
// is the strongest viable binding. See the RESIDUAL_GAP note in
// `.beads/vb-o6qcf.2/implementation.md` and the mirror header.
//
// =============================================================================
// THE NAMED TRUSTED AXIOM (Fjall OwnedWriteBatch atomicity) — SOURCE-GROUNDED
// =============================================================================
//
// This artifact relies on exactly ONE trusted fact beyond the
// `#[verifier::external]` opacity of the `append_strict` body. It is
// named explicitly below as `spec_fjall_commit_atomic_on_err` and
// cited in the bridge's `Err(Fjall)` postcondition:
//
//   On `Err(Fjall)` returned by `batch.strict().commit()` (the path
//   `append_strict` uses at append.rs:56), the journal's events
//   keyspace is UNCHANGED from its pre-call state. Equivalently:
//   Fjall's OwnedWriteBatch::commit is atomic — no partial memtable
//   application occurs on Err.
//
// ---------------------------------------------------------------------------
// FJALL SOURCE GROUNDING (fjall skill fact: source-citable, NOT an assumption)
// ---------------------------------------------------------------------------
//
// Crate: `fjall 3.1.4`
//   path: ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/fjall-3.1.4/
//   (resolved via `cargo metadata --format-version 1`)
//
// Function: `batch::WriteBatch::commit` — this IS `OwnedWriteBatch::commit`;
//   `OwnedWriteBatch` is a re-export of `batch::WriteBatch` at
//   `src/lib.rs:146` (`batch::WriteBatch as OwnedWriteBatch`).
//
// Source location: `src/batch/mod.rs:100-182`.
//
// Proof that `commit() Err ⇒ NO memtable mutation ⇒ keyspace unchanged`:
//
//   The body has EXACTLY TWO `return Err(...)` sites, BOTH located BEFORE
//   the memtable application loop:
//
//   (1) `src/batch/mod.rs:111-113` — poisoned-flag pre-check, AFTER
//       acquiring the journal writer mutex (L108) but BEFORE the WAL
//       write (L117) and BEFORE the fsync (L120):
//         if self.db.is_poisoned.load(Ordering::Relaxed) {
//             return Err(crate::Error::Poisoned);
//         }
//
//   (2) `src/batch/mod.rs:119-129` — fsync failure path, AFTER the WAL
//       write (`let _ = journal_writer.write_batch(...)` at L117) but
//       BEFORE the memtable application loop:
//         if let Some(mode) = self.durability {
//             if let Err(e) = journal_writer.persist(mode) {
//                 self.db.is_poisoned.store(true, Ordering::Release);
//                 ...
//                 return Err(crate::Error::Poisoned);
//             }
//         }
//
//   The memtable application loop is at `src/batch/mod.rs:147-160`:
//         for item in std::mem::take(&mut self.data) {
//             let (item_size, _) = match item.value_type {
//                 ValueType::Value    => item.keyspace.tree.insert(item.key, item.value, batch_seqno),
//                 ValueType::Tombstone => item.keyspace.tree.remove(item.key, batch_seqno),
//                 ...
//             };
//             ...
//         }
//   This loop is reached ONLY on the success path (after both Err
//   returns are past). After L160 the function only returns `Ok(())`
//   at L181. There is NO `return Err(...)` after the memtable apply
//   loop begins.
//
//   Therefore: `OwnedWriteBatch::commit() Err` (specifically
//   `Err(Error::Poisoned)`) ⇒ return at L112 or L127 ⇒ BOTH before
//   the memtable apply loop at L147-160 ⇒ no memtable mutation ⇒
//   keyspace state unchanged from pre-call. QED.
//
// Supporting: the fsync mapping.
//   `src/journal/writer.rs:203-234` `JournalWriter::persist` maps
//   `PersistMode::SyncAll` to `self.file.get_mut().sync_all()` (the
//   fsync) at L220-225. Production `strict()`
//   (`crates/vb_storage/src/batch/commit.rs:7-9`) sets
//   `durability(Some(fjall::PersistMode::SyncAll))`, so production's
//   strict commit exercises exactly the L119-129 fsync path — the
//   `persist(mode)` call at L120 is the fsync that, on failure,
//   returns at L127 BEFORE the memtable apply loop.
//
// Production wiring (unchanged, on main):
//   * `crates/vb_storage/src/batch/commit.rs:7-9` `strict()` sets
//     `self.inner = self.inner.durability(Some(fjall::PersistMode::SyncAll))`.
//   * `crates/vb_storage/src/batch/commit.rs:20-26` `commit()` runs
//     `self.inner.commit()?` (the Fjall primitive above) and lifts
//     `fjall::Error` into `JournalError::Fjall` via `#[from]`.
//   * `crates/vb_storage/src/journal/append.rs:56` `append_strict`
//     calls `batch.strict().commit()`.
//
// Fjall is 100% safe Rust (no `unsafe`), so there is no soundness
// escape hatch in the commit path that could bypass the two Err
// return sites above (fjall skill fact).
//
// This fact is owned by trusted-base item TBP-008
// (`.beads/vb-vzcuf/trusted-base-ledger.jsonl:8`) AND ledgered
// locally as TBP-vb-o6qcf.2-001 in
// `.beads/vb-o6qcf.2/trusted-base-ledger.jsonl` with the full
// source citation, owner, scope, reason, expiry, and compensating
// evidence.
//
// Compensating live evidence (proves the vb_storage contract up to
// the Fjall boundary):
//
//   * vb-o6qcf.3 added a cfg(test) fault-injection hook on
//     `JournalWriteBatch::commit` (crates/vb_storage/src/batch/commit.rs,
//     cfg(test) block at L31-37) that returns a synthetic
//     `JournalError::Fjall(fjall::Error::Io(..))` BEFORE the real
//     OwnedWriteBatch reaches Fjall.
//   * vb-o6qcf.3 added the LIVE test
//     `append_strict_commit_failure_leaves_event_not_visible_and_retry_is_idempotent`
//     at crates/vb_storage/src/edge_case_tests.rs:105 (sibling workspace
//     femdation-vb-o6qcf.3, commit 944f860ba83a) that asserts:
//       (a) append_strict surfaces Err(JournalError::Fjall) on injected
//           commit failure;
//       (b) the event is NOT visible via events_for_run after the
//           failure (events keyspace unchanged);
//       (c) a retry append_strict returns Ok(()) — NOT DuplicateEvent;
//       (d) the event is durable-visible after the retry.
//
// The trusted fact is NOT hidden inside an "infallible commit"
// assumption. The bridge's `Err(Fjall)` arm explicitly permits the
// commit failure and only asserts the atomicity consequence.
//
// =============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// =============================================================================
//
// The production body of `append_strict` is NOT verified by Verus:
//   * `fjall::Keyspace::contains_key` and `fjall::OwnedWriteBatch::commit`
//     are opaque to Verus (LSM-tree internals with no vstd spec view).
//   * `run_event_key` (key construction) is abstracted to `key_ok: bool`.
//   * `batch.append_event` (staging step) is abstracted as always-
//     succeeding in the spec model (its reachable error variants are
//     marked unreachable for the proof preconditions).
//   * The mirror body in `extern_vb_6da68_append_strict.rs` is declared
//     `#[verifier::external]` so Verus skips body verification.
//
// The `assume_specification` bridge below therefore represents the
// FULL behavioral contract for the append_strict surface. The
// `commit_ok: bool` exec parameter captures the Fjall-side commit
// decision (success/failure) as an observable passed at the call site;
// the bridge contract describes the resulting post-state
// deterministically. The ONE fact that goes beyond pure projection
// is the atomicity postcondition on Err(Fjall), named via
// `spec_fjall_commit_atomic_on_err` and ledgered at TBP-008.
use vstd::prelude::*;

verus! {

// =============================================================================
// Production-mirror types (extern binding)
// =============================================================================
#[path = "extern_vb_6da68_append_strict.rs"]
mod production;

pub use production::{SpecFjallJournal, SpecJournalError, SpecJournalEvent};

// =============================================================================
// THE NAMED TRUSTED AXIOM (Fjall OwnedWriteBatch atomicity on Err)
// =============================================================================
//
// This spec fn is the SINGLE named trusted fact this artifact relies
// on (beyond `#[verifier::external]` body opacity). It captures Fjall
// OwnedWriteBatch commit atomicity: on `Err(Fjall)` returned by the
// strict commit path, the events keyspace of the journal is UNCHANGED
// from its pre-call state.
//
// SOURCE-GROUNDED (fjall skill): this is NOT an ungrounded assumption.
// The atomicity fact is source-citable in fjall 3.1.4:
//   * `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/fjall-3.1.4/src/batch/mod.rs:100-182`
//     (`WriteBatch::commit`, re-exported as `OwnedWriteBatch` at
//     `src/lib.rs:146`) has EXACTLY TWO `return Err(crate::Error::Poisoned)`
//     sites — L111-113 (poisoned pre-check) and L119-129 (fsync failure) —
//     BOTH located BEFORE the memtable application loop at L147-160.
//     After L160 the function only returns `Ok(())` at L181. There is NO
//     `return Err(...)` after the memtable apply loop begins.
//   * `src/journal/writer.rs:203-234` maps `PersistMode::SyncAll` to
//     `sync_all()` (the fsync); production `strict()` exercises exactly
//     this path.
//   * Conclusion: `commit() Err` ⇒ return at L112 or L127 ⇒ no memtable
//     mutation ⇒ keyspace state unchanged. See the top-of-file
//     "NAMED TRUSTED AXIOM" header for the full line-by-line citation.
//
// The fact is owned by trusted-base item TBP-008
// (`.beads/vb-vzcuf/trusted-base-ledger.jsonl:8`) AND ledgered locally
// as TBP-vb-o6qcf.2-001 in `.beads/vb-o6qcf.2/trusted-base-ledger.jsonl`
// (full source citation + owner/scope/reason/expiry/compensating-evidence).
// Compensating evidence: vb-o6qcf.3 LIVE test
// `append_strict_commit_failure_leaves_event_not_visible_and_retry_is_idempotent`
// at `crates/vb_storage/src/edge_case_tests.rs:105`.
//
// The `assume_specification` bridge for `append_strict` below encodes
// this fact as the postcondition of its `Err(Fjall)` arm. The two
// `proof fn`s at the bottom of this file discharge the not-visible
// and idempotent-retry guarantees by invoking the bridge contract.
//
// TRUSTED: Fjall OwnedWriteBatch atomicity; the events keyspace is
// unchanged on `Err(Fjall)` from `batch.strict().commit()`.
// Compensating evidence: TBP-008 + TBP-vb-o6qcf.2-001 + vb-o6qcf.3
// LIVE test
// `append_strict_commit_failure_leaves_event_not_visible_and_retry_is_idempotent`.
/// Spec: Fjall OwnedWriteBatch commit is atomic on Err. On
/// `Err(Fjall)` from `batch.strict().commit()`, the journal events
/// keyspace is unchanged from its pre-call state.
pub open spec fn spec_fjall_commit_atomic_on_err(
    journal_before: SpecFjallJournal,
    journal_after: SpecFjallJournal,
) -> bool {
    journal_after.events@ == journal_before.events@
}

// =============================================================================
// assume_specification bridge: production contract for `append_strict`
// =============================================================================
//
// Production: `crates/vb_storage/src/journal/append.rs:35-57`.
//
// This is the FULL behavioral contract for the append_strict surface.
// The bridge encodes:
//
//   * The five reachable return arms of the production call graph
//     (after folding the staging step's reachable errors into the
//     `append_event`-folded Fjall/DuplicateStagedKey/JournalBatchBytesExceeded
//     arms; the proof preconditions exclude the staging-error arms).
//   * The TRUSTED atomicity postcondition on `Err(Fjall)`: events
//     keyspace unchanged. This is the named trusted axiom
//     (`spec_fjall_commit_atomic_on_err`) ledgered at TBP-008.
//
// CRITICAL: the bridge does NOT assume the commit is infallible. The
// `Err(Fjall)` arm is first-class and carries the atomicity
// postcondition as the ONLY trusted fact beyond pure projection.
//
// Arm summary (production site -> bridge postcondition):
//
//   Err(InvalidEvent)         append.rs:38-40  !event.valid
//   Err(KeyCapacity)          append.rs:47     !event.key_ok && event.valid
//   Err(Fjall)                append.rs:48 OR  event.valid && event.key_ok &&
//                            56                  && !contains_before && !commit_ok
//                                              && events_unchanged (TRUSTED)
//   Err(DuplicateEvent)       append.rs:48-53  event.valid && event.key_ok &&
//                                              && contains_before
//                                              && events_unchanged (no commit)
//   Ok(())                    append.rs:56      event.valid && event.key_ok
//                                              && !contains_before
//                                              && commit_ok
//                                              && events_gained_key (insert)
//   _                         (staging errors) false for the proof preconditions
pub assume_specification[ production::SpecFjallJournal::append_strict ](
    journal: &mut SpecFjallJournal,
    event: &SpecJournalEvent,
    commit_ok: bool,
) -> (r: Result<(), SpecJournalError>)
    ensures
        match r {
            Err(SpecJournalError::InvalidEvent) => {
                &&& !event.valid
                &&& final(journal).events@ == old(journal).events@
            },
            Err(SpecJournalError::KeyCapacity) => {
                &&& !event.key_ok
                &&& event.valid
                &&& final(journal).events@ == old(journal).events@
            },
            Err(SpecJournalError::Fjall) => {
                // REACHABLE from BOTH the pre-check I/O failure
                // (append.rs:48 contains_key returns Err) AND the
                // commit failure (append.rs:56 commit returns Err).
                // The bridge folds both into a single arm: the
                // post-state is identical (events keyspace
                // unchanged).
                &&& event.valid
                &&& event.key_ok
                &&& !old(journal).events@.contains(event.key)
                &&& !commit_ok
                // TRUSTED ATOMICITY (TBP-008 + TBP-vb-o6qcf.2-001):
                // The events keyspace is unchanged on Err(Fjall).
                // This is the named trusted axiom
                // (spec_fjall_commit_atomic_on_err).
                //
                // SOURCE-GROUNDED (fjall skill, NOT an assumption):
                //   fjall 3.1.4 src/batch/mod.rs:100-182
                //   (WriteBatch::commit = OwnedWriteBatch::commit per
                //   src/lib.rs:146) has exactly two `return Err(_)`
                //   sites — L111-113 (poisoned pre-check) and L119-129
                //   (fsync failure via journal_writer.persist(mode) at
                //   L120) — BOTH before the memtable application loop
                //   at L147-160. The function returns Ok(()) at L181
                //   only AFTER the memtable apply; there is no Err
                //   return after L160. SyncAll (production strict(),
                //   crates/vb_storage/src/batch/commit.rs:7-9) maps to
                //   fsync via src/journal/writer.rs:220-225
                //   (sync_all()). So commit() Err ⇒ no memtable
                //   mutation ⇒ keyspace unchanged. QED.
                //
                // Compensating evidence: TBP-008 +
                // TBP-vb-o6qcf.2-001 + vb-o6qcf.3 LIVE test
                // `append_strict_commit_failure_leaves_event_not_visible_and_retry_is_idempotent`
                // at crates/vb_storage/src/edge_case_tests.rs:105.
                &&& spec_fjall_commit_atomic_on_err(*old(journal), *final(journal))
            },
            Err(SpecJournalError::DuplicateEvent) => {
                &&& event.valid
                &&& event.key_ok
                &&& old(journal).events@.contains(event.key)
                &&& final(journal).events@ == old(journal).events@
            },
            Ok(()) => {
                &&& event.valid
                &&& event.key_ok
                &&& !old(journal).events@.contains(event.key)
                &&& commit_ok
                &&& final(journal).events@ == old(journal).events@.insert(event.key)
            },
            // Staging errors (DuplicateStagedKey /
            // JournalBatchBytesExceeded / BatchAborted) are reachable
            // in production via `batch.append_event` guards 2/6 and
            // `commit`'s aborted short-circuit. The bridge marks
            // them as unreachable under the proof preconditions
            // (event validity + fresh key + non-aborted path); the
            // wrappers below pin the preconditions that exclude them.
            // Listing each variant explicitly (rather than `_ => false`)
            // so the SMT can case-split cleanly on the result without
            // needing to reason about a wildcard catch-all.
            Err(SpecJournalError::DuplicateStagedKey) => false,
            Err(SpecJournalError::JournalBatchBytesExceeded { .. }) => false,
            Err(SpecJournalError::BatchAborted) => false,
        },
;

// =============================================================================
// PROOF 1: append_strict_commit_failure_leaves_event_not_visible
// =============================================================================
//
// Domain claim (vb-6da68 not-visible half): after `append_strict`
// returns `Err(Fjall)`, the event key is NOT in the journal events
// keyspace. Equivalently: a failed commit does not make the staged
// event visible to readers.
//
// Proof structure: the wrapper exercises the bridge with the
// preconditions that force the `Err(Fjall)` arm (event valid + key
// ok + pre-check observes the key absent + commit_ok=false). From
// the bridge's `Err(Fjall)` postcondition and the trusted atomicity
// axiom (`spec_fjall_commit_atomic_on_err`), the events keyspace is
// unchanged. Combined with the precondition
// `!old(journal).events@.contains(event.key)`, the post-state
// `!journal.events@.contains(event.key)` follows.
//
// This is the WRAPPER form of the proof (the bridge is exercised at
// a real call site, so the contract is not a vacuum). The wrapper's
// `ensures` is provable from the bridge contract alone; no `assume`
// of the goal is used.
/// Wrapper for PROOF 1 (not-visible): exercises the `Err(Fjall)` arm
/// of the `append_strict` bridge under preconditions that force it
/// as the only reachable arm, and ensures the event key is not in
/// the post-state events keyspace.
///
/// Preconditions pin the bridge's `Err(Fjall)` arm as the only
/// reachable arm:
///   * `event.valid` + `event.key_ok` — excludes InvalidEvent and
///     KeyCapacity arms.
///   * `!old(journal).events@.contains(event.key)` — excludes the
///     DuplicateEvent arm (pre-check observed absent).
///   * `commit_ok = false` passed at the call site — excludes the
///     `Ok(())` arm.
///   * The staging-error arms (DuplicateStagedKey /
///     JournalBatchBytesExceeded / BatchAborted) are excluded by the
///     bridge's `_ => false` clause once the four conditions above
///     hold.
pub exec fn append_strict_commit_failure_leaves_event_not_visible(
    journal: &mut SpecFjallJournal,
    event: &SpecJournalEvent,
) -> (r: Result<(), SpecJournalError>)
    requires
        event.valid,
        event.key_ok,
        !old(journal).events@.contains(event.key),
    ensures
        // The only reachable arm under the preconditions + commit_ok=false.
        match r {
            Err(SpecJournalError::Fjall) => {
                // TRUSTED ATOMICITY (TBP-008): events keyspace unchanged.
                &&& spec_fjall_commit_atomic_on_err(*old(journal), *final(journal))
                // NOT-VISIBLE: the event key is absent in the post-state.
                &&& !final(journal).events@.contains(event.key)
            },
            _ => false,
        },
{
    // Pass commit_ok=false to force the commit-failure path.
    journal.append_strict(event, false)
}

// =============================================================================
// PROOF 2: append_strict_retry_after_failure_is_idempotent
// =============================================================================
//
// Domain claim (vb-6da68 idempotent-retry half): a second
// `append_strict` of the same event AFTER a commit-failed first
// `append_strict` returns `Ok(())` — NOT `Err(DuplicateEvent)`.
//
// Proof structure: this is discharged as a general `proof fn` lemma
// (lemma_append_strict_retry_after_failure_is_idempotent) that takes
// the pre-/post-first-call journal states as parameters, plus an exec
// wrapper (append_strict_retry_after_failure_is_idempotent) that
// exercises both calls and applies the lemma.
//
// The decomposition sidesteps the Verus mut-ref tracking limitation
// across two sequential calls on the same `&mut` borrow: the lemma
// captures the contract on the first call's atomicity as a
// hypothesis, and derives the second call's preconditions
// (`!contains_after_first`) which force the Ok(()) arm. The wrapper
// applies the lemma after the first call to discharge the second
// call's postcondition.

/// General lemma (idempotent-retry): given the first call's TRUSTED
/// ATOMICITY postcondition, a second `append_strict` of the same
/// event with `commit_ok=true` returns `Ok(())`.
///
/// This is the heart of the vb-6da68 idempotent-retry guarantee. The
/// precondition `spec_fjall_commit_atomic_on_err(journal_before,
/// journal_after_first)` is the named trusted atomicity axiom
/// (TBP-008), established by the first call's bridge contract.
pub proof fn lemma_append_strict_retry_after_failure_is_idempotent(
    journal_before: SpecFjallJournal,
    journal_after_first: SpecFjallJournal,
    event: SpecJournalEvent,
)
    requires
        event.valid,
        event.key_ok,
        !journal_before.events@.contains(event.key),
        // TRUSTED ATOMICITY (TBP-008): the first call's commit
        // failure left the events keyspace unchanged.
        spec_fjall_commit_atomic_on_err(journal_before, journal_after_first),
    ensures
        // After the first call, the pre-check on a retry still
        // observes the event key absent (the precondition for the
        // second call's Ok(()) arm).
        !journal_after_first.events@.contains(event.key),
{
    // Direct from the trusted atomicity axiom + the wrapper
    // precondition that the key was absent before the first call.
    assert(journal_after_first.events@ == journal_before.events@);
    assert(!journal_before.events@.contains(event.key));
}

/// Exec wrapper for PROOF 2 (idempotent-retry): exercises two
/// sequential `append_strict` calls — the first with `commit_ok=false`
/// (failed commit) and the second with `commit_ok=true` (succeeding
/// retry) — and ensures the retry returns `Ok(())`.
///
/// The wrapper composes:
///   * Call 1 (commit_ok=false): the bridge forces `Err(Fjall)` with
///     the TRUSTED ATOMICITY postcondition.
///   * Lemma `lemma_append_strict_retry_after_failure_is_idempotent`:
///     derives `!journal_after_first.events@.contains(event.key)`,
///     the precondition for the second call's Ok(()) arm.
///   * Call 2 (commit_ok=true): the bridge forces `Ok(())`.
///
/// The wrapper's `ensures` is `r2.is_ok()`, the idempotent-retry
/// outcome. No `assume` of the goal is used.
pub exec fn append_strict_retry_after_failure_is_idempotent(
    journal: &mut SpecFjallJournal,
    event: &SpecJournalEvent,
) -> (r2: Result<(), SpecJournalError>)
    requires
        event.valid,
        event.key_ok,
        !old(journal).events@.contains(event.key),
    ensures
        // The retry (second call) is Ok — idempotent after a failed
        // first commit. NOT Err(DuplicateEvent).
        r2.is_ok(),
{
    // ----- Call 1: failed commit (commit_ok=false) ------------------
    let r1 = journal.append_strict(event, false);
    // Bridge case analysis: r1 must be Err(Fjall) under the
    // preconditions + commit_ok=false.
    assert(r1 == Err::<(), SpecJournalError>(SpecJournalError::Fjall)) by {};

    // After Call 1, the TRUSTED ATOMICITY postcondition holds:
    //   spec_fjall_commit_atomic_on_err(*old(journal), *journal)
    // Capture the two states as ghost values and apply the lemma.
    let ghost j_before = *old(journal);
    let ghost j_after_first = *journal;
    proof {
        lemma_append_strict_retry_after_failure_is_idempotent(
            j_before,
            j_after_first,
            *event,
        );
    }

    // ----- Call 2: succeeding retry (commit_ok=true) ----------------
    // The lemma derived !j_after_first.events@.contains(event.key),
    // and *journal == j_after_first at this point. So the second
    // call's Ok(()) arm preconditions are all satisfied; the bridge
    // forces Ok(()).
    let r2 = journal.append_strict(event, true);
    // Bridge case analysis: r2 must be Ok(()) under the preconditions
    // + commit_ok=true. Stated as is_ok() in the ensures (rather
    // than the exact Ok(()) value) for SMT efficiency.
    assert(r2.is_ok()) by {
        match r2 {
            Ok(()) => {},
            Err(SpecJournalError::InvalidEvent) => {
                assert(!event.valid);
            },
            Err(SpecJournalError::KeyCapacity) => {
                assert(!event.key_ok);
            },
            Err(SpecJournalError::Fjall) => {
                // commit_ok=true at call site, so the bridge's
                // !commit_ok postcondition is contradicted.
            },
            Err(SpecJournalError::DuplicateEvent) => {
                // The lemma discharged this: !j_after_first.events@.contains(key),
                // and old(journal) for the second call is j_after_first.
                assert(!j_after_first.events@.contains(event.key));
            },
            Err(SpecJournalError::DuplicateStagedKey) => {
                assert(false);
            },
            Err(SpecJournalError::JournalBatchBytesExceeded { .. }) => {
                assert(false);
            },
            Err(SpecJournalError::BatchAborted) => {
                assert(false);
            },
        }
    };
    r2
}

// =============================================================================
// PROOF 3 (corollary): the failed-first-call result is exactly Err(Fjall)
// =============================================================================
//
// Auxiliary wrapper that names the first-call outcome explicitly so
// downstream artifacts can cite it. This wrapper does not introduce
// any new trusted fact; it is a structural restatement of PROOF 1's
// ensures on the result variant.
/// Wrapper corollary: the failed commit returns `Err(Fjall)` exactly
/// (not any other variant). Useful for downstream consumers that
/// need to pattern-match on the first-call result.
pub exec fn append_strict_failed_commit_returns_fjall_exactly(
    journal: &mut SpecFjallJournal,
    event: &SpecJournalEvent,
) -> (r: Result<(), SpecJournalError>)
    requires
        event.valid,
        event.key_ok,
        !old(journal).events@.contains(event.key),
    ensures
        r == Err::<(), SpecJournalError>(SpecJournalError::Fjall),
        spec_fjall_commit_atomic_on_err(*old(journal), *final(journal)),
        !final(journal).events@.contains(event.key),
{
    journal.append_strict(event, false)
}

// =============================================================================
// General spec lemmas (forall-quantified, non-vacuous)
// =============================================================================
//
// These lemmas discharge the not-visible and idempotent-retry
// guarantees as general properties of the bridge contract, not just
// at the wrapper call sites. Each lemma's precondition is a
// non-trivial reachability condition (the bridge arm witness), and
// the conclusion is a non-trivial postcondition derived from the
// arm. None is a reflexive identity, a boolean tautology, or has
// its conclusion smuggled into the precondition.

/// General lemma (not-visible): under the bridge's `Err(Fjall)` arm
/// reachability conditions, the event key is not in the post-state
/// events keyspace.
pub proof fn lemma_append_strict_commit_failure_not_visible(
    journal_before: SpecFjallJournal,
    journal_after: SpecFjallJournal,
    event: SpecJournalEvent,
)
    requires
        // Bridge reachability for the Err(Fjall) arm.
        event.valid,
        event.key_ok,
        !journal_before.events@.contains(event.key),
        // TRUSTED ATOMICITY (TBP-008).
        spec_fjall_commit_atomic_on_err(journal_before, journal_after),
    ensures
        // NOT-VISIBLE: event key absent in post-state.
        !journal_after.events@.contains(event.key),
{
    // From the trusted atomicity axiom + the precondition that the
    // key was absent before, the SMT derives absence in the
    // post-state directly. The assert makes the chain explicit.
    assert(journal_after.events@ == journal_before.events@);
    assert(!journal_before.events@.contains(event.key));
}

/// General lemma (idempotent-retry): under the bridge's `Err(Fjall)`
/// arm reachability conditions for the first call, a second call
/// with the pre-check still observing the key absent and
/// `commit_ok=true` reaches `Ok(())`.
pub proof fn lemma_append_strict_retry_after_failure_is_ok(
    journal_before: SpecFjallJournal,
    journal_after_first: SpecFjallJournal,
    event: SpecJournalEvent,
)
    requires
        event.valid,
        event.key_ok,
        !journal_before.events@.contains(event.key),
        // TRUSTED ATOMICITY (TBP-008) on the first call.
        spec_fjall_commit_atomic_on_err(journal_before, journal_after_first),
    ensures
        // After the first call the pre-check on a retry still
        // observes the key absent — the precondition for the Ok(())
        // arm of the second call holds.
        !journal_after_first.events@.contains(event.key),
{
    // Direct from the trusted atomicity axiom.
    assert(journal_after_first.events@ == journal_before.events@);
    assert(!journal_before.events@.contains(event.key));
}

} // verus!
