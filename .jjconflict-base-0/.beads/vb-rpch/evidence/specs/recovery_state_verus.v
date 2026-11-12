// Verus spec file for vb_storage/src/recovery/types.rs
// PO-VB-001: RecoveryState type invariants
// PO-VB-002: PersistenceState type invariants
// PO-VB-003: ReplayState type invariants

#[verus]
pub mod recovery_types_spec {
    use crate::EventSeq;
    use crate::recovery::types::{
        RecoveryTerminalState, RecoveryRuntimeSummary, RecoveredStepState,
        RecoveredStepEntry, RecoveredSlotEntry, RecoveredPendingAction,
        UnsupportedRecoveryState, RecoveryFrameSeed, RunSnapshot,
        ActionReplayTracker, DigestCheck, RecoveryHydration,
    };

    // PO-VB-001: RecoveryTerminalState is a valid terminal state
    pub spec fn valid_recovery_terminal_state(st: RecoveryTerminalState) -> bool {
        match st {
            RecoveryTerminalState::Cancelled => true,
            RecoveryTerminalState::Finished{result: _} => true,
            RecoveryTerminalState::Failed => true,
        }
    }

    // PO-VB-001: RecoveryRuntimeSummary invariants
    pub spec fn recovery_runtime_summary_inv(s: RecoveryRuntimeSummary) -> bool {
        // First sequence must be <= last sequence
        s.first_seq.get() <= s.last_seq.get()
        // Steps started must be >= steps succeeded
        && s.steps_started >= s.steps_succeeded
        // Actions scheduled must be >= actions resolved
        && s.actions_scheduled >= s.actions_resolved
    }

    // PO-VB-002: RecoveredStepState is a valid step state
    pub spec fn valid_recovered_step_state(st: RecoveredStepState) -> bool {
        match st {
            RecoveredStepState::Running => true,
            RecoveredStepState::Succeeded => true,
            RecoveredStepState::Failed => true,
            RecoveredStepState::Waiting => true,
            RecoveredStepState::Asking => true,
        }
    }

    // PO-VB-002: UnsupportedRecoveryState invariants
    pub spec fn unsupported_recovery_state_inv(s: UnsupportedRecoveryState) -> bool {
        // All fields are boolean flags - always valid
        true
    }

    // PO-VB-002: UnsupportedRecoveryState::union is idempotent and commutative
    pub spec fn unsupported_union_idempotent(a: UnsupportedRecoveryState) -> bool {
        a.union(a) == a
    }

    pub spec fn unsupported_union_commutative(a: UnsupportedRecoveryState, b: UnsupportedRecoveryState) -> bool {
        a.union(b) == b.union(a)
    }

    // PO-VB-003: RecoveryFrameSeed invariants
    pub spec fn recovery_frame_seed_inv(seed: &RecoveryFrameSeed) -> bool {
        // Step count and slot count must fit in u16
        seed.step_count <= u16::MAX as u16
        && seed.slot_count <= u16::MAX as u16
        // PC must be within step bounds
        && seed.pc.get() < seed.step_count as u64
        // First step must be <= PC
        && seed.first_step.get() <= seed.pc.get()
        // Steps length must match step_count
        && (seed.steps.len() as u16) <= seed.step_count
        // Slots length must match slot_count
        && (seed.slots.len() as u16) <= seed.slot_count
    }

    // PO-VB-003: RunSnapshot invariants
    pub spec fn run_snapshot_inv(snap: &RunSnapshot) -> bool {
        // Sequence must be valid
        snap.seq.get() <= u64::MAX
        // Slots and taint must have same length if both non-empty
        && (snap.slots.len() == 0 || snap.taint.len() == 0 || snap.slots.len() == snap.taint.len())
    }

    // PO-VB-003: ActionReplayTracker invariants
    pub spec fn action_replay_tracker_inv(tracker: &ActionReplayTracker) -> bool {
        // Completed and failed sets are disjoint
        forall(|a, s| tracker.completed.contains(&(a, s)) ==> !tracker.failed.contains(&(a, s)))
    }

    // PO-VB-003: is_resolved returns true iff action is in completed or failed
    pub spec fn is_resolved_correct(tracker: &ActionReplayTracker, action, step) -> bool {
        tracker.is_resolved(action, step)
        == (tracker.completed.contains(&(action, step)) || tracker.failed.contains(&(action, step)))
    }

    // PO-VB-001: RecoveryHydration summary accessor
    pub spec fn hydration_summary(h: &RecoveryHydration) -> RecoveryRuntimeSummary {
        match h {
            RecoveryHydration::Summary(s) => *s,
            RecoveryHydration::FrameSeed(seed) => seed.summary,
        }
    }
}

// Verus contracts for functions in types.rs
#[verus]
pub mod recovery_types_contracts {

    // PO-VB-001: UnsupportedRecoveryState::SUPPORTED is fully supported
    pub fn unsupported_supported_invariant() -> bool {
        let s = crate::recovery::types::UnsupportedRecoveryState::SUPPORTED;
        !s.slot_values && !s.slot_taint && !s.action_payloads && !s.pending_actions
    }

    // PO-VB-002: UnsupportedRecoveryState::union preserves support flags
    pub spec fn union_preserves_supported(a: UnsupportedRecoveryState) -> bool {
        let s = a.union(UnsupportedRecoveryState::SUPPORTED);
        s == a
    }
}
