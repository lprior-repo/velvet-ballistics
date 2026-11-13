use super::LaneToken;
use super::group::GroupControl;
use super::group_types::*;
use crate::value::{SlotValue, Taint};

impl GroupControl {
    pub(super) fn finish_termination(&mut self, term: Termination) {
        self.lanes.iter_mut().for_each(|rec| match *rec {
            LaneRecord::Vacant => {}
            LaneRecord::Live(tok) => *rec = LaneRecord::CancelRequested(tok),
            LaneRecord::CancelRequested(_) => {}
        });
        if self.pool.live_count() == 0 {
            self.phase = GroupPhase::Terminal(term);
        } else {
            self.phase = GroupPhase::Draining(term);
        }
    }

    fn first_success(&mut self, token: LaneToken, value: SlotValue, taint: Taint) {
        if self.chosen_winner.is_some() {
            return;
        }
        let winner = Winner {
            token,
            value,
            taint,
        };
        self.chosen_winner = Some(winner);
        self.finish_termination(Termination::Winner(winner));
    }

    pub(super) fn advance_after_completion(&mut self, token: LaneToken, outcome: LaneCompletion) {
        match (self.mode, outcome) {
            (GroupMode::FirstSuccess, LaneCompletion::Succeeded { value, taint }) => {
                self.first_success(token, value, taint);
            }
            (GroupMode::FirstSuccess, LaneCompletion::Failed) => {
                if self.completed == self.total_items {
                    self.finish_termination(Termination::Failed(GroupFailure::AllBranchesFailed));
                }
            }
            (_, LaneCompletion::Failed) => {
                self.finish_termination(Termination::Failed(GroupFailure::BranchFailed {
                    ordinal: token.item_ordinal(),
                }));
            }
            (_, LaneCompletion::Succeeded { .. }) => {
                if self.completed == self.total_items {
                    self.finish_termination(Termination::Ordered);
                }
            }
        }
    }
}
