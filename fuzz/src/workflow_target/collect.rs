//! Collect pagination fuzz target body.

pub fn fuzz_collect_page_pagination(data: &[u8]) {
    if data.is_empty() {
        return;
    }
    let Some(&byte0) = data.first() else {
        return;
    };
    let slot_count = u16::from(byte0.wrapping_rem(16)).saturating_add(1);
    let list_len = usize::from(byte0.wrapping_rem(8));
    let page_size =
        usize::from(data.get(1).copied().unwrap_or(1).wrapping_rem(8)).saturating_add(1);
    let Ok(mut run) = vb_core::RunFrame::new(
        vb_core::RunId::new(1),
        vb_core::StepIdx::ZERO,
        2,
        slot_count,
    ) else {
        return;
    };
    let mut store = vb_core::ValueStore::new();
    let items: Vec<vb_core::SlotValue> = (0..list_len)
        .map(|i| vb_core::SlotValue::I64(i64::try_from(i).unwrap_or(0)))
        .collect();
    let list_id = match store.insert_list(items.into_boxed_slice()) {
        Ok(id) => id,
        Err(_) => return,
    };
    let _slot_list_seed = run.write_slot_with_taint(
        vb_core::SlotIdx::new(0),
        vb_core::SlotValue::List(list_id),
        vb_core::Taint::Clean,
    );
    use vb_runtime::primitives::collect::{CollectStates, collect_page};
    let mut states = CollectStates::new();
    let result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        vb_core::SlotIdx::new(0),
        vb_core::StepIdx::new(1),
        vb_core::StepIdx::new(1),
    );
    match result {
        Ok(status) => assert!(matches!(status, vb_core::EngineSignal::Continue)),
        Err(_error) => {}
    }
    exercise_collect_start(page_size, list_len, list_id, slot_count, &mut store);
    exercise_collect_non_list(slot_count, &mut store);
}

fn exercise_collect_start(
    page_size: usize,
    list_len: usize,
    list_id: vb_core::ListId,
    slot_count: u16,
    store: &mut vb_core::ValueStore,
) {
    use vb_runtime::primitives::collect::{CollectStates, collect_start};
    let Ok(mut run_zero) = vb_core::RunFrame::new(
        vb_core::RunId::new(3),
        vb_core::StepIdx::ZERO,
        2,
        slot_count,
    ) else {
        return;
    };
    let _slot_list_seed_zero = run_zero.write_slot_with_taint(
        vb_core::SlotIdx::new(0),
        vb_core::SlotValue::List(list_id),
        vb_core::Taint::Clean,
    );
    let mut states_zero = CollectStates::new();
    // `page_size` is `usize`-bounded by the fuzz input; the contract of
    // `collect_start` requires a `u32` payload bound. The `try_from`
    // translation is the safe replacement for the historical `as u32`
    // silent truncation.
    let Ok(page_size_u32) = u32::try_from(page_size) else {
        return;
    };
    let zero_page_result = collect_start(
        &mut run_zero,
        store,
        &mut states_zero,
        vb_core::SlotIdx::new(0),
        page_size_u32,
        page_size_u32,
        vb_core::StepIdx::new(1),
        vb_core::StepIdx::new(1),
        None,
        None,
    );
    if page_size == 0 {
        assert!(zero_page_result.is_err());
    }
    if page_size > 0
        && list_len > 0
        && list_len < page_size
        && let Ok(signal) = zero_page_result
    {
        assert!(matches!(
            signal,
            vb_core::EngineSignal::Continue | vb_core::EngineSignal::Finished(..)
        ));
    }
}

fn exercise_collect_non_list(slot_count: u16, store: &mut vb_core::ValueStore) {
    use vb_runtime::primitives::collect::{CollectStates, collect_page};
    let Ok(mut run_non_list) = vb_core::RunFrame::new(
        vb_core::RunId::new(2),
        vb_core::StepIdx::ZERO,
        2,
        slot_count,
    ) else {
        return;
    };
    let _slot_int_seed = run_non_list.write_slot_with_taint(
        vb_core::SlotIdx::new(0),
        vb_core::SlotValue::I64(42),
        vb_core::Taint::Clean,
    );
    let mut states2 = CollectStates::new();
    let non_list_result = collect_page(
        &mut run_non_list,
        store,
        &mut states2,
        vb_core::SlotIdx::new(0),
        vb_core::StepIdx::new(1),
        vb_core::StepIdx::new(1),
    );
    assert!(non_list_result.is_err());
}
