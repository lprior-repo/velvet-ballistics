# QA Report — vb-2i0t: quality: Atomize xtask Section 77 command-center gates

**Date:** 2026-05-09
**State:** 7 — Manual QA Smoke Test
**Workspace:** /home/lewis/src/Velvet-ballistics/vb-2i0t (bead workspace; xtask is at /home/lewis/src/Velvet-ballistics/xtask)

---

## Execution Evidence

### Test 1 — `cargo build -p xtask`

```
$ rtk cargo build -p xtask 2>&1 | tail -20
warning: `xtask` (bin "xtask") generated 24 warnings (run `cargo fix --bin "xtask" -p xtask` to apply 3 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.83s
cargo build (0 crates compiled)
```

**Result:** BUILD PASS (warnings only, no errors)

---

### Test 2 — `cargo clippy -p xtask --all-targets --all-features -- -D warnings`

```
$ rtk cargo clippy -p xtask --all-targets --all-features -- -D warnings 2>&1 | tail -10
[full output: ~/.local/share/rtk/tee/1778363015_cargo_clippy.log]

cargo clippy: 65 errors, 3 warnings
```

Selected errors by category:

| Category | Count | Examples |
|---|---|---|
| `clippy::expect_used` | 5+ | `xtask/tests/integration_gates.rs:16`, `:131`, `:259`, `:296`, `:326` |
| `clippy::unwrap_used` | 6+ | `xtask/src/evidence.rs:687`, `:714`, `:715`, `:687`, `:714`, `:715` |
| `clippy::panic` | 2 | `xtask/src/evidence.rs:1023`, `:1283` |
| `clippy::string_slice` | 1 | `xtask/tests/integration_gates.rs:481` |
| `clippy::let_underscore_must_use` | 9+ | `xtask/tests/integration_gates.rs:22`, `xtask/src/evidence.rs:329`, `:330` |
| `unused_variables` | 4+ | `xtask/src/evidence.rs:258`, `:313`, `:339` |
| `dead_code` (unused fns) | 16+ | `run_check_gate`, `run_clippy_gate`, `run_nextest_gate`, `run_forbidden_scan_gate`, `run_hotpath_scan_gate`, `run_miri_gate`, `run_mutants_gate`, `run_llvm_cov_gate`, `run_fuzz_build_gate`, `run_fuzz_smoke_gate`, `run_coverage_gate`, `run_mutants_smoke_gate`, `run_bench_build_gate`, `run_feature_powerset_gate`, `run_source_length_gate`, `run_maxperf_gate` |
| `slicing may panic` | 2 | `xtask/src/evidence.rs:309`, `:1220` |
| `indexing may panic` | 8+ | `xtask/src/evidence.rs:308`, `:314`, `:332` |

**Result:** BUILD FAIL — 65 clippy errors, 3 warnings

---

### Test 3 — `cargo test -p xtask`

```
$ rtk cargo test -p xtask 2>&1 | head -50
[warnings about unused variables and dead code]
[bash_metadata]: bash tool terminated command after exceeding timeout 120000 ms
```

**Result:** TIMEOUT — test did not complete within 120s

---

## Phase 1 — Discovery

| Check | Result |
|---|---|
| Binary builds | PASS (with warnings) |
| Source structure | xtask/src/ has evidence.rs, gates.rs + 5 others |

## Phase 2 — Happy Path

| Check | Result |
|---|---|
| Build completes without error | PASS |
| Evidence types defined | PASS (GateEvidence, WhyFailed, GateStatus in evidence.rs) |

## Phase 3 — Hostile Interrogation

| Check | Result |
|---|---|
| clippy pass with `-D warnings` | FAIL — 65 errors |
| test suite completes | TIMEOUT |
| No `panic!` in production | FAIL — 2 panic! calls in evidence.rs:1023, :1283 |
| No `unwrap()` in production | FAIL — 6+ unwrap() in evidence.rs |
| No `expect()` in production | FAIL — 5+ expect() in tests; `#[allow]` not applied |
| No unchecked indexing | FAIL — 8+ indexing may panic in evidence.rs |
| Dead code < threshold | FAIL — 16+ unused functions in gates.rs |

## Findings

### CRITICAL (block merge)

1. **clippy::expect_used / unwrap_used / panic in production code**
   - File: `xtask/src/evidence.rs` lines 687, 714, 715, 1023, 1283
   - Violates zero-unwrap/expect/panic engineering rule
   - Commands like `cargo clippy -p xtask --all-targets --all-features -- -D warnings` emit 65 errors

2. **clippy failures in test file**
   - File: `xtask/tests/integration_gates.rs` lines 16, 22, 131, 259, 296, 326, 365, 385, 451, 481
   - `expect()` on Result, `let _ =` on must_use, unused variables, string slice indexing

3. **16+ unused gate runner functions**
   - File: `xtask/src/gates.rs`
   - `run_check_gate`, `run_clippy_gate`, `run_nextest_gate`, `run_forbidden_scan_gate`, `run_hotpath_scan_gate`, `run_miri_gate`, `run_mutants_gate`, `run_llvm_cov_gate`, `run_fuzz_build_gate`, `run_fuzz_smoke_gate`, `run_coverage_gate`, `run_mutants_smoke_gate`, `run_bench_build_gate`, `run_feature_powerset_gate`, `run_source_length_gate`, `run_maxperf_gate`
   - These appear to be stubs that were never connected to the command-center dispatch

4. **Evidence YAML serialization — NOT VERIFIED**
   - The `GateEvidence` struct is defined correctly (kind, gate_name, command, exit_code, log, status, why_failed)
   - No test confirmed that running a gate actually produces a `.evidence/*.yaml` file

### MAJOR

5. **Test suite timeout** — `cargo test -p xtask` did not complete within 120s; likely hanging on integration gate tests

---

## VERDICT: **FAIL**

**Summary:** The xtask crate does not pass `cargo clippy -D warnings`. The atomization work in Section 77 defined the evidence types but did not connect them to a working command-center dispatch, and significant engineering rule violations remain in the evidence module. The dead code in `gates.rs` suggests partial implementation of gate runners that were never wired up.

**Recommendation:** Fix all clippy errors (unwrap/expect/panic/indexing), connect the 16 stub gate runners or remove them, add a smoke test that verifies evidence YAML is actually written to `.evidence/`, and ensure the test suite completes in finite time.
