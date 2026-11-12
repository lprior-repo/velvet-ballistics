#[cfg(test)]
mod proptest_reentry {
    use proptest::prelude::*;

    use vb_core::frame::{RunFrame, StepState};
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::value::SlotValue;
    use vb_core::value_store::ValueStore;

    use crate::primitives::collect::{CollectStates, collect_next, collect_start};
    use crate::primitives::for_each::{for_each_next, for_each_start};
    use crate::primitives::helpers::jump_to_body;
    use crate::primitives::reduce::{reduce_next, reduce_start};
    use crate::primitives::repeat::{repeat_check, repeat_start};
    use crate::test_harness::list_in_slot;

    /// Helper: creates a fresh RunFrame with generous step and slot counts.
    fn fresh_frame() -> RunFrame {
        crate::test_harness::fresh_frame(8, 12)
    }

    /// Minimal workflow with a single I64 constant.
    fn minimal_workflow_with_const(cv: i64) -> vb_core::workflow::CompiledWorkflow {
        use vb_core::value::ConstValue;
        use vb_core::workflow::{
            CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts, node,
        };

        let parts = WorkflowParts {
            semantic_source_identity: None,
            name: Box::from("proptest_reduce"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([9; 32]),
            nodes: vec![node(StepIdx::ZERO, CompiledNodeKind::Nop, None, None)].into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(cv)].into_boxed_slice(),
            slot_count: 12,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };
        CompiledWorkflow::try_from_parts(parts).unwrap()
    }

    /// Arbitrary StepState strategy — all 7 states are valid.
    fn arb_step_state() -> impl Strategy<Value = StepState> {
        prop_oneof![
            Just(StepState::Pending),
            Just(StepState::Running),
            Just(StepState::Succeeded),
            Just(StepState::Failed),
            Just(StepState::Waiting),
            Just(StepState::Asking),
            Just(StepState::Cancelled),
        ]
    }

    /// Arbitrary small list of I64 items (length 0..=20).
    fn arb_i64_list() -> impl Strategy<Value = Vec<SlotValue>> {
        prop::collection::vec(any::<i64>().prop_map(SlotValue::I64), 0..=20usize)
    }

    // ── PROP-1: jump_to_body_state_transitions ─────────────────────────

    // PROP-1: For any initial StepState, `jump_to_body` never returns an
    // error. After the call, the body step's state is always `Pending`
    // (for Succeeded → Pending via `mark_pending`; all other states
    // unchanged because `jump_to` resets PC but not state).
    proptest! {
        #[test]
        fn prop1_jump_to_body_never_errors(state in arb_step_state()) {
            let mut run = fresh_frame();
            let body = StepIdx::new(1);

            // Set the body step to the arbitrary state
            match state {
                StepState::Pending => {
                    run.mark_pending(body).map_err(|e| format!("mark_pending: {e:?}")).unwrap();
                }
                StepState::Running => {
                    run.mark_running(body).map_err(|e| format!("mark_running: {e:?}")).unwrap();
                }
                StepState::Succeeded => {
                    run.mark_succeeded(body).map_err(|e| format!("mark_succeeded: {e:?}")).unwrap();
                }
                StepState::Failed => {
                    run.mark_failed(body).map_err(|e| format!("mark_failed: {e:?}")).unwrap();
                }
                StepState::Waiting => {
                    run.mark_running(body).map_err(|e| format!("mark_running: {e:?}")).unwrap();
                    run.mark_waiting(body).map_err(|e| format!("mark_waiting: {e:?}")).unwrap();
                }
                StepState::Asking => {
                    run.mark_running(body).map_err(|e| format!("mark_running: {e:?}")).unwrap();
                    run.mark_asking(body).map_err(|e| format!("mark_asking: {e:?}")).unwrap();
                }
                StepState::Cancelled => {
                    run.mark_cancelled(body).map_err(|e| format!("mark_cancelled: {e:?}")).unwrap();
                }
                _ => {}
            }

            let result = jump_to_body(&mut run, body);
            prop_assert!(result.is_ok(), "jump_to_body must never error; state={state:?}, err={:?}", result.err());

            let signal = result.unwrap();
            prop_assert_eq!(signal, vb_core::EngineSignal::Continue);

            // PC must be at body
            prop_assert_eq!(run.pc(), body);

            // For Succeeded, state transitions to Pending; others stay unchanged
            let step_state_after = run.step_state(body).unwrap();
            if state == StepState::Succeeded {
                prop_assert!(step_state_after == StepState::Pending,
                    "Succeeded must transition to Pending; got {step_state_after:?}");
            }
        }
    }

