use super::{
    ValueStore,
    lists::{list_index, validate_list_len},
};
use crate::value::join_taint;
use crate::{CoreError, CoreResult, ListId, SlotValue, Taint};

type ListPayload = (Box<[SlotValue]>, Box<[Taint]>);

impl ValueStore {
    /// Iterates corresponding stored values and their exact item taints.
    pub fn list_items_with_taint(
        &self,
        id: ListId,
    ) -> CoreResult<impl ExactSizeIterator<Item = (SlotValue, Taint)> + '_> {
        let values = self.list(id)?;
        let taints =
            self.list_taints
                .get(list_index(id)?)
                .ok_or(CoreError::InternalInvariantViolation {
                    reason: "list taints missing",
                })?;
        if values.len() != taints.len() {
            return Err(CoreError::InternalInvariantViolation {
                reason: "list values and taints length mismatch",
            });
        }
        Ok(values.iter().copied().zip(taints.iter().copied()))
    }
    /// Prepares an immutable list copy with an optional appended tainted item.
    pub fn prepare_list_append(
        &self,
        id: ListId,
        item: Option<(SlotValue, Taint)>,
    ) -> CoreResult<ListPayload> {
        let entries = self.list_items_with_taint(id)?;
        let len = entries
            .len()
            .checked_add(usize::from(item.is_some()))
            .ok_or(CoreError::AllocationFailed)?;
        validate_list_len(len)?;
        let mut values = Vec::new();
        let mut taints = Vec::new();
        values
            .try_reserve_exact(len)
            .map_err(|_| CoreError::AllocationFailed)?;
        taints
            .try_reserve_exact(len)
            .map_err(|_| CoreError::AllocationFailed)?;
        for (value, taint) in entries.chain(item) {
            values.push(value);
            taints.push(taint);
        }
        Ok((values.into_boxed_slice(), taints.into_boxed_slice()))
    }
    /// Preserves the first occurrence order and joins duplicate item provenance.
    pub fn prepare_list_unique(&self, id: ListId) -> CoreResult<ListPayload> {
        let entries = self.list_items_with_taint(id)?;
        let mut values = Vec::new();
        let mut taints = Vec::new();
        values
            .try_reserve_exact(entries.len())
            .map_err(|_| CoreError::AllocationFailed)?;
        taints
            .try_reserve_exact(entries.len())
            .map_err(|_| CoreError::AllocationFailed)?;
        for (value, taint) in entries {
            if let Some(index) = values.iter().position(|existing| *existing == value) {
                let target =
                    taints
                        .get_mut(index)
                        .ok_or(CoreError::InternalInvariantViolation {
                            reason: "unique item taint missing",
                        })?;
                *target = join_taint(*target, taint);
            } else {
                values.push(value);
                taints.push(taint);
            }
        }
        Ok((values.into_boxed_slice(), taints.into_boxed_slice()))
    }
    /// Joins provenance only for the values inspected by ordered membership.
    pub fn list_contains_with_taint(
        &self,
        id: ListId,
        needle: SlotValue,
    ) -> CoreResult<(bool, Taint)> {
        let mut taint = Taint::Clean;
        for (value, item_taint) in self.list_items_with_taint(id)? {
            taint = join_taint(taint, item_taint);
            if value == needle {
                return Ok((true, taint));
            }
        }
        Ok((false, taint))
    }
}

#[cfg(test)]
mod tests;
