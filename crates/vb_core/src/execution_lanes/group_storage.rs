use super::super::group::GroupControl;
use super::super::group_types::*;
use super::{GroupId, LaneError, LanePool, Slot};

#[derive(Debug, PartialEq, Eq)]
pub struct GroupStorage {
    state: State,
    max_items: u32,
    max_lanes: u16,
    last_generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Vacant(Reserved),
    Active(GroupControl),
}

#[derive(Debug, PartialEq, Eq)]
struct Reserved {
    slots: Vec<Slot>,
    results: Vec<ResultCell>,
    lanes: Vec<LaneRecord>,
}

fn reserve<T>(n: usize) -> Result<Vec<T>, GroupError> {
    let mut v: Vec<T> = Vec::new();
    v.try_reserve_exact(n).map_err(|_| GroupError::Allocate)?;
    Ok(v)
}

impl Reserved {
    fn empty() -> Self {
        Reserved {
            slots: Vec::new(),
            results: Vec::new(),
            lanes: Vec::new(),
        }
    }

    fn reserve(items: usize, lanes: usize) -> Result<Self, GroupError> {
        Ok(Reserved {
            slots: reserve(lanes)?,
            results: reserve(items)?,
            lanes: reserve(lanes)?,
        })
    }
}

impl GroupStorage {
    pub fn reserve(max_items: u32, max_lanes: u16) -> Result<Self, GroupError> {
        if max_lanes == 0 {
            return Err(GroupError::Lane(LaneError::ZeroCapacity));
        }
        let items = usize::try_from(max_items).map_err(|_| GroupError::RepresentationOverflow)?;
        let lanes = usize::from(max_lanes);
        Ok(GroupStorage {
            state: State::Vacant(Reserved::reserve(items, lanes)?),
            max_items,
            max_lanes,
            last_generation: 0,
        })
    }

    pub fn active(&self) -> Option<&GroupControl> {
        match self.state {
            State::Active(ref gc) => Some(gc),
            _ => None,
        }
    }

    pub fn active_mut(&mut self) -> Option<ActiveGroup<'_>> {
        match self.state {
            State::Active(ref mut gc) => Some(ActiveGroup::new(gc)),
            _ => None,
        }
    }
}

#[path = "group_storage/activate.rs"]
mod activation;

#[path = "group_storage/retire.rs"]
mod retirement;

#[cfg(test)]
#[path = "group_storage/tests.rs"]
mod tests;

#[path = "group_storage/control.rs"]
mod control;
pub use control::ActiveGroup;
