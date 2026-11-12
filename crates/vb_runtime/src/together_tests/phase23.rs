use super::*;

// ========================================================================
// Phase 23: Together bounded branches
// ========================================================================

// -- 1. Bounded branches: together respects branch limits --

#[test]
fn phase23_together_start_accepts_max_u16_branches() {
    // Given u16::MAX = 65535 branches (the maximum that fits in u16)
    let mut run = fresh_frame_with(u16::MAX, 2);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let branches: Vec<StepIdx> = (0..u16::MAX).map(|i| StepIdx::new(i)).collect();
    // When calling together_start with exactly u16::MAX branches
    let result = together_start(
        &mut run,
        &mut store,
        &branches,
        StepIdx::new(0),
        Some(output),
    );
    // Then it succeeds
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
}

#[test]
fn phase23_together_start_rejects_more_than_u16_max_branches() {
    // Given u16::MAX + 1 branches (one more than fits in u16)
    let mut run = fresh_frame_with(u16::MAX, 2);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let mut branches: Vec<StepIdx> = (0..u16::MAX).map(|i| StepIdx::new(i)).collect();
    branches.push(StepIdx::new(0));
    // When calling together_start with u16::MAX + 1 branches
    let result = together_start(
        &mut run,
        &mut store,
        &branches,
        StepIdx::new(0),
        Some(output),
    );
    // Then it returns TogetherBranchLimitExceeded
    assert_branch_limit_error(result);
}

#[test]
fn phase23_together_start_single_branch_within_limit() {
    // Given a single branch (well within any reasonable limit)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let branch = StepIdx::new(1);
    // When calling together_start with 1 branch
    let result = together_start(
        &mut run,
        &mut store,
        &[branch],
        StepIdx::new(2),
        Some(output),
    );
    // Then it succeeds and jumps to the branch
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), branch);
}

#[test]
fn phase23_together_start_many_branches_within_u16_limit() {
    // Given 256 branches (within u16 range)
    let mut run = fresh_frame_with(300, 2);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let branches: Vec<StepIdx> = (1..=256).map(StepIdx::new).collect();
    let first_branch = branches[0];
    // When calling together_start
    let result = together_start(
        &mut run,
        &mut store,
        &branches,
        StepIdx::new(299),
        Some(output),
    );
    // Then it succeeds and jumps to the first branch
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), first_branch);
}

// -- 2. Branch state tracking: each branch has independent state --

#[test]
fn phase23_branch_zero_independent_state_does_not_touch_accumulator() {
    // Given an accumulator with pre-existing data and an output slot
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);
    fixture.seed(accumulator, vec![SlotValue::I64(100)]);
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::I64(999)),
        Ok(()),
        "write"
    );
    // When calling together_branch for branch 0
    let result = fixture.branch(0, (entry, StepIdx::new(6)), (accumulator, Some(output)));
    // Then accumulator is untouched (still has 1 item)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_branch_zero_independent_state_does_not_touch_accumulator_list1(&fixture, accumulator);
}

#[test]
fn phase23_branch_one_appends_own_result_independently() {
    // Given an empty accumulator and output = I64(42)
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);
    fixture.seed(accumulator, vec![]);
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::I64(42)),
        Ok(()),
        "write"
    );
    // When calling together_branch for branch 1
    let result = fixture.branch(1, (entry, StepIdx::new(6)), (accumulator, Some(output)));
    // Then accumulator has exactly I64(42) from branch 1
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_branch_one_appends_own_result_independently_list1(&fixture, accumulator);
}

#[test]
fn phase23_sequential_branches_build_independent_accumulator_state() {
    // Simulate 3 branches: branch 0 runs first, branch 1 appends result A,
    // branch 2 appends result B. Each branch contributes independently.
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);

    // Branch 0: does not append, just jumps to entry
    fixture.seed(accumulator, vec![]);
    let result0 = fixture.branch(0, (entry, StepIdx::new(6)), (accumulator, Some(output)));
    assert_eq!(result0, Ok(vb_core::EngineSignal::Continue));

    // Simulate branch 0 body writing I64(10) to output
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::I64(10)),
        Ok(()),
        "write"
    );

    // Branch 1: appends I64(10)
    let result1 = fixture.branch(1, (entry, StepIdx::new(6)), (accumulator, Some(output)));
    assert_eq!(result1, Ok(vb_core::EngineSignal::Continue));

    // Simulate branch 1 body writing I64(20) to output
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::I64(20)),
        Ok(()),
        "write"
    );

    // Branch 2: appends I64(20)
    let result2 = fixture.branch(2, (entry, StepIdx::new(6)), (accumulator, Some(output)));
    assert_eq!(result2, Ok(vb_core::EngineSignal::Continue));

    // Accumulator now has [I64(10), I64(20)] from branches 1 and 2
    assert_sequential_branches_build_independent_accumulator_state_list1(&fixture, accumulator);
}

