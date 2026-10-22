#![forbid(unsafe_code)]
//! Actor-owned pure live policy evaluator.
//!
//! Runtime evaluation of action tickets against live policy configuration,
//! with pinned admission policy and emergency overlay.
//!
//! Plain owned state with no interior mutability, Arc, Mutex, or atomics.
//! Configuration is applied via `configure`. Emergency deny is monotonic
//! once activated.
//!
//! Produces immutable decision records that retain full identity information
//! but cannot mint new effect authority.

use crate::{RuntimeError, RuntimeResult};
use vb_core::action::ActionTicket;
use vb_core::policy::{
    CurrentLivePolicy, DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy,
    PolicyDigest, PolicyLayer, PolicyRule, encode_taint_canonical,
};
use vb_core::value::Taint;

/// Live policy evaluator maintaining live configuration and emergency
/// overlay state.
///
/// Owned by a single actor. Constructor takes no admission pin — that is
/// supplied per-run to `evaluate`.
pub struct LivePolicy {
    /// Current live policy configuration.
    current_live: CurrentLivePolicy,
    /// Whether emergency deny overlay is active (monotonic).
    emergency_deny_active: bool,
}

impl Default for LivePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl LivePolicy {
    /// Creates a new live policy evaluator.
    ///
    /// Initializes with Unconfigured live policy and inactive overlay.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_live: CurrentLivePolicy::Unconfigured,
            emergency_deny_active: false,
        }
    }

    /// Returns the current live policy configuration.
    #[must_use]
    pub fn live(&self) -> &CurrentLivePolicy {
        &self.current_live
    }

    /// Returns whether emergency deny overlay is active.
    #[must_use]
    pub fn emergency_active(&self) -> bool {
        self.emergency_deny_active
    }

    /// Configures the live policy with a verified artifact and rule.
    pub fn configure(&mut self, digest: PolicyDigest, rule: PolicyRule) {
        self.current_live = CurrentLivePolicy::Configured { digest, rule };
    }

    /// Activates the emergency deny overlay.
    ///
    /// Monotonic: once active, remains active for the lifetime of this
    /// evaluator. No clear/reset API.
    pub fn activate_emergency_deny(&mut self) {
        self.emergency_deny_active = true;
    }

    /// Evaluates an action ticket against live policy.
    ///
    /// # Arguments
    /// - `ticket`: the action ticket to evaluate (full 8-field identity)
    /// - `pinned_admission`: the pinned admission policy for this run
    /// - `input_bytes`: canonical value bytes of the action input
    /// - `taint`: taint level of the action input
    ///
    /// # Evaluation logic
    /// - Emergency overlay active → Denied at Emergency layer
    /// - Policy unconfigured → Denied at Live layer
    /// - Matching rule → rule's code at Live layer
    /// - Non-matching rule → Denied at Live layer
    pub fn evaluate(
        &self,
        ticket: ActionTicket,
        pinned_admission: PinnedAdmissionPolicy,
        input_bytes: &[u8],
        taint: Taint,
    ) -> RuntimeResult<DecisionRecord> {
        let input_digest = compute_input_digest(input_bytes, taint, ticket)?;
        let live_identity = self.current_live.identity();

        let (outcome, layer, emergency_influence) = self.evaluate_layer(ticket)?;

        Ok(DecisionRecord {
            ticket,
            admission: pinned_admission,
            current_live: live_identity,
            input_digest,
            taint,
            outcome,
            layer,
            emergency_influence,
            compat_influence: InfluenceState::NotEvaluated,
        })
    }

    fn evaluate_layer(
        &self,
        ticket: ActionTicket,
    ) -> RuntimeResult<(DecisionCode, PolicyLayer, InfluenceState)> {
        if self.emergency_deny_active {
            return Ok((
                DecisionCode::Denied,
                PolicyLayer::Emergency,
                InfluenceState::Active,
            ));
        }

        self.evaluate_live_layer(ticket)
    }

    fn evaluate_live_layer(
        &self,
        ticket: ActionTicket,
    ) -> RuntimeResult<(DecisionCode, PolicyLayer, InfluenceState)> {
        match &self.current_live {
            CurrentLivePolicy::Unconfigured => Ok((
                DecisionCode::Denied,
                PolicyLayer::Live,
                InfluenceState::Inactive,
            )),
            CurrentLivePolicy::Configured { rule, .. } => {
                if rule.action == ticket.action {
                    Ok((rule.code, PolicyLayer::Live, InfluenceState::Inactive))
                } else {
                    Ok((
                        DecisionCode::Denied,
                        PolicyLayer::Live,
                        InfluenceState::Inactive,
                    ))
                }
            }
        }
    }
}

fn compute_input_digest(
    input_bytes: &[u8],
    taint: Taint,
    ticket: ActionTicket,
) -> RuntimeResult<PolicyDigest> {
    let taint_byte = encode_taint_canonical(taint).ok_or_else(|| {
        let core = vb_core::errors::CoreError::InternalInvariantViolation {
            reason: "unknown taint variant",
        };
        RuntimeError::Core {
            source: Box::new(core),
        }
    })?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"velvet-ballistics:policy:input:");
    hasher.update(input_bytes);
    hasher.update(b"velvet-ballistics:policy:taint:");
    hasher.update(&[taint_byte]);
    hasher.update(b"velvet-ballistics:policy:ticket:");
    push_ticket_parts(&mut hasher, ticket);

    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(result.as_bytes());
    Ok(PolicyDigest::from_bytes(output))
}

fn push_ticket_parts(hasher: &mut blake3::Hasher, ticket: ActionTicket) {
    hasher.update(b"run:");
    hasher.update(&ticket.run.get().to_be_bytes());
    hasher.update(b"step:");
    hasher.update(&ticket.step.get().to_be_bytes());
    hasher.update(b"seq:");
    hasher.update(&ticket.seq.get().to_be_bytes());
    hasher.update(b"action:");
    hasher.update(&ticket.action.get().to_be_bytes());
    hasher.update(b"run_attempt:");
    hasher.update(&ticket.run_attempt.to_be_bytes());
    hasher.update(b"attempt:");
    hasher.update(&ticket.attempt.to_be_bytes());
    hasher.update(b"key:");
    hasher.update(&ticket.idempotency_key.to_be_bytes());
    hasher.update(b"capacity:");
    hasher.update(&ticket.capacity.to_be_bytes());
}

#[cfg(test)]
#[path = "policy/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "policy/ticket_field_variation_tests.rs"]
mod ticket_field_variation_tests;

#[cfg(test)]
#[path = "policy/taint_tests.rs"]
mod taint_tests;

#[cfg(test)]
#[path = "policy/emergency_and_admission_tests.rs"]
mod emergency_and_admission_tests;
