# test-plan.md

bead_id: vb-core-lower-control-primitives
bead_title: "compiler: Lower v1 control primitives from YAML AST"
phase: 8
updated_at: 2026-05-15T00:00:00Z
attempt: 1

## Test Strategy

Tests are written to be executable NOW without vb-f04l landing. The vb-f04l bead provides the Verus/Kani proof infrastructure; unit tests and proptest for WaitKind exhaustiveness can proceed independently.

### Scope: vb_compile crate (crates/vb_compile/src/lib.rs, lower/mod.rs)

#### 1. Unit Tests for lower_* Functions

Target: existing `lower_*` public functions in `vb_compile/src/lib.rs`:
- `lower_set` (line 289)
- `lower_do` (line 306)
- `lower_choose` (line 329)
- `lower_for_each` (line 354)
- `lower_together` (line 397)
- `lower_collect` (line 446)
- `lower_reduce` (line 496)
- `lower_repeat` (line 548)
- `lower_wait` (line 615)
- `lower_ask` (line 645)
- `lower_finish` (line 686)

**Approach**: Direct unit tests using existing test helpers (`make_parts_for_lower`, `do_node`, `finish_node`).
Each test verifies the produced `CompiledNodeKind` and `CompiledNode` field values.

#### 2. Proptest for WaitKind Exhaustiveness

Target: `WaitKind` enum (line 604) — two variants:
- `Until { deadline: SlotIdx }`
- `Event { event: SlotIdx, timeout: Option<SlotIdx> }`

**Approach**: Proptest strategy that generates all combinations of:
- Both variants
- Valid `SlotIdx` values within u16 range
- `None` vs `Some` for timeout field

#### 3. Clippy / Format / Build Gate

**Approach**: Run `cargo clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings`
Expected: CLEAN (zero warnings, zero errors).

### Out of Scope (blocked on vb-f04l)

- Kani harnesses (vb-f04l provides the proof infrastructure)
- Miri stateful tests (vb-f04l integration required)
- Verus specs (vb-f04l provides the Verus proof writer)

## Test Cases

### lower_set
- [ ] lower_set produces `SetConst` node with correct output slot and value
- [ ] lower_set with no next step has `next: None`

### lower_do
- [ ] lower_do produces `Do` node with correct action and input slot
- [ ] lower_do records the input slot in builder

### lower_choose
- [ ] lower_choose produces `ChooseSlot` node
- [ ] lower_choose rejects empty branches with no otherwise
- [ ] lower_choose records each branch condition slot

### lower_for_each
- [ ] lower_for_each produces exactly 2 nodes: `ForEachStart` + `ForEachNext`
- [ ] ForEachStart has correct input, item_slot, limit, body, done
- [ ] ForEachNext has correct iterator_slot (= item_slot), body, done

### lower_together
- [ ] lower_together produces exactly 2 nodes: `TogetherStart` + `TogetherJoin`
- [ ] TogetherStart has correct branch_count and accumulator slot
- [ ] TogetherJoin has correct branch_count and accumulator
- [ ] lower_together rejects > u16::MAX branches

### lower_collect
- [ ] lower_collect produces exactly 3 nodes: `CollectStart`, `CollectPage`, `CollectFinish`
- [ ] CollectStart has correct source, limit, page_size, body, done
- [ ] CollectPage has correct collector_slot (= source), body, done
- [ ] CollectFinish has correct collector_slot

### lower_reduce
- [ ] lower_reduce produces exactly 3 nodes: `ReduceStart`, `ReduceNext`, `ReduceFinish`
- [ ] ReduceStart has correct input, accumulator, initial, body, done
- [ ] ReduceNext has correct iterator_slot (= accumulator), body, done
- [ ] ReduceFinish has correct accumulator

### lower_repeat
- [ ] lower_repeat produces exactly 3 nodes: `RepeatStart`, `RepeatAttempt`, `RepeatFinish`
- [ ] RepeatStart has correct max_attempts, body, done
- [ ] RepeatAttempt has correct attempt_slot (= id+1) and output slot
- [ ] RepeatFinish has correct result (= attempt_slot)
- [ ] lower_repeat computes attempt_slot as id+1 (id-plus-one invariant)

### lower_wait
- [ ] lower_wait(Until) produces `WaitUntil` with correct deadline_slot
- [ ] lower_wait(Event) produces `WaitEvent` with correct event and timeout_slot
- [ ] lower_wait(Event with Some timeout) records both event and timeout slots
- [ ] WaitKind exhaustiveness: proptest covers all WaitKind variants

### lower_ask
- [ ] lower_ask produces exactly 2 nodes: `Ask` + `AskResume`
- [ ] Ask has correct prompt, answer, timeout_slot
- [ ] AskResume has correct answer slot (= output)
- [ ] lower_ask computes resume = id+1 (id-plus-one invariant)
- [ ] lower_ask rejects id == u16::MAX (overflow)

### lower_finish
- [ ] lower_finish produces `Finish` node with correct result slot
- [ ] lower_finish records the result slot

### SlotCompiler
- [ ] slot_count returns 0 for empty builder
- [ ] slot_count returns correct count after recording slots
- [ ] record_slot tracks maximum slot index

### Proptest: WaitKind
- [ ] WaitKind::Until generates with valid SlotIdx
- [ ] WaitKind::Event generates with valid event SlotIdx
- [ ] WaitKind::Event generates with None timeout
- [ ] WaitKind::Event generates with Some timeout and valid SlotIdx

## Execution Plan

1. Write unit tests in `crates/vb_compile/src/lib.rs` test module
2. Add proptest for WaitKind (requires `proptest` feature in Cargo.toml if not present)
3. Run `cargo clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings`
4. Run `cargo test -p vb_compile --lib`
5. Verify all tests pass

## Risk Notes

- vb-f04l is DISCOVERY_BLOCKED for Kani/Miri/Verus — those lanes are deferred
- TLA+ spec is syntax-valid per TLC model checking (prior bead)
- Clippy ERR is INTEGRATED (runs against existing lib.rs)
- WaitKind proptest can proceed without vb-f04l
