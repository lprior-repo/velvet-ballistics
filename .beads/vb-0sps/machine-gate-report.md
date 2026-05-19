# Machine Gate Report — vb-0sps State 11 Attempt 2

## Gate Summary

| # | Command | Package/Scope | Expected | Actual | Status |
|---|---------|---------------|----------|--------|--------|
| 1 | `cargo build --workspace` | workspace | build succeeds | `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 0.07s` (0 crates compiled) | **PASS** |
| 2 | `cargo nextest -p vb_codegen` | vb_codegen | tests pass | `374 tests run: 374 passed, 0 skipped` | **PASS** |
| 3 | `cargo nextest -p velvet-ballastics-workspace-tests` | velvet-ballastics-workspace-tests | tests pass | `1210 tests run: 1210 passed, 0 skipped` | **PASS** |
| 4 | `cargo kani -p vb_codegen` | vb_codegen | verification succeeds | `VERIFICATION:- SUCCESSFUL`; `Complete - 5 successfully verified harnesses, 0 failures, 5 total` | **PASS** |

## Detailed Results

### Gate 1: `cargo build --workspace`
- **Exit Code:** 0
- **Build Time:** 0.07s
- **Artifacts:** 0 crates compiled (no stale artifacts)
- **Warnings:** None in output
- **PASS**

### Gate 2: `cargo nextest -p vb_codegen`
- **Exit Code:** 0
- **Test Time:** 2.275s
- **Tests:** 374 passed, 0 skipped
- **Platform:** 3 binaries
- **PASS**

### Gate 3: `cargo nextest -p velvet-ballastics-workspace-tests`
- **Exit Code:** 0
- **Test Time:** 1.524s
- **Tests:** 1210 passed, 0 skipped
- **Platform:** 51 binaries
- **PASS**

### Gate 4: `cargo kani -p vb_codegen`
- **Exit Code:** 0
- **Verification Time:** ~0.08s total across 5 harnesses
- **Harnesses:**
  - `kani_generated_runtime::slot_bounds_model_distinguishes_valid_and_invalid_indices`: 2 checks, all SUCCESS
  - `kani_generated_runtime::invalid_action_resume_preserves_slot_and_journal_model`: 2 checks, all SUCCESS
  - `kani_generated_runtime::journal_capacity_precheck_prevents_overflowing_append`: 4 checks, all SUCCESS
  - `kani_generated_runtime::no_contract_action_rejects_clean_output_from_tainted_input`: 15 checks, all SUCCESS
  - `kani_generated_runtime::join_taint_is_monotonic_for_generated_lattice_model`: 4 checks, all SUCCESS
- **Total Checks:** 27 assertions, 0 failed
- **PASS**

## BDD Test Target (Gate 3 sub-component)

- **Command:** `cargo test -p velvet-ballastics-workspace-tests --test vb_0sps_generated_ir_parity_bdd`
- **Result:** 34 tests passed (1 suite, 0.00s)
- **Status:** PASS

## Combined POST-006 Test (Gates 2+3 sub-component)

- **Command:** `cargo test -p velvet-ballastics-workspace-tests --test vb_0sps_generated_ir_parity_bdd && cargo test -p vb_codegen --all-features`
- **Result:** 34 + 374 = 408 tests passed
- **Status:** PASS

---

**MACHINE GATE STATUS: ALL 4 GATES PASS**

No FAIL_LOCAL, no FAIL_REGRESSION, no DEFERRED_GLOBAL. Bead advances to State 12.