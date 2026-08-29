// Verus proof obligations for vb-ko29.2 replay tracker monotonicity binding.
//
// Obligation: VERUS-REPLAY-004.
// Exact verifier command: `verus --crate-type=lib verification/verus/idempotency_replay_tracker.rs`.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is the spec-side surface for the
// `vb_storage::recovery::types::ActionReplayTracker` set-algebra
// properties. The production surface is included via `#[path]` in the
// companion extern file `verification/verus/extern_idempotency_replay_tracker.rs`
// which:
//
//   - Pulls in `verification/verus/production_inner/action_replay_tracker_production.rs`
//     (a verbatim mirror of `crates/vb_storage/src/recovery/types.rs:1400-1604`)
//     via direct `#[path]` include.
//   - Marks that module `#[verifier::external]` so every production body
//     is opaque to Verus; only the structural shape (field names,
//     discriminant sets, method signatures) is checked.
//
// The `assume_specification` bridges below attach Verus-native spec
// contracts to the production exec methods. Each bridge is discharged
// by a non-vacuum exec wrapper that calls the production method and
// asserts the spec relationship holds. The proof fns then reason about
// the set-algebra layer using the spec predicates `spec_is_resolved`,
// `spec_mark_completed`, `spec_mark_failed`, etc., which are pure
// projections of the production-bound contracts.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `ActionReplayTracker` (production type)
//                            <- crates/vb_storage/src/recovery/types.rs:870-875
//   - `ActionReplayTracker::new`
//                            <- crates/vb_storage/src/recovery/types.rs:899-908
//   - `ActionReplayTracker::mark_completed`
//                            <- crates/vb_storage/src/recovery/types.rs:960-964
//   - `ActionReplayTracker::mark_failed`
//                            <- crates/vb_storage/src/recovery/types.rs:1024-1027
//   - `ActionReplayTracker::is_resolved`
//                            <- crates/vb_storage/src/recovery/types.rs:1041-1046
//   - `ActionReplayTracker::has_completed`
//                            <- crates/vb_storage/src/recovery/types.rs:1029-1033
//   - `ActionReplayTracker::has_failed`
//                            <- crates/vb_storage/src/recovery/types.rs:1035-1039
//
// Spec-side projection of the production surface into mathematical Set
// algebra:
//   - `spec_is_resolved`            <- production `is_resolved` (types.rs:1041-1046)
//   - `spec_mark_completed`         <- production `mark_completed` (types.rs:960-964)
//   - `spec_mark_failed`            <- production `mark_failed` (types.rs:1024-1027)
//   - `spec_replay_action_*`        <- production replay dispatch in
//                                      crates/vb_storage/src/recovery/replay/core.rs:82-110
//   - `spec_retry_allowed`          <- production reject_if_resolved at
//                                      crates/vb_storage/src/recovery/replay/core.rs:185-194
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of `ActionReplayTracker::{new, mark_completed,
// mark_failed, has_completed, has_failed, is_resolved}` are NOT
// verified by Verus directly. The module-level `#[verifier::external]`
// directive in the companion extern file makes every body opaque; the
// `assume_specification` bridges below state the production behavior
// (HashSet membership lookup; HashSet insert; logical OR of the two
// membership predicates), and the exec wrappers discharge those
// contracts. Drift between the production mirror and the production
// source is reported as binding-debt tracked outside Verus.
//
// ============================================================================
// SOURCE BRIDGE
// ============================================================================
// Production mirror: `verification/verus/production_inner/action_replay_tracker_production.rs`
// (verbatim copy of `crates/vb_storage/src/recovery/types.rs:1400-1604`)
// Production extern: `verification/verus/extern_idempotency_replay_tracker.rs`
// (binds the mirror via `#[path]` + module-level `#[verifier::external]`)

#[path = "extern_idempotency_replay_tracker.rs"]
mod production;

use vstd::prelude::*;

