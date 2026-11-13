use super::{LaneError, LanePool, PreparedLease};

impl LanePool {
    pub(super) fn prepare_admission(&self) -> Result<PreparedLease, LaneError> {
        let (index, token) = self.next_token()?;
        let next_ordinal = self
            .next_ordinal
            .checked_add(1)
            .ok_or(LaneError::RepresentationOverflow)?;
        let after = index
            .checked_add(1)
            .ok_or(LaneError::RepresentationOverflow)?;
        Ok(PreparedLease {
            index,
            token,
            next_ordinal,
            next_cursor: if after >= self.slots.len() { 0 } else { after },
        })
    }
}
