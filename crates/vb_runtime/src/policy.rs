#![forbid(unsafe_code)]
//! Policy decision recording at the action outbox boundary.
//!
//! Implements master §§77.3,77.11: separate pinned admission/replay policy
//! from current live-effect policy with monotonic emergency-deny overlay.

use std::sync::{Arc, RwLock};

use vb_core::action::ActionTicket;
use vb_core::ids::WorkflowDigest;
use vb_core::policy::{DecisionCode, PolicyDigest, PolicyLayer};

use crate::error::{RuntimeError, RuntimeResult};

/// Complete policy decision record journaled at the effect boundary.
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    /// Policy layer that made the decision.
    pub layer: PolicyLayer,
    /// Decision code.
    pub code: DecisionCode,
    /// Digest of the policy configuration.
    pub policy_digest: PolicyDigest,
    /// Digest of the input (ticket) to the decision.
    pub input_digest: PolicyDigest,
    /// Whether emergency deny overlay was active.
    pub emergency_deny_active: bool,
    /// Whether compatibility logic influenced the decision.
    pub compatibility_influence: bool,
}

/// Typed live-effect policy configuration.
///
/// Controls which actions are permitted to execute at the outbox boundary.
/// Grant validation, capability scope narrowing, and policy versioning are
/// owned by other beads; this type provides the concrete permit/deny/suspend
/// decision mechanism required by this bead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveEffectPolicy {
    /// Permit all actions.
    AllowAll,
    /// Deny all actions.
    DenyAll,
    /// Suspend all actions for external review.
    SuspendAll,
}

/// Emergency deny state: monotonic overlay that can suspend future effects.
#[derive(Debug, Clone)]
enum EmergencyDenyState {
    /// No emergency deny active.
    Inactive,
    /// Emergency deny active, future effects suspended.
    Active,
}

/// Policy decision recorder component.
///
/// Evaluates current live-effect policy + emergency overlay at the outbox
/// boundary and records the decision. The recorder is shared across shards.
pub struct PolicyDecisionRecorder {
    /// Pinned admission policy digest (bound at run admission).
    admission_policy_digest: PolicyDigest,
    /// Current live-effect policy digest.
    live_effect_policy_digest: PolicyDigest,
    /// Current live-effect policy configuration.
    live_effect_policy: LiveEffectPolicy,
    /// Monotonic emergency deny overlay state.
    emergency_deny: RwLock<EmergencyDenyState>,
}

impl PolicyDecisionRecorder {
    /// Create a new policy decision recorder.
    ///
    /// `admission_policy_digest` is the digest of the accepted artifact
    /// admitted for this run. It is pinned at admission and cannot be
    /// updated.
    pub fn new(admission_policy_digest: WorkflowDigest) -> Self {
        let policy_digest = PolicyDigest(admission_policy_digest);
        Self {
            admission_policy_digest: policy_digest,
            live_effect_policy_digest: policy_digest,
            live_effect_policy: LiveEffectPolicy::AllowAll,
            emergency_deny: RwLock::new(EmergencyDenyState::Inactive),
        }
    }

    /// Compute input digest from ticket fields.
    ///
    /// Uses deterministic computation from ticket identity fields only.
    fn compute_input_digest(&self, ticket: &ActionTicket) -> PolicyDigest {
        // Deterministic digest from ticket identity fields.
        // Uses blake3 hash of packed ticket fields.
        let mut hasher = blake3::Hasher::new();
        hasher.update(&ticket.run.get().to_be_bytes());
        hasher.update(&ticket.step.get().to_be_bytes());
        hasher.update(&ticket.seq.get().to_be_bytes());
        hasher.update(&ticket.action.get().to_be_bytes());
        hasher.update(&ticket.attempt.to_be_bytes());
        let hash = hasher.finalize();
        let mut digest_bytes = [0u8; 32];
        digest_bytes.copy_from_slice(hash.as_bytes());
        PolicyDigest(WorkflowDigest::from_bytes(digest_bytes))
    }

