# test-writer-report.md

bead_id: vb-core-lower-control-primitives
bead_title: "compiler: Lower v1 control primitives from YAML AST"
phase: 8
updated_at: 2026-05-15T00:00:00Z
attempt: 1

## Summary

42 new unit tests written for `vb_compile` crate covering all `lower_*` public functions
and `WaitKind` exhaustiveness. Tests compile, pass clippy, and pass format checks.

## Tests Written

All tests added to `crates/vb_compile/src/lib.rs` inside `#[cfg(test)] mod tests`.

### lower_set (2 tests)
- `lower_set_produces_set_const_node` — verifies SetConst kind, output slot, value, next
- `lower_set_with_no_next_step` — verifies next: None when no next step

### lower_do (2 tests)
- `lower_do_produces_do_node` — verifies Do kind, action, input slot
- `lower_do_records_input_slot` — verifies input slot is recorded in builder

### lower_choose (3 tests)
- `lower_choose_produces_choose_slot_node` — verifies ChooseSlot kind, branches, otherwise
- `lower_choose_rejects_empty_branches_with_no_otherwise` — verifies EmptyBranchTable error
- `lower_choose_records_branch_condition_slots` — verifies all condition slots recorded

### lower_for_each (3 tests)
- `lower_for_each_produces_two_nodes` — verifies exactly 2 nodes (ForEachStart + ForEachNext)
- `lower_for_each_start_node_fields` — verifies all ForEachStart fields
- `lower_for_each_next_node_fields` — verifies iterator_slot == item_slot invariant

### lower_together (4 tests)
- `lower_together_produces_two_nodes` — verifies exactly 2 nodes (TogetherStart + TogetherJoin)
- `lower_together_start_node_has_accumulator` — verifies TogetherStart has accumulator output
- `lower_together_join_node_has_accumulator` — verifies TogetherJoin has accumulator output
- `lower_together_rejects_too_many_branches` — verifies PrimitiveLoweringLimitExceeded for > u16::MAX branches

### lower_collect (4 tests)
- `lower_collect_produces_three_nodes` — verifies exactly 3 nodes
- `lower_collect_start_node_fields` — verifies CollectStart fields
- `lower_collect_page_node_fields` — verifies collector_slot == source invariant
- `lower_collect_finish_node_fields` — verifies CollectFinish uses source as collector_slot

### lower_reduce (4 tests)
- `lower_reduce_produces_three_nodes` — verifies exactly 3 nodes
- `lower_reduce_start_node_fields` — verifies ReduceStart fields
- `lower_reduce_next_node_fields` — verifies iterator_slot == accumulator invariant
- `lower_reduce_finish_node_fields` — verifies ReduceFinish uses accumulator

### lower_repeat (4 tests)
- `lower_repeat_produces_three_nodes` — verifies exactly 3 nodes
- `lower_repeat_start_node_fields` — verifies RepeatStart fields
- `lower_repeat_attempt_node_has_id_plus_one_slot` — verifies attempt_slot = id + 1 (id-plus-one invariant)
- `lower_repeat_finish_node_fields` — verifies RepeatFinish result == attempt_slot

### lower_wait (3 tests)
- `lower_wait_until_produces_wait_until_node` — verifies WaitUntil kind and deadline_slot
- `lower_wait_event_produces_wait_event_node` — verifies WaitEvent kind with None timeout
- `lower_wait_event_with_timeout_records_both_slots` — verifies event and timeout slots both recorded

### WaitKind exhaustiveness (4 tests)
- `waitkind_until_variant_exists` — compile-time exhaustiveness for Until variant
- `waitkind_event_variant_with_none_timeout_exists` — Event with timeout: None
- `waitkind_event_variant_with_some_timeout_exists` — Event with timeout: Some
- `waitkind_is_exhaustive_two_variants` — match exhaustiveness check (compile-time guarantee)

### lower_ask (4 tests)
- `lower_ask_produces_two_nodes` — verifies exactly 2 nodes (Ask + AskResume)
- `lower_ask_node_fields` — verifies Ask fields (prompt, timeout_slot)
- `lower_ask_resume_node_has_id_plus_one` — verifies resume id = id + 1 (id-plus-one invariant)
- `lower_ask_rejects_max_id_overflow` — verifies PrimitiveLoweringLimitExceeded for id == u16::MAX

### lower_finish (2 tests)
- `lower_finish_produces_finish_node` — verifies Finish kind and result slot
- `lower_finish_records_result_slot` — verifies result slot is recorded in builder

### SlotCompiler (3 tests)
- `slot_compiler_empty_has_zero_count` — verifies empty builder has slot_count == 0
- `slot_compiler_records_max_slot_index` — verifies max slot tracking (5 → slot_count 6)
- `slot_compiler_push_constant_returns_index` — verifies constant indices increment correctly

## Execution Evidence

```
cargo clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings
  => cargo clippy: No issues found

cargo fmt -p vb_compile -- --check
  => (no output = clean)

cargo test -p vb_compile --lib
  => cargo test: 298 passed (1 suite, 2.25s)
     (298 = 256 pre-existing + 42 new)
```

## Not Covered (blocked on vb-f04l)

- Kani harnesses for idempotency gate parity verification
- Miri stateful tests for slot allocation
- Verus specs/proofs for id-plus-one invariants in lower_repeat and lower_ask

These require vb-f04l (DISCOVERY_BLOCKED for Kani/Miri/Verus tooling).

## Artifacts

- test-plan.md: `.beads/vb-core-lower-control-primitives/test-plan.md`
- test-writer-report.md: this file
- Updated STATE.md: `.beads/vb-core-lower-control-primitives/STATE.md`
