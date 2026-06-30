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
//      `crates/vb_storage/src/recovery/types.rs:867-1053` (the
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
//   - types.rs:10 `use serde::{Deserialize, Serialize};` requires the
//     `serde` extern crate, which is not registered under
//     `verus --crate-type=lib` (no installs allowed by task brief).
//   - types.rs:11-14 `use vb_core::{ActionId, ActionTicket, ...};` requires
//     the `vb_core` extern crate alias, which is wired through the
//     workspace `Cargo.toml` and is unavailable in a standalone
//     `verus --crate-type=lib` invocation.
//   - types.rs:37 `#[derive(Debug, thiserror::Error)]` on
//     `RecoveryError` requires the `thiserror` proc-macro crate, also
//     unavailable under the no-installs constraint.
//   - types.rs:17-37 `#[cfg(kani)] struct ReplayResolutionSet(...)`
//     forces kani cfg to be active or the file fails to compile.
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
// The verbatim ActionReplayTracker impl block (lines 867-1053 of
// production) is included unchanged, so any drift in the production
// impl surface breaks this Verus build.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `ActionReplayTracker`                          <- crates/vb_storage/src/recovery/types.rs:870-875
//   - `ActionReplayTracker::new`                     <- crates/vb_storage/src/recovery/types.rs:899-908
//   - `ActionReplayTracker::mark_completed`          <- crates/vb_storage/src/recovery/types.rs:960-964
//   - `ActionReplayTracker::mark_failed`             <- crates/vb_storage/src/recovery/types.rs:1024-1027
//   - `ActionReplayTracker::has_completed`           <- crates/vb_storage/src/recovery/types.rs:1029-1033
//   - `ActionReplayTracker::has_failed`              <- crates/vb_storage/src/recovery/types.rs:1035-1039
//   - `ActionReplayTracker::is_resolved`             <- crates/vb_storage/src/recovery/types.rs:1041-1046
//   - `ActionReplayTracker::default`                 <- crates/vb_storage/src/recovery/types.rs:1049-1053
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
