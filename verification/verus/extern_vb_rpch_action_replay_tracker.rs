// SPDX-License-Identifier: MIT
//
// Extern surface for `vb_rpch_action_replay_tracker` Verus spec.
//
// =============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// =============================================================================
//
// This file is the production-binding surface for the
// `vb_rpch_action_replay_tracker.rs` Verus spec. It contains:
//
//   1. A direct `#[path]` inclusion of the production mirror at
//      `verification/verus/production_inner/action_replay_tracker_production.rs`,
//      which is itself a VERBATIM copy of
//      `crates/vb_storage/src/recovery/types.rs:666-852` (the
//      `ActionReplayTracker` impl block) with only the
//      `vb_core::ids::*` newtypes and the `RecoveryError`/
//      `RecoveryResult` aliases substituted for in-tree stub
//      versions that compile under `verus --crate-type=lib`. This
//      structural binding means any rename, discriminant drift, or
//      signature change in the production source breaks this Verus
//      build at compile time. See the drift policy header in
//      `production_inner/action_replay_tracker_production.rs`.
//
//   2. A `SpecActionReplayTracker` mirror struct with public
//      `completed` and `failed` fields. The production struct's
//      private inner types (`ActionScheduleEvidence`,
//      `ActionCompletionEvidence`) prevent a transparent
//      `external_type_specification` wrapper; the mirror is
//      therefore a parallel struct with PUBLIC field names matching
//      production byte-for-byte so spec contracts that read
//      `tracker.completed` / `tracker.failed` resolve naturally.
//      Field types are abstracted: production uses
//      `HashSet<(ActionId, StepIdx)>` and the mirror uses
//      `HashSet<(u16, u16)>` (the production newtypes are u16
//      newtype wrappers at crates/vb_core/src/ids/mod.rs:55,58;
//      the abstraction is recorded as BINDING DEBT D1 in
//      `extern_idempotency_replay_tracker.rs` and below).
//
//   3. `#[verifier::external]` on every exec method body so Verus
//      skips body verification. The `assume_specification` bridges
//      in the companion spec file attach the production contracts.
//      The exec methods on the mirror follow the production
//      signatures (param names, parameter order, return type).
//
// =============================================================================
// BINDING LEDGER
// =============================================================================
//
// Production source: `crates/vb_storage/src/recovery/types.rs:669-846`.
//
// Production mirror included via `#[path]`:
//   - `ActionReplayTracker`                          <- types.rs:668-674
//   - `ActionReplayTracker::new`                     <- types.rs:700-707
//   - `ActionReplayTracker::mark_completed`          <- types.rs:761-763
//   - `ActionReplayTracker::mark_failed`             <- types.rs:824-826
//   - `ActionReplayTracker::has_completed`           <- types.rs:830-832
//   - `ActionReplayTracker::has_failed`              <- types.rs:836-838
//   - `ActionReplayTracker::is_resolved`             <- types.rs:843-845
//
// Method correspondence (production exec -> mirror exec):
//   - `ActionReplayTracker::new`         -> `SpecActionReplayTracker::new`
//   - `ActionReplayTracker::mark_completed` -> `SpecActionReplayTracker::mark_completed`
//   - `ActionReplayTracker::mark_failed`    -> `SpecActionReplayTracker::mark_failed`
//   - `ActionReplayTracker::has_completed`  -> `SpecActionReplayTracker::has_completed`
//   - `ActionReplayTracker::has_failed`     -> `SpecActionReplayTracker::has_failed`
//   - `ActionReplayTracker::is_resolved`    -> `SpecActionReplayTracker::is_resolved`
//
// The production fields `scheduled_tickets` and
// `completed_envelopes` are NOT mirrored (they are not read or
// mutated by `has_completed`/`has_failed`/`is_resolved`/
// `mark_completed`/`mark_failed`). Adding a spec for
// `mark_scheduled_ticket_effect` or `mark_completed_envelope_effect`
// would require extending the mirror; recorded as BINDING DEBT D2.
//
// =============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// =============================================================================
//
// The production bodies of `mark_completed`, `mark_failed`,
// `has_completed`, `has_failed`, `is_resolved`, and `new` are NOT
// verified by Verus directly (the production mirror is
// `#[verifier::external]` at module level). The
// `assume_specification` bridges in the companion spec file state
// the production behavior; exec wrappers in the spec file are the
// non-vacuum witnesses that the bridge contracts hold. Drift
// between the production mirror and the production source is
// reported as binding-debt tracked outside Verus.
//
// =============================================================================
// BINDING DEBT
// =============================================================================
//
// D1: `ActionId`/`StepIdx` newtype layer abstracted to `u16`. The
//     production field type is `HashSet<(ActionId, StepIdx)>`; the
//     mirror uses `HashSet<(u16, u16)>`. A drift that widens
//     ActionId or StepIdx (e.g., to u32) would require updating
//     both the production field type and the mirror's exec
//     signatures. Tracked in the proof-debt ledger outside this
//     file.
//
// D2: The two production fields `scheduled_tickets` and
//     `completed_envelopes` are not mirrored. The spec covers
//     only the has_completed/has_failed/is_resolved/mark_completed/
//     mark_failed surface. Adding a spec for
//     `mark_scheduled_ticket_effect` or
//     `mark_completed_envelope_effect` would require extending the
//     mirror to include these fields.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use std::collections::HashSet;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION INCLUSION via #[path] — STRUCTURAL drift detection
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the verbatim production mirror at
// `production_inner/action_replay_tracker_production.rs`. The
// mirror is marked `#[verifier::external]` at module level so the
// production bodies are opaque to Verus; the inclusion still
// validates Rust resolution (field names, discriminant sets, fn
// signatures) at compile time. Any drift in the production impl
// surface breaks this Verus build.
//
// Note: `prod_src` is `pub` so the spec file can re-export
// `ActionReplayTracker` for the production type bridge
// (`#[verifier::external_type_specification]` in the spec file).
// The spec file uses its own `SpecActionReplayTracker` mirror
// (declared below) for spec-side reasoning because the
// production struct's private inner types (`ActionScheduleEvidence`,
// `ActionCompletionEvidence`) cannot be wrapped transparently.
//
// Drift detection: a phantom `prod_methods_drift_check` fn below
// calls the production methods with arguments of the production
// types, forcing Rust to look up the production method names at
// compile time. Any rename of these methods in production breaks
// the lookup and fails this Verus build.
#[verifier::external]
#[path = "production_inner/action_replay_tracker_production.rs"]
pub mod prod_src;

