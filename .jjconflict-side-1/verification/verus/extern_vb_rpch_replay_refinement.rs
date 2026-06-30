// SPDX-License-Identifier: MIT
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_rpch_replay_refinement.rs` Verus spec. The spec proves replay
// refinement properties — the TLA+ RecoveryReplayFull.tla semantics
// projected onto the Rust `ActionReplayTracker` surface — for the
// two production methods that the recovery replay engine exercises
// at crates/vb_storage/src/recovery/replay/core.rs:
//
//   * `ActionReplayTracker::is_resolved(action, step)`
//        - crates/vb_storage/src/recovery/types.rs:843-845
//        - production body:
//          `self.completed.contains(&(action, step)) ||
//           self.failed.contains(&(action, step))`
//        - exercised by `replay_events` via `reject_if_resolved`
//          (recovery/replay/core.rs:185) BEFORE scheduling any new
//          action. The Rust tracker uses 2-tuple keys because
//          recovery replays one specific (run, attempt) at a time.
//
//   * `ActionReplayTracker::mark_completed(action, step)`
//        - crates/vb_storage/src/recovery/types.rs:761-763
//        - production body:
//          `self.completed.insert((action, step));`
//        - exercised by `replay_events` after observing an
//          `ActionCompletedEvent` (recovery/replay/core.rs:135).
//
// ============================================================================
// STRUCTURAL BINDING — production mirror via #[path]
// ============================================================================
//
// This file directly includes the verbatim production mirror at
//   verification/verus/production_inner/action_replay_tracker_production.rs
// via `#[path]`. The mirror is a line-for-line copy of
// crates/vb_storage/src/recovery/types.rs:666-852 (the
// ActionReplayTracker impl block), with only the `vb_core` newtypes
// and the `RecoveryError`/`RecoveryResult` aliases substituted for
// in-tree stub versions that compile under `verus --crate-type=lib`
// (the workspace `vb_core` extern alias and the `thiserror`/`serde`
// proc macros are unavailable in a standalone Verus invocation; the
// drift-policy header in the mirror file documents every
// substitution). Module-level `#[verifier::external]` makes every
// body in the included module opaque to Verus; the inclusion still
// validates Rust resolution (field names, discriminant sets, fn
// signatures) at compile time. Any drift in the production impl
// surface breaks this Verus build.
//
// ============================================================================
// SPEC MIRROR — SpecActionReplayTracker
// ============================================================================
//
// The production struct's private inner types
// (`ActionScheduleEvidence`, `ActionCompletionEvidence`) prevent a
// transparent `external_type_specification` wrapper. The spec uses
// its own `SpecActionReplayTracker` mirror with PUBLIC fields
// matching production byte-for-byte so spec contracts that read
// `tracker.completed` / `tracker.failed` resolve naturally. Field
// types are abstracted: production uses
// `HashSet<(ActionId, StepIdx)>` (u16 newtype wrappers at
// crates/vb_core/src/ids/mod.rs) and the mirror uses
// `HashSet<(u16, u16)>`. The abstraction is recorded as BINDING
// DEBT D1 below and mirrors the binding debt in the sibling
// `extern_vb_rpch_action_replay_tracker.rs`.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Production source: `crates/vb_storage/src/recovery/types.rs:666-852`
// (verbatim mirror at `production_inner/action_replay_tracker_production.rs`).
//
// Methods bound (production exec -> mirror exec):
//
//   * `ActionReplayTracker::is_resolved`     <- types.rs:843-845
//        -> `SpecActionReplayTracker::is_resolved`
//   * `ActionReplayTracker::mark_completed`  <- types.rs:761-763
//        -> `SpecActionReplayTracker::mark_completed`
//
// The production struct fields `scheduled_tickets` and
// `completed_envelopes` are NOT mirrored. The refinement spec only
// reasons about `is_resolved` and `mark_completed`, which neither
// read nor mutate those fields. Adding a spec for
// `mark_scheduled_ticket_effect` or
// `mark_completed_envelope_effect` would require extending the
// mirror to include these fields; recorded as BINDING DEBT D2.
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of `is_resolved` and `mark_completed` are
// NOT verified by Verus directly. The production mirror module is
// marked `#[verifier::external]` at module level, and the mirror
// methods below are also `#[verifier::external]`. The contracts
// attached via `assume_specification` in the companion spec file
// `vb_rpch_replay_refinement.rs` state the production behavior, and
// the exec wrappers in the spec file exercise the production
// methods to discharge the contracts. Drift between the production
// mirror and the production source is reported as binding debt
// tracked outside Verus.
//
// ============================================================================
// BINDING DEBT
// ============================================================================
//
// D1: `ActionId`/`StepIdx` newtype layer abstracted to `u16`. The
//     production field type is `HashSet<(ActionId, StepIdx)>`; the
//     mirror uses `HashSet<(u16, u16)>`. A drift that widens
//     ActionId or StepIdx (e.g., to u32) would require updating
//     both the production field type and the mirror's exec
//     signatures.
//
// D2: The production fields `scheduled_tickets` and
//     `completed_envelopes` are not mirrored. The spec covers only
//     `is_resolved` and `mark_completed`. Adding a spec for
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
// `prod_src` is `pub` so the spec file can re-export the
// production types if a future binding needs them. The spec file
// primarily uses the `SpecActionReplayTracker` mirror declared
// below for spec-side reasoning because the production struct's
// private inner types cannot be wrapped transparently.
//
// Drift detection: a phantom `prod_methods_drift_check` fn below
// calls the production methods with arguments of the production
// types, forcing Rust to look up the production method names at
// compile time. Any rename of these production methods (or the
// production struct) breaks the lookup and fails this Verus build.
#[verifier::external]
#[path = "production_inner/action_replay_tracker_production.rs"]
pub mod prod_src;

