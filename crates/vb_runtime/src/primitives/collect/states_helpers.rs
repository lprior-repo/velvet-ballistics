use super::*;

pub(super) fn updated_pages_total(
    states: &CollectStates,
    key: (RunId, SlotIdx),
    state: &CollectPaginationState,
) -> Result<u64, EngineError> {
    let pages = match states.entries.get(&key) {
        None => 1,
        Some(prior) => u64::from(prior.current_page != state.current_page),
    };
    states
        .total_pages_delivered
        .checked_add(pages)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "collect page counter overflow".into(),
        })
}
