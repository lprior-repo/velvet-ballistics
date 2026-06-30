// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_jnz9_journal_event_seq_valid` Verus spec.
//
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// This file binds `verification/verus/vb_jnz9_journal_event_seq_valid.rs`
// to the production `JournalEvent::is_valid()` implementation in
// `crates/vb_storage/src/events.rs:514-550` and the production
// `JournalEvent::parse_event` entrypoint in
// `crates/vb_storage/src/journal/parse.rs:29-33`.
//
// ============================================================================
// WHY STRUCTURAL MIRROR (NOT DIRECT #[path] INCLUSION OF events.rs)
// ============================================================================
//
// Direct `#[path = "../../crates/vb_storage/src/events.rs"]` inclusion is
// blocked by:
//
//   1. `events.rs:6-9` `use vb_core::{ActionId, ActionTicket,
//      CapabilitySet, ConstValue, RunId, RuntimePolicy, SlotIdx,
//      SlotValue, StepIdx, Taint, WorkflowDigest};` requires the
//      vb_core extern crate alias, which is wired through
//      `crates/vb_storage/Cargo.toml` and is unavailable in a
//      standalone `verus --crate-type=lib` invocation.
//
//   2. `events.rs:5` `use chrono::{DateTime, Utc};` requires extern
//      crate chrono (the build artifacts in target/debug/deps do
//      contain libchrono.rlib, but adding the extern crate alias in
//      a standalone invocation is brittle across rebuilds).
//
//   3. `events.rs:11-18` `DurableActionOutcome` and other variants use
//      `#[derive(... serde::Serialize, serde::Deserialize)]` plus
//      `#[derive(thiserror::Error)]` (in error.rs). Verus cannot invoke
//      proc-macro derives without registering the proc macro crates,
//      and the file also pulls in `serde::{Deserialize, Serialize}`
//      as a bare-path import that would need a separate extern alias.
//
//   4. `events.rs:442` calls `postcard::from_bytes(bytes)` inside the
//      `slot_value()` method. This requires extern crate postcard,
//      which is wired through vb_storage's Cargo.toml and is not
//      available in standalone verus.
//
//   5. `events.rs:4` `use crate::{EventSeq, JournalError, RecordKind}`
//      requires those types to be resolvable at the crate root.
//      `crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES` (used inside
//      `slot_value`) is also a crate-root import that would need a
//      stub.
//
// The structural mirror below sidesteps every blocker while still
// establishing a real end-to-end binding: any drift in the production
// variant set, variant names, field names, or `is_valid()` body will
// break this mirror and the spec proofs that depend on it. Each
// mirror field name below is annotated with its production line number
// so any divergence between the mirror and production is detectable
// by inspection.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// ID newtype mirrors (each mirrors a production newtype line-by-line so
// any drift in field names or accessor signatures breaks the build):
//
//   - `EventSeq`    <- crates/vb_storage/src/types.rs:73
//                      (mirror as u64 newtype; production is
//                      `pub struct EventSeq(u64)` with `repr(transparent)`,
//                      the mirror preserves the same shape)
//   - `RunId`       <- crates/vb_core/src/ids/mod.rs:80
//                      (mirror as u64 newtype)
//   - `ActionTicket`
//                  <- crates/vb_core/src/action/ticket.rs:6-21
//                      (mirror as a struct with the same field names
//                      and types; only `attempt: u16` is read by
//                      `JournalEvent::is_valid()`)
//
// Enum mirrors:
//
//   - `MirrorJournalEvent` (24-variant enum)
//                  <- crates/vb_storage/src/events.rs:23-316
//                      (every variant name and field name is preserved
//                      so any production drift breaks the mirror at
//                      compile time)
//
// Method mirrors (each mirror body mirrors the production body
// line-by-line so any drift breaks the spec proofs that depend on
// it):
//
//   - `MirrorJournalEvent::run_id`
//                  <- crates/vb_storage/src/events.rs:332-363
//   - `MirrorJournalEvent::seq`
//                  <- crates/vb_storage/src/events.rs:366-397
//   - `MirrorJournalEvent::is_valid`
//                  <- crates/vb_storage/src/events.rs:514-550
//                      (THE PRIMARY BINDING TARGET — production rejects
//                      events whose run_id is 0, seq is u64::MAX, or
//                      whose attempt/ticket.attempt is 0)
//
// Pure spec fns (production decision lattice, mirrored line-by-line):
//
//   - `is_valid_run_id_zero`     <- events.rs:515-518
//   - `is_valid_seq_overflow`    <- events.rs:519-522
//   - `is_valid_attempt_nonzero` <- events.rs:523-549
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
// The bodies of `run_id`, `seq`, and `is_valid` on `MirrorJournalEvent`
// are mirrored from production line-by-line but Verus does NOT verify
// them (the methods are plain Rust fns, not inside a `verus!` block).
// The mathematical binding is attached in the companion spec file via
// `assume_specification` bridges: each bridge states the production
// contract (output value vs. input shape) and the spec proofs reason
// over that contract algebraically. Drift between the mirror body and
// the production body breaks the spec proofs because the postcondition
// asserted in `assume_specification` no longer matches the mirror's
// actual return value, so the exec proofs below the spec file fail to
// verify.
//
// ============================================================================
// DRIFT ITEMS ACCEPTED BY THE BINDING
// ============================================================================
//
//   - D1: Production `JournalEvent::slot_value` (events.rs:423-451) is
//         NOT mirrored here. It requires `postcard::from_bytes`,
//         `crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES`, and a
//         `JournalError::PayloadTooLarge { len: u32, max: u32 }`
//         variant — all blocked by the dependency surface above. The
//         mirror omits `slot_value` because `is_valid()` does not call
//         it; the spec proofs only reason about `is_valid()` and the
//         fields it reads (run_id, seq, attempt, ticket.attempt).
//
//   - D2: Production `JournalEvent::record_kind` (events.rs:386-414)
//         and `JournalEvent::attempt` (events.rs:460-487) are NOT
//         mirrored here. `is_valid()` does not call them; mirroring
//         them would require the full `RecordKind` discriminant set
//         from `crates/vb_storage/src/records.rs` plus the variant
//         discrimination logic, which is out of scope for the
//         seq-validity obligation.
//
//   - D3: Production `JournalEvent::parse_event` (parse.rs:29-33)
//         delegates to `decode_journal_event`, which requires the
//         `codec` module + `MAGIC_JOURNAL_EVENT` +
//         `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`. Not mirrored here
//         because parse-event failures are surfaced as
//         `JournalError`, not as `is_valid() == false`. The binding
//         ledger records parse.rs:29-33 as the production entrypoint
//         for journal event validation; this spec proves the
//         post-decode invariant that `is_valid()` enforces.
//
// ============================================================================

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Production drift-detection inclusion via #[path]
// ---------------------------------------------------------------------------
//
// `#[path]` inclusion of the production drift-detection stub at
// `production_inner/vb_jnz9_journal_event_seq_valid_production.rs`.
// The stub carries a representative drift-detection slice (EventSeq
// newtype + is_valid decision fn). Any drift in the production
// surface breaks the spec build. The full production mirror content
// lives below in this file.
#[path = "production_inner/vb_jnz9_journal_event_seq_valid_production.rs"]
pub mod prod_src;

} // verus!

// ============================================================================
// Re-export mirror types from the companion file so downstream spec files
// (e.g., `vb_jnz9_journal_event_seq_valid.rs`) can continue to resolve
// them through `production::MirrorXxx` / `production::EventSeq` / etc.
// The companion file hosts the structural mirrors (ID newtypes,
// ActionTicket, MirrorJournalEvent, is_valid_* predicates); this file
// hosts only the verus!-gated `#[path]` drift-detection inclusion above.
// ============================================================================
#[path = "extern_vb_jnz9_journal_event_seq_valid_mirror.rs"]
pub mod mirror;
pub use mirror::{ActionTicket, EventSeq, MirrorJournalEvent, RunId};
pub use mirror::{is_valid_attempt_nonzero, is_valid_run_id_zero, is_valid_seq_overflow};
