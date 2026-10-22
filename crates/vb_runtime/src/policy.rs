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

/// Typed live-effect policy rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveEffectPolicyRule {
    /// Permit all actions.
    PermitAll,
    /// Deny all actions.
    DenyAll,
    /// Suspend all actions for external review.
    SuspendAll,
}

/// Typed live-effect policy configuration.
///
/// `Unconfigured` is a typed deny — it is not a missing-admission fallback
/// that defaults to allow. An operator must explicitly configure policy
/// before any live effects are authorized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentLivePolicy {
    /// No policy configured — all dispatches are denied.
    Unconfigured,
    /// A real bounded policy with its authoritative digest.
    Configured { digest: PolicyDigest, rule: LiveEffectPolicyRule },
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
/// Live policy configuration is supplied per-run through an explicit
/// constructor seam; it is never derived from admission or replay history.
pub struct PolicyDecisionRecorder {
    /// Pinned admission policy digest (bound at run admission).
    admission_policy_digest: PolicyDigest,
    /// Current live-effect policy configuration.
    live_effect_policy: RwLock<CurrentLivePolicy>,
    /// Monotonic emergency deny overlay state.
    emergency_deny: RwLock<EmergencyDenyState>,
}

impl PolicyDecisionRecorder {
    /// Create a new policy decision recorder with a real pinned
    /// admission digest and no live policy configured (typed deny).
    ///
    /// `admission_policy_digest` is the digest of the accepted artifact
    /// admitted for this run. It is pinned at admission and cannot be
    /// updated. The live-effect policy starts as `Unconfigured`, so
    /// dispatches are denied until an operator explicitly configures it.
    pub fn new(admission_policy_digest: WorkflowDigest) -> Self {
        Self {
            admission_policy_digest: PolicyDigest(admission_policy_digest),
            live_effect_policy: RwLock::new(CurrentLivePolicy::Unconfigured),
            emergency_deny: RwLock::new(EmergencyDenyState::Inactive),
        }
    }

    /// Compute input digest from ticket identity plus actual input value and taint.
    ///
    /// Identity-only hashing is insufficient — the digest must bind the actual
    /// effect being authorized, including the value it will write.
    fn compute_input_digest(
        &self,
        ticket: &ActionTicket,
        input_value: &[u8],
        input_taint: vb_core::value::Taint,
    ) -> PolicyDigest {
        let mut hasher = blake3::Hasher::new();
        // Ticket identity
        hasher.update(&ticket.run.get().to_be_bytes());
        hasher.update(&ticket.step.get().to_be_bytes());
        hasher.update(&ticket.seq.get().to_be_bytes());
        hasher.update(&ticket.action.get().to_be_bytes());
        hasher.update(&ticket.attempt.to_be_bytes());
        // Actual input value
        hasher.update(input_value);
        // Taint discriminator as u8
        let taint_disc: u8 = match input_taint {
            vb_core::value::Taint::Clean => 0,
            vb_core::value::Taint::DerivedFromSecret => 1,
            vb_core::value::Taint::Secret => 2,
            vb_core::value::Taint::Random => 3,
            vb_core::value::Taint::TimeDependent => 4,
            _ => 255,
        };
        hasher.update(&taint_disc.to_be_bytes());
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
        input_value: &[u8],
        input_taint: vb_core::value::Taint,
    ) -> RuntimeResult<PolicyDecision> {
        let input_digest = self.compute_input_digest(ticket, input_value, input_taint);

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
                policy_digest: self.live_effect_policy_digest(),
                input_digest,
                emergency_deny_active: true,
                compatibility_influence: false,
            });
        }

        // Evaluate current live-effect policy configuration.
        let policy = self.live_effect_policy.read().map_err(|_| {
            RuntimeError::UnsupportedOperation {
                operation: "policy_decision_poisoned",
            }
        })?;

        let (layer, code, policy_digest) = match *policy {
            CurrentLivePolicy::Unconfigured => (
                PolicyLayer::LiveEffect,
                DecisionCode::Denied,
                PolicyDigest(WorkflowDigest::from_bytes([0u8; 32])),
            ),
            CurrentLivePolicy::Configured { digest, rule } => {
                let code = match rule {
                    LiveEffectPolicyRule::PermitAll => DecisionCode::Allowed,
                    LiveEffectPolicyRule::DenyAll => DecisionCode::Denied,
                    LiveEffectPolicyRule::SuspendAll => DecisionCode::Suspended,
                };
                (PolicyLayer::LiveEffect, code, digest)
            }
        };

        Ok(PolicyDecision {
            layer,
            code,
            policy_digest,
            input_digest,
            emergency_deny_active: false,
            compatibility_influence: false,
        })
    }

    /// Live policy digest for journaling.
    fn live_effect_policy_digest(&self) -> PolicyDigest {
        let policy = self
            .live_effect_policy
            .read()
            .expect("policy lock not poisoned");
        match *policy {
            CurrentLivePolicy::Unconfigured => PolicyDigest(WorkflowDigest::from_bytes([0u8; 32])),
            CurrentLivePolicy::Configured { digest, .. } => digest,
        }
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

    /// Update live-effect policy configuration through the explicit
    /// public runtime seam. Does not affect historical decisions already journaled.
    pub fn update_live_effect_policy(
        &self,
        policy: CurrentLivePolicy,
    ) -> RuntimeResult<()> {
        let mut live = self.live_effect_policy.write().map_err(|_| {
            RuntimeError::UnsupportedOperation {
                operation: "policy_decision_poisoned",
            }
        })?;
        *live = policy;
        Ok(())
    }

    /// Get the pinned admission policy digest.
    pub fn admission_policy_digest(&self) -> PolicyDigest {
        self.admission_policy_digest
    }

    /// Get the current live-effect policy configuration.
    pub fn current_live_effect_policy(&self) -> RuntimeResult<CurrentLivePolicy> {
        let live = self.live_effect_policy.read().map_err(|_| {
            RuntimeError::UnsupportedOperation {
                operation: "policy_decision_poisoned",
            }
        })?;
        Ok(*live)
    }
}