#[test]
fn phase23_different_branch_values_do_not_interfere() {
    // Given an accumulator with one value and a different value in output
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);
    fixture.seed(accumulator, vec![SlotValue::I64(1), SlotValue::I64(2)]);
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::Bool(true)),
        Ok(()),
        "write"
    );
    // When branch 3 appends Bool(true)
    let result = fixture.branch(3, (entry, StepIdx::new(6)), (accumulator, Some(output)));
    // Then accumulator has [I64(1), I64(2), Bool(true)] without corruption
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_different_branch_values_do_not_interfere_list1(&fixture, accumulator);
}

// -- 3. Join semantics: all branches must complete before join --

#[test]
fn phase23_join_appends_last_branch_result_to_accumulator() {
    // Given a completed 2-branch scenario with results [I64(10)] in accumulator
    // and I64(20) in output (the last branch result)
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    fixture.seed(accumulator, vec![SlotValue::I64(10)]);
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::I64(20)),
        Ok(()),
        "write"
    );
    // When calling together_join
    assert!(fixture.run.add_parallel_in_flight(2).is_ok());
    let result = fixture.join(
        2,
        (accumulator, Some(output)),
        (Some(next_step), StepIdx::ZERO),
    );
    // Then output has [I64(10), I64(20)] - all 2 branches merged
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(fixture.run.pc(), next_step);
    assert_join_appends_last_branch_result_to_accumulator_list1(&fixture, output);
}

#[test]
fn phase23_join_three_branches_all_results_collected() {
    // Given 3 branches: accumulator has [I64(10), I64(20)], output has I64(30)
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    fixture.seed(accumulator, vec![SlotValue::I64(10), SlotValue::I64(20)]);
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::I64(30)),
        Ok(()),
        "write"
    );
    // When calling together_join with branch_count=3
    assert!(fixture.run.add_parallel_in_flight(3).is_ok());
    let result = fixture.join(
        3,
        (accumulator, Some(output)),
        (Some(next_step), StepIdx::ZERO),
    );
    // Then output has all 3 branch results
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_join_three_branches_all_results_collected_list1(&fixture, output);
}

#[test]
fn phase23_join_produces_list_in_output_slot() {
    // Given valid accumulator and output state
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(7)]);
    assert_eq!(run.write_slot(output, SlotValue::I64(8)), Ok(()), "write");
    // When calling together_join
    assert!(run.add_parallel_in_flight(2).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        2,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then the output slot holds a SlotValue::List (the merged results)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    let final_val = *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"));
    assert!(matches!(final_val, SlotValue::List(_)));
}

#[test]
fn phase23_join_merges_taint_from_accumulator_and_output() {
    // Given an accumulator with a derived taint and output with secret taint
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    seed_derived_accumulator(&mut fixture, accumulator);

    restore_derived_list(&mut fixture, accumulator);

    assert_eq!(
        fixture
            .run
            .write_slot_with_taint(output, SlotValue::I64(20), Taint::Secret),
        Ok(()),
        "write"
    );
    // When calling together_join
    assert!(fixture.run.add_parallel_in_flight(2).is_ok());
    let result = fixture.join(
        2,
        (accumulator, Some(output)),
        (Some(next_step), StepIdx::ZERO),
    );
    // Then the output taint is the join of DerivedFromSecret and Secret = Secret
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_joined_secret_taint(&fixture, output);
}

// -- 4. Failure policy: single branch failure behavior (fail-fast) --

#[test]
fn phase23_branch_failure_when_accumulator_corrupted_propagates_error() {
    // Given a branch operation where the accumulator is corrupted (not a list)
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    // Accumulator holds a non-list value (simulating corruption)
    assert_eq!(
        fixture.run.write_slot(accumulator, SlotValue::Bool(false)),
        Ok(()),
        "write"
    );
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::I64(42)),
        Ok(()),
        "write"
    );
    // When calling together_branch with branch=1 (triggers append)
    let result = fixture.branch(
        1,
        (StepIdx::new(3), StepIdx::new(4)),
        (accumulator, Some(output)),
    );
    // Then it returns TypeMismatch (fail-fast on corrupted state)
    assert_corrupt_boolean_accumulator(result);
}

