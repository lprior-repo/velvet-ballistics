# Test Plan Review: MAJOR-2 — trybuild Silent Pass

## VERDICT: REJECTED

---

## Axis 1 — Contract Parity: **LETHAL**

**Finding**: No `contract.md` is cited or present in the test plan. The skill requires:
> "Every `pub fn` in `contract.md` must have ≥1 BDD scenario in `test-plan.md`"

The test plan at `test-plan-trybuild.md` describes testing `trybuild_compile_fail_tests()`, but there is no `contract.md` that defines this function's preconditions, postconditions, or error variants. Without a contract, there is no way to verify:
- Which error variants exist
- What the exact error types are (e.g., `NoCompileFailFixturesFound` vs generic string)
- What the preconditions for the function are
- Whether the scenarios cover all contract clauses

**Evidence**: The plan references no contract.md. Multiple contract.md files exist in `.beads/` directories, but none are cited in this plan.

**MANDATE**: A `contract.md` must be created or identified that defines `trybuild_compile_fail_tests() -> Result<(), String>` with explicit error variants before this plan can be approved.

---

## Axis 2 — Assertion Sharpness: **LETHAL**

**Finding**: The plan uses `Ok(())` as the sole assertion in multiple scenarios:

| Scenario | Line | Assertion |
|----------|------|-----------|
| single valid fixture | 73 | `Then: The test result is `Ok(())`` |
| multiple compile-fail fixtures — all valid | 115 | `Then: The test result is `Ok(())`` |

**Rationale**: `Ok(())` verifies only success, not that the function produced a specific value. The skill states:
> `is_ok()` → **LETHAL**

While these are "happy path" scenarios where compile-fail fixtures match expected errors, the skill rule is absolute: `is_ok()` as sole assertion is forbidden. The plan must specify what `Ok(())` actually means in context — specifically, what trybuild reported as the expected error confirmation.

**Second Finding**: Error assertions are vague throughout:

| Scenario | Then clause |
|---------|------------|
| empty compile-fail directory | `Err containing a message indicating no compile-fail cases were found` |
| missing compile-fail directory | `Err indicating the fixtures directory cannot be read` |
| non-.rs files only | `Err containing a message indicating no compile-fail cases were found` |
| unexpected error | `Err with details about the mismatch` |
| unexpected pass | `Err indicating the fixture unexpectedly compiled when a failure was expected` |
| missing stderr | `Err indicating the expected error file is missing` |
| one invalid | `Err identifying the failing fixture by path` |

These do not specify **exact error variants** (e.g., `Err(Error::NoFixturesFound { path: ... })`), only descriptive messages. The skill requires:
> Must be: `Err(Error::ExactVariant { field: value })`

**MANDATE**: Every `Err` scenario must name the exact error variant and field values that would be returned by the fixed implementation.

---

## Axis 3 — Trophy Allocation: **LETHAL**

**Finding**: 0 unit tests planned for a function with non-trivial input processing.

The plan acknowledges:
- Proptest invariants: 0
- Fuzz targets: 0
- Kani harnesses: 0

The plan claims "Trophy allocation: 2 integration / 0 unit / 0 e2e / 0 static", but the actual scenario count is **9 integration scenarios** for **2 behaviors** covering **1 function**. This creates a misleading ratio.

More critically, the `trybuild_compile_fail_tests()` function at `crates/vb_codegen/tests/trybuild_tests.rs:14` contains non-trivial logic:
1. Reads directory (`std::fs::read_dir`)
2. Filters by extension
3. Collects files
4. Early-return on empty (`if fixture_files.is_empty() { return Ok(()); }` — THIS IS THE BUG)
5. Iterates and calls `t.compile_fail()`

The early-return bug (step 4) is the entire reason for this test plan. Yet there are **zero unit tests** that verify the empty-directory check behavior in isolation. The plan tests the entire trybuild harness but does not test the conditional logic itself.

**MANDATE**: Add unit tests that verify the empty-directory check logic with a mock or stub of the directory reading, before integration testing the full trybuild harness.