/// Test helper: construct a PermitAll policy with a test digest.
#[cfg(test)]
pub fn test_permit_all_policy() -> CurrentLivePolicy {
    use vb_core::ids::WorkflowDigest;
    CurrentLivePolicy::Configured {
        digest: PolicyDigest(WorkflowDigest::from_bytes([0u8; 32])),
        rule: LiveEffectPolicyRule::PermitAll,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
    use vb_core::value::Taint;

    fn make_ticket(run: u64, step: u64, seq: u64, action: u64) -> ActionTicket {
        ActionTicket {
            run: RunId::new(run),
            step: StepIdx::new(step as u16),
            seq: SeqNo::new(seq),
            action: ActionId::new(action as u16),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: 0,
            capacity: 3,
        }
    }

    fn empty_input() -> Vec<u8> {
        Vec::new()
    }

    #[test]
    fn unconfigured_policy_denies_all_dispatches() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let ticket = make_ticket(1, 0, 0, 0);
        let decision = recorder
            .evaluate_action(&ticket, &empty_input(), Taint::Clean)
            .unwrap();
        assert_eq!(decision.code, DecisionCode::Denied);
        assert_eq!(decision.layer, PolicyLayer::LiveEffect);
        assert!(!decision.emergency_deny_active);
    }

    #[test]
    fn allow_all_permits_after_explicit_configuration() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let config_digest = WorkflowDigest::from_bytes([1u8; 32]);
        recorder
            .update_live_effect_policy(CurrentLivePolicy::Configured {
                digest: PolicyDigest(config_digest),
                rule: LiveEffectPolicyRule::PermitAll,
            })
            .unwrap();
        let ticket = make_ticket(1, 0, 0, 0);
        let decision = recorder
            .evaluate_action(&ticket, &empty_input(), Taint::Clean)
            .unwrap();
        assert_eq!(decision.code, DecisionCode::Allowed);
        assert_eq!(decision.layer, PolicyLayer::LiveEffect);
        assert!(!decision.emergency_deny_active);
        assert_eq!(decision.policy_digest, PolicyDigest(config_digest));
    }

    #[test]
    fn deny_all_denies_after_explicit_configuration() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let config_digest = WorkflowDigest::from_bytes([1u8; 32]);
        recorder
            .update_live_effect_policy(CurrentLivePolicy::Configured {
                digest: PolicyDigest(config_digest),
                rule: LiveEffectPolicyRule::DenyAll,
            })
            .unwrap();
        let ticket = make_ticket(1, 0, 0, 0);
        let decision = recorder
            .evaluate_action(&ticket, &empty_input(), Taint::Clean)
            .unwrap();
        assert_eq!(decision.code, DecisionCode::Denied);
        assert_eq!(decision.layer, PolicyLayer::LiveEffect);
        assert_eq!(decision.policy_digest, PolicyDigest(config_digest));
    }

    #[test]
    fn suspend_all_suspends_after_explicit_configuration() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let config_digest = WorkflowDigest::from_bytes([2u8; 32]);
        recorder
            .update_live_effect_policy(CurrentLivePolicy::Configured {
                digest: PolicyDigest(config_digest),
                rule: LiveEffectPolicyRule::SuspendAll,
            })
            .unwrap();
        let ticket = make_ticket(1, 0, 0, 0);
        let decision = recorder
            .evaluate_action(&ticket, &empty_input(), Taint::Clean)
            .unwrap();
        assert_eq!(decision.code, DecisionCode::Suspended);
        assert_eq!(decision.layer, PolicyLayer::LiveEffect);
        assert_eq!(decision.policy_digest, PolicyDigest(config_digest));
    }

    #[test]
    fn emergency_deny_suspends_regardless_of_live_policy() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let config_digest = WorkflowDigest::from_bytes([1u8; 32]);
        recorder
            .update_live_effect_policy(CurrentLivePolicy::Configured {
                digest: PolicyDigest(config_digest),
                rule: LiveEffectPolicyRule::PermitAll,
            })
            .unwrap();
        recorder.activate_emergency_deny().unwrap();
        let ticket = make_ticket(1, 0, 0, 0);
        let decision = recorder
            .evaluate_action(&ticket, &empty_input(), Taint::Clean)
            .unwrap();
        assert_eq!(decision.code, DecisionCode::Suspended);
        assert_eq!(decision.layer, PolicyLayer::EmergencyDeny);
        assert!(decision.emergency_deny_active);
    }

    #[test]
    fn input_digest_includes_actual_value() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let ticket = make_ticket(1, 0, 0, 0);
        let value_a = vec![1u8, 2, 3];
        let value_b = vec![4u8, 5, 6];
        let digest_a = recorder.compute_input_digest(&ticket, &value_a, Taint::Clean);
        let digest_b = recorder.compute_input_digest(&ticket, &value_b, Taint::Clean);
        assert_ne!(digest_a, digest_b, "different values must yield different digests");
    }

    #[test]
    fn input_digest_includes_taint() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let ticket = make_ticket(1, 0, 0, 0);
        let value = vec![1u8, 2, 3];
        let digest_a = recorder.compute_input_digest(&ticket, &value, Taint::Clean);
        let digest_b = recorder.compute_input_digest(&ticket, &value, Taint::DerivedFromSecret);
        assert_ne!(digest_a, digest_b, "different taints must yield different digests");
    }

    #[test]
    fn input_digest_is_deterministic() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let ticket1 = make_ticket(1, 0, 0, 0);
        let ticket2 = make_ticket(1, 0, 0, 0);
        let value = vec![1u8, 2, 3];
        let digest1 = recorder.compute_input_digest(&ticket1, &value, Taint::Clean);
        let digest2 = recorder.compute_input_digest(&ticket2, &value, Taint::Clean);
        assert_eq!(digest1, digest2, "same input must yield same digest");
    }

    #[test]
    fn emergency_deny_is_monotonic() {
        let recorder = PolicyDecisionRecorder::new(WorkflowDigest::from_bytes([0u8; 32]));
        let config_digest = WorkflowDigest::from_bytes([1u8; 32]);
        recorder
            .update_live_effect_policy(CurrentLivePolicy::Configured {
                digest: PolicyDigest(config_digest),
                rule: LiveEffectPolicyRule::PermitAll,
            })
            .unwrap();
        let ticket = make_ticket(1, 0, 0, 0);
        let decision1 = recorder
            .evaluate_action(&ticket, &empty_input(), Taint::Clean)
            .unwrap();
        assert!(!decision1.emergency_deny_active);
        recorder.activate_emergency_deny().unwrap();
        let decision2 = recorder
            .evaluate_action(&ticket, &empty_input(), Taint::Clean)
            .unwrap();
        assert!(decision2.emergency_deny_active);
        let decision3 = recorder
            .evaluate_action(&ticket, &empty_input(), Taint::Clean)
            .unwrap();
        assert!(decision3.emergency_deny_active);
    }

    #[test]
    fn admission_policy_digest_is_pinned() {
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let recorder = PolicyDecisionRecorder::new(digest);
        assert_eq!(recorder.admission_policy_digest(), PolicyDigest(digest));
    }

    #[test]
    fn live_effect_policy_updates_do_not_affect_admission_digest() {
        let admission_digest = WorkflowDigest::from_bytes([1u8; 32]);
        let recorder = PolicyDecisionRecorder::new(admission_digest);
        let live_digest = WorkflowDigest::from_bytes([2u8; 32]);
        recorder
            .update_live_effect_policy(CurrentLivePolicy::Configured {
                digest: PolicyDigest(live_digest),
                rule: LiveEffectPolicyRule::DenyAll,
            })
            .unwrap();
        assert_eq!(recorder.admission_policy_digest(), PolicyDigest(admission_digest));
        assert_eq!(
            recorder.current_live_effect_policy().unwrap(),
            CurrentLivePolicy::Configured {
                digest: PolicyDigest(live_digest),
                rule: LiveEffectPolicyRule::DenyAll
            }
        );
    }
}