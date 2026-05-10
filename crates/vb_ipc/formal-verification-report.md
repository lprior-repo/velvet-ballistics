# Formal Verification Report

**CRATE:** vb_ipc
**WORKSPACE:** /home/lewis/src/Velvet-ballistics
**DATE:** 2026-05-10

---

## STATUS: REJECTED

---

## Inputs

- **TEST-PLAN.md:** EXISTS — outlines 76 tautological `assert!(false, ...)` assertions, 152 `expect()` calls, and coverage targets (≥70% for handlers/dispatch/client)
- **proof-obligations.jsonl:** MISSING — no such file exists in vb_ipc crate directory
- **traceability-matrix.jsonl:** MISSING — no such file exists in vb_ipc crate directory
- **contract-verification-review.md:** MISSING — no such file exists in vb_ipc crate directory
- **formal-waivers.jsonl:** MISSING — no such file exists

---

## Tool Availability

| Tool | Status | Version |
|------|--------|---------|
| cargo | Available | 1.97.0-nightly |
| cargo-kani | Available | 0.67.0 |
| rust-verification-gauntlet.sh | Available | scripts/rust-verification-gauntlet.sh |
| moon | Available | .moon/tasks/all.yml |
| cargo llvm-cov | Available | coverage tool |
| cargo clippy | Available | nightly-2026-04-28 |
| cargo-fuzz | Not checked | — |
| cargo-mutants | Not checked | — |

---

## Contract Artifacts Check

| Artifact | Required | Found |
|----------|----------|-------|
| proof-obligations.jsonl | YES | **NO** |
| traceability-matrix.jsonl | YES | **NO** |
| contract-verification-review.md with STATUS: APPROVED | YES | **NO** |
| formal-waivers.jsonl | NO | NO |

**BLOCKER:** Cannot execute formal verification gauntlet — required contract artifacts are absent.

---

## Obligation Results

### Layer: `gauntlet-fast` (verify-fast)
- **Command:** `bash scripts/rust-verification-gauntlet.sh fast` (runs fmt + clippy + check)
- **Result:** FAIL
- **Evidence:**
  - `cargo clippy -p vb_ipc`: 0 errors, 2 warnings — **PASS**
  - `cargo test -p vb_ipc`: 400 passed — **PASS**
  - `rust-verification-gauntlet.sh fast`: FAILS at fmt step due to unrelated whitespace diffs in `xtask/tests/integration_gates.rs`

### Layer: `kani`
- **Command:** `cargo kani -p vb_ipc`
- **Result:** PASS (no harnesses to verify)
- **Evidence:**
  ```
  Manual Harness Summary:
  No proof harnesses (functions with #[kani::proof]) were found to verify.
  ```
- **Note:** TEST-PLAN.md Section 7 describes 2 planned Kani harnesses (`header_construction_is_safe` and `u32_to_usize_is_correct_or_error`), but **no proof harnesses exist in the codebase**.

### Layer: `coverage`
- **Command:** `cargo llvm-cov -p vb_ipc`
- **Result:** FAIL — instrumentation anomaly
- **Evidence:**
  - Tests run and pass (400 tests)
  - Coverage report shows 0.10% region coverage, 0.28% function coverage, 0.14% line coverage
  - This contradicts TEST-PLAN.md's reported 69.37% coverage
  - Likely cause: llvm-cov not collecting coverage data during test run

### Layer: `clippy`
- **Command:** `cargo clippy -p vb_ipc`
- **Result:** PASS
- **Evidence:** 0 errors, 2 warnings

### Layer: `test`
- **Command:** `cargo test -p vb_ipc`
- **Result:** PASS
- **Evidence:** 400 passed (2 suites, 0.28s)

---

## Test Quality Findings

### Tautological Assertions (76 total)

The crate contains **76 `assert!(false, ...)` tautologies** across 6 files, as documented in TEST-PLAN.md:

| File | Count | Lines |
|------|-------|-------|
| handlers.rs | 48 | 1125, 1154, 1171, 1185, 1202, 1218, 1236, 1254, 1272, 1286, 1303, 1321, 1353, 1373, 1390, 1407, 1519, 1529, 1543, 1552, 1564, 1573, 1584, 1593, 1668, 1682, 1698, 1711, 1727, 1740, 1755, 1768, 1871, 1885, 1900, 1913, 1941, 1953 |
| trace.rs | 6 | 301, 320, 340, 357, 381, 409 |
| impl_tests.rs | 12 | 252, 255, 354, 357, 430, 433, 672, 675, 915, 1304, 1319, 1322 |
| metrics.rs | 9 | 118, 143, 168, 186, 203, 222, 253, 310, 388 |
| ids.rs | 8 | 208, 219, 231, 244, 366, 377, 389, 402 |
| action_output.rs | 3 | 127, 146, 164 |

### expect() Usage

TEST-PLAN.md reports 152 `expect()` calls in test infrastructure. Not verified in this run.

### Coverage Targets (from TEST-PLAN.md)

| Module | Current | Target |
|--------|---------|--------|
| handlers.rs | 44% | ≥70% |
| dispatch.rs | 23% | ≥50% |
| client.rs | 48% | ≥70% |

Coverage measurement is unreliable (see above).

---

## Kani Harnesses

**Status:** 0 harnesses found

TEST-PLAN.md Section 7 describes 2 planned harnesses:
1. `header_construction_is_safe` — verify `IpcFrameHeader::new` with valid inputs never panics
2. `u32_to_usize_is_correct_or_error` — verify `u32_to_usize` returns correct value or error

Neither harness exists in the codebase. **These must be implemented before Kani verification can run.**

---

## Waivers

- None — no formal-waivers.jsonl exists

---

## Residual Risk

1. **76 tautological assertions** create a false sense of test coverage — these tests always pass regardless of behavior
2. **No proof-obligations.jsonl** — cannot trace verification claims to specific obligations
3. **No traceability-matrix.jsonl** — cannot verify coverage of requirements
4. **No contract-verification-review.md with STATUS: APPROVED** — formal verification gate was never passed by contract reviewer
5. **No Kani harnesses** — zero formal memory-safety proofs exist
6. **Coverage instrumentation anomaly** — actual coverage of handlers/dispatch/client is unmeasurable with current tool invocation
7. **152 `expect()` calls** in test infrastructure (TEST-PLAN.md claim; not independently verified)

---

## Verdict Context

The TEST-PLAN.md itself states the crate is in REJECTED state due to test quality issues:
> **Current state**: 400 tests pass, 76 `assert!(false, ...)` tautologies, 152 `expect()` in test code, 69.37% coverage  
> **Target**: 0 tautologies, ≤20 `expect()` in integration tests, ≥5x density, coverage ≥70% handlers/dispatch/client

The presence of 76 tautologies alone is sufficient to reject — these tests prove nothing.

---

## Recommendation

1. **Do not advance** this crate beyond current state until:
   - All 76 `assert!(false, ...)` tautologies are replaced with proper assertions or `unreachable!()`
   - `proof-obligations.jsonl` and `traceability-matrix.jsonl` are created via `rust-contract` workflow
   - `contract-verification-review.md` is produced with `STATUS: APPROVED`
   - At least 1 Kani proof harness is implemented for `IpcFrameHeader` or `u32_to_usize`
   - Coverage instrumentation is fixed and ≥70% is achieved for handlers/dispatch/client
   - `expect()` count is reduced to ≤20 in test infrastructure
2. Run `moon run :verify-fast` after fmt fixes to clear workspace-level gauntlet failures
