use super::Runtime;
use crate::{RuntimeError, RuntimeResult};
use vb_core::ActionTicket;
use vb_core::Taint;
use vb_core::policy::{DecisionCode, DecisionRecord, PinnedAdmissionPolicy, PolicyLayer};

/// Context for evaluating and persisting admission decisions.
pub(super) struct AdmissionEvalContext {
    pub ticket: ActionTicket,
    pub admission_pin: PinnedAdmissionPolicy,
    pub encoded_bytes: Vec<u8>,
    pub input_taint: Taint,
}

/// Dispatch-layer policy decision helper.
///
/// LivePolicy.evaluate returns Allowed/Denied/Suspended at the Live layer
/// but does not reject Unverified admission. This helper forces Denied at
/// the Admission layer for Unverified while preserving emergency overlay
/// priority and true live evaluation outcomes.
impl Runtime {
    fn dispatch_policy_decision(&self, decision: DecisionRecord) -> RuntimeResult<DecisionRecord> {
        // Emergency overlay is highest priority: always honor its decision.
        if decision.layer == PolicyLayer::Emergency {
            return Ok(decision);
        }

        // Unverified admission must be denied at the Admission layer for
        // every Unverified admission, regardless of prior outcome
        // (Allowed/Denied/Suspended). Emergency layer remains highest priority.
        if decision.admission == PinnedAdmissionPolicy::Unverified {
            return Ok(DecisionRecord {
                ticket: decision.ticket,
                admission: decision.admission,
                current_live: decision.current_live,
                input_digest: decision.input_digest,
                taint: decision.taint,
                outcome: DecisionCode::Denied,
                layer: PolicyLayer::Admission,
                emergency_influence: decision.emergency_influence,
                compat_influence: decision.compat_influence,
            });
        }

        Ok(decision)
    }

    /// Evaluate admission policy, apply admission priority, persist decision,
    /// and return rejection if not Allowed.
    ///
    /// This helper reads the durable emergency marker, mirrors it into the
    /// owned live policy, evaluates the actual ticket/admission_pin/encoded_bytes/
    /// input_taint, applies the admission priority rule (Unverified → Denied at
    /// Admission), persists the full final decision, and returns policy rejection
    /// carrying the decision if not Allowed.
    pub(super) fn evaluate_admission_and_persist(
        &mut self,
        ctx: AdmissionEvalContext,
    ) -> Result<DecisionRecord, DecisionRecord> {
        let durable_emergency = self.read_durable_emergency_state()?;
        if durable_emergency {
            self.live_policy.activate_emergency_deny();
        }
        let decision = self.live_policy.evaluate(
            ctx.ticket,
            ctx.admission_pin,
            &ctx.encoded_bytes,
            ctx.input_taint,
        )?;
        let decision = self.dispatch_policy_decision(decision)?;
        let shard = self.shard_for_mut(ctx.ticket.run)?;
        shard.append_policy_decision_strict(decision.clone())?;
        if decision.outcome != DecisionCode::Allowed {
            return Err(decision);
        }
        Ok(decision)
    }

    /// Helper for configure_live_policy: updates owned live policy state.
    /// Does not persist; configuration is actor-owned runtime state.
    pub(super) fn configure_live_policy_helper(
        &mut self,
        digest: vb_core::policy::PolicyDigest,
        rule: vb_core::policy::PolicyRule,
    ) {
        self.live_policy.configure(digest, rule);
    }

    /// Helper for activate_emergency_deny: persists emergency marker,
    /// verifies the returned state is Active, then mirrors into owned live policy.
    pub(super) fn activate_emergency_deny_helper(&mut self) -> RuntimeResult<()> {
        let journal = self
            .journal
            .storage_journal()
            .ok_or(RuntimeError::RecoveryNotAvailable)?;
        let state = journal.activate_emergency_deny()?;
        if !matches!(state, vb_storage::EmergencyDenyState::Active) {
            return Err(RuntimeError::RecoveryNotAvailable);
        }
        self.live_policy.activate_emergency_deny();
        Ok(())
    }

    fn read_durable_emergency_state(&self) -> RuntimeResult<bool> {
        let journal = self
            .journal
            .storage_journal()
            .ok_or(RuntimeError::RecoveryNotAvailable)?;
        let state = journal.emergency_deny_state()?;
        Ok(matches!(state, vb_storage::EmergencyDenyState::Active))
    }
}
