use super::LaneToken;
use super::group::GroupControl;
use super::group_prepare::PreparedGroupEvent;
use super::group_types::*;

impl PreparedGroupEvent<'_> {
    pub fn commit(self, receipt: GroupCommitReceipt) -> Result<GroupTransition, GroupError> {
        if receipt.event != self.event {
            return Err(GroupError::InvalidReceipt);
        }
        match self.event.kind {
            GroupEventKind::Admission(t) => {
                let t = self.group.commit_admission(t)?;
                self.group.revision = self.event.revision;
                Ok(t)
            }
            GroupEventKind::Completion { token, outcome } => {
                let t = self
                    .group
                    .commit_completion(token, outcome, self.next_completed)?;
                self.group.revision = self.event.revision;
                Ok(t)
            }
            GroupEventKind::Drained(t) => {
                let t = self.group.commit_drain(t)?;
                self.group.revision = self.event.revision;
                Ok(t)
            }
            GroupEventKind::Aborted(reason) => {
                let t = self.group.commit_abort(reason);
                self.group.revision = self.event.revision;
                Ok(t)
            }
        }
    }
}

impl GroupControl {
    fn commit_admission(&mut self, token: LaneToken) -> Result<GroupTransition, GroupError> {
        let lane = self
            .lanes
            .get_mut(usize::from(token.physical_slot()))
            .ok_or(GroupError::RepresentationOverflow)?;
        let actual = self.pool.admit()?;
        *lane = LaneRecord::Live(actual);
        Ok(GroupTransition::Admitted(actual))
    }

    fn commit_completion(
        &mut self,
        token: LaneToken,
        outcome: LaneCompletion,
        next_completed: u32,
    ) -> Result<GroupTransition, GroupError> {
        let result_idx = usize::try_from(token.item_ordinal())
            .map_err(|_| GroupError::RepresentationOverflow)?;
        let result = self
            .results
            .get_mut(result_idx)
            .ok_or(GroupError::RepresentationOverflow)?;
        let lane = self
            .lanes
            .get_mut(usize::from(token.physical_slot()))
            .ok_or(GroupError::RepresentationOverflow)?;
        self.pool.release(token)?;
        *result = match outcome {
            LaneCompletion::Succeeded { value, taint } => ResultCell::Value { value, taint },
            LaneCompletion::Failed => ResultCell::Failed,
        };
        *lane = LaneRecord::Vacant;
        self.completed = next_completed;
        self.advance_after_completion(token, outcome);
        Ok(GroupTransition::Changed(self.phase))
    }

    fn commit_drain(&mut self, token: LaneToken) -> Result<GroupTransition, GroupError> {
        let term = match self.phase {
            GroupPhase::Draining(term) => term,
            _ => return Err(GroupError::InvalidTransition),
        };
        let lane = self
            .lanes
            .get_mut(usize::from(token.physical_slot()))
            .ok_or(GroupError::RepresentationOverflow)?;
        self.pool.release(token)?;
        *lane = LaneRecord::Vacant;
        if self.pool.live_count() == 0 {
            self.phase = GroupPhase::Terminal(term);
        }
        Ok(GroupTransition::Changed(self.phase))
    }

    fn commit_abort(&mut self, reason: AbortReason) -> GroupTransition {
        let failure = match reason {
            AbortReason::Canceled => GroupFailure::Canceled,
            AbortReason::ParentFailed => GroupFailure::ParentFailed,
        };
        self.finish_termination(Termination::Failed(failure));
        GroupTransition::Changed(self.phase)
    }
}
