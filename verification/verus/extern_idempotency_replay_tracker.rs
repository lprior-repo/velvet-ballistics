// SPDX-License-Identifier: MIT
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// This file is the production-binding surface for the
// `idempotency_replay_tracker.rs` Verus spec. It contains:
//
//   1. A direct `#[path]` inclusion of the production-source mirror at
//      `verification/verus/production_inner/action_replay_tracker_production.rs`.
//      Its tracker implementation is copied from current production. The
//      existing local identifier/error projections remain; every ticket field
//      is retained, with transparent RunId/SeqNo payloads represented by u64.
//      Retry authorization calls the actual linked RetryTimerV1 validator and
//      canonical key checker; neither dependency is replaced with a stub.
//
//   2. A module-level `#[verifier::external]` directive so every body
//      in the included production module is opaque to Verus. The
//      mathematical contracts are attached via `assume_specification`
//      in the companion spec file `idempotency_replay_tracker.rs`,
//      and `exec fn` wrappers in that spec file actually invoke the
//      production exec fns to discharge the contracts.
//
// ============================================================================
// WHY THIS REMAINS A WEAK STRUCTURAL MIRROR
// ============================================================================
// The proof retains its existing readable identifier/error projections and
// relaxed private-field visibility; it does not verify the native tracker body.
// The current tracker struct and implementation are copied from
// `crates/vb_storage/src/recovery/types.rs:1446-1669`, with retry-authority
// methods from `crates/vb_storage/src/recovery/retry_authority.rs:5-69`.
// RunId/SeqNo use their transparent u64 payloads; ticket conversion preserves
// every field. Native vb_core/vb_storage dependencies are now linked explicitly,
// so their unavailability is not a reason for this boundary. RetryTimerV1
// validation and canonical action-key validation execute those real crates.
// The module remains external to Verus and its existing assumed contracts are
// unchanged. Neither a mirror pass nor native dependency linkage proves the
// tracker implementation correct or turns these projections into native types.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `ActionReplayTracker`                          <- crates/vb_storage/src/recovery/types.rs:1446-1452
//   - `ActionReplayTracker::new`                     <- crates/vb_storage/src/recovery/types.rs:1480-1488
//   - `ActionReplayTracker::mark_completed`          <- crates/vb_storage/src/recovery/types.rs:1552-1554
//   - `ActionReplayTracker::mark_failed`             <- crates/vb_storage/src/recovery/types.rs:1641-1643
//   - `ActionReplayTracker::has_completed`           <- crates/vb_storage/src/recovery/types.rs:1647-1649
//   - `ActionReplayTracker::has_failed`              <- crates/vb_storage/src/recovery/types.rs:1653-1655
//   - `ActionReplayTracker::is_resolved`             <- crates/vb_storage/src/recovery/types.rs:1660-1662
//   - `ActionReplayTracker::default`                 <- crates/vb_storage/src/recovery/types.rs:1666-1668
//   - Private `mark_scheduled_ticket_effect`, `require_scheduled_ticket`,
//     `mark_completed_envelope_effect`, `mark_completed_envelope` are
//     present in the included module but not bound by the spec; they
//     are referenced in the binding ledger for completeness only.
// ============================================================================
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies in
// `production_inner/action_replay_tracker_production.rs` are NOT
// verified by Verus (per the module-level `#[verifier::external]`
// directive below). The mathematical contracts attached via
// `assume_specification` in the companion spec file
// `idempotency_replay_tracker.rs` are the trusted base: they state
// what the production bodies do, but Verus does not independently
// confirm the bodies satisfy those contracts. The `exec fn` wrappers
// in the spec file are the non-vacuum witnesses that the binding is
// exercised — they invoke the production exec fn and assert the spec
// contract holds.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

#[verifier::external]
#[path = "production_inner/action_replay_tracker_production.rs"]
pub mod prod_src;

// Re-export the production `ActionReplayTracker` and its public
// proof-surface accessors so the companion spec file can reference
// them as `production::ActionReplayTracker`,
// `production::ActionReplayTracker::is_resolved`, etc. The re-exports
// do not change the trusted boundary: every re-exported name is
// backed by the `#[verifier::external]` body from `prod_src`.
pub use prod_src::ActionReplayTracker;

} // verus!
