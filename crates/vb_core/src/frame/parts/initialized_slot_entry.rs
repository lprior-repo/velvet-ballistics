fn initialized_slot_entry(
    (index, cell): (usize, &crate::value::SlotCell),
) -> Option<CoreResult<(SlotIdx, SlotValue, Taint)>> {
    cell.value_ref().map(|slot_value| {
        u16::try_from(index)
            .map_err(|_| CoreError::InternalInvariantViolation {
                reason: "slot index exceeds SlotIdx range".into(),
            })
            .map(|raw| {
                let taint = cell.taint().unwrap_or(Taint::Clean);
                (SlotIdx::new(raw), *slot_value, taint)
            })
    })
}