---

## Axis 4 — Boundary Completeness: **MAJOR**

**Finding**: Multiple boundaries not specified for the directory-reading logic.

| Boundary | Specified? |
|----------|------------|
| Minimum valid input | No |
| Maximum valid input | No |
| One-below-minimum (empty dir) | Yes |
| One-above-maximum | No |
| Empty / zero | Yes |
| Overflow potential | No |

The plan does not specify:
- What happens when `compile-fail/` has 10,000 .rs files (overflow/performance)
- What happens when a fixture file path is 10,000 characters long (path too long)
- What happens when the directory is unreadable (permissions)
- What happens when `std::fs::read_dir` returns an I/O error other than NotFound

**MANDATE**: Add boundary scenarios for:
- Directory exists but is unreadable (permission denied)
- Directory has an extremely large number of files (performance boundary)
- Path to fixture exceeds `PATH_MAX` or similar OS limit

---

## Axis 5 — Mutation Survivability: **MAJOR**

**Finding**: The mutation checkpoints describe catching `return Ok(())` in the empty-directory branch, but the plan does not describe what exact assertion would catch it.

The implementation at `crates/vb_codegen/tests/trybuild_tests.rs:26-34`:
```rust
if fixture_files.is_empty() {
    eprintln!("NOTE: No compile-fail fixtures found in {}...");
    return Ok(());  // BUG: should be Err
}
```

The test plan says this should return `Err` with a message. But:
1. The plan does not name the exact error variant
2. The plan does not specify what error message string is expected
3. Therefore, a test could pass with ANY error message containing ANY substring, including `Ok(())` if the implementation were modified to wrap the message differently

**Mutation analysis**:
- If `return Ok(());` were changed to `return Err("No fixtures".into());`, would the test catch it? Yes, if it checks exact error variant.
- If the check `fixture_files.is_empty()` were removed entirely, would the test catch it? No — the test would then call `t.compile_fail()` on an empty vector, which is silently ignored by trybuild.

**MANDATE**: The mutation checkpoint "removing the early return pattern" must have a corresponding test that explicitly verifies an error is returned when the directory is empty and the check is bypassed.

---

## Axis 6 — Evidence Plan Audit: **MINOR**

**Finding**:holzmann-test-rules.md is not referenced in the plan, and Rule 5 (State Your Assumptions) is only partially satisfied.

The Given blocks are present but lack explicit preconditions about:
- Whether temp directories are used and cleaned up
- Whether test isolation is guaranteed across scenarios
- Whether the filesystem state before each scenario is explicitly documented

The plan acknowledges in Open Questions that error type selection is unresolved (question 1: "Should the empty-directory error message be a specific error type?").

**MANDATE**: Resolve Open Questions 1-3 before finalizing the plan. The error type must be defined, not left as an open question.

---

## Summary of Findings

| Severity | Count | Examples |
|----------|-------|---------|
| **LETHAL** | 3 | No contract.md cited; `Ok(())` as sole assertion; zero unit tests for non-trivial logic |
| **MAJOR** | 2 | Vague error assertions without exact variants; mutation checkpoint unclear on exact assertion |
| **MINOR** | 1 | Evidence plan incomplete (Open Questions unresolved) |

---

## MANDATE — Required Before Resubmission

1. **Cite or create `contract.md`** for `trybuild_compile_fail_tests()` defining:
   - Exact error variants (at minimum: `NoFixturesFound`, `DirectoryNotReadable`, `FixtureMismatch`)
   - Preconditions: directory must exist and be readable
   - Postconditions: returns `Ok(())` only when at least one fixture passes

2. **Replace all `Ok(())` assertions** with exact expected values from trybuild output (e.g., verify the trybuild `TestCases` reported N passing fixtures)

3. **Specify exact error variants** for all `Err` scenarios, not just descriptive messages

4. **Add unit tests** for the empty-directory conditional logic before integration testing

5. **Add boundary scenarios** for unreadable directory, large file count, and path length overflow

6. **Resolve all Open Questions** (1-3) before submission
