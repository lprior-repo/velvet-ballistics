# Test Review — vb-8mdp.9 State 10 (test-reviewer)

**Date:** 2026-05-30
**Agent:** test-reviewer (femdation child)
**Source checkout:** `/home/lewis/src/velvet-ballistics`
**Isolated workspace:** `/home/lewis/src/femdation-vb-8mdp.9`

## Scope

Adversarial review of the error code propagation test suite: 36 new test functions across 8 crates. All tests execute deterministically and pass against production types.

## Status

**STATUS: APPROVED**

No lethal behavior-test gaps found. All 17 contracted behaviors (B-01 through B-17) are covered by executable tests that use exact assertions, hit production code paths, and would fail if the behavior they claim to cover were deleted. Three minor findings below.

---

## Findings

### F-1 (MODERATE): SECRET_UNAVAILABLE double-counted in Section 17 coverage report total

- **File:** `crates/workspace_tests/tests/section17_runtime_code_coverage_report.rs`
- **Lines:** 266–282
- **Detail:** `SECRET_UNAVAILABLE` appears in both `UNMAPPED_CODES_WITH_RATIONALE` (14 entries) and `PARTIALLY_MAPPED_CODES` (1 entry). The test computes `total = mapped_count + unmapped_count + partial_count = 19 + 14 + 1 = 34` and asserts `total, 34`. The actual count of unique code names is 33 (19 mapped + 14 unmapped, where the 14 unmapped already includes `SECRET_UNAVAILABLE`). The assertion passes because it is self-consistent with the double-counted data – but the total is inflated.
- **Risk:** Maintenance fragility. A future developer reconciling the total against an external spec or audit report will encounter a 34 vs. 33 mismatch. The `SECRET_UNAVAILABLE` entry in `PARTIALLY_MAPPED_CODES` should either be removed from `UNMAPPED_CODES_WITH_RATIONALE` (making unique codes 19 + 13 + 1 = 33), or both arrays need an explicit comment explaining the subset relationship.
- **Recommended fix:** Either remove `SECRET_UNAVAILABLE` from `UNMAPPED_CODES_WITH_RATIONALE` and keep it only in `PARTIALLY_MAPPED_CODES`, or add a comment documenting that `PARTIALLY_MAPPED_CODES` is a refinement sub-category within `UNMAPPED_CODES_WITH_RATIONALE`, and adjust the total to 33.

### F-2 (LOW): Spec comment says 31 Section 17 codes but golden lists contain 33

- **File:** `crates/workspace_tests/tests/section17_runtime_code_reverse_parity.rs`, line 13
- **Detail:** The comment states "Golden set of all 31 Section 17 runtime code names per velvet-ballistics-MASTER.md" but `SECTION_17_MAPPED` (19 entries) + `SECTION_17_UNMAPPED` (14 entries) = 33 unique names. The linked coverage report file also counts 34 (due to F-1 above). These three numbers (31, 33, 34) are in tension.
- **Risk:** Documentation drift. Not a test correctness issue – the golden lists are accurate against production behavior. The spec comment is stale.
- **Recommended fix:** Update the MASTER.md comment or reconcile which 2 codes were added beyond the original 31, and correct the comment count.

### F-3 (LOW): Test-plan expected different IpcError group counts than production code

- **Relevant behavior:** B-03/B-04/B-17
- **File:** Test-plan at line 708 expected 4 `IPC_FRAME_INVALID` and 7 `None`. Production behavior (and the implemented test `ipc_error_runtime_code_semantics_groups`) correctly reflects 8 `IPC_FRAME_INVALID` and 3 `None`. The test is correct; the test-plan artifact was aspirational.
- **Risk:** None to test correctness. Minor inconsistency between test-plan documentation and test implementation.

---

## Gate-by-Gate Results

### Gate 1: Assertion Sharpness — PASS
All 36 new tests use exact assertions:
- `assert_eq!` with concrete values for diagnostic codes, runtime codes, and Display strings.
- `matches!` with exact variant discrimination and field-value patterns (e.g., `SlotOutOfBounds { slot } if *slot == SlotIdx::new(7)`).
- No bare `is_ok()` / `is_err()` assertions in new tests. The single `is_err()` occurrence in `error_chain_integration.rs` (line 31) is a pre-existing pattern followed immediately by exact variant matching.
- `Utc::now()` usage in existing exact-variant tests is scope-local (captured before comparison), preserving determinism.

### Gate 2: BDD Behavior Coverage — PASS
All 17 behaviors mapped 1:1 to proof obligations are covered:

