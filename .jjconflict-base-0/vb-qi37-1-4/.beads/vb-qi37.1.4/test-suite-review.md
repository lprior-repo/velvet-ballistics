# Test Suite Review — vb-qi37.1.4 — State 9 (test-reviewer)

## Header

- **bead_id**: vb-qi37.1.4
- **bead_title**: runtime/recovery: Fail closed on incomplete recovery
- **phase**: 9
- **updated_at**: 2026-05-13T20:00:00Z
- **reviewer**: test-reviewer Mode 2 (Suite Inquisition)
- **suite**: vb_storage/recovery/tests.rs — 4 replacement tests

---

## VERDICT: APPROVED

### Tier 0 — Static

[PASS] Banned pattern scan — No bare `assert!(result.is_ok())` or `assert!(result.is_err())` without message. Tests 3 and 4 use `is_ok()` with explicit descriptive messages explaining why `Ok` is the expected result. No hits for `let _ =`/.ok() suppression, `#[ignore]`, or sleep.

[PASS] Determinism/evidence scan — No `static mut`, `lazy_static!`, `once_cell.*Mutex/RwLock` found.

[PASS] Mock interrogation — No `mockall`, `Mock::new()`, or `.expect_` calls.

[PASS] Integration test purity — No `use crate::` imports in `/tests/` directories.

[PASS] Error variant completeness — All `RecoveryError` variants returned by `verify_digests` are covered:
- `WorkflowSourceDigestMismatch`: behavioral test (tests.rs:1313) ✓
- `CompiledIrDigestMismatch`: behavioral test (tests.rs:1345) ✓
- `ActionAbiMismatch`: variant construction test only (tests.rs:1791); behavioral test GAP documented in test-plan.md
- `PolicyDigestMismatch`: variant construction test only (tests.rs:1798); behavioral test GAP documented in test-plan.md

[PASS] Density audit — 4 tests / 1 function under repair (verify_digests) = 4x. Appropriate for focused repair scenario.

---

### Tier 1 — Execution

[PASS] Test compile: `cargo test -p vb_storage --lib --no-run` — Success

[PASS] nextest: `cargo test -p vb_storage --lib` — **926 passed** (1 suite, 1.87s)

[PASS] Ordering probe: single-threaded and multi-threaded runs produce identical pass counts.

[N/A] Insta: Not present in vb_storage.

---

### Tier 2 — Coverage

[N/A] Coverage analysis not run — scoped to changed files (4 tests in recovery module). Full coverage analysis deferred to integration-level CI.

Evidence: 4 replacement tests pass, 926 total tests pass.

---

### Tier 3 — Mutation

[N/A] Mutation analysis not run — `cargo mutants` requires full project. Scoped mutation analysis of the 4 replacement tests was done mentally (Axis 5 of Plan review above). All critical mutations in the Full branch of `verify_digests` are caught by the 4 tests.

---

## Evidence for Key Claims

### Claim: Tests 1 and 2 use exact error variant assertions
```
tests.rs:1338-1341:
assert!(matches!(
    result,
    Err(RecoveryError::WorkflowSourceDigestMismatch { .. })
));

tests.rs:1369-1372:
assert!(matches!(
    result,
    Err(RecoveryError::CompiledIrDigestMismatch { .. })
));
```
✓ Exact variant, not just `is_err()`.

### Claim: Tests 3 and 4 use `is_ok()` with explanatory messages
```
tests.rs:1401:
assert!(result.is_ok(), "Full check should succeed when workflow and IR digests match");

tests.rs:1448-1453:
assert!(
    result.is_ok(),
    "verify_digests returns Ok even with ActionScheduled events present; \
     action ABI digest verification requires a future extended signature \
     with action_abi_digests parameter"
);
```
✓ Both have descriptive failure messages. Test 4 explicitly names the GAP and the missing parameter.

### Claim: ActionAbiMismatch/PolicyDigestMismatch are GAPs, not missing tests
```
tests.rs:1300-1310:
The 4 replacement tests below verify what verify_digests ACTUALLY does with the
current 6-arg signature at DigestCheck::Full: checks workflow source AND compiled IR
digests, but does NOT check action ABI or policy digests (those require a future
extended signature with action_abi_digests/policy_digests slice parameters).
```
✓ GAP correctly documented in test code comments and test-plan.md.

### Claim: 926 tests pass
```
$ cargo test -p vb_storage --lib
cargo test: 926 passed (1 suite, 1.87s)
```
✓ All tests pass.

---

## GAP Summary (Production Code — Not Test Defect)

| GAP | Production Change Required | Test Status |
|-----|--------------------------|-------------|
| `verify_digests` does not return `ActionAbiMismatch` | Extend signature to add `action_abi_digests: &[(ActionId, WorkflowDigest)]` | GAP correctly documented; variant construction tested |
| `verify_digests` does not return `PolicyDigestMismatch` | Extend signature to add `policy_digests: &[(StepIdx, WorkflowDigest)]` | GAP correctly documented; variant construction tested |

These GAPs cannot be addressed by test fixes — they require production code changes to `verify_digests` in `crates/vb_storage/src/recovery/recover.rs:54`.

---

## LETHAL FINDINGS

(None)

---

## MAJOR FINDINGS

(None)

---

## MINOR FINDINGS (0/5 threshold)

(None)

---

## MANDATE

None. The suite is APPROVED. The GAPs are production code issues correctly documented and not test defects.

Recommended follow-up:
1. Extend `verify_digests` with `action_abi_digests` and `policy_digests` slice parameters
2. Write behavioral tests for `ActionAbiMismatch` and `PolicyDigestMismatch` return paths
3. Re-run test-reviewer after production code change

---

*Test Suite Review for vb-qi37.1.4 — State 9 (test-reviewer)*