// Phantom drift-detection helper. The body is
// `#[verifier::external]` (opaque to Verus), but the
// `prod_src::ActionReplayTracker::*` method references force Rust
// to resolve the production method names at compile time. A rename
// of `is_resolved` or `mark_completed` in production breaks this
// fn's compilation.
#[verifier::external]
fn prod_methods_drift_check(
    t: &mut prod_src::ActionReplayTracker,
    action: prod_src::ActionId,
    step: prod_src::StepIdx,
) {
    let _ = t.is_resolved(action, step);
    t.mark_completed(action, step);
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
// wrappers at crates/vb_core/src/ids/mod.rs). See BINDING DEBT D1.
pub struct SpecActionReplayTracker {
    /// Mirror of production `completed: HashSet<(ActionId, StepIdx)>`
    /// at crates/vb_storage/src/recovery/types.rs:671.
    pub completed: HashSet<(u16, u16)>,
    /// Mirror of production `failed: HashSet<(ActionId, StepIdx)>`
    /// at crates/vb_storage/src/recovery/types.rs:672.
    pub failed: HashSet<(u16, u16)>,
}

impl SpecActionReplayTracker {
    /// Mirror of `ActionReplayTracker::is_resolved` at
    /// crates/vb_storage/src/recovery/types.rs:843-845. Production
    /// body (line 844):
    /// `self.completed.contains(&(action, step)) ||
    ///  self.failed.contains(&(action, step))`.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`). The `assume_specification` bridge
    /// in the companion spec file attaches the production contract.
    #[verifier::external]
    pub fn is_resolved(&self, action: u16, step: u16) -> bool {
        self.completed.contains(&(action, step)) || self.failed.contains(&(action, step))
    }

    /// Mirror of `ActionReplayTracker::mark_completed` at
    /// crates/vb_storage/src/recovery/types.rs:761-763. Production
    /// body (line 762): `self.completed.insert((action, step));`.
    /// The production fn does not mutate `failed`,
    /// `scheduled_tickets`, or `completed_envelopes`.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`). The `assume_specification` bridge
    /// in the companion spec file attaches the production contract.
    #[verifier::external]
    pub fn mark_completed(&mut self, action: u16, step: u16) {
        self.completed.insert((action, step));
    }
}

} // verus!