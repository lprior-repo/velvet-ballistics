# Test Suite Review — vb-xi2f.24 State 10

**Review mode**: Suite review (implementation + tests)
**Reviewer**: test-reviewer
**Date**: 2026-06-01
**Status**: APPROVED WITH FINDINGS

## Summary

48 behavior tests written in three phases for the Reduce Multi-Step Body Lowering bead (vb-xi2f.24). All 48 active tests compile and pass deterministically. Test assertions use concrete variant destructuring and exact value comparisons — zero bare `is_ok()`, `is_err()`, `unwrap()`, `expect()`, or `panic()` in behavior assertions. Phase 3 (7 tests) are commented out as TDD compile-fail specifications and must be uncommented after `emit_reduce_body_steps` is implemented.

## Test Execution Evidence

```
$ cargo test -p vb_compile --lib
cargo test: 507 passed, 4 ignored (1 suite, 2.40s)

$ cargo test -p vb_compile --lib -- canonical_body_step_width
cargo test: 14 passed (1 suite, 0.00s)

$ cargo test -p vb_compile --lib -- body_width
cargo test: 17 passed (1 suite, 0.00s)

$ cargo test -p vb_compile --lib -- lower_canonical_aggregate
cargo test: 14 passed (1 suite, 0.00s)

$ cargo test -p vb_compile --lib -- multi_step
cargo test: 4 passed (1 suite, 0.00s)

$ cargo test -p vb_compile --lib -- tdd_red
cargo test: 5 passed (1 suite, 0.00s)
```

Zero flakes on repeated runs. No sleeps, no hidden mutable state, no `#[ignore]` on behavior tests.

---

## Gate Results

| Gate | Result | Details |
|------|--------|---------|
| 1. Tests compile and execute deterministically | APPROVED | 507 pass, 0 fail, confirmed deterministic across repeated runs. |
| 2. Integration tests use public API only | APPROVED | All tests are in `mod_compile_lowering/tests.rs` (in-crate unit/integration). Use `pub(super)` functions correctly. No mocks. Real `SlotCompiler`, real `lower_canonical_aggregate`, real `body_width`. |
| 3. Tests assert behavior, not implementation details | APPROVED | Assertions verify width values, error variant names, node counts, StepIdx positions, next-link chains — all observable behavior. No tests inspect internal data structures or call private functions. |
| 4. No ignored tests, sleeps, mocks, hidden mutable state, silent error suppression | APPROVED | Zero `#[ignore]` in vb-xi2f.24 tests. Zero `sleep()`. Zero mocks. `SlotCompiler::new()` creates fresh state per test. Errors are always destructured and asserted. |
| 5. Mutation thought experiment | APPROVED WITH FINDINGS | See Section below. |
| 6. Snapshot tests | N/A | No snapshot tests in this suite. |
| 7. Resource-heavy commands bounded | N/A | Behavior tests are lightweight unit/integration (sub-3s total). No broad verifier commands. |
| 8. Commented-out tests are not evidence | FINDING | 7 Phase 3 tests commented out (F-001 below). They are correctly marked as blocked and not claimed as evidence, but represent core behavior gaps. |

---

## Findings

### F-001 (HIGH): 7 Phase 3 tests commented out — core multi-step behaviors not executable

**Severity**: HIGH
**Rule**: Suite Review Gate 8 — commented-out tests are not test evidence.
**Affected file**: `crates/vb_compile/src/mod_compile_lowering/tests.rs:2659-2773`
**Affected behaviors**: B17-B22 (sequential assignment), B23-B28 (chain integrity), B35 (single-step equivalence), B54 direct (empty body via emit_reduce_body_steps)

**Details**:
Seven test functions are commented out behind a `PHASE-3-BLOCKED` marker. They call `emit_reduce_body_steps` which does not exist yet. The comment block includes:
1. `emit_reduce_body_steps_assigns_sequential_distinct_step_indices` (B17-B22)
2. `emit_reduce_body_steps_single_step_next_points_to_next_parameter` (B25)
3. `emit_reduce_body_steps_first_step_next_points_to_second_when_multi_step` (B23)
4. `emit_reduce_body_steps_last_step_next_points_to_next_parameter` (B24)
5. `emit_reduce_body_steps_all_next_links_are_some` (B27)
6. `emit_reduce_body_steps_empty_body_returns_step_field_shape` (B54 direct)
7. `emit_reduce_body_steps_produces_same_ir_as_emit_single_body_set_for_single_set` (B35)