#[test]
fn phase23_branch_failure_preserves_existing_accumulator_state() {
    // Given an accumulator with data from a prior branch
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    fixture.seed(accumulator, vec![SlotValue::I64(10)]);
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::Bool(false)),
        Ok(()),
        "write"
    );
    // When calling together_branch with branch=1 but accumulator corrupted after
    // First make the accumulator corrupt
    assert_eq!(
        fixture.run.write_slot(accumulator, SlotValue::Bool(false)),
        Ok(()),
        "write"
    );
    let result = fixture.branch(
        1,
        (StepIdx::new(3), StepIdx::new(4)),
        (accumulator, Some(output)),
    );
    // Then the error is returned (fail-fast)
    assert!(result.is_err());
    // And the output slot still holds the value from the last successful branch
    assert_preserved_boolean_output(&fixture, output);
}

#[test]
fn phase23_join_failure_when_output_slot_missing() {
    // Given a valid accumulator but no output slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    // When calling together_join with output=None
    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        1,
        accumulator,
        None,
        Some(StepIdx::new(5)),
        StepIdx::ZERO,
    );
    // Then it returns MissingOutputSlot (fail-fast)
    assert_phase_join_output_error(result);
}

#[test]
fn phase23_start_failure_when_branches_empty() {
    // Given no branches
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    // When calling together_start with empty branches
    let result = together_start(&mut run, &mut store, &[], StepIdx::new(2), Some(output));
    // Then it returns InvalidCompiledWorkflow (fail-fast: cannot start with 0 branches)
    assert_phase_empty_start_error(result);
}

// -- 5. Partial failure handling: some branches succeed, some fail --

#[test]
fn phase23_partial_failure_first_branch_succeeds_second_corrupt_accumulator() {
    // Simulate: branch 0 succeeds, branch 1 would succeed, but then
    // a corrupted accumulator causes failure
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);

    complete_first_two_branches(&mut fixture, (accumulator, output), entry);

    // Verify accumulated state so far: [I64(10)]
    assert_partial_failure_first_branch_succeeds_second_corrupt_accumulator_list1(
        &fixture,
        accumulator,
    );

    assert_corrupt_third_branch(&mut fixture, (accumulator, output), entry);
}

#[test]
fn phase23_partial_failure_output_preserved_after_branch_error() {
    // Given a scenario where branches have partially executed
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);

    append_null_branch(&mut fixture, (accumulator, output));

    // Now corrupt the accumulator
    assert_eq!(
        fixture.run.write_slot(accumulator, SlotValue::I64(999)),
        Ok(()),
        "write"
    );

    // Branch 2 fails
    let fail_result = fixture.branch(
        2,
        (StepIdx::new(3), StepIdx::new(4)),
        (accumulator, Some(output)),
    );
    assert!(fail_result.is_err());

    // The output slot still holds Null from the last successful branch
    let output_val = *fixture
        .run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"));
    assert_eq!(output_val, SlotValue::Null);
}

#[test]
fn phase23_join_with_single_branch_collects_one_result() {
    // Given a single-branch together: accumulator empty, output = I64(42)
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    fixture.seed(accumulator, vec![]);
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::I64(42)),
        Ok(()),
        "write"
    );
    // When calling together_join with branch_count=1
    assert!(fixture.run.add_parallel_in_flight(1).is_ok());
    let result = fixture.join(
        1,
        (accumulator, Some(output)),
        (Some(next_step), StepIdx::ZERO),
    );
    // Then output has [I64(42)] (the single branch result)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_join_with_single_branch_collects_one_result_list1(&fixture, output);
}

#[test]
fn phase23_taint_propagation_across_branches() {
    // Verify that taint from each branch is propagated into the accumulator
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);

    // Start with clean accumulator
    fixture.seed(accumulator, vec![]);
    // Branch 0 body writes I64(10) with Secret taint
    assert_eq!(
        fixture
            .run
            .write_slot_with_taint(output, SlotValue::I64(10), Taint::Secret),
        Ok(()),
        "write"
    );

    // Branch 1 appends I64(10) - accumulator should pick up Secret taint
    let result = fixture.branch(1, (entry, StepIdx::new(6)), (accumulator, Some(output)));
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    // Accumulator taint should be Secret from the branch output
    let acc_taint = fixture
        .run
        .read_taint(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"));
    assert_eq!(acc_taint, Taint::Secret);
}

