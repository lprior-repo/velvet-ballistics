use super::group_types::*;
use super::{GroupId, LanePool, LaneToken};

/// Bounded pure group control; values remain handles into the caller-owned run store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupControl {
    pub(super) mode: GroupMode,
    pub(super) group: GroupId,
    pub(super) pool: LanePool,
    pub(super) total_items: u32,
    pub(super) results: Vec<ResultCell>,
    pub(super) lanes: Vec<LaneRecord>,
    pub(super) completed: u32,
    pub(super) revision: u64,
    pub(super) phase: GroupPhase,
    pub(super) chosen_winner: Option<Winner>,
}

impl GroupControl {
    pub fn new(
        mode: GroupMode,
        group: GroupId,
        total_items: u32,
        declared_cap: u16,
        max_total_items: u32,
        max_lanes: u16,
    ) -> Result<Self, GroupError> {
        let pool = LanePool::new(
            &group,
            total_items,
            declared_cap,
            max_total_items,
            max_lanes,
        )?;
        if total_items == 0 && mode != GroupMode::ForEachOrdered {
            return Err(GroupError::EmptyGroup);
        }
        if mode != GroupMode::ForEachOrdered && u32::from(declared_cap) < total_items {
            return Err(GroupError::InsufficientCapacity);
        }
        let lane_cap = u32::from(declared_cap).min(total_items);
        let phase = if total_items == 0 {
            GroupPhase::Terminal(Termination::Ordered)
        } else {
            GroupPhase::Running
        };
        Ok(GroupControl {
            mode,
            group,
            pool,
            total_items,
            results: Self::reserved(
                usize::try_from(total_items).map_err(|_| GroupError::RepresentationOverflow)?,
                ResultCell::Pending,
            )?,
            lanes: Self::reserved(
                usize::try_from(lane_cap).map_err(|_| GroupError::RepresentationOverflow)?,
                LaneRecord::Vacant,
            )?,
            completed: 0,
            revision: 0,
            phase,
            chosen_winner: None,
        })
    }

    fn reserved<T: Clone>(len: usize, value: T) -> Result<Vec<T>, GroupError> {
        let mut v = Vec::new();
        v.try_reserve_exact(len).map_err(|_| GroupError::Allocate)?;
        v.resize(len, value);
        Ok(v)
    }

    pub fn phase(&self) -> GroupPhase {
        self.phase
    }
    pub fn chosen_winner(&self) -> Option<Winner> {
        self.chosen_winner
    }
    pub fn live_count(&self) -> usize {
        self.pool.live_count()
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn outcome(&self) -> GroupOutcome<'_> {
        match &self.phase {
            GroupPhase::Terminal(Termination::Ordered) => GroupOutcome::Ordered(&self.results),
            GroupPhase::Terminal(Termination::Winner(w)) => GroupOutcome::Winner(*w),
            GroupPhase::Terminal(Termination::Failed(f)) => GroupOutcome::Failed(*f),
            _ => GroupOutcome::Pending,
        }
    }

    pub(super) fn require_running(&self) -> Result<(), GroupError> {
        match &self.phase {
            GroupPhase::Running => Ok(()),
            _ => Err(GroupError::InvalidTransition),
        }
    }

    pub(super) fn record(&self, token: LaneToken) -> Result<&LaneRecord, GroupError> {
        self.pool.validate_live(token)?;
        self.lanes
            .get(usize::from(token.physical_slot()))
            .ok_or(GroupError::RepresentationOverflow)
    }

    pub(super) fn pending(&self, token: LaneToken) -> Result<(), GroupError> {
        let i = usize::try_from(token.item_ordinal())
            .map_err(|_| GroupError::RepresentationOverflow)?;
        match self.results.get(i) {
            Some(ResultCell::Pending) => Ok(()),
            Some(_) => Err(GroupError::InvalidTransition),
            None => Err(GroupError::RepresentationOverflow),
        }
    }
}

#[cfg(test)]
#[path = "group_tests.rs"]
mod tests;