These cover the CENTRAL behaviors of this bead: sequential StepIdx assignment, next-link chain integrity, single-step regression equivalence, and direct empty body handling by the multi-step dispatcher.

**Justification accepted**: The rationale (compile-fail, function doesn't exist) is technically valid. The test-writer has written complete test functions with correct assertions. The unblocking instructions are clear.

**Requirement**: These 7 tests MUST be uncommented, compiled, and proven to pass before bead closure. The Phase 2 TDD-red tests (which currently pass on the error arm) will transition to the success arm at the same time, providing double coverage. Do not close this bead until Phase 3 tests are active and green.

### F-002 (MEDIUM): TDD-red tests are self-passing dual-arm — no red→green signal

**Severity**: MEDIUM
**Rule**: Mutation resistance — tests that never fail provide limited regression protection.
**Affected tests**: 
- `lower_canonical_aggregate_multi_step_two_set_body_tdd_red` (L2449)
- `lower_canonical_aggregate_multi_step_three_set_body_tdd_red` (L2475)
- `lower_canonical_aggregate_multi_step_mixed_set_do_body_tdd_red` (L2503)
- `reduce_body_width_node_count_parity_two_set_body_tdd_red` (L2545)
- `lower_canonical_aggregate_body_ids_do_not_overlap_reduce_next_tdd_red` (L2573)

**Details**:
These tests use a dual-arm `match` pattern:

```rust
match result {
    Ok(builder) => { /* TDD GREEN assertion */ }
    Err(errors) => { /* TDD RED assertion */ }
}
```

Current execution: returns `Err(StepFieldShape)` → matches Err arm → asserts `has_step_field_shape` → **PASS**
After implementation: returns `Ok(builder)` → matches Ok arm → asserts node count / ID constraints → **PASS**

The tests are **always green** regardless of implementation state. While this is a deliberate progressive-testing pattern (the test automatically transitions when implementation changes), it has two consequences:

1. **No red→green signal**: When `emit_reduce_body_steps` is wired in, these tests don't change from FAIL to PASS — they were already PASS. The implementation author gets no TDD signal that the behavior changed.
2. **Error variant regression hole**: If someone changes the error from `StepFieldShape` to `StepIndexOutOfRange`, these tests still pass (the Err arm assertion `has_step_field_shape` fails, but... actually no — it would fail!). Correction: the test DOES catch wrong error variants.

**Verdict**: NOT a blocking issue. The dual-arm pattern is valid for progressive testing. The assertions within each arm are correct and will catch regressions in both states. The primary concern is that the test-writer-report claims these "FAIL RUNTIME" when they actually pass — this is a terminology error in the report, not a test quality defect.

**Recommendation**: Consider renaming these tests to remove `_tdd_red` suffix (since they pass in both states) or add a comment explaining the progressive-testing pattern. No test code changes needed.

### F-003 (MEDIUM): Contract C1 primitives tested as rejections only

**Severity**: MEDIUM
**Rule**: Contract parity — tests must cover contract acceptance criteria.
**Affected tests**: Lines 1861-1919 (Collect, Repeat, Choose, Together rejection tests)
**Contract ref**: C1 — `canonical_body_step_width() shall accept Reduce, Collect, Together, Repeat, and Choose variants`

**Details**:
The suite includes tests that verify Collect, Together, Repeat, and Choose are REJECTED with `UnsupportedStepPrimitive`. This matches the current implementation state. However, contract C1 says these primitives SHALL be accepted. If contract C1 is binding for this bead, acceptance tests are missing. If contract C1 is aspirational (future beads per out-of-scope Section 5), the rejection tests are correct and C1 should be narrowed.

Coordination with F-001 from test-plan-review.md: the plan review identifies the same scope ambiguity. Resolution should be consistent across both reviews.

**Recommendation**: Either:
- A) Add happy-path width tests for Collect/Together/Repeat/Choose (if in-scope), OR
- B) Narrow contract C1 to `{Set, Do, ForEach, Reduce}` and keep rejection tests as-is (if out-of-scope).
Resolution is a product-owner decision, not a test-writer defect.

### F-004 (LOW): `body_width_nested_reduce_rejected_pre_widening` — dual-arm transition test

