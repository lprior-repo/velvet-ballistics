# Manual QA Final Report: vb-qi37.4.1

## QA Gate: FAIL

### Execution Evidence

**Command:** `cargo nextest run -p vb_storage --test accepted_artifact_red_phase`
```
error: no test target named `accepted_artifact_red_phase` in `vb_storage` package
help: available test targets:
    manual_qa_smoke
    recovery_integration
    vb_h6ix_integration
```

**Command:** `cargo nextest run -p vb_storage` (all tests)
```
error[E0277]: `*mut journal::FjallJournal` cannot be shared between threads safely
   --> crates/vb_storage/src/batch.rs:787:25
    |
787 |         assert_not_sync(&batch);
```

**Command:** `cargo nextest run -p vb_storage --test recovery_integration`
```
    Summary [   0.019s] 16 tests run: 16 passed, 0 skipped
```

**Command:** `cargo nextest run -p vb_storage --test manual_qa_smoke`
```
    Summary [   0.009s] 4 tests run: 4 passed, 0 skipped
```

### Findings

#### CRITICAL: Test target `accepted_artifact_red_phase` not registered

The test file `crates/vb_storage/tests/accepted_artifact_red_phase.rs` exists but is NOT registered as a `[[test]]` target in `crates/vb_storage/Cargo.toml`. Only `recovery_integration` is registered.

```toml
[[test]]
name = "recovery_integration"
```

The test file `accepted_artifact_red_phase.rs` (251 lines, 27 tests) must be added to `Cargo.toml` to be executable.

#### CRITICAL: Pre-existing test compilation error

`crates/vb_storage/src/batch.rs:780-787` contains `assert_not_send` and `assert_not_sync` functions that fail to compile when building lib tests:

```
error[E0277]: `*mut journal::FjallJournal` cannot be shared between threads safely
   --> crates/vb_storage/src/batch.rs:787:25
787 |         assert_not_sync(&batch);
```

This blocks running the full test suite and is unrelated to vb-qi37.4.1 changes.

#### Implementation Correctness (per implementation.md)

The implementation report correctly identifies 17 test failures as TEST DESIGN bugs:
- **Category A** (6 tests): Debug format assertions comparing string output rather than field values
- **Category B** (1 test): Wrong error scenario - test name expects 2-gate proof but `submit_minimal` creates 15-gate artifact
- **Category C** (11 tests): Testing `submit_artifact` but expecting `admit_artifact_run_v1` behavior (different function scope)

These are contract/scope mismatches per Section 3 and Section 5 of `contract.md`.

### Verification

| Check | Status |
|-------|--------|
| Requested test exists as file | EXISTS |
| Requested test is registered | **MISSING** |
| Lib build succeeds | PASS |
| recovery_integration tests | 16/16 PASS |
| manual_qa_smoke tests | 4/4 PASS |
| Full test suite compiles | **FAILS** (pre-existing batch.rs issue) |

### Conclusion

The 17 test failures described in `implementation.md` are documented as TEST DESIGN bugs (wrong function scope per contract Section 3), not implementation bugs. However, **the requested test target `accepted_artifact_red_phase` cannot be executed** because:
1. It is not registered in `Cargo.toml`
2. A pre-existing compilation error in `batch.rs` blocks the full test build

STATUS: **FAIL** — requested test cannot run due to registration gap and pre-existing compilation error.