// Phantom drift-detection helper. The body is `#[verifier::external]`
// (opaque to Verus), but the `prod_src::ActionReplayTracker::*`
// method references force Rust to resolve the production method
// names at compile time. A rename of any of these production
// methods (or the production struct) breaks this fn's compilation.
#[verifier::external]
fn prod_methods_drift_check(t: &mut prod_src::ActionReplayTracker, action: prod_src::ActionId, step: prod_src::StepIdx) {
    let _ = prod_src::ActionReplayTracker::new();
    let _ = t.has_completed(action, step);
    let _ = t.has_failed(action, step);
    let _ = t.is_resolved(action, step);
    t.mark_completed(action, step);
    t.mark_failed(action, step);
}

// ---------------------------------------------------------------------------
// Spec-side mirror struct (production-bound field names)
// ---------------------------------------------------------------------------
//
// Field names match production byte-for-byte so spec contracts
// that read `tracker.completed` / `tracker.failed` resolve
// naturally. Field types are abstracted: production uses
// `HashSet<(ActionId, StepIdx)>` and the mirror uses
// `HashSet<(u16, u16)>` (the production newtypes are u16 newtype
// wrappers at crates/vb_core/src/ids/mod.rs:55,58). The
// abstraction is recorded as BINDING DEBT D1 above.
pub struct SpecActionReplayTracker {
    /// Mirror of production `completed: HashSet<(ActionId, StepIdx)>`
    /// at crates/vb_storage/src/recovery/types.rs:671.
    pub completed: HashSet<(u16, u16)>,
    /// Mirror of production `failed: HashSet<(ActionId, StepIdx)>`
    /// at crates/vb_storage/src/recovery/types.rs:672.
    pub failed: HashSet<(u16, u16)>,
}

impl SpecActionReplayTracker {
    /// Mirror of `ActionReplayTracker::new` at
    /// crates/vb_storage/src/recovery/types.rs:700-707. The
    /// production body constructs four empty collections; the
    /// mirror mirrors only `completed` and `failed` per BINDING
    /// DEBT D2. Body is NOT `#[verifier::external]` so Verus
    /// can verify the trivial post-condition directly.
    pub fn new() -> Self {
        Self {
            completed: HashSet::new(),
            failed: HashSet::new(),
        }
    }

    /// Mirror of `ActionReplayTracker::has_completed` at
    /// crates/vb_storage/src/recovery/types.rs:830-832. Production
    /// body (line 831):
    /// `self.completed.contains(&(action, step))`.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`). The `assume_specification`
    /// bridge in the companion spec file attaches the production
    /// contract.
    #[verifier::external]
    pub fn has_completed(&self, action: u16, step: u16) -> bool {
        self.completed.contains(&(action, step))
    }

    /// Mirror of `ActionReplayTracker::has_failed` at
    /// crates/vb_storage/src/recovery/types.rs:836-838. Production
    /// body (line 837):
    /// `self.failed.contains(&(action, step))`.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`).
    #[verifier::external]
    pub fn has_failed(&self, action: u16, step: u16) -> bool {
        self.failed.contains(&(action, step))
    }

    /// Mirror of `ActionReplayTracker::is_resolved` at
    /// crates/vb_storage/src/recovery/types.rs:843-845. Production
    /// body (line 844):
    /// `self.completed.contains(&(action, step)) ||
    ///  self.failed.contains(&(action, step))`.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`).
    #[verifier::external]
    pub fn is_resolved(&self, action: u16, step: u16) -> bool {
        self.has_completed(action, step) || self.has_failed(action, step)
    }

    /// Mirror of `ActionReplayTracker::mark_completed` at
    /// crates/vb_storage/src/recovery/types.rs:761-763. Production
    /// body (line 762): `self.completed.insert((action, step));`.
    /// The production fn does not mutate `failed`,
    /// `scheduled_tickets`, or `completed_envelopes`.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`).
    #[verifier::external]
    pub fn mark_completed(&mut self, action: u16, step: u16) {
        self.completed.insert((action, step));
    }

    /// Mirror of `ActionReplayTracker::mark_failed` at
    /// crates/vb_storage/src/recovery/types.rs:824-826. Production
    /// body (line 825): `self.failed.insert((action, step));`.
    /// The production fn does not mutate `completed`,
    /// `scheduled_tickets`, or `completed_envelopes`.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`).
    #[verifier::external]
    pub fn mark_failed(&mut self, action: u16, step: u16) {
        self.failed.insert((action, step));
    }
}

} // verus!