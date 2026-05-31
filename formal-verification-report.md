# Formal Verification Report

**Bead:** vb-y9d3v  
**State:** 12 (formal-verifier)  
**Schema:** formal-verification-report/v1  
**Generated:** 2026-05-30T18:54:31Z  
**Workdir:** /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-y9d3v  
**Agent:** formal-verifier (femdation delegate)

---

## Executive Summary

| Category | Count | Status |
|----------|-------|--------|
| Total obligations planned | 41 | — |
| PASS | 20 | proptest (10) + Flux (10) |
| FAIL_LOCAL | 21 | Kani (10) + Verus (10) + Fuzz (1) |
| FAIL_REGRESSION | 0 | — |
| FAIL_GLOBAL | 0 | — |
| WAIVED | 0 | No waivers approved |
| BLOCKED_TOOLING | 0 | All tools available but some artifacts fail |

**Verdict:** 20 PASS / 21 FAIL_LOCAL. Cannot validate state 12 as fully passing. Proptest and Flux-rs refinements pass. Kani verification timed out exploring dependencies. Verus proofs contain type errors. Fuzz target not registered.

**Workspace test baseline:** 12,793 tests pass, 27 ignored — confirms state 11 implementation claim.

---

## Tooling Availability

| Tool | Path | Status |
|------|------|--------|
| `verus` | `/home/lewis/.local/bin/verus` | AVAILABLE (v0.2026.05.05) |
| `cargo-flux` | `/home/lewis/.cargo/bin/cargo-flux` | AVAILABLE |
| `kani` | `cargo kani` (plugin) | AVAILABLE (v0.67.0) |
| `cargo-fuzz` | `cargo fuzz` (plugin) | AVAILABLE (v0.13.1) |
| `cargo` | `/home/lewis/.cargo/bin/cargo` | AVAILABLE (nightly-2026-04-28) |
| `proptest` | (Rust lib within vb_runtime) | AVAILABLE (via cargo test) |

All required tools are available. No tooling blockers.

---

## Per-Verifier Execution Results

### 1. Proptest (verifier: proptest)

**Command:** `cargo test -p vb_runtime -- proptest_attempt_fence --nocapture`  
**Exit status:** 0  
**Evidence:** 14 tests passed, 0 failed  
**Obligations covered:** PO-vb-y9d3v-0004, -0008, -0012, -0016, -0020, -0024, -0028, -0032, -0036, -0040

All 10 proptest obligations PASS. The properties test stale/current/future attempt combinations, retry capacity bounds, terminal run rejection, missing run errors, and attempt freshness across the full u16 range.

### 2. Flux-rs (verifier: flux-rs)

**Command:** `bash scripts/flux-check-package.sh vb_runtime`  
**Exit status:** 0  
**Evidence:** Compiled to `flux` profile with zero errors. One non-blocking warning about `cfg(verus)` condition name.  
**Obligations covered:** PO-vb-y9d3v-0003, -0007, -0011, -0015, -0019, -0023, -0027, -0031, -0035, -0039

All 10 Flux-rs obligations PASS. The `#[extern_spec]` refinements on `ActionTicket`, `RuntimeError`, and `RetryPolicy` compile and wire correctly into `vb_runtime`. No refinement violations detected at the Flux type-check level.

### 3. Kani (verifier: kani)

**Command (per PO):** `bash scripts/kani-list.sh vb_runtime`  
**Exit status:** 0 (list succeeded)  
**Evidence:** 13 harnesses discovered in `kani_attempt_fence_harnesses.rs`. All compile.

**Command (actual verification per RRO):** `cargo kani -p vb_runtime`  
**Exit status:** TIMEOUT (600s)  
**Evidence:** Verification timed out exploring `memcmp` loops in the `fjall` LSM-tree dependency. The harnesses were never reached. ~5900 unwind iterations of `builtin-library-memcmp` before timeout.

**Obligations covered:** PO-vb-y9d3v-0001, -0005, -0009, -0013, -0017, -0021, -0025, -0029, -0033, -0037

All 10 Kani obligations FAIL_LOCAL. The harnesses compile and are listed correctly, but the full verification cannot complete within practical bounds due to unbounded dependency exploration. The `kani-list.sh` command (as specified in the PO) succeeds, but that only lists harnesses — it does not verify them. The RRO evidence command (`cargo kani -p vb_runtime`) timed out.

**Root cause:** Kani explores the entire dependency graph including `fjall` LSM-tree storage code. The memcmp loops in the storage layer consume all verification budget before reaching the attempt-fence harnesses. Mitigation would require `#[kani::stub]` annotations or `--harness` filtering to scope verification to specific proof functions.

### 4. Verus (verifier: verus)