    /// Evaluate policy at the outbox boundary for a new action ticket.
    ///
    /// Returns a PolicyDecision that must be journaled before the action
    /// is dispatched to the executor. If the decision is Denied or Suspended,
    /// the caller must not dispatch the action.
    pub fn evaluate_action(
        &self,
        ticket: &ActionTicket,
    ) -> RuntimeResult<PolicyDecision> {
        let input_digest = self.compute_input_digest(ticket);

        // Check emergency deny overlay (monotonic).
        let emergency_active = {
            let state = self.emergency_deny.read().map_err(|_| {
                RuntimeError::UnsupportedOperation {
                    operation: "policy_decision_poisoned",
                }
            })?;
            matches!(*state, EmergencyDenyState::Active)
        };

        if emergency_active {
            return Ok(PolicyDecision {
                layer: PolicyLayer::EmergencyDeny,
                code: DecisionCode::Suspended,
                policy_digest: self.live_effect_policy_digest,
                input_digest,
                emergency_deny_active: true,
                compatibility_influence: false,
            });
        }

        // Evaluate current live-effect policy configuration.
        let (layer, code) = match self.live_effect_policy {
            LiveEffectPolicy::AllowAll => (PolicyLayer::LiveEffect, DecisionCode::Allowed),
            LiveEffectPolicy::DenyAll => (PolicyLayer::LiveEffect, DecisionCode::Denied),
            LiveEffectPolicy::SuspendAll => (PolicyLayer::LiveEffect, DecisionCode::Suspended),
        };

        Ok(PolicyDecision {
            layer,
            code,
            policy_digest: self.live_effect_policy_digest,
            input_digest,
            emergency_deny_active: false,
            compatibility_influence: false,
        })
    }

    /// Reconstruct historical policy decision during replay.
    ///
    /// During replay, the decision is restored from the journal event,
    /// not re-evaluated. This preserves the replay theorem.
    pub fn reconstruct_from_journal(&self, decision: &PolicyDecision) -> PolicyDecision {
        decision.clone()
    }

    /// Activate emergency deny overlay (monotonic).
    ///
    /// Once active, cannot be cleared. Future action dispatches will be suspended.
    pub fn activate_emergency_deny(&self) -> RuntimeResult<()> {
        let mut state = self.emergency_deny.write().map_err(|_| {
            RuntimeError::UnsupportedOperation {
                operation: "policy_decision_poisoned",
            }
        })?;
        *state = EmergencyDenyState::Active;
        Ok(())
    }

    /// Update live-effect policy configuration.
    ///
    /// Called when a new policy configuration is loaded. Does not affect
    /// historical decisions already journaled.
    pub fn update_live_effect_policy(&mut self, policy: LiveEffectPolicy, digest: WorkflowDigest) {
        self.live_effect_policy = policy;
        self.live_effect_policy_digest = PolicyDigest(digest);
    }

    /// Get the pinned admission policy digest.
    pub fn admission_policy_digest(&self) -> PolicyDigest {
        self.admission_policy_digest
    }

    /// Get the current live-effect policy configuration.
    pub fn current_live_effect_policy(&self) -> LiveEffectPolicy {
        self.live_effect_policy
    }
}