    // ── PROP-2: for_each_n_items_all_reentry ────────────────────────────

    // PROP-2: For any list of 1..=N items, for_each processes every item
    // without panic, body re-entries succeed (Succeeded→Pending), and the
    // final for_each_next routes to `done`.
    proptest! {
        #[test]
        fn prop2_for_each_n_items_all_reentry(items in arb_i64_list()) {
            let mut run = fresh_frame();
            let mut store = ValueStore::new();

            let input = SlotIdx::new(0);
            let item_slot = SlotIdx::new(1);
            let iterator_slot = SlotIdx::new(2);
            let body = StepIdx::new(1);
            let done = StepIdx::new(2);

            list_in_slot(&mut run, &mut store, input, items.clone());

            // Start for_each
            let start = for_each_start(
                &mut run,
                &mut store,
                input,
                item_slot,
                1000u32,
                body,
                done,
                Some(iterator_slot),
            );
            prop_assert!(start.is_ok(), "for_each_start must succeed");

            if items.is_empty() {
                // Empty list: should jump directly to done
                prop_assert_eq!(run.pc(), done, "empty list should route to done");
                return Ok(());
            }

            // First item is bound; PC is at body
            prop_assert_eq!(run.pc(), body);
            prop_assert_eq!(
                *run.read_slot(item_slot).unwrap(),
                items[0],
                "first item must be bound"
            );

            // Process all items: body Succeeded → for_each_next
            let body_step = StepIdx::new(1);
            for (i, _expected_item) in items.iter().enumerate() {
                // Body step runs for current item → Succeeded
                run.mark_running(body_step).unwrap();
                run.mark_succeeded(body_step).unwrap();

                if i + 1 < items.len() {
                    // More items remain — for_each_next re-entry
                    let next = for_each_next(
                        &mut run,
                        &mut store,
                        iterator_slot,
                        body,
                        done,
                        Some(item_slot),
                    );
                    prop_assert!(next.is_ok(), "for_each_next[{i}] must succeed");
                    let pc = run.pc();
                    prop_assert!(pc == body, "PC must be body for item {next_idx}, got {pc:?}",
                        next_idx = i + 1);
                    prop_assert_eq!(
                        *run.read_slot(item_slot).unwrap(),
                        items[i + 1],
                        "item {} must be bound",
                        i + 1
                    );
                } else {
                    // Last item done — for_each_next should route to done
                    let next = for_each_next(
                        &mut run,
                        &mut store,
                        iterator_slot,
                        body,
                        done,
                        Some(item_slot),
                    );
                    prop_assert!(next.is_ok(), "final for_each_next must succeed");
                    prop_assert_eq!(run.pc(), done, "PC must be done after last item");
                }
            }
        }
    }

    // ── PROP-3: reduce_accumulation_reentry ─────────────────────────────