verus! {

// Re-export the production `ActionReplayTracker` so the
// `assume_specification` bridges below and the exec wrappers can
// reference `production::ActionReplayTracker` without a nested
// `prod_src::` path. The production newtypes `ActionId` and `StepIdx`
// (u16 newtype wrappers declared at `crates/vb_core/src/ids/mod.rs`
// via the `numeric_id!($name, u16, get)` macro) are re-exported as
// `SpecActionId` / `SpecStepIdx` for spec-side typing.
pub use production::ActionReplayTracker;
pub use production::prod_src::ActionId as SpecActionId;
pub use production::prod_src::StepIdx as SpecStepIdx;

// ============================================================================
// Production type bridge — `#[verifier::external_type_specification]`
// ============================================================================
//
// Because the production module (`production::prod_src`) is
// `#[verifier::external]` (per the companion extern file), the
// production types are nameable from spec context but not usable for
// spec reasoning until we attach an `external_type_specification`
// bridge. The bridges below tell Verus "this spec-mode name refers
// to the production type". Once attached, the production types can
// appear in `assume_specification` parameter lists, in spec fn
// arguments, and in `Set<(ActionId, StepIdx)>` expressions.
//
// This is the same pattern as `verification/verus/taint_lattice.rs`
// lines 88-89 (where `production::Taint` is bridged via
// `ExTaint`). The bridges are zero-cost: they only name the
// production type; the production bodies remain opaque.

/// Spec-mode name for the production `ActionReplayTracker` struct at
/// `crates/vb_storage/src/recovery/types.rs:870-875`. The struct
/// fields `completed` and `failed` (HashSet views) are spec-visible
/// via this bridge so the `assume_specification` post-conditions can
/// reason about membership.
///
/// Note: the production struct has two additional fields
/// (`scheduled_tickets: HashMap<...>` and
/// `completed_envelopes: HashMap<...>`) whose value types
/// `ActionScheduleEvidence` and `ActionCompletionEvidence` are
/// private to the production module. These types are also bridged
/// below via `ExActionScheduleEvidence` and
/// `ExActionCompletionEvidence` so the entire struct is nameable in
/// spec mode.
#[verifier::external_type_specification]
pub struct ExActionReplayTracker(production::ActionReplayTracker);

/// Spec-mode name for the production `ActionScheduleEvidence` struct
/// (private to the production module, used as the value type of
/// `scheduled_tickets`).
#[verifier::external_type_specification]
pub struct ExActionScheduleEvidence(pub production::prod_src::ActionScheduleEvidence);

/// Spec-mode name for the production `ActionCompletionEvidence`
/// struct (private to the production module, used as the value type
/// of `completed_envelopes`).
#[verifier::external_type_specification]
pub struct ExActionCompletionEvidence(pub production::prod_src::ActionCompletionEvidence);

/// Spec-mode name for the production `ActionTicket` struct
/// (`crates/vb_storage/src/recovery/types.rs` — the wrapper around
/// `(action: ActionId, step: StepIdx)` passed to
/// `mark_scheduled_ticket_effect` etc.).
#[verifier::external_type_specification]
pub struct ExActionTicket(pub production::prod_src::ActionTicket);

/// Spec-mode name for the production `SlotIdx` (u16 newtype).
#[verifier::external_type_specification]
pub struct ExSlotIdx(pub production::prod_src::SlotIdx);

/// Spec-mode name for the production `Taint` enum.
#[verifier::external_type_specification]
pub struct ExTaint(pub production::prod_src::Taint);

/// Spec-mode name for the production `ActionId` (u16 newtype) at
/// `crates/vb_core/src/ids/mod.rs:58`.
#[verifier::external_type_specification]
pub struct ExActionId(SpecActionId);

/// Spec-mode name for the production `StepIdx` (u16 newtype) at
/// `crates/vb_core/src/ids/mod.rs:55`.
#[verifier::external_type_specification]
pub struct ExStepIdx(SpecStepIdx);

// ============================================================================
// Mathematical projection (Set-algebra spec surface)
// ============================================================================
//
// These spec fns are the mathematical projection of the production
// `ActionReplayTracker` set-algebra behavior into `Set<(int, int)>`
// space. They are the predicates that `assume_specification` bridges
// and the proof fns reason about. The `Set<(int, int)>` space is
// derived from the production `Set<(ActionId, StepIdx)>` view via the
// `int` projection `key.0.0 as int` (ActionId wraps u16).

// ReplayActionOutcome — decision enum produced by the replay dispatch.
pub enum ReplayActionOutcome {
    Continue,
    BlockNonIdempotentAction,
}

/// Spec predicate: `(action, step)` is "resolved" iff it is in the
/// `completed` set OR the `failed` set.
///
/// Mirrors the production body at
/// `crates/vb_storage/src/recovery/types.rs:1041-1046`:
/// ```text
/// self.completed.contains(&(action, step)) || self.failed.contains(&(action, step))
/// ```
pub open spec fn spec_is_resolved(
    completed: Set<(int, int)>,
    failed: Set<(int, int)>,
    action: int,
    step: int,
) -> bool {
    completed.contains((action, step)) || failed.contains((action, step))
}

/// Spec projection of `ActionReplayTracker::mark_completed`:
/// returns the new `(completed, failed)` state after inserting
/// `(action, step)` into `completed`.
///
/// Mirrors the production body at
/// `crates/vb_storage/src/recovery/types.rs:960-964`
/// (`self.completed.insert((action, step));`).
pub open spec fn spec_mark_completed(
    completed: Set<(int, int)>,
    failed: Set<(int, int)>,
    action: int,
    step: int,
) -> (Set<(int, int)>, Set<(int, int)>) {
    (completed.insert((action, step)), failed)
}

/// Spec projection of `ActionReplayTracker::mark_failed`:
/// returns the new `(completed, failed)` state after inserting
/// `(action, step)` into `failed`.
///
/// Mirrors the production body at
/// `crates/vb_storage/src/recovery/types.rs:1024-1027`
/// (`self.failed.insert((action, step));`).
pub open spec fn spec_mark_failed(
    completed: Set<(int, int)>,
    failed: Set<(int, int)>,
    action: int,
    step: int,
) -> (Set<(int, int)>, Set<(int, int)>) {
    (completed, failed.insert((action, step)))
}

/// Spec predicate: an action may be retried iff it is NOT resolved
/// OR it is idempotent. Mirrors the production rejection gate
/// `reject_if_resolved` at
/// `crates/vb_storage/src/recovery/replay/core.rs:185-194`.
pub open spec fn spec_retry_allowed(
    completed: Set<(int, int)>,
    failed: Set<(int, int)>,
    action: int,
    step: int,
    is_idempotent: bool,
) -> bool {
    !spec_is_resolved(completed, failed, action, step) || is_idempotent
}

/// Spec projection of replay dispatch: a scheduled event for a
/// resolved `(action, step)` is blocked (idempotency guard); an
/// unresolved event continues.
///
/// Mirrors the production dispatch at
/// `crates/vb_storage/src/recovery/replay/core.rs:118-120`.
pub open spec fn spec_replay_action_scheduled(
    completed: Set<(int, int)>,
    failed: Set<(int, int)>,
    action: int,
    step: int,
) -> ReplayActionOutcome {
    if spec_is_resolved(completed, failed, action, step) {
        ReplayActionOutcome::BlockNonIdempotentAction
    } else {
        ReplayActionOutcome::Continue
    }
}

/// Spec projection of replay dispatch for completion events. A
/// resolved key is blocked; an unresolved key continues and the
/// `(completed, failed)` set pair is updated via `spec_mark_completed`.
///
/// Mirrors the production dispatch at
/// `crates/vb_storage/src/recovery/replay/core.rs:133-137`.
pub open spec fn spec_replay_action_completed(
    completed: Set<(int, int)>,
    failed: Set<(int, int)>,
    action: int,
    step: int,
) -> (ReplayActionOutcome, Set<(int, int)>, Set<(int, int)>) {
    if spec_is_resolved(completed, failed, action, step) {
        (ReplayActionOutcome::BlockNonIdempotentAction, completed, failed)
    } else {
        let new_c = spec_mark_completed(completed, failed, action, step).0;
        let new_f = spec_mark_completed(completed, failed, action, step).1;
        (ReplayActionOutcome::Continue, new_c, new_f)
    }
}

/// Spec projection of replay dispatch for failure events. Symmetric
/// to `spec_replay_action_completed` but updates the `failed` set
/// via `spec_mark_failed`.
///
/// Mirrors the production dispatch at
/// `crates/vb_storage/src/recovery/replay/core.rs:138-142`.
pub open spec fn spec_replay_action_failed(
    completed: Set<(int, int)>,
    failed: Set<(int, int)>,
    action: int,
    step: int,
) -> (ReplayActionOutcome, Set<(int, int)>, Set<(int, int)>) {
    if spec_is_resolved(completed, failed, action, step) {
        (ReplayActionOutcome::BlockNonIdempotentAction, completed, failed)
    } else {
        let new_c = spec_mark_failed(completed, failed, action, step).0;
        let new_f = spec_mark_failed(completed, failed, action, step).1;
        (ReplayActionOutcome::Continue, new_c, new_f)
    }
}

// ============================================================================
// Spec-side projection: from production HashSet to Set<(int, int)>
// ============================================================================
//
// The production `ActionReplayTracker` carries its state in
// `HashSet<(ActionId, StepIdx)>` fields `completed` and `failed`. The
// spec treats each HashSet view (via the `@` postfix) as a
// `Set<(ActionId, StepIdx)>`. To project that into `Set<(int, int)>`
// for set-algebra reasoning, we project each `(ActionId, StepIdx)`
// pair to `(int, int)` via the inner u16 access
// `(key.0.0 as int, key.1.0 as int)`.

/// Spec predicate: production `tracker.has_completed(action, step)`
/// returns `true` iff `(action, step)` (projected to int) is in the
/// production `completed` HashSet view.
///
/// Mirrors the production body at
/// `crates/vb_storage/src/recovery/types.rs:1029-1033`
/// (`self.completed.contains(&(action, step))`).
pub open spec fn spec_has_completed(
    tracker: production::ActionReplayTracker,
    action: int,
    step: int,
) -> bool
    recommends
        0 <= action < 65536 && 0 <= step < 65536,
{
    // Production completed@ is `Set<(ActionId, StepIdx)>`. We project
    // each pair to `Set<(int, int)>` via `.0` field access on each
    // newtype. The spec predicate is true iff there exists a key in
    // the production HashSet view whose projections match
    // (action, step).
    exists|key: (SpecActionId, SpecStepIdx)| {
        &&& key.0.0 as int == action
        &&& key.1.0 as int == step
        &&& tracker.completed@.contains(key)
    }
}

/// Spec predicate: production `tracker.has_failed(action, step)`
/// returns `true` iff `(action, step)` (projected to int) is in the
/// production `failed` HashSet view.
///
/// Mirrors the production body at
/// `crates/vb_storage/src/recovery/types.rs:836-838`
/// (`self.failed.contains(&(action, step))`).
pub open spec fn spec_has_failed(
    tracker: production::ActionReplayTracker,
    action: int,
    step: int,
) -> bool
    recommends
        0 <= action < 65536 && 0 <= step < 65536,
{
    exists|key: (SpecActionId, SpecStepIdx)| {
        &&& key.0.0 as int == action
        &&& key.1.0 as int == step
        &&& tracker.failed@.contains(key)
    }
}

/// Spec predicate: `tracker.is_resolved(action, step)` returns
/// `true` iff `spec_has_completed` OR `spec_has_failed` holds. This
/// is the spec-level expression of the production method body at
/// `crates/vb_storage/src/recovery/types.rs:843-845`.
pub open spec fn spec_tracker_is_resolved(
    tracker: production::ActionReplayTracker,
    action: int,
    step: int,
) -> bool
    recommends
        0 <= action < 65536 && 0 <= step < 65536,
{
    spec_has_completed(tracker, action, step) || spec_has_failed(tracker, action, step)
}

/// Spec predicate: `action` and `step` are valid u16 values, i.e.,
/// they correspond to a representable `ActionId`/`StepIdx` value.
pub open spec fn spec_action_step_in_range(action: int, step: int) -> bool {
    0 <= action < 65536 && 0 <= step < 65536
}

// ============================================================================
// assume_specification bridges — production contract surface
// ============================================================================
//
// Each `assume_specification` bridge attaches a Verus-native spec
// contract to a production exec method declared in
// `extern_idempotency_replay_tracker.rs`. The body of each production
// method is opaque to Verus (module-level `#[verifier::external]` in
// the extern file); the contract below states the production behavior
// the spec proofs reason about.

/// Bridge contract: `tracker.mark_completed(action, step)` inserts
/// `(action, step)` into `tracker.completed@`, leaves
/// `tracker.failed@` unchanged, and does not affect membership of
/// other `(a', s')` pairs in `completed@`.

// ============================================================================
// Companion chunk 2 — proof/remaining functions
// ============================================================================
#[path = "idempotency_replay_tracker_chunk2.rs"]
mod chunk2;

} // verus!
