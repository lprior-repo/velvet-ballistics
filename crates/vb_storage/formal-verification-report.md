# Formal Verification Report

STATUS: REJECTED

## Inputs
- proof-obligations.jsonl: **MISSING** — no such file in crate directory or workspace beads
- traceability-matrix.jsonl: **MISSING** — no such file in crate directory or workspace beads
- contract-verification-review.md: **MISSING** — no such file with STATUS: APPROVED for vb_storage
- TEST-PLAN.md: PRESENT at /home/lewis/src/Velvet-ballistics/crates/vb_storage/TEST-PLAN.md

## Tool Availability
- lake: MISSING (not installed)
- rust-verification-gauntlet.sh: EXISTS at /home/lewis/src/Velvet-ballistics/scripts/rust-verification-gauntlet.sh
- scripts/verify-lean.sh: NOT FOUND
- cargo kani: EXISTS (Kani Rust Verifier 0.67.0)
- cargo careful: NOT FOUND
- moon: EXISTS (moon v2)
- cargo fuzz: EXISTS (in workspace fuzz/ directory)
- cargo bolero: NOT FOUND
- lockbud: NOT FOUND
- cargo mutants: EXISTS (smoke task configured for vb_core only)
- cargo llvm-cov: EXISTS (moon task :coverage configured)
- cargo asm / cargo-show-asm: NOT FOUND
- cargo semver-checks: NOT FOUND
- cargo auditable: NOT FOUND
- cargo cyclonedx: NOT FOUND
- crux: NOT FOUND
- saw: NOT FOUND
- hax: NOT FOUND

## Kani Verification

**Command:** `rustup run nightly-2026-04-28 cargo kani -p vb_storage`

**Result:** FAIL — No proof harnesses found

```
Manual Harness Summary:
No proof harnesses (functions with #[kani::proof]) were found to verify.
```

**Gap:** The TEST-PLAN.md Section 6 specifies Kani harnesses for `process_lock.rs` (process_lock_acquire_no_panic, read_holder_pid_bounds) and `error.rs` (diagnostic_code_nonzero, all_variants_have_display), but none are implemented in the crate source.

## Test Execution

**Command:** `rustup run nightly-2026-04-28 cargo nextest run -p vb_storage --all-features --no-capture`

**Result:** PASS — 1026/1026 tests passed in 8.578s

```
Summary [   8.578s] 1026 tests run: 1026 passed, 0 skipped
```

## Cargo Check

**Command:** `rustup run nightly-2026-04-28 cargo check -p vb_storage --all-features`

**Result:** PASS — compiled successfully

## Obligation Results

**ID:** vb_storage-formal-verification-missing-inputs
**Layer:** formal-verifier
**Checker:** input validation
**Command:** N/A (required inputs absent)
**Result:** FAIL
**Evidence:** proof-obligations.jsonl MISSING, traceability-matrix.jsonl MISSING, contract-verification-review.md MISSING

**ID:** vb_storage-kani-harnesses
**Layer:** kani
**Checker:** cargo kani
**Command:** `cargo kani -p vb_storage`
**Result:** FAIL
**Evidence:** "No proof harnesses (functions with #[kani::proof]) were found to verify."

## Waivers
- None.

## Residual Risk

**LETHAL-1 (journal.rs:329 — impl Drop silently discards persist error):** No formal verification. Drop implementation uses `let _ = e;` to silently discard persist failures. No Kani proof, no model checking.

**LETHAL-2 (process_lock.rs — silent I/O discards with `#[allow(clippy::let_underscore_must_use)]`):** Four `let _ =` patterns at lines 492, 494, 500, 502 discard set_len, write!, rewind, and read_to_string errors. No formal verification. No Kani harness despite being specified in TEST-PLAN.md.

**LETHAL-3 (integration tests use `crate::` imports):** Integration tests in tests/ directory use internal `crate::` paths. This is an import purity violation. No formal enforcement.

**Coverage Gaps (from TEST-PLAN.md):**
- artifacts.rs: 0% coverage (LETHAL)
- error.rs: 33.85% coverage (LETHAL)
- process_lock.rs: 44.19% coverage (LETHAL)

**Clippy (VERDICT context):** TEST-PLAN.md claims 288 clippy errors. Live clippy check passes cleanly for vb_storage — this may indicate prior fixes or a stale VERDICT context.

## Blocker Summary

1. **MISSING proof-obligations.jsonl** — cannot run formal verification gauntlet without obligation bundle
2. **MISSING traceability-matrix.jsonl** — no traceability between contract and obligations
3. **MISSING contract-verification-review.md with STATUS: APPROVED** — skill rule requires this before gauntlet execution
4. **NO Kani harnesses implemented** — TEST-PLAN.md specifies 4 harnesses, zero exist
5. **3 LETHAL issues unverified** — Drop silently discards errors, process_lock I/O discards, integration test import purity
