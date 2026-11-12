# Test Review — vb-ynp2k (independent review of vb-hrvix six constructor input-slot tests)

- **Delta reviewed:** `8a7e2340f..27090007f600`.
- **Tests under review:** six `#[test]` fns added in `crates/vb_core/src/workflow/tests.rs:586-813`.
- **Independent re-run:** `python3 /tmp/vb-run-gate.py 300 .beads/vb-ynp2k/evidence/actual-input-slot-tests cargo test -p vb_core --lib input_slot -- --test-threads=1` → **7 passed, 0 failed** (6 new + 1 pre-existing engine case), 2176 filtered out, exit 0, 11.058s.

## Verdict

**STATUS: APPROVED** — test suite is behavior-correct, concrete, and mutation-resistant. Two findings requiring author action before landing: **T1 (MEDIUM, evidence integrity)** and **T2 (LOW, field-level assertions)**. Neither is a code blocker: I independently confirmed the correct command passes 7/7.

---

## Test inventory and coverage matrix

| # | Test (line) | Input | slot_count | Expected | Behavior covered |
|---|---|---|---|---|---|
| 1 | `workflow_input_slots_accept_valid_increasing_indices` (586) | `[0,1,2]` I64/Bool/Symbol | 4 | `Ok` | valid strictly-increasing |
| 2 | `workflow_input_slots_accept_empty` (631) | `[]` | 1 | `Ok` | empty boundary |
| 3 | `workflow_input_slots_reject_out_of_bounds` (663) | `[5]` | 3 | `Err(InputSlotOutOfBounds)` | bounds check |
| 4 | `workflow_input_slots_reject_duplicate` (698) | `[0,0]` | 3 | `Err(DuplicateInputSlot)` | duplicate check |
| 5 | `workflow_input_slots_reject_non_canonical_order` (739) | `[1,0]` | 3 | `Err(NonCanonicalInputSlotOrder)` | strict increasing / descending |
| 6 | `workflow_input_slots_out_of_bounds_before_duplicate` (780) | `[5,5]` | 3 | `Err(InputSlotOutOfBounds)` | precedence: bounds before duplicate |

Coverage is complete against the production `validate_input_slots` contract (`workflow/mod.rs:876-906`): all three error variants, the empty case, the valid case, and the bounds-before-duplicate precedence. Each reject test asserts the **exact error variant** (not `is_err()`).

### Mutation resistance (would the behavior be caught if deleted?)

- Delete `validate_input_slots` entirely (return `Ok`) → tests 3,4,5,6 fail (expect `Err`, get `Ok`). **Caught.**
- Delete the bounds arm → test 3 (`[5]`, single element) would return `Ok`. **Caught.**
- Delete the duplicate arm → test 4 (`[0,0]`) would fall through to order check (`0 < 0` false) → `Ok`. **Caught.**
- Delete the order arm → test 5 (`[1,0]`) would return `Ok`. **Caught.**
- Reorder precedence (duplicate before bounds) → test 6 (`[5,5]`) would return `DuplicateInputSlot` instead of `InputSlotOutOfBounds`. **Caught.**

The reject tests are genuinely behavior-anchoring; the two `Ok` tests are legitimate positive/empty boundary coverage and do not by themselves need to catch deletion (the reject tests carry that burden).

### Fixture correctness

Each fixture exercises `CompiledWorkflow::try_from_parts`, which runs `validate_parts` → `validate_budget` → `validate_input_slots` (`workflow/mod.rs:38-40`). The fixtures (single `Nop` node, `entry=0`, empty accessors/expressions, `resource_contract(1, n, 1, 1, 1)`) are the established valid-shape pattern already used throughout this test file, so `validate_parts`/`validate_budget` pass and the input-slot error is genuinely what fires. The 7/7 green run confirms this empirically.

---

## Findings

### T1 (MEDIUM) — false test evidence in the final checkpoint handoff

- **Evidence file:** `.beads/vb-hrvix/evidence/cargo-test-input-slots.json` (final checkpoint 27090007f).
- **What it actually ran:** `cargo test -p velvet-ballistics --lib -- workflow_input_slots --exact --test-threads=1` → **"running 0 tests" / "0 passed"**, exit 0 (84.8s spent compiling the wrong crate).
- **Why zero:** (a) wrong package — these tests live in `crates/vb_core`, not the `velvet-ballistics` facade crate; (b) wrong filter — `--exact` with `workflow_input_slots` matches no test (real names are `workflow_input_slots_accept_*` etc.).
- **Contradiction:** the handoff/`contract.md` verification table cites this command as the "Production input_slots tests" gate with exit 0, implying the six constructor cases passed. A zero-test filtered run is **not evidence** (test-reviewer gate 8).
- **Correct evidence exists but is stale:** `.beads/vb-hrvix/evidence/prod-tests-input-slots.json` (earlier checkpoint dc2e55d57) ran `cargo test --lib -p vb_core -- input_slot` → 7 passed — matching my independent re-run.
- **Author fix:** regenerate the final-checkpoint test evidence with the correct command (`cargo test -p vb_core --lib input_slot -- --test-threads=1`), replace/annotate `cargo-test-input-slots.*`, and correct the `contract.md` verification table. The code itself is correct; this is evidence integrity only.

### T2 (LOW) — reject assertions wildcard the error fields

- **Source:** `tests.rs:676-678`, `711-713`, `752-754`, `793-795` use `Err(WorkflowError::… { .. })`.
- **Problem:** variant is asserted (good), but the payload fields (`slot`, `kind`, `prev_slot`, `curr_slot`) are wildcarded. A bug that returned the correct variant with the wrong slot index (e.g. `InputSlotOutOfBounds { slot: 0 }` instead of `slot: 5`) would pass.
- **Severity rationale:** LOW, not lethal — the variant discriminates the three behavior classes, which is the contract's core; field identity is secondary diagnostic data.
- **Author fix (optional strengthening):** bind fields, e.g. test 3 assert `slot == SlotIdx::new(5)`; test 5 assert `prev_slot == SlotIdx::new(1) && curr_slot == SlotIdx::new(0)`.

---

## Non-findings / notes

- Tests are `#[cfg(test)]` unit tests inside `vb_core`, using internal constructor API — appropriate for constructor-level behavior testing; the "public API only" rule applies to integration tests, not these.
- No ignored/sleep/mock/shared-state violations. Deterministic. `Result<(), String>` with explicit `match` — no silent error suppression.
- The pre-existing engine case `rejects_do_with_out_of_bounds_input_slot` (matched by the `input_slot` filter) also passes and is unaffected by this delta.
