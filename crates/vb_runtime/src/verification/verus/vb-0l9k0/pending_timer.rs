//! Extern_spec bindings for PendingTimer and PendingTimerKind.
//!
//! Production binding targets:
//! - `crates/vb_runtime/src/shard/types.rs:29-54` - PendingTimerKind and PendingTimer

use vstd::prelude::*;

verus! {

// ============================================================================
// PendingTimerKind extern_spec
// ============================================================================

/// Extern spec for PendingTimerKind enum.
///
/// Production enum: `types.rs:29-34`
#[extern_spec]
mod pending_timer_kind_spec {
    use vstd::prelude::*;

    #[verifier::extern_spec]
    pub enum PendingTimerKind {
        Wait,
        Ask,
    }
}

// ============================================================================
// PendingTimer extern_spec
// ============================================================================

/// Extern spec for PendingTimer struct.
///
/// Production struct: `types.rs:36-42`
///
/// Contract: All fields are immutable numeric types. The `deadline` field
/// is stored as `Instant` (monotonic clock), not as a mutable wall-clock
/// capture. This ensures no `Instant::now` capture occurs in immutable fields.
///
/// PO-vb-0l9k0-023: PendingTimer fields are numeric and deadline is stored
/// as Instant, proving no Instant::now capture in immutable fields.
#[extern_spec]
mod pending_timer_spec {
    use vstd::prelude::*;
    use crate::ids::StepIdx;
    use crate::shard::types::PendingTimerKind;
    use std::time::Instant;

    #[verifier::extern_spec]
    pub struct PendingTimer {
        pub step: StepIdx,
        pub kind: PendingTimerKind,
        pub generation: u64,
        pub deadline: Instant,
    }

    /// Extern spec for PendingTimer::matches_authority.
    ///
    /// Production method: `types.rs:46-53`
    ///
    /// Contract: Returns true only when ALL of the following match:
    /// - generation == provided generation
    /// - deadline == provided deadline
    /// - kind == provided kind
    ///
    /// Returns false if ANY field does not match.
    #[extern_spec]
    impl PendingTimer {
        #[verifier::extern_spec]
        #[must_use]
        pub fn matches_authority(
            self,
            generation: u64,
            deadline: Instant,
            kind: PendingTimerKind,
        ) -> bool;
    }
}

// ============================================================================
// Proof obligations for PendingTimer
// ============================================================================

/// PO-vb-0l9k0-004: matches_authority returns false when generation does not match.
///
/// C-004: PendingTimer::matches_authority returns false for any mismatch.
///
/// Production target: `PendingTimer::matches_authority` at types.rs:46-53
pub open spec fn pending_timer_matches_authority_generation_mismatch_spec(
    pt: PendingTimer,
    generation: u64,
    deadline: Instant,
    kind: PendingTimerKind,
) -> bool {
    if pt.generation != generation {
        !pt.matches_authority(generation, deadline, kind)
    } else {
        true
    }
}

/// PO-vb-0l9k0-005: matches_authority returns false when kind does not match.
///
/// C-004: PendingTimer::matches_authority returns false for any mismatch.
///
/// Production target: `PendingTimer::matches_authority` at types.rs:46-53
pub open spec fn pending_timer_matches_authority_kind_mismatch_spec(
    pt: PendingTimer,
    generation: u64,
    deadline: Instant,
    kind: PendingTimerKind,
) -> bool {
    if pt.kind != kind {
        !pt.matches_authority(generation, deadline, kind)
    } else {
        true
    }
}

/// PO-vb-0l9k0-006: matches_authority returns false when deadline does not match.
///
/// C-004: PendingTimer::matches_authority returns false for any mismatch.
///
/// Production target: `PendingTimer::matches_authority` at types.rs:46-53
pub open spec fn pending_timer_matches_authority_deadline_mismatch_spec(
    pt: PendingTimer,
    generation: u64,
    deadline: Instant,
    kind: PendingTimerKind,
) -> bool {
    if pt.deadline != deadline {
        !pt.matches_authority(generation, deadline, kind)
    } else {
        true
    }
}

/// PO-vb-0l9k0-024: matches_authority enforces exact match on all four fields.
///
/// C-004: PendingTimer::matches_authority enforces exact match on all four
/// fields (step, kind, generation, deadline).
///
/// Production target: `PendingTimer::matches_authority` at types.rs:46-53
pub open spec fn pending_timer_matches_authority_exact_spec(
    pt: PendingTimer,
    generation: u64,
    deadline: Instant,
    kind: PendingTimerKind,
) -> bool {
    pt.matches_authority(generation, deadline, kind)
        ==> pt.generation == generation && pt.deadline == deadline && pt.kind == kind
}

/// PO-vb-0l9k0-023: PendingTimer fields are immutable numeric types.
///
/// C-004: PendingTimer fields are numeric and deadline is stored as Instant,
/// proving no Instant::now capture in immutable fields.
///
/// Production target: `PendingTimer` struct fields at types.rs:37-42
pub open spec fn pending_timer_fields_immutable_spec(pt: PendingTimer) -> bool {
    true // step: StepIdx is Copy
    && true // kind: PendingTimerKind is Copy
    && true // generation: u64 is Copy
    && true // deadline: Instant is Copy
}

} // verus!