#[test]
fn phase23_start_creates_empty_list_output_even_with_multiple_branches() {
    // Given 3 branches
    let mut run = fresh_frame_with(8, 2);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let branches = [StepIdx::new(3), StepIdx::new(4), StepIdx::new(5)];
    // When calling together_start
    let result = together_start(
        &mut run,
        &mut store,
        &branches,
        StepIdx::new(2),
        Some(output),
    );
    // Then output slot has an empty list as the initial accumulator
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_multi_start_empty_list(&run, &store, output);
}

#[test]
fn phase23_join_preserves_order_of_branch_results() {
    // Given 4 branches with results accumulated in order
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    fixture.seed(
        accumulator,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::I64(4)),
        Ok(()),
        "write"
    );
    // When calling together_join with branch_count=4
    assert!(fixture.run.add_parallel_in_flight(4).is_ok());
    let result = fixture.join(
        4,
        (accumulator, Some(output)),
        (Some(next_step), StepIdx::ZERO),
    );
    // Then the final list preserves insertion order [1, 2, 3, 4]
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_join_preserves_order_of_branch_results_list1(&fixture, output);
}

fn assert_branch_zero_independent_state_does_not_touch_accumulator_list1(
    fixture: &Fixture,
    accumulator: SlotIdx,
) {
    let Fixture { run, store } = fixture;
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(100)));
        }
        other => {
            assert!(has_list_variant(other));
        }
    }
}

fn assert_branch_one_appends_own_result_independently_list1(
    fixture: &Fixture,
    accumulator: SlotIdx,
) {
    let Fixture { run, store } = fixture;
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(42)));
        }
        other => {
            assert!(has_list_variant(other));
        }
    }
}

fn assert_sequential_branches_build_independent_accumulator_state_list1(
    fixture: &Fixture,
    accumulator: SlotIdx,
) {
    let Fixture { run, store } = fixture;
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 2);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(20)));
        }
        other => {
            assert!(has_list_variant(other));
        }
    }
}

fn assert_different_branch_values_do_not_interfere_list1(fixture: &Fixture, accumulator: SlotIdx) {
    let Fixture { run, store } = fixture;
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 3);
            assert_eq!(items.get(0), Some(&SlotValue::I64(1)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(2)));
            assert_eq!(items.get(2), Some(&SlotValue::Bool(true)));
        }
        other => {
            assert!(has_list_variant(other));
        }
    }
}

fn assert_join_appends_last_branch_result_to_accumulator_list1(fixture: &Fixture, output: SlotIdx) {
    let Fixture { run, store } = fixture;
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 2);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(20)));
        }
        other => {
            assert!(has_list_variant(other));
        }
    }
}

fn assert_join_three_branches_all_results_collected_list1(fixture: &Fixture, output: SlotIdx) {
    let Fixture { run, store } = fixture;
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 3);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(20)));
            assert_eq!(items.get(2), Some(&SlotValue::I64(30)));
        }
        other => {
            assert!(has_list_variant(other));
        }
    }
}

fn assert_partial_failure_first_branch_succeeds_second_corrupt_accumulator_list1(
    fixture: &Fixture,
    accumulator: SlotIdx,
) {
    let Fixture { run, store } = fixture;
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
        }
        other => {
            assert!(has_list_variant(other));
        }
    }
}

fn assert_join_with_single_branch_collects_one_result_list1(fixture: &Fixture, output: SlotIdx) {
    let Fixture { run, store } = fixture;
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(42)));
        }
        other => {
            assert!(has_list_variant(other));
        }
    }
}

fn assert_join_preserves_order_of_branch_results_list1(fixture: &Fixture, output: SlotIdx) {
    let Fixture { run, store } = fixture;
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 4);
            assert_eq!(items.get(0), Some(&SlotValue::I64(1)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(2)));
            assert_eq!(items.get(2), Some(&SlotValue::I64(3)));
            assert_eq!(items.get(3), Some(&SlotValue::I64(4)));
        }
        other => {
            assert!(has_list_variant(other));
        }
    }
}

fn assert_preserved_boolean_output(fixture: &Fixture, output: SlotIdx) {
    let output_val = *fixture
        .run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"));
    assert_eq!(output_val, SlotValue::Bool(false));
}

