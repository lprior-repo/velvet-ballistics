#![forbid(unsafe_code)]

use crate::{RuntimeError, RuntimeResult};

/// Returns the origin only when the journal contains a canonical unique prefix.
#[must_use]
#[cfg(test)]
pub(crate) fn hydrate_run_budget_origin(events: &[vb_storage::JournalEvent]) -> Option<u64> {
    let run = events.first()?.run_id();
    let budgeted = matches!(
        events.get(1),
        Some(vb_storage::JournalEvent::RunAdmissionBudget { .. })
    );
    parse_budget_origin_prefix(events, run, budgeted)
        .ok()
        .flatten()
}

/// Validates origin identity, uniqueness, order, and admission binding.
pub(crate) fn recover_run_budget_origin(
    events: &[vb_storage::JournalEvent],
    run: vb_core::RunId,
    admission: &crate::admission::RunAdmission,
) -> RuntimeResult<Option<u64>> {
    let budgeted = admission.budget().is_some();
    let found = parse_budget_origin_prefix(events, run, budgeted)?;
    resolve_run_started_millis(Some(admission), found)
}

fn parse_budget_origin_prefix(
    events: &[vb_storage::JournalEvent],
    run: vb_core::RunId,
    budgeted: bool,
) -> RuntimeResult<Option<u64>> {
    validate_origin_header(events, run, budgeted)?;
    let mut prefix = BudgetOriginPrefix::new(run, budgeted);
    events
        .iter()
        .skip(2)
        .try_for_each(|event| prefix.accept(event))?;
    prefix.finish()
}

fn validate_origin_header(
    events: &[vb_storage::JournalEvent],
    run: vb_core::RunId,
    budgeted: bool,
) -> RuntimeResult<()> {
    let accepted = events.first().ok_or_else(invalid_origin_prefix)?;
    let admission = events.get(1).ok_or_else(invalid_origin_prefix)?;
    if !matches!(accepted, vb_storage::JournalEvent::RunAccepted { run: found, seq, .. }
        if *found == run && seq.get() == 0)
    {
        return Err(invalid_origin_prefix());
    }
    let valid_admission = match admission {
        vb_storage::JournalEvent::RunAdmissionBudget {
            run: found, seq, ..
        } => *found == run && seq.get() == 1 && budgeted,
        vb_storage::JournalEvent::RunAdmission {
            run: found, seq, ..
        } => *found == run && seq.get() == 1 && !budgeted,
        _ => false,
    };
    if valid_admission {
        Ok(())
    } else {
        Err(invalid_origin_prefix())
    }
}

struct BudgetOriginPrefix {
    run: vb_core::RunId,
    budgeted: bool,
    next_seq: u64,
    compatibility_seen: bool,
    tenant_seen: bool,
    origin: Option<u64>,
    closed: bool,
}

impl BudgetOriginPrefix {
    fn new(run: vb_core::RunId, budgeted: bool) -> Self {
        Self {
            run,
            budgeted,
            next_seq: 2,
            compatibility_seen: false,
            tenant_seen: false,
            origin: None,
            closed: false,
        }
    }

    fn accept(&mut self, event: &vb_storage::JournalEvent) -> RuntimeResult<()> {
        match event {
            vb_storage::JournalEvent::CompatibilityDecisionEvent { run, seq, .. } => {
                self.record_optional(run, seq, true)?;
                self.compatibility_seen = true;
            }
            vb_storage::JournalEvent::TenantAdmissionEvent { run, seq, .. } => {
                self.record_optional(run, seq, false)?;
                self.tenant_seen = true;
            }
            vb_storage::JournalEvent::RunBudgetOriginEvent {
                run,
                seq,
                started_millis,
            } => self.record_origin(run, seq, *started_millis)?,
            _ => self.closed = true,
        }
        Ok(())
    }

    fn record_optional(
        &mut self,
        run: &vb_core::RunId,
        seq: &vb_storage::EventSeq,
        compatibility: bool,
    ) -> RuntimeResult<()> {
        let repeated = if compatibility {
            self.compatibility_seen
        } else {
            self.tenant_seen
        };
        if self.closed || self.origin.is_some() || repeated {
            return Err(invalid_origin_prefix());
        }
        self.validate_position(run, seq)?;
        self.advance()
    }

    fn record_origin(
        &mut self,
        run: &vb_core::RunId,
        seq: &vb_storage::EventSeq,
        started_millis: u64,
    ) -> RuntimeResult<()> {
        if self.closed || !self.budgeted || self.origin.is_some() {
            return Err(invalid_origin_prefix());
        }
        self.validate_position(run, seq)?;
        self.origin = Some(started_millis);
        self.advance()
    }

    fn validate_position(
        &self,
        run: &vb_core::RunId,
        seq: &vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        if *run == self.run && seq.get() == self.next_seq {
            Ok(())
        } else {
            Err(invalid_origin_prefix())
        }
    }

    fn advance(&mut self) -> RuntimeResult<()> {
        self.next_seq = self
            .next_seq
            .checked_add(1)
            .ok_or_else(invalid_origin_prefix)?;
        Ok(())
    }

    fn finish(self) -> RuntimeResult<Option<u64>> {
        if self.budgeted && self.origin.is_none() {
            return Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("budget_origin_missing"),
            });
        }
        Ok(self.origin)
    }
}

fn invalid_origin_prefix() -> RuntimeError {
    RuntimeError::RecoveryCannotResume {
        reason: String::from("budget_origin_invalid"),
    }
}

/// Resolves the single durable wall-clock origin for a recovered admission.
///
/// A budgeted admission without any durable origin is refused with a typed
/// missing-authority recovery error; it is never restarted at `now` and never
/// silently expired at epoch zero.
pub(crate) fn resolve_run_started_millis(
    admission: Option<&crate::admission::RunAdmission>,
    recovered: Option<u64>,
) -> RuntimeResult<Option<u64>> {
    let budgeted = admission.is_some_and(|admission| admission.budget().is_some());
    match (recovered, budgeted) {
        (Some(started), _) => Ok(Some(started)),
        (None, true) => Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("budget_origin_missing"),
        }),
        (None, false) => Ok(None),
    }
}