    // PROP-3: For any list of 1..=N I64 items, reduce processes each
    // item without panic, body re-entries succeed (Succeeded→Pending).
    // Note: This property tests the mechanics of re-entry, not accumulator
    // correctness (which depends on the body expression's behavior).
    proptest! {
        #[test]
        fn prop3_reduce_accumulation_reentry(items in arb_i64_list()) {
            let mut run = fresh_frame();
            let mut store = ValueStore::new();

            let input = SlotIdx::new(0);
            let accumulator = SlotIdx::new(1);
            let iterator_slot = SlotIdx::new(3);
            let body = StepIdx::new(1);
            let done = StepIdx::new(2);

            list_in_slot(&mut run, &mut store, input, items.clone());

            let plan = minimal_workflow_with_const(0i64);
            let start = reduce_start(
                &plan,
                &mut run,
                &mut store,
                input,
                accumulator,
                vb_core::ids::ConstIdx::new(0),
                body,
                done,
                Some(iterator_slot),
            );
            prop_assert!(start.is_ok(), "reduce_start must succeed");

            if items.is_empty() {
                // Empty list: should jump directly to done
                prop_assert_eq!(run.pc(), done, "empty list should route to done");
                return Ok(());
            }

            // First item bound; PC at body
            prop_assert_eq!(run.pc(), body);

            let body_step = StepIdx::new(1);
            let item_count = items.len();

            for i in 0..item_count {
                // Body completes for current item: Running → Succeeded
                run.mark_running(body_step).unwrap();
                run.mark_succeeded(body_step).unwrap();

                if i + 1 < item_count {
                    // reduce_next re-entry for remaining items
                    let next = reduce_next(
                        &mut run,
                        &mut store,
                        iterator_slot,
                        accumulator,
                        body,
                        done,
                        Some(SlotIdx::new(4)),
                    );
                    prop_assert!(next.is_ok(), "reduce_next[{i}] must succeed");
                    prop_assert_eq!(run.pc(), body, "PC must be body for item {}", i + 1);
                } else {
                    // Last item — reduce_next routes to done
                    let next = reduce_next(
                        &mut run,
                        &mut store,
                        iterator_slot,
                        accumulator,
                        body,
                        done,
                        Some(SlotIdx::new(4)),
                    );
                    prop_assert!(next.is_ok(), "final reduce_next must succeed");
                    prop_assert_eq!(run.pc(), done, "PC must be done after last item");
                }
            }
        }
    }

    // ── PROP-4: collect_pagination_reentry ──────────────────────────────

    // PROP-4: For any list of items with page_size, collect processes
    // all pages without panic, and body re-entry succeeds for each page.
    proptest! {
        #[test]
        fn prop4_collect_pagination_reentry(
            items in arb_i64_list(),
            page_size in 1u32..=10u32,
        ) {
            let mut run = fresh_frame();
            let mut store = ValueStore::new();
            let mut states = CollectStates::new();

            let source = SlotIdx::new(0);
            let collector_slot = SlotIdx::new(1);
            let body = StepIdx::new(1);
            let done = StepIdx::new(2);

            list_in_slot(&mut run, &mut store, source, items.clone());

            let start = collect_start(
                &mut run,
                &mut store,
                &mut states,
                source,
                1000,
                page_size,
                body,
                done,
                Some(collector_slot),
                None,
            );
            prop_assert!(start.is_ok(), "collect_start must succeed");

            if items.is_empty() {
                prop_assert_eq!(run.pc(), done, "empty list should route to done");
                return Ok(());
            }

            let ps = page_size as usize;
            // Non-empty list always routes to body first, retaining state.
            prop_assert_eq!(run.pc(), body);

            let body_step = StepIdx::new(1);
            let total_pages = (items.len() + ps - 1) / ps;

            for page_num in 0..total_pages {
                // Body processes current page → Succeeded
                run.mark_running(body_step).unwrap();
                run.mark_succeeded(body_step).unwrap();

                if page_num + 1 < total_pages {
                    // More pages — collect_next re-entry
                    let next = collect_next(
                        &mut run,
                        &mut store,
                        &mut states,
                        collector_slot,
                        body,
                        done,
                    );
                    prop_assert!(next.is_ok(),
                        "collect_next page {} must succeed (total: {total_pages})", page_num + 1);
                    let pc = run.pc();
                    prop_assert!(pc == body,
                        "PC must be body for page {next_page} (got {pc:?})", next_page = page_num + 1);
                } else {
                    // Last page — collect_next routes to done
                    let next = collect_next(
                        &mut run,
                        &mut store,
                        &mut states,
                        collector_slot,
                        body,
                        done,
                    );
                    prop_assert!(next.is_ok(),
                        "final collect_next must succeed (page {total_pages})");
                    prop_assert_eq!(run.pc(), done,
                        "PC must be done after last page");
                }
            }
        }
    }