**Severity**: LOW
**Rule**: Tests should have deterministic assertions, not conditional success.
**Affected test**: `body_width_nested_reduce_rejected_pre_widening` (L2071)

**Details**:
This test matches both `Ok(width)` and `Err(UnsupportedStepPrimitive)` arms, similar to F-002. When `canonical_body_step_width` is widened to accept Reduce, the Ok arm will execute with `width = 7` (post-widening). Currently the Err arm executes with `primitive = "reduce"`.

**Observation**: The computed width is 7, not 8. The test-plan's B07 BDD scenario tentatively uses 8. The test-writer correctly computed 7 (3 overhead + nested 'reduce' width of 4 = 7). This confirms the test-plan F-003 calculation error.

**Verdict**: Not blocking. The test is correct and the dual-arm pattern is acceptable for this transitional state.

### F-005 (LOW): Pre-existing `is_ok()` in same file (not vb-xi2f.24)

**Severity**: LOW
**Rule**: Assertions must be concrete.
**Affected test**: `canonical_body_step_width_accepts_for_each` (L1628, vb-xi2f.21 test)
**File**: `tests.rs:1636` — `assert!(result.is_ok(), ...)`

**Details**:
This is a pre-existing test from vb-xi2f.21 (ForEach body step width), not part of the vb-xi2f.24 test suite. It uses bare `is_ok()` followed by `result.ok()`. This does not affect the vb-xi2f.24 suite quality rating.

**Recommendation**: Fix in a separate bead (vb-xi2f.21 maintenance). Not blocking for this review.

---

## Banned Pattern Compliance (vb-xi2f.24 tests only)

| Pattern | Count | Verdict |
|---------|-------|---------|
| `is_ok()` without inner value | 0 | CLEAN |
| `is_err()` without error variant | 0 | CLEAN |
| `unwrap()` in behavior assertions | 0 | CLEAN |
| `expect()` in behavior assertions | 0 | CLEAN |
| `panic!()` in tests | 0 | CLEAN |
| `todo!()` / `unimplemented!()` | 0 | CLEAN |
| `dbg!()` | 0 | CLEAN |
| `#[ignore]` on behavior tests | 0 | CLEAN |
| Mock/stub of production types | 0 | CLEAN |
| `sleep()` or timing dependency | 0 | CLEAN |

Note: `.expect()` calls in test fixture construction (e.g., `compile_reduce_body(&body).expect("single Set body must compile")`) are fixture setup, not behavior assertions. The actual behavior assertions are the subsequent `assert_eq!` on node fields. This pattern is acceptable per the rubric.

---

## Mutation Thought Experiment

For the ACTIVE (non-commented) tests, what mutations would escape detection?

| Mutation | Caught By | Strength |
|----------|-----------|----------|
| `canonical_body_step_width` — remove Set from match | `canonical_body_step_width_returns_one_for_set` | STRONG (asserts exact width 1) |
| `canonical_body_step_width` — return Ok(0) for ForEach | `canonical_body_step_width_returns_three_for_foreach_with_one_set_body` | STRONG (asserts exact width 3) |
| `canonical_body_step_width` — accept Finish silently | `canonical_body_step_width_rejects_finish_with_unsupported_step_primitive` | STRONG (asserts exact error variant + field) |
| `canonical_body_step_width` — swap "finish" for "done" in error | `canonical_body_step_width_rejects_finish_...` (asserts `primitive == "finish"`) | STRONG (exact string match) |
| `body_width` — use `+` instead of `checked_add` | `body_width_returns_step_index_out_of_range_when_width_overflows_usize` | STRONG (asserts exact error variant at overflow) |
| `body_width` — under-count by 1 for Set | `body_width_returns_overhead_plus_n_for_n_set_steps` (asserts exact 5) | STRONG |
| `lower_canonical_aggregate` — wrong body_step (id instead of id+1) | `lower_canonical_aggregate_reduce_start_body_equals_id_plus_one` | STRONG (asserts exact StepIdx::new(1)) |
| `lower_canonical_aggregate` — swap done/next computation | `reduce_finish_id_is_next_step_plus_one` | STRONG (asserts exact StepIdx::new(3)) |
| `lower_canonical_aggregate` — lose parent next | `reduce_finish_next_is_parent_aggregate_next` | STRONG (asserts Some(StepIdx::new(20))) |
| `emit_single_body_set` — reject empty body silently | `lower_canonical_aggregate_rejects_empty_body_with_step_field_shape` | STRONG (asserts StepFieldShape on "steps") |
| `lower_canonical_aggregate` — accept empty body and emit nodes | `lower_canonical_aggregate_rejects_empty_body_with_step_field_shape` (asserts Err) | STRONG |
| **Sequential StepIdx assignment removed** | **NOT CAUGHT** (Phase 3 test commented out) | **GAP** (F-001) |
| **Next-link chain broken (last→None)** | **NOT CAUGHT** (Phase 3 test commented out) | **GAP** (F-001) |
| **emit_reduce_body_steps produces different IR than emit_single_body_set** | **NOT CAUGHT** (Phase 3 equivalence test commented out) | **GAP** (F-001) |

