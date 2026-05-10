# Formal Verification Report

STATUS: REJECTED

## Inputs
- proof-obligations.jsonl: **MISSING** (not found in crate directory)
- traceability-matrix.jsonl: **MISSING** (not found in crate directory)
- contract-verification-review.md: **MISSING** (not found in crate directory)
- TEST-PLAN.md: PRESENT at /home/lewis/src/Velvet-ballistics/crates/vb_expr/TEST-PLAN.md

## Tool Availability
- lake: NOT APPLICABLE (no Lean proof project)
- rust-verification-gauntlet.sh: **MISSING**
- scripts/verify-lean.sh: **MISSING** (950B but failed/not applicable)
- cargo kani: **AVAILABLE** (v0.67.0)
- cargo careful: NOT INSTALLED
- moon: **AVAILABLE** (v2.2.3)
- cargo fuzz: NOT FOUND
- cargo bolero: NOT FOUND
- lockbud: NOT FOUND
- cargo mutants: NOT FOUND
- cargo llvm-cov: NOT FOUND
- cargo asm / cargo-show-asm: NOT FOUND
- cargo semver-checks: NOT FOUND
- cargo auditable: NOT FOUND
- cargo cyclonedx: NOT FOUND
- crux: NOT FOUND
- saw: NOT FOUND
- hax: NOT FOUND

## Obligation Results

### Layer: static-analysis (clippy)
- id: clippy-check
- layer: static-analysis
- checker: cargo clippy -p vb_expr
- command: rtk cargo clippy -p vb_expr
- result: **PASS**
- evidence: "cargo clippy: 0 errors, 2 warnings" — crate compiles and passes clippy with zero errors.

### Layer: test
- id: unit-tests
- layer: test
- checker: cargo test -p vb_expr
- command: rtk cargo test -p vb_expr
- result: **PASS**
- evidence: "278 passed (2 suites, 0.00s)" — all 278 tests pass.

### Layer: kani
- id: kani-proofs
- layer: kani
- checker: cargo kani -p vb_expr
- command: cargo kani -p vb_expr
- result: **FAIL**
- evidence: "No proof harnesses (functions with #[kani::proof]) were found to verify." — TEST-PLAN.md Section 6 specifies 3 kani proof harnesses (eval_binary_op_addition_i64, eval_binary_op_division_special_cases, eval_expr_program_stack_never_overflows), but NO actual `#[kani::proof]` functions exist in the crate source.

### Layer: gauntlet-fast
- id: verify-fast
- layer: gauntlet-fast
- checker: moon run :verify-fast
- command: moon run :verify-fast
- result: **FAIL**
- evidence: "Error: process::failed — × Process git failed: terminated" — rustup/git infrastructure failure during toolchain installation. Not a code defect, but the gate could not execute.

## Waivers
- None — no formal-waivers.jsonl exists in the crate directory.

## Blocker: Missing Required Inputs

Per the formal-verifier skill, the following required inputs were NOT found in `/home/lewis/src/Velvet-ballistics/crates/vb_expr/`:
1. `proof-obligations.jsonl` — missing
2. `traceability-matrix.jsonl` — missing
3. `contract-verification-review.md` — missing

These inputs are required before the verification gauntlet can run legitimately. Without them, there is no traceable obligation bundle to account for, no approved contract, and no basis for STATUS: APPROVED.

## Residual Risk

### Critical Gaps (TEST-PLAN.md vs Reality)
- **Kani proofs (Section 6)**: 3 harnesses specified in TEST-PLAN.md lines 352-384 as `#[kani::proof]` functions. **None exist in source.** Critical arithmetic overflow properties (i64::MIN/-1 overflow, i64::MAX+1 overflow, stack bounds) have NO formal verification.
- **Fuzz targets (Section 5)**: 3 fuzz targets specified (`fuzz_lex_expr`, `fuzz_parse_expr`, `fuzz_eval_expr_program`). No `fuzz/` directory exists in the crate.
- **Mutation testing (Section 7)**: Specified but no `cargo mutants` or equivalent tooling found.

### Test Quality (per TEST-PLAN.md VERDICT)
- **Clippy**: 161 `assert!` in Result-returning functions — clippy now passes, but TEST-PLAN.md indicates this was a prior rejection reason.
- **Holzmann violations**: 21 loops in test bodies — not verified here, but TEST-PLAN.md Section 9 identifies specific files with loop violations.
- **Line coverage**: 77.70% (target ≥90%) — untested in this run but flagged as gap.
- **Bytecode coverage**: ≥95% target not verified.

## Summary

| Obligation | Layer | Result |
|------------|-------|--------|
| clippy -p vb_expr | static | PASS |
| cargo test -p vb_expr | test | PASS |
| cargo kani -p vb_expr | kani | FAIL (no harnesses) |
| moon run :verify-fast | gauntlet-fast | FAIL (git/rustup infra) |

**STATUS: REJECTED** — Cannot approve. Missing proof-obligations.jsonl, traceability-matrix.jsonl, and contract-verification-review.md. Additionally, no kani proof harnesses exist despite TEST-PLAN.md specifying them. The gauntlet-fast lane also failed on infrastructure, not code quality.
