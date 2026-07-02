impl RunFrame {
    /// Reads an initialized slot.
    ///
    /// Returns `SlotOutOfBounds` when the index is outside the slot array,
    /// or `SlotUninitialized` when the index is valid but no value has been
    /// written to that slot yet.
    pub fn read_slot(&self, slot: SlotIdx) -> CoreResult<&SlotValue> {
        self.slots
            .get(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })?
            .as_ref()
            .ok_or(CoreError::SlotUninitialized { slot })
    }

    /// Writes a slot value without changing taint.
    pub fn write_slot(&mut self, slot: SlotIdx, value: SlotValue) -> CoreResult<()> {
        self.write_slot_with_taint(slot, value, Taint::Clean)
    }

    /// Writes a slot value and taint marker.
    pub fn write_slot_with_taint(
        &mut self,
        slot: SlotIdx,
        value: SlotValue,
        taint: Taint,
    ) -> CoreResult<()> {
        let index = slot.as_usize();
        *self
            .slots
            .get_mut(index)
            .ok_or(CoreError::SlotOutOfBounds { slot })? = Some(value);
        *self
            .taint
            .get_mut(index)
            .ok_or(CoreError::SlotOutOfBounds { slot })? = taint;
        Ok(())
    }

    /// Returns a compact copy of initialized slot values and taint markers.
    pub fn initialized_slots(&self) -> CoreResult<Vec<(SlotIdx, SlotValue, Taint)>> {
        self.slots
            .iter()
            .zip(self.taint.iter())
            .enumerate()
            .filter_map(initialized_slot_entry)
            .collect()
    }

    /// Returns a snapshot of all slot values (including uninitialized slots as None).
    #[must_use]
    pub fn slots_snapshot(&self) -> Vec<Option<SlotValue>> {
        self.slots.to_vec()
    }

    /// Returns a snapshot of all taint markers.
    #[must_use]
    pub fn taint_snapshot(&self) -> Vec<Taint> {
        self.taint.to_vec()
    }

    /// Returns a snapshot of all step states.
    #[must_use]
    pub fn states_snapshot(&self) -> Vec<StepState> {
        self.states.to_vec()
    }

    /// Reads a slot taint marker.
    ///
    /// Returns `SlotOutOfBounds` when the index is outside the slot array,
    /// or `SlotUninitialized` when the slot index is valid but has no value.
    pub fn read_taint(&self, slot: SlotIdx) -> CoreResult<Taint> {
        let index = slot.as_usize();
        let slot_value = self
            .slots
            .get(index)
            .ok_or(CoreError::SlotOutOfBounds { slot })?;
        if slot_value.is_none() {
            return Err(CoreError::SlotUninitialized { slot });
        }
        self.taint
            .get(index)
            .copied()
            .ok_or(CoreError::SlotOutOfBounds { slot })
    }

    #[allow(dead_code)]
    pub(crate) fn find_handle_taint(&self, value: &SlotValue) -> CoreResult<Taint> {
        match value {
            SlotValue::Object(id) => {
                let mut idx = 0usize;
                while idx < usize::from(self.slot_count) {
                    if let Some(Some(SlotValue::Object(vid))) = self.slots.get(idx)
                        && *vid == *id
                    {
                        return self.taint.get(idx).copied().ok_or(
                            CoreError::InternalInvariantViolation {
                                reason: "taint_slots_diverged",
                            },
                        );
                    }
                    idx = idx.saturating_add(1);
                }
                Ok(Taint::Clean)
            }
            SlotValue::List(id) => {
                let mut idx = 0usize;
                while idx < usize::from(self.slot_count) {
                    if let Some(Some(SlotValue::List(vid))) = self.slots.get(idx)
                        && *vid == *id
                    {
                        return self.taint.get(idx).copied().ok_or(
                            CoreError::InternalInvariantViolation {
                                reason: "taint_slots_diverged",
                            },
                        );
                    }
                    idx = idx.saturating_add(1);
                }
                Ok(Taint::Clean)
            }
            _ => Ok(Taint::Clean),
        }
    }

    /// Writes a slot taint marker.
    ///
    /// Rejects taint writes to uninitialized slots to prevent a taint/value
    /// desync where a slot carries a non-Clean taint but has no value.
    pub fn write_taint(&mut self, slot: SlotIdx, taint: Taint) -> CoreResult<()> {
        let index = slot.as_usize();
        let slot_value = self
            .slots
            .get(index)
            .ok_or(CoreError::SlotOutOfBounds { slot })?;
        if slot_value.is_none() {
            return Err(CoreError::SlotUninitialized { slot });
        }
        *self
            .taint
            .get_mut(index)
            .ok_or(CoreError::SlotOutOfBounds { slot })? = taint;
        Ok(())
    }
}
