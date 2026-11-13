use super::*;

struct PreparedSpec {
    mode: GroupMode,
    group: GroupId,
    total_items: u32,
    declared_cap: u16,
}

fn validate_generation(storage: &GroupStorage, generation: u64) -> Result<(), GroupError> {
    if storage.last_generation == u64::MAX {
        return Err(GroupError::Lane(LaneError::GenerationOverflow));
    }
    if generation <= storage.last_generation {
        return Err(GroupError::Lane(LaneError::InvalidGeneration));
    }
    Ok(())
}

fn validate_state(storage: &GroupStorage) -> Result<(), GroupError> {
    if !matches!(storage.state, State::Vacant(_)) {
        return Err(GroupError::InvalidTransition);
    }
    Ok(())
}

fn validate_policy(storage: &GroupStorage, spec: &PreparedSpec) -> Result<(), GroupError> {
    if spec.declared_cap == 0 {
        return Err(GroupError::Lane(LaneError::ZeroCapacity));
    }
    if spec.declared_cap > storage.max_lanes {
        return Err(GroupError::Lane(LaneError::CapacityOverPolicy));
    }
    if spec.total_items > storage.max_items {
        return Err(GroupError::Lane(LaneError::TotalOverPolicy));
    }
    if spec.total_items == 0 && spec.mode != GroupMode::ForEachOrdered {
        return Err(GroupError::EmptyGroup);
    }
    let cap = u32::from(spec.declared_cap);
    if spec.mode != GroupMode::ForEachOrdered && cap < spec.total_items {
        return Err(GroupError::InsufficientCapacity);
    }
    Ok(())
}

fn check_reserved(reserved: &Reserved, items: usize, lanes: usize) -> bool {
    reserved.results.capacity() >= items
        && reserved.slots.capacity() >= lanes
        && reserved.lanes.capacity() >= lanes
}

fn build_pool(spec: &PreparedSpec, slots: Vec<Slot>) -> LanePool {
    LanePool {
        group: spec.group,
        slots,
        cursor: 0,
        next_ordinal: 0,
        total_items: spec.total_items,
    }
}

fn initial_phase(spec: &PreparedSpec) -> GroupPhase {
    if spec.total_items == 0 {
        GroupPhase::Terminal(Termination::Ordered)
    } else {
        GroupPhase::Running
    }
}

fn build_control(
    spec: &PreparedSpec,
    pool: LanePool,
    results: Vec<ResultCell>,
    lanes: Vec<LaneRecord>,
) -> GroupControl {
    GroupControl {
        mode: spec.mode,
        group: spec.group,
        pool,
        total_items: spec.total_items,
        results,
        lanes,
        completed: 0,
        revision: 0,
        phase: initial_phase(spec),
        chosen_winner: None,
    }
}

fn initialized_control(
    spec: &PreparedSpec,
    mut reserved: Reserved,
    items: usize,
    lane_count: usize,
) -> GroupControl {
    reserved.slots.clear();
    reserved.slots.resize(
        lane_count,
        Slot {
            generation: 0,
            active_ordinal: None,
        },
    );
    reserved.results.clear();
    reserved.results.resize(items, ResultCell::Pending);
    reserved.lanes.clear();
    reserved.lanes.resize(lane_count, LaneRecord::Vacant);
    let pool = build_pool(spec, reserved.slots);
    build_control(spec, pool, reserved.results, reserved.lanes)
}

impl GroupStorage {
    pub fn activate(
        &mut self,
        mode: GroupMode,
        group: GroupId,
        total_items: u32,
        declared_cap: u16,
    ) -> Result<(), GroupError> {
        let spec = PreparedSpec {
            mode,
            group,
            total_items,
            declared_cap,
        };
        validate_state(self)?;
        validate_generation(self, group.generation())?;
        validate_policy(self, &spec)?;
        let (items, lane_count) = self.checked_lengths(&spec)?;
        self.install(&spec, items, lane_count)
    }

    fn checked_lengths(&self, spec: &PreparedSpec) -> Result<(usize, usize), GroupError> {
        let total_items = spec.total_items;
        let declared_cap = spec.declared_cap;
        let items = usize::try_from(total_items).map_err(|_| GroupError::RepresentationOverflow)?;
        let lane_count = usize::try_from(u32::from(declared_cap).min(total_items))
            .map_err(|_| GroupError::RepresentationOverflow)?;
        match self.state {
            State::Vacant(ref reserved) => {
                if !check_reserved(reserved, items, lane_count) {
                    return Err(GroupError::InsufficientCapacity);
                }
            }
            _ => return Err(GroupError::InvalidTransition),
        }
        Ok((items, lane_count))
    }

    fn install(
        &mut self,
        spec: &PreparedSpec,
        items: usize,
        lane_count: usize,
    ) -> Result<(), GroupError> {
        let old_state = core::mem::replace(&mut self.state, State::Vacant(Reserved::empty()));
        match old_state {
            State::Vacant(reserved) => {
                let control = initialized_control(spec, reserved, items, lane_count);
                self.state = State::Active(control);
                self.last_generation = spec.group.generation();
                Ok(())
            }
            State::Active(gc) => {
                self.state = State::Active(gc);
                Err(GroupError::InvalidTransition)
            }
        }
    }
}