**Active mutation kill rate**: 11/11 mutations on active production paths are caught by named tests with STRONG assertions. The 3 gap mutations are in `emit_reduce_body_steps` paths that don't exist yet — coverage gap is coextensive with the implementation gap.

---

## Per-Contract-Clause Coverage

| Clause | Behaviors | Active Tests | Phase 3 (commented) | Verdict |
|--------|-----------|-------------|---------------------|---------|
| C1 (Width calculation) | B01-B11 | 30 tests (Set, Do, ForEach, Reduce, errors, boundaries) | None | COVERED |
| C2 (Width-node parity) | B12-B16 | 2 tests (single-step active, multi-step TDD dual-arm) | None | PARTIAL (multi-step blocked on implementation) |
| C3 (Sequential assignment) | B17-B22 | 0 | 1 test (sequential indices) | **GAP** (F-001) |
| C4 (Next-link chain) | B23-B28 | 0 | 4 tests (chain links) | **GAP** (F-001) |
| C5 (ReduceStart/ReduceNext body ref) | B29-B31 | 4 tests (field verification, body parity) | None | COVERED |
| C6 (ReduceFinish position) | B32-B34 | 3 tests (id, next, parent next) | None | COVERED |
| C7 (Single-step equivalence) | B35-B38 | 0 | 1 test (equivalence) | **GAP** (F-001) |
| C8 (Nested reduce) | B39-B43 | 0 | 0 (no test written yet) | **GAP** (no test) |
| C9 (Symbolic diagnostics) | B44-B47 | 2 tests (code validity) | None | COVERED |
| C10 (Deterministic lowering) | B48-B49 | 2 tests (idempotent width) | None | PARTIAL (width determinism only, no full IR determinism) |
| C11 (No panic) | B50-B53 | 2 tests (catch_unwind) | None | COVERED |
| C12 (Empty body) | B54-B56 | 1 test (via lower_canonical_aggregate) | 1 test (via emit_reduce_body_steps) | COVERED (both paths) |

**Notable**: C8 (Nested Reduce Semantics) has zero tests — neither active nor commented-out. The test plan defines B39-B43 but no test was written by the test-writer. This is a coverage gap that should be addressed before bead closure.

---

## Verdict

The test suite demonstrates excellent craftsmanship:
- All 48 active tests use concrete, variant-specific assertions with exact value comparisons
- Zero banned patterns (bare `is_ok`, `is_err`, `unwrap`, `expect`, `panic`) in vb-xi2f.24 behavior assertions
- Error paths assert exact `CompileError` variant plus field values (e.g., `UnsupportedStepPrimitive { primitive: "finish" }`)
- Happy paths assert exact numeric values (widths, node counts, StepIdx positions)
- All tests use real production types (`SlotCompiler`, `body_width`, `lower_canonical_aggregate`) — no mocks
- Test names follow `[subject]_[outcome]_when_[condition]` convention
- Per-test DAMP principle (one behavior per test, explicit helper functions)

The three gaps are implementation-dependent:
1. **F-001 (HIGH)**: 7 Phase 3 tests commented out — must be uncommented when `emit_reduce_body_steps` exists
2. **F-003 (MEDIUM)**: Contract C1 scope ambiguity — Collect/Together/Repeat/Choose acceptance tests needed or contract narrowed
3. **C8 coverage gap**: Nested reduce tests (B39-B43) not written

**STATUS: APPROVED WITH FINDINGS** — no lethal test-quality defects. The 7 commented-out Phase 3 tests and C8 gap must be resolved before bead closure, but the existing active suite is clean, well-asserted, and provides strong regression protection for the implemented production code.
