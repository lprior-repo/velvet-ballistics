fn check_time_limit(state: &CollectPaginationState) -> Result<(), EngineError> {
    if let Some(limit_ms) = state.time_limit_ms {
        let elapsed = millis_since_epoch()?.saturating_sub(state.start_millis);
        if elapsed > limit_ms {
            return Err(EngineError::CollectTimeLimitExceeded);
        }
    }
    Ok(())
}

use std::sync::atomic::{AtomicU64, Ordering};

/// Process-wide override for the time source.
/// `u64::MAX` (the sentinel) means "no override; use wall clock".
static TIME_OVERRIDE_MILLIS: AtomicU64 = AtomicU64::new(u64::MAX);

/// Returns current time in milliseconds. Defaults to wall clock; tests can
/// inject a `DeterministicTimeSource` via the process-wide override.
fn millis_since_epoch() -> Result<u64, EngineError> {
    let override_val = TIME_OVERRIDE_MILLIS.load(Ordering::Acquire);
    if override_val != u64::MAX {
        return Ok(override_val);
    }
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| EngineError::InternalInvariantViolation {
            reason: "system time is before UNIX epoch".into(),
        })?
        .as_millis()
        .try_into()
        .map_err(|_| EngineError::InternalInvariantViolation {
            reason: "millis_since_epoch overflow".into(),
        })
}

/// Install a deterministic time source for tests. Process-wide.
pub fn install_deterministic_time_source_for_test(source: DeterministicTimeSource) {
    TIME_OVERRIDE_MILLIS.store(source.now_millis().unwrap_or(0), Ordering::Release);
}

/// Restore the real wall-clock time source (used in tests).
pub fn restore_real_time_source_for_test() {
    TIME_OVERRIDE_MILLIS.store(u64::MAX, Ordering::Release);
}

fn write_empty_collector(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    taint: Taint,
) -> Result<(), EngineError> {
    let empty_id = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
    run.write_slot_with_taint(collector, SlotValue::List(empty_id), taint)?;
    Ok(())
}

fn validate_item_limit(count: usize, limit: u32) -> Result<(), EngineError> {
    let max = usize::try_from(limit).map_err(|_| EngineError::CollectItemLimitExceeded)?;
    if count > max {
        Err(EngineError::CollectItemLimitExceeded)
    } else {
        Ok(())
    }
}

fn page_size_from(raw: u32) -> Result<usize, EngineError> {
    if raw == 0 {
        return Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect page_size must be nonzero",
        });
    }
    usize::try_from(raw).map_err(|_| EngineError::CollectPageLimitExceeded)
}

fn validate_page_bound(page_size: usize, limit: u32) -> Result<(), EngineError> {
    let max = usize::try_from(limit).map_err(|_| EngineError::CollectPageLimitExceeded)?;
    if page_size > max {
        Err(EngineError::CollectPageLimitExceeded)
    } else {
        Ok(())
    }
}

fn write_collected_page_with_taint(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    items: Box<[SlotValue]>,
    taint: Taint,
    source: ListId,
    start: usize,
) -> Result<ListId, EngineError> {
    let mut taints = Vec::new();
    taints
        .try_reserve_exact(items.len())
        .map_err(|_| EngineError::AllocationFailed)?;
    taints.extend(
        store
            .list_items_with_taint(source)?
            .skip(start)
            .take(items.len())
            .map(|(_, item)| item),
    );
    let page_id = store.insert_list_with_taint(items, taints.into_boxed_slice())?;
    run.write_slot_with_taint(collector, SlotValue::List(page_id), taint)?;
    Ok(page_id)
}

fn copy_prefix(items: &[SlotValue], page_size: usize) -> Result<Box<[SlotValue]>, EngineError> {
    let count = page_size.min(items.len());
    let mut page = Vec::with_capacity(count);
    let mut index = 0usize;
    while index < count {
        let value = *items
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect prefix index checked by loop bound".into(),
            })?;
        page.push(value);
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect prefix index overflow".into(),
            })?;
    }
    Ok(page.into_boxed_slice())
}

fn copy_page_range(
    items: &[SlotValue],
    start: usize,
    page_size: usize,
) -> Result<Box<[SlotValue]>, EngineError> {
    let remaining =
        items
            .len()
            .checked_sub(start)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect cursor beyond item count".into(),
            })?;
    let count = page_size.min(remaining);
    let mut page = Vec::with_capacity(count);
    let mut offset = 0usize;
    while offset < count {
        let index = checked_add_usize(start, offset, "collect page index overflow")?;
        let value = *items
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect page index checked by loop bound".into(),
            })?;
        page.push(value);
        offset = checked_add_usize(offset, 1, "collect page offset overflow")?;
    }
    Ok(page.into_boxed_slice())
}

fn validate_collect_state(
    state: &CollectPaginationState,
    source_len: usize,
) -> Result<(), EngineError> {
    if state.page_size > state.limit {
        return Err(EngineError::CollectPageLimitExceeded);
    }
    if state.item_count > state.limit {
        return Err(EngineError::CollectItemLimitExceeded);
    }
    if source_len != state.item_count {
        return Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect source length changed",
        });
    }
    Ok(())
}

fn checked_add_usize(
    left: usize,
    right: usize,
    reason: &'static str,
) -> Result<usize, EngineError> {
    left.checked_add(right).ok_or(EngineError::InternalInvariantViolation {
        reason: reason.into(),
    })
}