#[cfg(test)]
    fn make_ticket(run: u64, step: u16, seq: u64, action: u16) -> ActionTicket {
    use super::*;
    use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

    fn make_ticket(run: u64, step: u64, seq: u64, action: u64) -> ActionTicket {
        ActionTicket {
            run: RunId::new(run),
            step: StepIdx::new(step),
            seq: SeqNo::new(seq),
            action: ActionId::new(action),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: 0,
            capacity: 3,
        }
    }

    #[test]
    fn evaluate_action_allows_when_policy_is_allow_all() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let ticket = make_ticket(1, 0, 0, 0);
        let decision = recorder.evaluate_action(&ticket).unwrap();

        assert_eq!(decision.code, DecisionCode::Allowed);
        assert_eq!(decision.layer, PolicyLayer::LiveEffect);
        assert!(!decision.emergency_deny_active);
    }

    #[test]
    fn evaluate_action_denies_when_policy_is_deny_all() {
        let mut recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        recorder.update_live_effect_policy(
            LiveEffectPolicy::DenyAll,
            WorkflowDigest::from_bytes([1u8; 32]),
        );
        let ticket = make_ticket(1, 0, 0, 0);
        let decision = recorder.evaluate_action(&ticket).unwrap();

        assert_eq!(decision.code, DecisionCode::Denied);
        assert_eq!(decision.layer, PolicyLayer::LiveEffect);
        assert!(!decision.emergency_deny_active);
    }

    #[test]
    fn evaluate_action_suspends_when_policy_is_suspend_all() {
        let mut recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        recorder.update_live_effect_policy(
            LiveEffectPolicy::SuspendAll,
            WorkflowDigest::from_bytes([2u8; 32]),
        );
        let ticket = make_ticket(1, 0, 0, 0);
        let decision = recorder.evaluate_action(&ticket).unwrap();

        assert_eq!(decision.code, DecisionCode::Suspended);
        assert_eq!(decision.layer, PolicyLayer::LiveEffect);
        assert!(!decision.emergency_deny_active);
    }

    #[test]
    fn evaluate_action_suspends_when_emergency_deny_active() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        recorder.activate_emergency_deny().unwrap();
        let ticket = make_ticket(1, 0, 0, 0);
        let decision = recorder.evaluate_action(&ticket).unwrap();

        assert_eq!(decision.code, DecisionCode::Suspended);
        assert_eq!(decision.layer, PolicyLayer::EmergencyDeny);
        assert!(decision.emergency_deny_active);
    }

    #[test]
    fn compute_input_digest_is_deterministic() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let ticket1 = make_ticket(1, 0, 0, 0);
        let ticket2 = make_ticket(1, 0, 0, 0);

        let digest1 = recorder.compute_input_digest(&ticket1);
        let digest2 = recorder.compute_input_digest(&ticket2);

        assert_eq!(digest1, digest2);
    }

    #[test]
    fn compute_input_digest_differs_for_different_tickets() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let ticket1 = make_ticket(1, 0, 0, 0);
        let ticket2 = make_ticket(1, 0, 0, 1);

        let digest1 = recorder.compute_input_digest(&ticket1);
        let digest2 = recorder.compute_input_digest(&ticket2);

        assert_ne!(digest1, digest2);
    }

    #[test]
    fn emergency_deny_is_monotonic() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));

        // Initially inactive
        let ticket = make_ticket(1, 0, 0, 0);
        let decision1 = recorder.evaluate_action(&ticket).unwrap();
        assert!(!decision1.emergency_deny_active);

        // Activate
        recorder.activate_emergency_deny().unwrap();

        // Now active
        let decision2 = recorder.evaluate_action(&ticket).unwrap();
        assert!(decision2.emergency_deny_active);

        // Cannot deactivate (no API to do so)
        let decision3 = recorder.evaluate_action(&ticket).unwrap();
        assert!(decision3.emergency_deny_active);
    }

    #[test]
    fn admission_policy_digest_is_pinned() {
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let recorder = PolicyDecisionRecorder::new(digest);

        assert_eq!(recorder.admission_policy_digest().0, digest);
    }

    #[test]
    fn live_effect_policy_updates_do_not_affect_admission_digest() {
        let admission_digest = WorkflowDigest::from_bytes([1u8; 32]);
        let mut recorder = PolicyDecisionRecorder::new(admission_digest);

        recorder.update_live_effect_policy(
            LiveEffectPolicy::DenyAll,
            WorkflowDigest::from_bytes([2u8; 32]),
        );

        assert_eq!(recorder.admission_policy_digest().0, admission_digest);
        assert_eq!(recorder.current_live_effect_policy(), LiveEffectPolicy::DenyAll);
    }
}