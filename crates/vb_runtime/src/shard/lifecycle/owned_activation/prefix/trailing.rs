use super::{EventSeq, JournalEvent, RunId, RuntimeError, RuntimeResult, identity};

pub(super) struct TrailingPrefix {
    pub(super) next_seq: EventSeq,
    pub(super) origin: Option<u64>,
    pub(super) tenant: Option<vb_core::ids::TenantId>,
    compatibility_seen: bool,
}

pub(super) fn parse(
    run: RunId,
    events: &[JournalEvent],
    budgeted: bool,
) -> RuntimeResult<TrailingPrefix> {
    let mut prefix = TrailingPrefix {
        next_seq: EventSeq::new(2),
        origin: None,
        tenant: None,
        compatibility_seen: false,
    };
    for event in events {
        prefix.apply(run, event)?;
    }
    if budgeted && prefix.origin.is_none() {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "owned_activation_budget_origin_missing",
        });
    }
    Ok(prefix)
}

impl TrailingPrefix {
    fn apply(&mut self, run: RunId, event: &JournalEvent) -> RuntimeResult<()> {
        match event {
            JournalEvent::CompatibilityDecisionEvent {
                run: event_run,
                seq,
                ..
            } => self.record_compatibility(run, *event_run, *seq)?,
            JournalEvent::TenantAdmissionEvent {
                run: event_run,
                seq,
                tenant,
                ..
            } => self.record_tenant(run, *event_run, *seq, *tenant)?,
            JournalEvent::RunBudgetOriginEvent {
                run: event_run,
                seq,
                started_millis,
            } => self.record_origin(run, *event_run, *seq, *started_millis)?,
            _ => return Err(identity::mismatch()),
        }
        self.advance()
    }

    fn record_compatibility(
        &mut self,
        run: RunId,
        event_run: RunId,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.validate_identity(run, event_run, seq)?;
        if self.compatibility_seen {
            return Err(identity::mismatch());
        }
        self.compatibility_seen = true;
        Ok(())
    }

    fn record_tenant(
        &mut self,
        run: RunId,
        event_run: RunId,
        seq: EventSeq,
        tenant: vb_core::ids::TenantId,
    ) -> RuntimeResult<()> {
        self.validate_identity(run, event_run, seq)?;
        if self.tenant.replace(tenant).is_some() {
            return Err(identity::mismatch());
        }
        Ok(())
    }

    fn record_origin(
        &mut self,
        run: RunId,
        event_run: RunId,
        seq: EventSeq,
        origin: u64,
    ) -> RuntimeResult<()> {
        self.validate_identity(run, event_run, seq)?;
        if self.origin.replace(origin).is_some() {
            return Err(identity::mismatch());
        }
        Ok(())
    }

    fn validate_identity(&self, run: RunId, event_run: RunId, seq: EventSeq) -> RuntimeResult<()> {
        if event_run != run || seq != self.next_seq {
            return Err(identity::mismatch());
        }
        Ok(())
    }

    fn advance(&mut self) -> RuntimeResult<()> {
        self.next_seq = EventSeq::new(
            self.next_seq
                .get()
                .checked_add(1)
                .ok_or(vb_storage::JournalError::SequenceOverflow)?,
        );
        Ok(())
    }
}