**Command (per PO):** `bash scripts/verify-verus.sh --target vb-y9d3v-action-fence`  
**Exit status:** 0 (script ran, but target not in registry)  
**Evidence:** Script ran 5 registry targets (taint_lattice, step_state_machine, step_budget, resource_budget, vb_jpq724_events_for_run_production) — all PASS. The `--target vb-y9d3v-action-fence` flag was ignored by the script; the target is not registered in `contracts/proof_obligations.yaml`.

**Direct verification:** `verus --crate-type=lib crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs`  
**Exit status:** FAIL (compilation errors)  
**Evidence:** 3 type inference errors:
- Line 86: `Err(AttemptFenceError::StaleAttempt { incoming, current })` — cannot infer `Result<T, AttemptFenceError>` type parameter `T`
- Line 100: Same error for `AttemptBeyondMax`
- Line 247: Same error for `AttemptBeyondMax`

**Obligations covered:** PO-vb-y9d3v-0002, -0006, -0010, -0014, -0018, -0022, -0026, -0030, -0034, -0038

All 10 Verus obligations FAIL_LOCAL. The `vb_y9d3v_action_fence.rs` artifact exists and uses correct Verus syntax patterns, but contains type errors that prevent verification. The script-based execution path (registry-driven) does not cover this target. Direct verification fails.

### 5. Cargo-fuzz (verifier: cargo-fuzz)

**Command:** `cargo fuzz run fuzz_retry_codec -- -max_len=64 -runs=100000`  
**Exit status:** FAIL  
**Evidence:** `fuzz_retry_codec` is not registered as a `[[bin]]` target in `fuzz/Cargo.toml`. The source file exists at `fuzz/fuzz_targets/fuzz_retry_codec.rs` but the fuzz manifest does not declare it.

**Obligations covered:** PO-vb-y9d3v-0041

FAIL_LOCAL. Artifact exists but cannot be executed — not wired into the build system.

### 6. Workspace Test Baseline

**Command:** `cargo test --workspace -- --quiet`  
**Exit status:** 0  
**Evidence:** 12,793 tests passed, 27 ignored (229 suites, 18.69s)

Confirms state 11 implementation claim.

---

## Obligation Status Summary

| Obligation ID | Verifier | Status | Detail |
|---------------|----------|--------|--------|
| PO-vb-y9d3v-0001 | kani | FAIL_LOCAL | harness compiles, verification timed out |
| PO-vb-y9d3v-0002 | verus | FAIL_LOCAL | type errors in proof artifact |
| PO-vb-y9d3v-0003 | flux-rs | PASS | flux profile compiles cleanly |
| PO-vb-y9d3v-0004 | proptest | PASS | 14 tests pass |
| PO-vb-y9d3v-0005 | kani | FAIL_LOCAL | harness compiles, verification timed out |
| PO-vb-y9d3v-0006 | verus | FAIL_LOCAL | type errors in proof artifact |
| PO-vb-y9d3v-0007 | flux-rs | PASS | flux profile compiles cleanly |
| PO-vb-y9d3v-0008 | proptest | PASS | 14 tests pass |
| PO-vb-y9d3v-0009 | kani | FAIL_LOCAL | harness compiles, verification timed out |
| PO-vb-y9d3v-0010 | verus | FAIL_LOCAL | type errors in proof artifact |
| PO-vb-y9d3v-0011 | flux-rs | PASS | flux profile compiles cleanly |
| PO-vb-y9d3v-0012 | proptest | PASS | 14 tests pass |
| PO-vb-y9d3v-0013 | kani | FAIL_LOCAL | harness compiles, verification timed out |
| PO-vb-y9d3v-0014 | verus | FAIL_LOCAL | type errors in proof artifact |
| PO-vb-y9d3v-0015 | flux-rs | PASS | flux profile compiles cleanly |
| PO-vb-y9d3v-0016 | proptest | PASS | 14 tests pass |
| PO-vb-y9d3v-0017 | kani | FAIL_LOCAL | harness compiles, verification timed out |
| PO-vb-y9d3v-0018 | verus | FAIL_LOCAL | type errors in proof artifact |
| PO-vb-y9d3v-0019 | flux-rs | PASS | flux profile compiles cleanly |
| PO-vb-y9d3v-0020 | proptest | PASS | 14 tests pass |
| PO-vb-y9d3v-0021 | kani | FAIL_LOCAL | harness compiles, verification timed out |
| PO-vb-y9d3v-0022 | verus | FAIL_LOCAL | type errors in proof artifact |
| PO-vb-y9d3v-0023 | flux-rs | PASS | flux profile compiles cleanly |
| PO-vb-y9d3v-0024 | proptest | PASS | 14 tests pass |
| PO-vb-y9d3v-0025 | kani | FAIL_LOCAL | harness compiles, verification timed out |
| PO-vb-y9d3v-0026 | verus | FAIL_LOCAL | type errors in proof artifact |
| PO-vb-y9d3v-0027 | flux-rs | PASS | flux profile compiles cleanly |
| PO-vb-y9d3v-0028 | proptest | PASS | 14 tests pass |
| PO-vb-y9d3v-0029 | kani | FAIL_LOCAL | harness compiles, verification timed out |
| PO-vb-y9d3v-0030 | verus | FAIL_LOCAL | type errors in proof artifact |
| PO-vb-y9d3v-0031 | flux-rs | PASS | flux profile compiles cleanly |
| PO-vb-y9d3v-0032 | proptest | PASS | 14 tests pass |
| PO-vb-y9d3v-0033 | kani | FAIL_LOCAL | harness compiles, verification timed out |
| PO-vb-y9d3v-0034 | verus | FAIL_LOCAL | type errors in proof artifact |
| PO-vb-y9d3v-0035 | flux-rs | PASS | flux profile compiles cleanly |
| PO-vb-y9d3v-0036 | proptest | PASS | 14 tests pass |
| PO-vb-y9d3v-0037 | kani | FAIL_LOCAL | harness compiles, verification timed out |
| PO-vb-y9d3v-0038 | verus | FAIL_LOCAL | type errors in proof artifact |
| PO-vb-y9d3v-0039 | flux-rs | PASS | flux profile compiles cleanly |
| PO-vb-y9d3v-0040 | proptest | PASS | 14 tests pass |
| PO-vb-y9d3v-0041 | cargo-fuzz | FAIL_LOCAL | target not registered in fuzz/Cargo.toml |

