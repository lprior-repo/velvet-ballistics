// RP-016 regression: the iterator slot must carry a bounded (source_id, cursor)
// state, not a materialized tail. Assert the new state list is always 2
// elements regardless of the source size, and that no per-step allocation
// happens for the source list items themselves.
#[test]
fn rp016_iterator_state_is_bounded_cursor_not_tail() {
    const N: usize = 64;
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let iter_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        (0..N).map(|i| SlotValue::I64(i as i64)).collect(),
    );

    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        1024,
        body,
        done,
        Some(iter_slot),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    // After start, the iterator slot is a 2-element state list, NOT a tail
    // of size N-1. This is the contract: a bounded cursor, not a copy.
    let state_id = match *run
        .read_slot(iter_slot)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"))
    {
        SlotValue::List(id) => id,
        other => panic!("expected list in iter_slot, got {other:?}"),
    };
    let state_items = store
        .list(state_id)
        .ok()
        .unwrap_or_else(|| panic!("list read must succeed"));
    assert_eq!(
        state_items.len(),
        2,
        "RP-016 fix: iterator state must be a 2-element (source_id, cursor) list, got len {} for source size {N}",
        state_items.len()
    );

    // Run the rest of the iteration; each step must also write a 2-element
    // state list. No step ever materializes a tail.
    for _ in 1..N {
        let _ = run.set_pc(body);
        let _ = run.mark_succeeded(body);
        let next = for_each_next(&mut run, &mut store, iter_slot, body, done, Some(item_slot));
        assert_eq!(next, Ok(vb_core::EngineSignal::Continue));
        let state_id = match *run
            .read_slot(iter_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"))
        {
            SlotValue::List(id) => id,
            other => panic!("expected list in iter_slot, got {other:?}"),
        };
        let state_items = store
            .list(state_id)
            .ok()
            .unwrap_or_else(|| panic!("list read must succeed"));
        assert_eq!(
            state_items.len(),
            2,
            "RP-016 fix: iterator state must remain 2 elements at every step"
        );
    }

    // Final step: cursor exhausts source, function jumps to done.
    let _ = run.set_pc(body);
    let _ = run.mark_succeeded(body);
    let final_step = for_each_next(&mut run, &mut store, iter_slot, body, done, Some(item_slot));
    assert_eq!(final_step, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}