    // ── PROP-5: repeat_attempt_reentry ──────────────────────────────────

    // PROP-5: For any max_attempts in 1..=10, repeat runs exactly that
    // many times without panic, and each body re-entry succeeds via
    // repeat_check → jump_to_body (Succeeded→Pending).
    proptest! {
        #[test]
        fn prop5_repeat_attempt_reentry(max_attempts in 1u16..=10u16) {
            let mut run = fresh_frame();

            let attempt_slot = SlotIdx::new(0);
            let body = StepIdx::new(1);
            let done = StepIdx::new(2);

            let start = repeat_start(&mut run, max_attempts, body, done, Some(attempt_slot));
            prop_assert!(start.is_ok(), "repeat_start must succeed");
            prop_assert_eq!(run.pc(), body);

            let body_step = StepIdx::new(1);

            // After repeat_start, counter is (max, 0).
            // For each attempt: body runs → Succeeded, then repeat_check
            // increments the counter and decides: more?→body, exhausted?→done.
            for attempt_num in 0..max_attempts {
                // Body executes → Succeeded
                run.mark_running(body_step).unwrap();
                run.mark_succeeded(body_step).unwrap();

                // repeat_check increments and decides routing
                let rc = repeat_check(
                    &mut run,
                    attempt_slot,
                    done,
                    Some(body), // next=body (where to go if more attempts)
                    StepIdx::ZERO,
                );
                prop_assert!(rc.is_ok(),
                    "repeat_check attempt {} must succeed (max={max_attempts})", attempt_num);

                if attempt_num + 1 < max_attempts {
                    // More attempts remain — repeat_check routes back to body
                    let pc = run.pc();
                    prop_assert!(pc == body,
                        "PC must be body after repeat_check attempt {attempt_num} (max={max_attempts}, got {pc:?})");
                } else {
                    // Last attempt — repeat_check routes to done
                    let pc = run.pc();
                    prop_assert!(pc == done,
                        "PC must be done after final repeat_check (max={max_attempts}, got {pc:?})");
                }
            }
        }
    }

    // ── PROP-6: repeat_check_loop_back ──────────────────────────────────

    // PROP-6: Additional property: repeat_check correctly loops back to body
    // when attempts remain (complement to PROP-5).
    proptest! {
        #[test]
        fn prop6_repeat_check_loops_back_when_attempts_remain(
            max_attempts in 2u16..=10u16,
            current_attempt in 0u16..=8u16,
        ) {
            // Only test when current_attempt + 1 < max_attempts
            prop_assume!((current_attempt + 1) < max_attempts);

            let mut run = fresh_frame();

            let attempt_slot = SlotIdx::new(0);
            let done = StepIdx::new(2);
            let next_body = StepIdx::new(1);

            // Pre-load the slot with packed (max, current) state
            let packed: i64 = (i64::from(max_attempts) << 32) | i64::from(current_attempt);
            run.write_slot(attempt_slot, SlotValue::I64(packed)).unwrap();

            let body_step = StepIdx::new(1);
            run.mark_running(body_step).unwrap();
            run.mark_succeeded(body_step).unwrap();

            let rc = repeat_check(&mut run, attempt_slot, done, Some(next_body), StepIdx::ZERO);
            prop_assert!(rc.is_ok(),
                "repeat_check must succeed when attempts remain (max={max_attempts}, current={current_attempt})");
            prop_assert_eq!(run.pc(), next_body,
                "PC must be body_entry when attempts remain");

            // Verify counter incremented
            let updated = match *run.read_slot(attempt_slot).unwrap() {
                SlotValue::I64(v) => v,
                other => panic!("expected I64, got {other:?}"),
            };
            let (decoded_max, decoded_current) = {
                let max = (updated >> 32) as u16;
                let cur = (updated & 0xFFFF) as u16;
                (max, cur)
            };
            prop_assert_eq!(decoded_max, max_attempts);
            prop_assert_eq!(decoded_current, current_attempt + 1);
        }
    }
}
