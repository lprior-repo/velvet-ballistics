// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SHIM for vb-6da68 (append_strict commit) Verus spec
// ============================================================================
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance) — companion extern
// pattern recognized by `scripts/check-verus-production-binding.sh`.
//
// This file is a thin shim. It does NOT contain the production mirror
// itself; instead it `#[path]`-includes the in-tree mirror at
// `verification/verus/production_inner/vb_6da68_append_strict_production.rs`.
//
// The production-binding gate recognises this file as the WEAK
// companion extern pattern: the spec file at
// `verification/verus/vb-6da68-append-strict-commit.rs`
// `#[path]`-includes this shim, and this shim in turn `#[path]`-includes
// the in-tree mirror, which is the production surface that the binding
// gate audit walks to.
//
// The in-tree mirror is a verbatim copy of the production
// `FjallJournal::append_strict` (crates/vb_storage/src/journal/append.rs:35-57)
// + `JournalWriteBatch::{strict, commit}` (batch/commit.rs:5-26) +
// `JournalError` variants reachable from `append_strict`
// (error/mod.rs:21-188), with minimal substitutions (spec-mode type
// aliases like `SpecJournalError` / `SpecJournalEvent` /
// `SpecFjallJournal`, Fjall internals replaced by a `HashSet<u64>`
// projection of the events keyspace, and the Fjall commit decision
// exposed as an exec parameter `commit_ok: bool`). Drift between the
// mirror and the production source is tracked by
// `scripts/check-production-inner-drift.sh`.
//
// Why WEAK rather than STRONG: the production `FjallJournal::append_strict`
// transitively reaches `fjall::{Keyspace, OwnedWriteBatch, PersistMode,
// Error}`, `crate::{error::JournalError, events::JournalEvent,
// journal::FjallJournal, keys::run_event_key}`, and the postcard codec.
// None of these are Verus-modelable in a single-file
// `verus --crate-type=lib` invocation. The mirror sidesteps every
// blocker while preserving the field names, method signatures, and
// call ordering byte-for-byte (see the mirror's BINDING LEDGER), so
// any drift in production field names, method signatures, or call
// ordering breaks the `extern_vb_6da68_append_strict` Verus build.
//
// The RESIDUAL_GAP from not achieving STRONG binding is documented in
// `.beads/vb-o6qcf.2/implementation.md`: the binding is WEAK via the
// production_inner mirror + this companion extern. Upgrading to STRONG
// would require either (a) registering the vb_storage crate root and
// Fjall + vb_core as Verus extern crates (currently blocked by
// `verus --crate-type=lib` no-installs constraint and proc-macro
// dependencies), or (b) refactoring production `append.rs`/`commit.rs`
// to split the pure decision logic away from Fjall-touching I/O.
//
// The `assume_specification` bridge in the companion spec file
// (`vb-6da68-append-strict-commit.rs`) attaches the production
// contract to the `SpecFjallJournal::append_strict` exec fn, and the
// exec wrappers in that file exercise the bridge from `verus!`
// context, so the bridge is not used as a vacuum.
//
// ============================================================================
// THE NAMED TRUSTED AXIOM (visibility ledger) — SOURCE-GROUNDED
// ============================================================================
//
// This extern surface exposes exactly ONE trusted fact beyond the
// `#[verifier::external]` opacity of the `append_strict` body: the
// Fjall OwnedWriteBatch atomicity postcondition encoded in the
// bridge's `Err(Fjall)` arm. See the companion spec file
// (`vb-6da68-append-strict-commit.rs`) `spec_fjall_commit_atomic_on_err`
// and the `assume_specification` bridge for the exact statement.
//
// SOURCE GROUNDING (fjall skill — source-citable, NOT an assumption):
//   fjall 3.1.4 src/batch/mod.rs:100-182 (WriteBatch::commit, re-exported
//   as OwnedWriteBatch at src/lib.rs:146) has exactly two `return Err(_)`
//   sites — L111-113 (poisoned pre-check) and L119-129 (fsync failure
//   via journal_writer.persist(mode) at L120) — BOTH before the memtable
//   application loop at L147-160. The function returns Ok(()) at L181
//   only AFTER the memtable apply; there is no Err return after L160.
//   SyncAll (production strict()) maps to fsync via
//   src/journal/writer.rs:220-225 (sync_all()). So commit() Err ⇒ no
//   memtable mutation ⇒ keyspace unchanged. QED.
//
// Compensating evidence:
//   * .beads/vb-vzcuf/trusted-base-ledger.jsonl TBP-008 (upstream owner)
//   * .beads/vb-o6qcf.2/trusted-base-ledger.jsonl TBP-vb-o6qcf.2-001
//     (local ledger: owner/scope/reason/fjall-source-citation/expiry/
//      compensating-evidence)
//   * vb-o6qcf.3 LIVE test
//     `append_strict_commit_failure_leaves_event_not_visible_and_retry_is_idempotent`
//     at crates/vb_storage/src/edge_case_tests.rs:105
//
// The trusted fact is NOT an "infallible commit" assumption (the bug
// in vb-vzcuf-PS-004.rs:339-352 that this artifact supersedes). It
// explicitly permits `Err(Fjall)` and only asserts atomicity on Err.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/vb_6da68_append_strict_production.rs`. The mirror
// is the production surface for the `append_strict` call graph (see
// header above). Any drift in the mirror or in the production source
// breaks this Verus build.
//
// Note on the inner module name `production_inner`: this avoids the
// name collision with the outer `production` module that the spec
// file expects. The `pub use` re-exports below surface the spec-mode
// type aliases at the top level of this extern file so the spec
// file's `assume_specification[ production::SpecFjallJournal::append_strict ]`
// and similar references resolve correctly.
#[path = "production_inner/vb_6da68_append_strict_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types so the spec file can reference them
// as `production::SpecJournalError` etc. The `pub use` is at the top
// level so `production::SpecFjallJournal::append_strict` resolves
// through the spec file's `mod production` declaration.
pub use production_inner::{SpecFjallJournal, SpecJournalError, SpecJournalEvent};
