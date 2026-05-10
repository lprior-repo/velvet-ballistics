# QA Report - Bead vb-7gs9

**Bead:** vb-7gs9 (Shard scheduler bounded ownership evidence)
**Date:** 2026-05-09
**Workspace:** /home/lewis/src/Velvet-ballistics
**Current State:** 9 (QA Gate)

---

## Executive Summary

| Gate | Command | Exit Code | Result |
|------|---------|-----------|--------|
| cargo test --lib | `cargo test -p vb_core -p vb_runtime -p vb_storage --lib` | 0 | **PASS** |
| moon :quick | `moon run :quick` | 0 | **PASS** |
| moon :test | `moon run :test` | 100 | **FAIL** |

**Overall QA Status:** BLOCKED

---

## Detailed Results

### 1. Cargo Tests (vb_core, vb_runtime, vb_storage --lib) — PASS

```
$ cargo test -p vb_core -p vb_runtime -p vb_storage --lib
warning: `vb_core` (lib test) generated 16 warnings (unused imports)
warning: `vb_runtime` (lib test) generated 1 warning (unused import)
warning: `vb_storage` (lib test) generated 2 warnings
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.06s
     Running unittests src/lib.rs (target/debug/deps/vb_core-a81409c4b0b0cbf2)
     Running unittests src/lib.rs (target/debug/deps/vb_runtime-1367db8be4f519af)
     Running unittests src/lib.rs (target/debug/deps/vb_storage-4f45aac87f26ba5b)
cargo test: 3582 passed (3 suites, 0.84s)
```

**Exit Code:** 0
**Result:** PASS

### 2. Moon :quick — PASS

```
$ moon run :quick
▮▮▮▮ velvet-ballastics:quick (f3bb6e66)
Hello, world!
Hello, world!
Hello, world!
Hello, world!
▮▮▮▮ velvet-ballastics:quick (7ms, f3bb6e66)
Tasks: 1 completed
Time: 24s 114ms
```

**Exit Code:** 0
**Result:** PASS

### 3. Moon :test — FAIL

```
$ moon run :test
▮▮▮▮ velvet-ballastics:test (16s 822ms, ba392ee4)
     Summary [  16.531s] 9254/10777 tests run: 9253 passed, 1 failed, 0 skipped
velvet-ballastics:test |         FAIL [   0.005s] ( 9223/10777) vb_validate gate_08_accessor::tests::proptest_gate_08_reports_first_invalid_accessor_with_root_precedence
Error: task_runner::run_failed
  × Task velvet-ballastics:test failed to run.
  ╰─▶ Process set failed: exit code 100
```

**Exit Code:** 100
**Result:** FAIL

---

## Failing Test Analysis

### Test: `vb_validate gate_08_accessor::tests::proptest_gate_08_reports_first_invalid_accessor_with_root_precedence`

**Location:** `crates/vb_validate/src/gate_08_accessor.rs:485`

**Failure Details:**
```
minimal failing input: slot_count = 2, root = 0
left: Err(AccessorPathInvalid { accessor_index: 0, segment_index: 0 })
right: Ok(())
```

**Analysis:**

The test validates accessor path segments. With `slot_count = 2` and `root = 0`:
- Expected: `Ok(())` because `root (0) < slot_count (2)`
- Actual: `Err(AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`

This indicates a bug in `validate_gate_08_accessor_path_segments` where it incorrectly reports an accessor path as invalid when `root < slot_count`.

**Not in vb-7gs9 scope:** This bug is in `vb_validate` (accessor validation), not in `vb_runtime` (shard scheduler). The bead vb-7gs9 is about shard scheduler bounded ownership evidence, which is unrelated to accessor validation.

**Impact:** Blocks the full test suite from passing.

---

## Findings

### CRITICAL (block merge)

1. **Proptest failure in accessor validation**
   - **File:** `crates/vb_validate/src/gate_08_accessor.rs:485`
   - **Command:** `moon run :test`
   - **Actual:** Exit code 100, 1 test failed
   - **Expected:** Exit code 0, all tests pass
   - **Reproduction:** `cargo test -p vb_validate proptest_gate_08_reports_first_invalid_accessor_with_root_precedence`
   - **Minimal failing input:** `slot_count = 2, root = 0`
   - **Severity:** CRITICAL — blocks CI gate

### OBSERVATION

1. **Unused imports warnings**
   - File: `crates/vb_core/src/engine/tests/integration_*.rs`
   - Count: 16 warnings in vb_core tests
   - **Not blocking** — warnings don't fail the build

---

## Artifact Verification

| Artifact | Status |
|----------|--------|
| contract.md | EXISTS (18.3K) |
| test-plan.md | EXISTS (31.6K) |
| test-plan-review.md | EXISTS (5.1K) — APPROVED |
| moon-report.md | EXISTS |
| qa-report.md | THIS FILE |
| qa-review.md | TO BE WRITTEN |

---

## Conclusion

The QA gate is **BLOCKED** by a pre-existing proptest failure in `vb_validate`. The shard scheduler implementation (vb-7gs9 scope) passes all cargo tests. The failure is in accessor validation code unrelated to vb-7gs9, but it prevents the full test suite from passing.

**Recommendation:** File a bead for the accessor validation bug before this can be merged.