fn complete_first_two_branches(fixture: &mut Fixture, slots: (SlotIdx, SlotIdx), entry: StepIdx) {
    let (accumulator, output) = slots;
    // Branch 0 succeeds (no append)
    fixture.seed(accumulator, vec![]);
    let result0 = fixture.branch(0, (entry, StepIdx::new(6)), (accumulator, Some(output)));
    assert_eq!(result0, Ok(vb_core::EngineSignal::Continue));

    // Branch 0 body writes I64(10)
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::I64(10)),
        Ok(()),
        "write"
    );

    // Branch 1 succeeds (appends I64(10))
    let result1 = fixture.branch(1, (entry, StepIdx::new(6)), (accumulator, Some(output)));
    assert_eq!(result1, Ok(vb_core::EngineSignal::Continue));
}

fn assert_corrupt_third_branch(fixture: &mut Fixture, slots: (SlotIdx, SlotIdx), entry: StepIdx) {
    let (accumulator, output) = slots;
    // Branch 1 body writes I64(20)
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::I64(20)),
        Ok(()),
        "write"
    );

    // Corrupt the accumulator before branch 2
    assert_eq!(
        fixture.run.write_slot(accumulator, SlotValue::Bool(false)),
        Ok(()),
        "write"
    );

    // Branch 2 fails due to corrupted accumulator
    let result2 = fixture.branch(2, (entry, StepIdx::new(6)), (accumulator, Some(output)));
    // Then branch 2 returns TypeMismatch (partial failure)
    assert!(result2.is_err());
    assert_third_branch_error(result2);
}

fn append_null_branch(fixture: &mut Fixture, slots: (SlotIdx, SlotIdx)) {
    let (accumulator, output) = slots;
    // Branch 0 writes Null to output (valid)
    fixture.seed(accumulator, vec![]);
    assert_eq!(
        fixture.run.write_slot(output, SlotValue::Null),
        Ok(()),
        "write"
    );

    // Branch 1 appends Null
    let result = fixture.branch(
        1,
        (StepIdx::new(3), StepIdx::new(4)),
        (accumulator, Some(output)),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
}

fn seed_derived_accumulator(fixture: &mut Fixture, accumulator: SlotIdx) {
    fixture.seed(accumulator, vec![SlotValue::I64(10)]);
    assert_eq!(
        fixture.run.write_slot_with_taint(
            accumulator,
            SlotValue::I64(10),
            Taint::DerivedFromSecret
        ),
        Ok(()),
        "write"
    );
}

fn restore_derived_list(fixture: &mut Fixture, accumulator: SlotIdx) {
    // Reset accumulator to list after the overwrite
    let list_id = fixture
        .store
        .insert_list(vec![SlotValue::I64(10)].into_boxed_slice())
        .ok()
        .unwrap_or_else(|| panic!("insert"));
    fixture
        .run
        .write_slot_with_taint(
            accumulator,
            SlotValue::List(list_id),
            Taint::DerivedFromSecret,
        )
        .ok()
        .unwrap_or_else(|| panic!("write"));
}

fn assert_joined_secret_taint(fixture: &Fixture, output: SlotIdx) {
    let out_taint = fixture
        .run
        .read_taint(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"));
    assert_eq!(out_taint, Taint::Secret);
}

fn assert_corrupt_boolean_accumulator(result: Outcome) {
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => {
            assert_eq!(
                other,
                Err(EngineError::TypeMismatch {
                    expected: "list",
                    found: "boolean"
                })
            );
        }
    }
}

fn assert_branch_limit_error(result: Outcome) {
    match result {
        Err(EngineError::TogetherBranchLimitExceeded { max }) => {
            assert_eq!(max, u16::MAX);
        }
        other => {
            assert_eq!(
                other,
                Err(EngineError::TogetherBranchLimitExceeded { max: u16::MAX })
            );
        }
    }
}

fn assert_phase_join_output_error(result: Outcome) {
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(
                other,
                Err(EngineError::MissingOutputSlot {
                    step: StepIdx::ZERO
                })
            );
        }
    }
}

fn assert_phase_empty_start_error(result: Outcome) {
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => {
            assert_eq!(reason, "together_start requires at least one branch");
        }
        other => {
            assert_eq!(
                other,
                Err(EngineError::InvalidCompiledWorkflow {
                    reason: "together_start requires at least one branch"
                })
            );
        }
    }
}

fn assert_third_branch_error(result2: Outcome) {
    match result2 {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => {
            assert_eq!(
                other,
                Err(EngineError::TypeMismatch {
                    expected: "list",
                    found: "boolean"
                })
            );
        }
    }
}

fn assert_multi_start_empty_list(run: &RunFrame, store: &ValueStore, output: SlotIdx) {
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 0);
        }
        other => {
            assert!(has_list_variant(other));
        }
    }
}
