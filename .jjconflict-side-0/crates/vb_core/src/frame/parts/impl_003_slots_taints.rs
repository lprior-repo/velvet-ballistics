impl RunFrame {
    /// Reads an initialized slot.
    ///
    /// Returns `SlotOutOfBounds` when the index is outside the slot array,
    /// or `SlotUninitialized` when the index is valid but no value has been
    /// written to that slot yet.
    pub fn read_slot(&self, slot: SlotIdx) -> CoreResult<&SlotValue> {
        self.cells
            .get(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })?
            .value_ref()
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
        *self
            .cells
            .get_mut(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })? =
            crate::value::SlotCell::value(value, taint);
        Ok(())
    }

    /// Returns a compact copy of initialized slot values and taint markers.
    pub fn initialized_slots(&self) -> CoreResult<Vec<(SlotIdx, SlotValue, Taint)>> {
        self.cells
            .iter()
            .enumerate()
            .filter_map(initialized_slot_entry)
            .collect()
    }

    /// Returns a snapshot of all slot values (including uninitialized slots as None).
    #[must_use]
    pub fn slots_snapshot(&self) -> Vec<Option<SlotValue>> {
        self.cells
            .iter()
            .map(|cell| cell.value_ref().copied())
            .collect()
    }

    #[must_use]
    pub fn written_slot_count(&self) -> u64 {
        self.cells.iter().fold(0, |count, cell| {
            count.saturating_add(u64::from(cell.is_initialized()))
        })
    }

    /// Returns a snapshot of all taint markers.
    #[must_use]
    pub fn taint_snapshot(&self) -> Vec<Taint> {
        self.cells
            .iter()
            .map(|cell| cell.taint().unwrap_or(Taint::Clean))
            .collect()
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
        self.cells
            .get(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })?
            .taint()
            .ok_or(CoreError::SlotUninitialized { slot })
    }

    #[allow(dead_code)]
    pub(crate) fn find_handle_taint(&self, value: &SlotValue) -> CoreResult<Taint> {
        match value {
            SlotValue::Object(id) => {
                let mut idx = 0usize;
                while idx < usize::from(self.slot_count) {
                    if let Some(SlotValue::Object(vid)) =
                        self.cells.get(idx).and_then(|cell| cell.value_ref())
                        && *vid == *id
                    {
                        return self.cells.get(idx).and_then(|cell| cell.taint()).ok_or(
                            CoreError::InternalInvariantViolation {
                                reason: "taint_slots_diverged".into(),
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
                    if let Some(SlotValue::List(vid)) =
                        self.cells.get(idx).and_then(|cell| cell.value_ref())
                        && *vid == *id
                    {
                        return self.cells.get(idx).and_then(|cell| cell.taint()).ok_or(
                            CoreError::InternalInvariantViolation {
                                reason: "taint_slots_diverged".into(),
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
        let cell = self
            .cells
            .get_mut(index)
            .ok_or(CoreError::SlotOutOfBounds { slot })?;
        match cell {
            crate::value::SlotCell::Empty => {
                return Err(CoreError::SlotUninitialized { slot });
            }
            crate::value::SlotCell::Value { value, taint: slot_taint } => {
                *slot_taint = taint;
                let _ = value;
            }
        }
        Ok(())
    }
}
