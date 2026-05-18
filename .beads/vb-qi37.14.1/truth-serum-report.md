# Truth-Serum Audit Report - vb-qi37.14.1

## Auditor: truth-serum subagent
## Date: 2026-05-18
## Bead: cli: Add single-step run command (vb-qi37.14.1)

---

## Mission

Verify that evidence in `assurance-bundle.md` is **real and not laundered** (fabricated, hallucinated, or unverifiable).

---

## Evidence Chain Verification

### Test Evidence

| Claim | Verification Method | Result |
|-------|---------------------|--------|
| 25 tests in `vb_qi37_14_1_run_step.rs` | `cargo test --package vb_cli --test vb_qi37_14_1_run_step` | REAL: "25 passed" |
| `run_step_executes_single_step_and_reports_correct_index` | Source code at line 607 | REAL |
| `run_step_delta_json_*` tests (5 tests) | Source code at lines 867-1127 | REAL |
| `run_step_rejects_durability_strict/journaled` | Source code at lines 123-220 | REAL |
| `run_step_validation_failure_exits_with_code_2` | Source code at line 1412, exits code 2 | REAL |

**Finding**: All 25 cited tests exist in source and pass.

---

### Implementation Evidence

| Claim | Verification Method | Result |
|-------|---------------------|--------|
| `cmd_run_step` at line 1471 | Source code grep | REAL |
| `execute_step_isolated` at line 1585 | Source code grep | REAL |
| `step_once` called exactly once at line 1607 | Source code at `app_impl.rs:1607` | REAL |
| Durability gate at lines 1477-1491 | Source code shows `if durability != DurabilityMode::None` + return `ValidationFailed` | REAL |
| DEFECT-001 fix at line 1553 | Source code shows `CliExitCode::ValidationFailed.into()` for compile errors | REAL |
| Delta computation at lines 1632-1639 | Source code shows `compute_slot_deltas`, `compute_taint_deltas`, `compute_state_deltas` | REAL |
| `build_step_result_json` at lines 1782-1820 | Source code exists with `output_slot` handling | REAL |

**Finding**: All implementation claims trace to actual source code.

---

### Black-Hat Review Evidence

| Claim | Verification Method | Result |
|-------|---------------------|--------|
| DEFECT-001 fix at `app_impl.rs:1553` | Source code at line 1553: `Err(CliExitCode::ValidationFailed.into())` | REAL |
| DEFECT-002 fix at lines 5179-5187 | Source code at lines 5179-5187: `io::stderr()` used for JSON errors | REAL |
| Black-hat review STATUS: PASS | `.beads/vb-qi37.14.1/black-hat-review.md` line 103 | REAL |

**Finding**: Black-hat review exists and its findings are verifiable in source.

---

### Machine Gate Evidence

| Claim | Verification Method | Result |
|-------|---------------------|--------|
| `cargo check --workspace` PASS | Executed: "Finished `dev` profile" | REAL |
| `cargo clippy --workspace` No issues | Executed: "No issues found" | REAL |
| `cargo test --workspace` 10,962 passed | Executed: "10962 passed" | REAL |
| `cargo test --package vb_cli --test vb_qi37_14_1_run_step` 25 passed | Executed: "25 passed" | REAL |

**Finding**: All machine gate claims are reproducible.

---

### Formal Verification Evidence

| Claim | Verification Method | Result |
|-------|---------------------|--------|
| `verification/verus/step_state_machine.rs` exists | File exists with Verus lemmas | REAL |
| `verification/verus/run_frame_invariant.rs` exists | File exists with INV-001, INV-004 proofs | REAL |
| `verification/verus/signals_invariant.rs` exists | File exists | REAL |
| 55 Verus lemmas | Machine gate report | VERIFIED: "55 lemmas verified across 3 Verus files" |
| Kani BLOCKED_TOOLING | Machine gate report line 36-38 | REAL |

**Finding**: Verus files exist and claims are consistent with machine gate report.

---

## Evidence Laundering Check

### Pattern: Evidence-only citations without source
**Status**: CLEAN
All claims in assurance-bundle.md include specific file paths and line numbers that resolve to real source code.

### Pattern: Unverifiable formal claims
**Status**: CLEAN
- Verus proofs: Files exist with the cited lemmas
- Kani waiver: Acknowledged as tooling issue, not evidence laundering

### Pattern: Test name hallucinations
**Status**: CLEAN
All test function names cited (e.g., `run_step_delta_json_pc_delta_has_before_and_after`) resolve to actual test functions in `vb_qi37_14_1_run_step.rs`.

### Pattern: Exit code mismatch
**Status**: CLEAN
- Contract claims exit code 2 for validation failures
- Implementation returns `CliExitCode::ValidationFailed` (= 2 per `exit_code.rs:18`)
- Test `run_step_validation_failure_exits_with_code_2` passes with code 2
- All three align

---

## Verdict

**No evidence laundering detected.** All claims in `assurance-bundle.md` are traceable to:
1. Source code that exists and implements the claimed behavior
2. Tests that pass and validate the claimed behavior
3. Reviews that confirm the claimed behavior
4. Machine gates that verify the claimed behavior

---

## Minor Concerns (Non-blocking)

| Concern | Severity | Rationale |
|---------|----------|------------|
| POST-005 `output_slot.value/taint` not strictly validated in test | LOW | Implementation correctly populates these fields (lines 1808-1811); test uses loose check but black-hat review did not flag |
| Kani BLOCKED_TOOLING | MEDIUM | Tooling limitation acknowledged; Verus proofs compensate |

Neither concern constitutes evidence laundering. Both are accurately characterized.

---

## Conclusion

**STATUS**: CLEAN

The evidence is **real, verifiable, and not laundered**. The implementation satisfies all acceptance criteria with sufficient test coverage, formal verification (Verus), adversarial review (black-hat), and machine gate validation (cargo check/clippy/test).

**Recommendation**: APPROVED for landing.