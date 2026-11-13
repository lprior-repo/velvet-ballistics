use super::LaneToken;
use super::group::GroupControl;
use super::group_types::*;

/// Exclusive prepared event; dropping it leaves the entire group unchanged.
pub struct PreparedGroupEvent<'a> {
    pub(super) group: &'a mut GroupControl,
    pub(super) event: GroupEvent,
    pub(super) next_completed: u32,
}

impl<'a> PreparedGroupEvent<'a> {
    pub fn event(&self) -> GroupEvent {
        self.event
    }
}

impl GroupControl {
    fn prepared(
        &mut self,
        kind: GroupEventKind,
        next_completed: u32,
    ) -> Result<PreparedGroupEvent<'_>, GroupError> {
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(GroupError::RevisionOverflow)?;
        let event = GroupEvent {
            group: self.group,
            revision: next_revision,
            kind,
        };
        Ok(PreparedGroupEvent {
            group: self,
            event,
            next_completed,
        })
    }

    pub fn prepare_admit(&mut self) -> Result<PreparedGroupEvent<'_>, GroupError> {
        self.require_running()?;
        let token = self.pool.preview_admission()?;
        match self.lanes.get(usize::from(token.physical_slot())) {
            Some(LaneRecord::Vacant) => {}
            Some(_) => return Err(GroupError::InvalidTransition),
            None => return Err(GroupError::RepresentationOverflow),
        }
        self.pending(token)?;
        self.prepared(GroupEventKind::Admission(token), self.completed)
    }

    pub fn prepare_completion(
        &mut self,
        token: LaneToken,
        outcome: LaneCompletion,
    ) -> Result<PreparedGroupEvent<'_>, GroupError> {
        self.require_running()?;
        let lane = self.record(token)?;
        match lane {
            LaneRecord::Live(t) if *t == token => {}
            LaneRecord::Live(_) => return Err(GroupError::InvalidTransition),
            LaneRecord::Vacant => return Err(GroupError::InvalidTransition),
            LaneRecord::CancelRequested(_) => return Err(GroupError::InvalidTransition),
        }
        self.pending(token)?;
        let next_completed = self
            .completed
            .checked_add(1)
            .ok_or(GroupError::RepresentationOverflow)?;
        self.prepared(
            GroupEventKind::Completion { token, outcome },
            next_completed,
        )
    }

    /// Prepare acknowledgment that owned frames, queues, timers, asks and authority are drained.
    /// The caller supplies that guarantee; external physical effects cannot be undone here.
    pub fn prepare_drain(
        &mut self,
        winner_token: LaneToken,
    ) -> Result<PreparedGroupEvent<'_>, GroupError> {
        match self.phase {
            GroupPhase::Draining(_) => {}
            _ => return Err(GroupError::InvalidTransition),
        }
        let lane = self.record(winner_token)?;
        match lane {
            LaneRecord::CancelRequested(t) if *t == winner_token => {}
            _ => return Err(GroupError::InvalidTransition),
        }
        self.prepared(GroupEventKind::Drained(winner_token), self.completed)
    }

    pub fn prepare_abort(
        &mut self,
        reason: AbortReason,
    ) -> Result<PreparedGroupEvent<'_>, GroupError> {
        match self.phase {
            GroupPhase::Running | GroupPhase::Draining(_) => {}
            GroupPhase::Terminal(_) => return Err(GroupError::InvalidTransition),
        }
        self.prepared(GroupEventKind::Aborted(reason), self.completed)
    }
}
