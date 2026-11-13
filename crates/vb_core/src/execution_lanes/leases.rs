use crate::ids::StepIdx;

#[path = "lease_prepare.rs"]
mod preparation;
#[path = "group_storage.rs"]
pub(super) mod storage;

/// Errors from lane pool operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneError {
    /// Requested declared capacity is zero.
    ZeroCapacity,
    /// Declared capacity exceeds the policy maximum.
    CapacityOverPolicy,
    /// Declared total items exceeds the policy maximum.
    TotalOverPolicy,
    /// Physical slot allocation failed (out of memory).
    Allocate,
    /// No lanes available (pool full).
    Full,
    /// No more items to admit (exhausted).
    Exhausted,
    /// Token is stale (lease generation mismatch) or slot no longer leased.
    Stale,
    /// Token belongs to a different group or slot.
    Foreign,
    /// Token released twice.
    DuplicateRelease,
    /// Lease generation overflow (wraparound).
    GenerationOverflow,
    /// Invalid generation value for a group ID.
    InvalidGeneration,
    /// Index or count overflow during operation.
    RepresentationOverflow,
}

/// Identifies a group of items that share a lane pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupId {
    owner: StepIdx,
    generation: u64,
}

impl GroupId {
    /// Create a new group ID with a generation > 0.
    ///
    /// Returns an error if generation is zero.
    pub fn new(owner: StepIdx, generation: u64) -> Result<Self, LaneError> {
        if generation == 0 {
            return Err(LaneError::InvalidGeneration);
        }
        Ok(Self { owner, generation })
    }

    /// The owner step index within the run.
    #[inline]
    pub const fn owner(&self) -> StepIdx {
        self.owner
    }

    /// The generation for this group.
    #[inline]
    pub const fn generation(&self) -> u64 {
        self.generation
    }
}

/// A lease token identifying a single lane allocation.
///
/// Tokens are returned by [`LanePool::admit`] and must be passed to
/// [`LanePool::release`] when the item is done. Callers cannot construct
/// tokens directly; the identity data (group, physical slot, lease
/// generation, item ordinal) is private.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaneToken {
    group: GroupId,
    physical_slot: u16,
    lease_generation: u64,
    item_ordinal: u32,
}

impl LaneToken {
    /// The group this token belongs to.
    #[inline]
    pub const fn group(&self) -> GroupId {
        self.group
    }

    /// The physical slot index.
    #[inline]
    pub const fn physical_slot(&self) -> u16 {
        self.physical_slot
    }

    /// The lease generation.
    #[inline]
    pub const fn lease_generation(&self) -> u64 {
        self.lease_generation
    }

    /// The item ordinal this token was admitted under.
    #[inline]
    pub const fn item_ordinal(&self) -> u32 {
        self.item_ordinal
    }
}

/// A single physical lane slot.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Slot {
    generation: u64,
    active_ordinal: Option<u32>,
}

/// A bounded pool of execution lanes.
///
/// Each lane is leased to a single item at a time. At most `capacity` lanes
/// are leased simultaneously. Leases are identified by opaque tokens that
/// embed the physical slot, lease generation, and item ordinal for validation.
/// Uses deterministic round-robin slot allocation via a cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanePool {
    group: GroupId,
    slots: Vec<Slot>,
    cursor: usize,
    next_ordinal: u32,
    total_items: u32,
}

struct PreparedLease {
    index: usize,
    token: LaneToken,
    next_ordinal: u32,
    next_cursor: usize,
}

impl LanePool {
    /// `declared_cap` is the maximum number of concurrent leases.
    /// `total_items` is the total number of items this pool will admit.
    /// `max_lanes` and `max_total_items` are policy upper bounds.
    ///
    /// The actual physical capacity is `min(declared_cap, total_items)`.
    pub fn new(
        group: &GroupId,
        total_items: u32,
        declared_cap: u16,
        max_total_items: u32,
        max_lanes: u16,
    ) -> Result<Self, LaneError> {
        if declared_cap == 0 {
            return Err(LaneError::ZeroCapacity);
        }
        if declared_cap > max_lanes {
            return Err(LaneError::CapacityOverPolicy);
        }
        if total_items > max_total_items {
            return Err(LaneError::TotalOverPolicy);
        }
        let cap_u32 = u32::from(declared_cap).min(total_items);
        let cap = usize::try_from(cap_u32).map_err(|_| LaneError::RepresentationOverflow)?;
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(cap)
            .map_err(|_| LaneError::Allocate)?;
        slots.resize_with(cap, || Slot {
            generation: 0u64,
            active_ordinal: None,
        });

        Ok(LanePool {
            group: *group,
            slots,
            cursor: 0usize,
            next_ordinal: 0u32,
            total_items,
        })
    }