---

## Trusted-Base Dispositions

| TBP ID | Kind | Status | Resolution |
|--------|------|--------|------------|
| TBP-009 | verus external_body | TRUSTED (pending) | Validated: Verus specs use external_body. Production binding gap acknowledged. |
| TBP-010 | flux extern_spec | TRUSTED (pending) | Validated: Flux extern_spec refinements compile. No violation detected. |
| TBP-011 | kani bounds/assume | TRUSTED (verified) | Validated: Kani harnesses use assume for u16 bounds. Proptest covers full range. |
| TBP-012 | fuzz scaffold | TRUSTED (non-behavior) | Validated: Non-behavior-affecting. Fuzz target uses inline scaffolding. |
| TBP-013 | spec/impl gap | TRUSTED (pending) | Validated: Gap documented for future-attempt behavior. Production fix pending. |
| TBP-014 | verus blocked-tooling | RESCINDED | Tooling IS available. Verus binary found. Failure is artifact quality, not tooling. |
| TBP-015 | flux blocked-tooling | RESCINDED | Tooling IS available. cargo-flux found. Flux compilation succeeds. |

---

## Waiver Analysis

No waivers exist in `waiver-candidates.jsonl`. The single entry (`WC-vb-y9d3v-none`) states "No non-behavior exceptions identified". This is correct — all 41 obligations are behavior-affecting and no waiver is warranted. No `formal-waivers.jsonl` exists (not needed).

---

## RRO Mapping Status

All 41 RRO rows in `rust-refinement-obligations.jsonl` have `mapping_status: planned`. Source refs, behavior test refs, and evidence artifact refs all point to real files verified present in the workspace.

---

## Raw Evidence Locations

| Verifier | Evidence Path |
|----------|--------------|
| proptest | stdout: `cargo test -p vb_runtime -- proptest_attempt_fence` |
| flux-rs | stdout: `bash scripts/flux-check-package.sh vb_runtime` |
| kani | `.evidence/kani-list/vb_runtime.json` |
| verus | `.evidence/verus/` (5 registry targets, all PASS) |
| verus (vb-y9d3v) | stdout: `verus --crate-type=lib .../vb_y9d3v_action_fence.rs` (FAIL) |
| cargo test | stdout: 12,793 passed |

---

## Final Classification

| Classification | Count | Detail |
|----------------|-------|--------|
| **PASS** | **20** | proptest (10) + Flux-rs (10) |
| **FAIL_LOCAL** | **21** | Kani (10) timeout + Verus (10) type errors + Fuzz (1) unregistered |
| FAIL_REGRESSION | 0 | — |
| FAIL_GLOBAL | 0 | — |
| WAIVED | 0 | No waivers |

**State 12 cannot be validated as fully passing.** 20 of 41 obligations pass. 21 fail locally due to:
1. Verus type inference errors in `vb_y9d3v_action_fence.rs` (3 E0282 errors)
2. Kani verification timeout due to dependency graph exploration
3. Fuzz target not registered in `fuzz/Cargo.toml`

Both proptest and Flux-rs proofs provide strong evidence for the behavioral contracts. The Kani harnesses exist and compile but need scoping (stubs or `--harness`) to avoid storage-layer exploration. The Verus proofs need type annotation fixes.
