#![forbid(unsafe_code)]

use super::super::{Driver, invalid};
use crate::JournalEvent as E;
use crate::recovery::RecoveryResult;

impl Driver<'_> {
    pub(super) fn apply_admission_event(&mut self, event: &E) -> RecoveryResult<()> {
        if let Some(result) = self.apply_admission_header(event) {
            return result;
        }
        if let Some(result) = self.apply_origin_prefix_event(event) {
            return result;
        }
        match event {
            E::RunContinuation { .. } if self.accepted && self.admitted && !self.started => Ok(()),
            E::PolicyDecisionEvent { .. }
            | E::GateOpenedEvent { .. }
            | E::GateDecisionEvent { .. }
            | E::GateOpenedBoundEvent { .. }
            | E::GateDecisionBoundEvent { .. }
            | E::RetryCheckpoint { .. }
                if self.accepted =>
            {
                Ok(())
            }
            _ => Err(self.unsupported_event(event)),
        }
    }

    fn apply_admission_header(&mut self, event: &E) -> Option<RecoveryResult<()>> {
        match event {
            E::RunAccepted { workflow, seq, .. } => {
                Some(self.accept_run_header(*workflow, seq.get(), event))
            }
            E::RunAdmission { seq, .. } => Some(self.accept_admission(seq.get(), false, event)),
            E::RunAdmissionBudget { seq, .. } => {
                Some(self.accept_admission(seq.get(), true, event))
            }
            _ => None,
        }
    }

    fn accept_run_header(
        &mut self,
        workflow: vb_core::WorkflowDigest,
        seq: u64,
        event: &E,
    ) -> RecoveryResult<()> {
        if self.accepted || workflow != self.plan.digest() || seq != 0 {
            return Err(self.unsupported_event(event));
        }
        self.accepted = true;
        Ok(())
    }

    fn accept_admission(&mut self, seq: u64, budgeted: bool, event: &E) -> RecoveryResult<()> {
        if !self.accepted || self.admitted || self.started || seq != 1 {
            return Err(self.unsupported_event(event));
        }
        self.admitted = true;
        self.budgeted_admission = budgeted;
        Ok(())
    }

    fn apply_origin_prefix_event(&mut self, event: &E) -> Option<RecoveryResult<()>> {
        match event {
            E::CompatibilityDecisionEvent { seq, .. } => {
                Some(self.accept_optional_admission(seq.get(), true, event))
            }
            E::TenantAdmissionEvent { seq, .. } => {
                Some(self.accept_optional_admission(seq.get(), false, event))
            }
            E::RunBudgetOriginEvent { seq, .. } => {
                Some(self.accept_budget_origin(seq.get(), event))
            }
            _ => None,
        }
    }

    fn accept_optional_admission(
        &mut self,
        seq: u64,
        compatibility: bool,
        event: &E,
    ) -> RecoveryResult<()> {
        if !self.valid_optional_admission(seq, compatibility) {
            return Err(self.unsupported_event(event));
        }
        if compatibility {
            self.compatibility_seen = true;
        } else {
            self.tenant_seen = true;
        }
        self.advance_admission_seq()
    }

    fn accept_budget_origin(&mut self, seq: u64, event: &E) -> RecoveryResult<()> {
        if !self.valid_budget_origin(seq) {
            return Err(self.unsupported_event(event));
        }
        self.origin_seen = true;
        self.advance_admission_seq()
    }

    fn valid_optional_admission(&self, seq: u64, compatibility: bool) -> bool {
        self.accepted
            && self.admitted
            && !self.admission_prefix_closed
            && !self.origin_seen
            && seq == self.next_admission_seq
            && if compatibility {
                !self.compatibility_seen
            } else {
                !self.tenant_seen
            }
    }

    fn valid_budget_origin(&self, seq: u64) -> bool {
        self.accepted
            && self.admitted
            && self.budgeted_admission
            && !self.admission_prefix_closed
            && !self.origin_seen
            && seq == self.next_admission_seq
    }

    fn advance_admission_seq(&mut self) -> RecoveryResult<()> {
        self.next_admission_seq = self
            .next_admission_seq
            .checked_add(1)
            .ok_or_else(|| invalid(self.frame.pc(), "admission sequence overflow"))?;
        Ok(())
    }
}