    /// Find the next free slot using round-robin from cursor.
    fn find_free_slot(&self, start: usize) -> Option<usize> {
        (start..self.slots.len()).chain(0..start).find(|&index| {
            self.slots
                .get(index)
                .is_some_and(|slot| slot.active_ordinal.is_none())
        })
    }

    /// Admit the next item in the group, returning a lease token.
    ///
    /// Uses deterministic round-robin slot allocation. If the pool is full
    /// (all slots leased) or exhausted (all items admitted), returns an
    /// error.
    pub fn admit(&mut self) -> Result<LaneToken, LaneError> {
        let prepared = self.prepare_admission()?;
        let slot = self
            .slots
            .get_mut(prepared.index)
            .ok_or(LaneError::RepresentationOverflow)?;
        slot.generation = prepared.token.lease_generation;
        slot.active_ordinal = Some(prepared.token.item_ordinal);
        self.next_ordinal = prepared.next_ordinal;
        self.cursor = prepared.next_cursor;
        Ok(prepared.token)
    }

    /// Preview whether the next item could be admitted without mutating.
    pub(super) fn preview_admission(&self) -> Result<LaneToken, LaneError> {
        Ok(self.prepare_admission()?.token)
    }

    fn next_token(&self) -> Result<(usize, LaneToken), LaneError> {
        if self.next_ordinal >= self.total_items {
            return Err(LaneError::Exhausted);
        }
        let index = self.find_free_slot(self.cursor).ok_or(LaneError::Full)?;
        let slot = self
            .slots
            .get(index)
            .ok_or(LaneError::RepresentationOverflow)?;
        let lease_generation = slot
            .generation
            .checked_add(1)
            .ok_or(LaneError::GenerationOverflow)?;
        let physical_slot = u16::try_from(index).map_err(|_| LaneError::RepresentationOverflow)?;
        Ok((
            index,
            LaneToken {
                group: self.group,
                physical_slot,
                lease_generation,
                item_ordinal: self.next_ordinal,
            },
        ))
    }

    /// Validate that the token is live on this pool; returns the slot index.
    fn validated_slot(&self, token: LaneToken) -> Result<usize, LaneError> {
        if token.group != self.group {
            return Err(LaneError::Foreign);
        }
        let slot_idx = usize::from(token.physical_slot);
        let slot = self
            .slots
            .get(slot_idx)
            .ok_or(LaneError::RepresentationOverflow)?;
        if slot.generation != token.lease_generation {
            return Err(LaneError::Stale);
        }
        match slot.active_ordinal {
            None => return Err(LaneError::DuplicateRelease),
            Some(ord) if ord != token.item_ordinal => return Err(LaneError::Stale),
            Some(_) => {}
        }
        Ok(slot_idx)
    }

    /// Release a lane lease.
    ///
    /// Validates that the token is current, belongs to this pool, and has
    /// not already been released. If valid, the slot becomes available for
    /// reuse.
    pub fn release(&mut self, token: LaneToken) -> Result<(), LaneError> {
        let slot_idx = self.validated_slot(token)?;
        let slot_mut = self
            .slots
            .get_mut(slot_idx)
            .ok_or(LaneError::RepresentationOverflow)?;
        slot_mut.active_ordinal = None;
        Ok(())
    }

    /// Validate that the given token is currently live in the pool.
    pub fn validate_live(&self, token: LaneToken) -> Result<(), LaneError> {
        self.validated_slot(token).map(|_| ())
    }

    /// Number of currently leased lanes.
    pub fn live_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.active_ordinal.is_some())
            .count()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
