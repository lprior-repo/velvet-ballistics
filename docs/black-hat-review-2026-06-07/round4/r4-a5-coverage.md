# Round 4 Agent A5 — Test Density + Coverage Gap (Critical)

**Reviewer:** black-hat-reviewer · **STATUS: REJECTED — SHIP-BLOCKER · 88/100**

The 5x test-density contract is a master-mandated release gate. Actual is 3.99x (≈20% short). Coverage evidence (tarpaulin-report.json, coverage.log, llvm-cov.log) is fabricated 1–3 byte stubs. There is no llvm-cov summary, no enforced gate, and no live coverage number.

## Per-File Density Audit (Section 36)

| File | Functions | Tests | Density | 5x target | Status |
|---|---|---|---|---|---|
| vb_core/src/frame.rs (1,254 LoC) | 27 | 46 | **1.70x** | 5x | FAIL — worst offender |
| vb_compile (master review) | 79 | 316 | 4.00x | 5x | FAIL — self-marked [PASS] in test-suite-review.md:11 |
| vb_core (whole crate) | 143 | 1873 | 13.1x | 5x | PASS |
| vb_expr edge_case_tests.rs | 13 helpers | 78 | 6.0x | 5x | PASS at file level; per-helper mixed |
| Workspace density | — | 16,041 | **3.99x** | 5x | **FAIL** |

## Coverage Evidence Audit

- `tarpaulin-report.json`: 3 bytes (`{}` + newline)
- `xtask/.evidence/vb-test/coverage.log`: 1 line "fixture-backed gate execution; no raw tool output"
- `xtask/.evidence/vb-test/llvm-cov.log`: same 1-line stub
- `../../.evidence/vb-itest-deep-agg/llvm-cov.yaml`: exit_code 101, no number

**The repo's "coverage evidence" is three stubs that themselves admit they contain no data.**

## Q7: Is the 5x Rule Enforced in CI?

**No.** Evidence:
- `.moon/tasks/all.yml:429-449` (coverage task smoke-only, no threshold)
- `scripts/guard-zero-tests.sh:112` checks `applicable_count > 0`, not density
- No `density`/`5x`/`>=5` literal in `.moon/`, `scripts/`, `xtask/src/`
- vb_compile test-suite-review.md:11 self-reports 4.00x and writes `[PASS]`

## Spot-Check Results

| Error Variant | Test Location | Status |
|---|---|---|
| RuntimeError::QueueFull | vb_test_runtime_ipc_resource_behavior.rs:611 | PASS (with exact variant) |
| CoreError::InternalInvariantViolation | frame/tests_and_verification.rs:738-754 | PASS (9+ direct test sites) |
| CoreError::SlotOutOfBounds | frame/tests_and_verification.rs:580-586 | PASS (with slot value match) |

| Helper | Tests | Status |
|---|---|---|
| eval_helper_empty_with_store | 8 | PASS (≥5x) |
| eval_helper_unique_with_store | 4 | **FAIL (below 5x)** |
| eval_helper_contains_with_store | 5 | PASS |
| eval_helper_has_with_store | 4 | **FAIL (below 5x)** |
| eval_helper_append_with_store | 4 | **FAIL (below 5x)** |
| eval_helper_append_if_with_store | 3 | **FAIL (below 5x)** |
| eval_helper_sum_with_store | 6 | PASS |
| eval_helper_merge_with_store | 8 | PASS |

## Top 3 Worst Findings

1. **The repo's "coverage evidence" is three stubs that admit they contain no data.** `tarpaulin-report.json` is `{}` plus newline (3 bytes). The two `coverage.log` and `llvm-cov.log` files are both 1 line: "fixture-backed gate execution; no raw tool output." The `lastRun.json` cache record says the gate ran and `exitCode:0` but stores no percentage. There is no coverage number anywhere in the repository, despite Section 40 mandating a coverage gate in CI.

2. **The 5x density contract is comment-only and has been violated by 20% for at least one full review cycle.** No moon task, no script, no xtask command, and no cargo alias enforces 5x. `guard-zero-tests.sh` checks `>0` tests, not density. The `coverage` moon task is a 1-test smoke that admits it is not a coverage gate. `vb_compile/test-suite-review.md:11` self-reports 4.00x and writes `[PASS]`, laundering a contract violation as a green check. `vb_core/src/frame.rs` is at 1.70x (46/27) on the file Section 36 explicitly demands full coverage of.

3. **Branch coverage is broken in the project's toolchain and Section 36 requires it.** `vb_ipc/test-suite-review.md:20` documents `cargo-llvm-cov reported 0 branches for all files`. The master requires statement + branch + path coverage on all hot paths. 0 of 3 sub-requirements of Section 36 are demonstrably enforced.

## Required Repair Actions

1. **CRITICAL**: Delete `tarpaulin-report.json` (3-byte stub) and replace with a real `llvm-cov-summary.json` produced by a full run. Persist raw `llvm-cov.log` and the JSON.
2. **CRITICAL**: Add a `test-density` moon task that runs a new `scripts/check-test-density.sh` script. Asserts total `#[test] / pub fn >= 5.0` (or per-crate as documented). Fails moon CI on any crate below 5.0x.
3. **CRITICAL**: Edit `crates/vb_compile/test-suite-review.md:11` from `[PASS]` to `[FAIL]`.
4. **HIGH**: Add ≥ 89 `#[test]` to `crates/vb_core/src/frame/tests_and_verification.rs` to reach 5.0x density on the 27 `pub fn` of `frame.rs`.
5. **HIGH**: Investigate and fix why `cargo-llvm-cov` reports 0 branches for all files.
6. **MEDIUM**: Fix the documentation in `crates/vb_expr/src/helpers/tests/edge_case_tests.rs:3-7` — there are 13 helpers, not 12. Add tests for `eval_unique`, `eval_has`, `eval_append`, `eval_append_if` to bring per-helper density to ≥ 5.0x.
7. **PROCESS**: Reject any future bead submission whose `../../.evidence/<bead>/coverage*` file is a stub.

## Verdict: SHIP-BLOCKER

Accepting this PR as ACCEPTABLE-AS-DEBT would set the precedent that 3-byte stubs satisfy 5x contracts, that [PASS] badges can override arithmetic, and that "fixture-backed gate execution; no raw tool output" is a CI log line.
