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
//      That mirror is a VERBATIM copy of
//      `crates/vb_storage/src/recovery/types.rs:1429-1634` (the
//      `ActionReplayTracker` impl block) with only the
//      `vb_core::ids::*` newtypes and the `RecoveryError`/`RecoveryResult`
//      aliases substituted for in-tree stub versions that compile under
//      `verus --crate-type=lib` without `serde`/`thiserror` proc-macro
//      crates. The substitution is documented in the mirror file's
//      drift-policy header.
//
//   2. A module-level `#[verifier::external]` directive so every body
//      in the included production module is opaque to Verus. The
//      mathematical contracts are attached via `assume_specification`
//      in the companion spec file `idempotency_replay_tracker.rs`,
//      and `exec fn` wrappers in that spec file actually invoke the
//      production exec fns to discharge the contracts.
//
// ============================================================================
// WHY THE PRODUCTION MIRROR (NOT DIRECT #[path] TO types.rs)
// ============================================================================
// Direct `#[path = "../../crates/vb_storage/src/recovery/types.rs"]`
// inclusion is blocked by:
//   - `crates/vb_storage/src/recovery/types.rs:1429`
//     `#[derive(..., serde::Serialize, serde::Deserialize)]` on
//     `RunSnapshot` (fully-qualified paths; no `use serde` import)
//     requires the `serde` extern crate, which is not registered under
//     `verus --crate-type=lib` (no installs allowed by task brief).
//   - `crates/vb_storage/src/recovery/types.rs:10-13`
//     `use vb_core::{ActionId, ActionTicket, ...};` requires
//     the `vb_core` extern crate alias, which is wired through the
//     workspace `Cargo.toml` and is unavailable in a standalone
//     `verus --crate-type=lib` invocation.
//   - `crates/vb_storage/src/recovery/types.rs:15-33`
//     `#[cfg(kani)] struct ReplayResolutionSet(...)`
//     forces kani cfg to be active or the file fails to compile.
//
// Note: production previously carried a `#[derive(Debug,
// thiserror::Error)]` on `RecoveryError`; that derive has been removed
// (now a plain `#[derive(Debug)]` at
// `crates/vb_storage/src/recovery/types.rs:36-37`), so thiserror is no
// longer a blocker. The mirror is still required by the `serde` and
// `vb_core` blockers above.
//
// The in-tree mirror file at
// `verification/verus/production_inner/action_replay_tracker_production.rs`
// sidesteps every blocker by:
//   - replacing `serde::{Deserialize, Serialize}` and
//     `vb_core::{ActionId, StepIdx, SlotIdx, ActionTicket, Taint}` with
//     local stub structs that preserve field names, `Copy + PartialEq
//     + Eq + Hash` bounds, and the `new`/`get` accessor surface;
//   - replacing the `RecoveryError` derive with a manual mirror
//     containing only the two variants ActionReplayTracker constructs
//     (`NonIdempotentActionBlocked`, `ReplayDivergence`);
//   - omitting the `#[cfg(kani)]` block and the rest of types.rs.
// The verbatim ActionReplayTracker impl block (lines 1400-1604 of
// production) is included unchanged, so any drift in the production
// impl surface breaks this Verus build.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `ActionReplayTracker`                          <- crates/vb_storage/src/recovery/types.rs:1446-1452
//   - `ActionReplayTracker::new`                     <- crates/vb_storage/src/recovery/types.rs:375-377
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
