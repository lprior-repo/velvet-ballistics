# QA Report — vb-i94f: Runtime Taint Propagation

## STATUS: FAIL

---

## Execution Evidence

### Test 1: Integration Taint Propagation Tests

```
cargo test -p vb_core -- integration_taint_propagation
```

**Result:** 54 passed, 1466 filtered out (6 suites, 0.00s)

**Exit Code:** 0 (tests pass)

**Evidence:**
- All 54 taint propagation integration tests passed
- Test suite covers expression evaluation, object field access, list indexing, and finish paths
- No panics or crashes during test execution

---

### Test 2: Clippy Lint Gate

```
cargo clippy -p vb_core --all-targets --all-features -- -D warnings
```

**Result:** 336 errors, 3 warnings

**Exit Code:** Non-zero (clippy blocked)

**Evidence:**
```
  error: unused import: `ResourceContract`
    --> crates/vb_core/src/engine/tests/integration_accessor.rs:12:18
     |
  12 |     PathSegment, ResourceContract, WorkflowParts,
     |                  ^^^^^^^^^^^^^^^^

  error: used `panic!()` or assertion in a function that returns `Result` (150x)
    crates/vb_core/tests/section36_mandatory_coverage.rs:165:1
    ...

  error: `panic` should not be present in production code (60x)
    crates/vb_core/tests/section36_mandatory_coverage.rs:560:18
    ...

  error: used `expect()` on an `Option` value (43x)
    crates/vb_core/src/action.rs:1076:21
    ...

  error: called `ok().expect()` on a `Result` value (43x)
    crates/vb_core/src/action.rs:1076:21
    ...

  error: indexing may panic (35x)
    crates/vb_core/tests/section36_mandatory_coverage.rs:282:16
    ...

  error: using a potentially dangerous silent `as` conversion (15x)
    crates/vb_core/tests/section36_mandatory_coverage.rs:1901:56
    ...
```

---

## Findings

### CRITICAL (block merge)

| Count | Issue | Location |
|-------|-------|----------|
| 150 | `panic!()` in functions returning `Result` | `section36_mandatory_coverage.rs` + action.rs |
| 60 | `panic` in production code | `section36_mandatory_coverage.rs` |
| 43 | `expect()` on Option | `action.rs` |
| 43 | `ok().expect()` on Result | `action.rs` |
| 35 | Indexing that may panic | `section36_mandatory_coverage.rs` |
| 15 | Silent `as` conversions | `section36_mandatory_coverage.rs`, `ops.rs` |
| 8 | `unwrap()` on Result | `section36_mandatory_coverage.rs` |
| 3 | Unused imports in test files | `integration_*.rs` |

### Root Cause
The bead implementation uses `section36_mandatory_coverage.rs` as a test scaffold which violates Engineering Rules (no `panic`, `unwrap`, `expect`). These are pre-existing violations in test files, but clippy `-D warnings` treats them as errors.

---

## Verdict

**Taint propagation functionality: PASS** (54 tests pass, proof works)

**Clippy gate: FAIL** (336 violations block merge)

The taint propagation logic is proven correct by the test suite. However, the clippy gate is blocked by 336 lint violations, primarily in `section36_mandatory_coverage.rs` and `action.rs`. These must be addressed before this bead can merge.

---

## Required Fixes

1. Replace all `panic!()` in Result-returning functions with `Result<T, E>` propagation
2. Replace all `expect()`/`unwrap()` with proper error handling (`?` operator with `?Res`/`?Opt`)
3. Remove unused imports from test files
4. Replace unsafe indexing with `.get()` or explicit bounds checking
5. Fix silent `as` conversions with explicit `TryFrom` or checked casts

---

*Report generated: 2026-05-09*