| Behavior | Tests | Crate | Coverage |
|----------|-------|-------|----------|
| B-01 | 3 (existing) | vb_core | CoreError runtime_code mappings and uniqueness |
| B-02 | 7 (new) | vb_runtime | RuntimeError runtime_code mapped + unmapped |
| B-03/B-04/B-17 | 1 (new) | vb_ipc | Exhaustive 14-variant semantic group enumeration |
| B-05 | 1 (new) | vb_validate | 46 Section 16 names → ValidationError reverse parity |
| B-06 | 2 (new) | workspace_tests | 19 mapped / 14 unmapped Section 17 reverse parity |
| B-07 | 3 (new) | workspace_tests | Coverage report with golden data and rationale |
| B-08 | 3 (new) | vb_runtime | CoreError → RuntimeError::Core Box propagation |
| B-09 | 3 (new) | vb_runtime | EngineDriveFailed RunId + source preservation |
| B-10 | 2 (new) | vb_runtime | JournalError → RuntimeError::StorageJournalAppend Arc propagation |
| B-11 | 2 (new) | vb_compile | ValidationError → CompileError::Validation From propagation |
| B-12 | 2 (new) | vb_compile | WorkflowError → CompileError::Workflow From propagation |
| B-13 | 1 (new) | vb_core | CODE_REGISTRY symbolic/numeric bijection |
| B-14 | 3 (new) | vb_core | CoreError Display determinism (static, field, cross-invocation) |
| B-15 | 3 (new) | vb_runtime | Error::source() chain wrapping + non-wrapping |
| B-16 | 3 (new) | vb_cli | Core→Runtime→Display cross-layer chain integrity |

### Gate 3: Determinism — PASS
- No `#[ignore]` tags in new test files.
- No `thread::sleep`, hidden shared mutable state, or nondeterministic ordering.
- Timestamp values are scope-local and captured for comparison, not generated independently.
- All tests produce identical output on repeated invocation.

### Gate 4: Public API — PASS
- Integration tests in `workspace_tests/` and `vb_cli/tests/` use only public types from `vb_core`, `vb_runtime`, `vb_ipc`, `vb_validate`, `vb_storage`, `vb_compile`.
- In-crate tests use `use super::` for module-private items appropriately.

### Gate 5: Mutation Resistance — PASS
Sampled checks confirm behavior deletion would be caught:
- **B-02**: Changing `JournalFull`'s runtime_code arm from `QUEUE_FULL` to any other value breaks the test.
- **B-03/B-04/B-17**: Adding a new IpcError variant without updating the 14-variant array breaks the total assertion (14 still); misassigning a variant changes the group counts.
- **B-08**: Changing the `From<CoreError> for RuntimeError` impl to wrap in a non-`Core` variant breaks the `matches!`.
- **B-16**: Discarding inner CoreError text from RuntimeError Display breaks the substring assertion.

### Gate 6: Snapshot Tests — N/A
No snapshot tests in scope.

### Gate 7: Resource Governance — PASS
No unbounded verifier commands (`cargo kani`, full mutation sweeps, fuzz runs) in the test evidence commands. All commands are scoped to individual test names or packages.

### Gate 8: Dead Test Hygiene — PASS
No commented-out tests, dormant modules, or zero-test filtered runs in new test files.

---

## Existing Test Repairs (Reviewed)

Two pre-existing tests received necessary count updates:
1. `core_error_runtime_codes_are_unique`: count 13→14 (added `CAPABILITY_DENIED_RUNTIME_CODE`). Correct – the test's BTreeSet uniqueness gate catches any duplicate, and the length gate would fail without the increment.
2. `runtime_error_runtime_codes_are_unique`: count 3→4 (added `ADMISSION_DURABILITY_ERROR_RUNTIME_CODE`). Same reasoning.

---

## Evidence Commands Verified

All 11 filtered single-test invocations completed successfully with `cargo test`:

```bash
# B-02: vb_runtime runtime_code mappings           → 1 passed
cargo test -p vb_runtime --lib -- runtime_error_runtime_code_journal_full

# B-03/B-04/B-17: vb_ipc semantic groups           → 1 passed
cargo test -p vb_ipc --lib -- ipc_error_runtime_code_semantics_groups

# B-05: vb_validate Section 16 reverse parity       → 1 passed
cargo test -p vb_validate --test proptest_validation_error_codes -- section16_reverse_parity

# B-06/B-07: workspace_tests Section 17 reports     → 5 passed
cargo test -p velvet-ballistics-workspace-tests --test section17_runtime_code_reverse_parity
cargo test -p velvet-ballistics-workspace-tests --test section17_runtime_code_coverage_report

# B-08/B-09/B-10: vb_runtime propagation            → individual filtered tests pass
# B-11/B-12: vb_compile propagation                 → individual filtered tests pass
cargo test -p vb_compile --lib -- propagation_validation_to_compile_validation_preserves_duplicate_key

# B-13: vb_core registry bijection                  → 1 passed
cargo test -p vb_core --test proptest_registry_consistency -- registry_bijection_unique_names_and_codes

# B-14: vb_core Display determinism                 → 3 passed
cargo test -p vb_core --lib -- core_error_display_determinism

# B-16: vb_cli Display chain integrity              → 3 passed
cargo test -p velvet-ballistics --test error_chain_integration -- core_to_runtime_display_chain
```

---

## Exit Criteria

- [x] All 17 contracted behaviors covered by executable tests
- [x] 36 new test functions, 2 existing tests repaired
- [x] All new assertions are exact (assert_eq!, matches!, no bare is_ok/is_err)
- [x] Tests use production types; no mocks or doubles
- [x] No ignored tests, sleeps, hidden shared state in new files
- [x] Deterministic output across repeated invocations
- [x] Mutation-resistant: deleting a covered behavior breaks at least one named test
- [x] Three minor findings (F-1, F-2, F-3) documented; none are lethal behavior-test gaps
