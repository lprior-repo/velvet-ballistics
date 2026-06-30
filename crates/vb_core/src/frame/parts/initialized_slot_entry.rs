fn initialized_slot_entry(
    (index, (value, taint)): (usize, (&Option<SlotValue>, &Taint)),
) -> Option<CoreResult<(SlotIdx, SlotValue, Taint)>> {
    value.as_ref().map(|slot_value| {
        u16::try_from(index)
            .map_err(|_| CoreError::InternalInvariantViolation {
                reason: "slot index exceeds SlotIdx range",
            })
            .map(|raw| (SlotIdx::new(raw), *slot_value, *taint))
    })
}
