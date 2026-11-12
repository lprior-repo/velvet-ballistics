# Test Plan — vb-qi37.12.1

## Overview

This is a **verification-only audit bead**. The "test plan" documents the verification strategy used to confirm production code is free of silent discard sites.

## Verification Strategy

### 1. Static Analysis (Primary Verification Layer)

**Tool**: grep/ripgrep + clippy

**Approach**:
1. Grep search for `.unwrap()`, `.expect(`, `panic!` patterns across all production source files
2. Filter out test files (`*_tests.rs`, `tests/`, `test_`)
3. Verify remaining matches are inside `#[cfg(test)]` modules or `#[test]` functions
4. Run clippy with denial lints for `unwrap_used`, `expect_used`, `panic`

**Evidence**: grep output + clippy report

### 2. Build Verification (Secondary Layer)

**Tool**: `cargo build --all-targets --all-features`

**Approach**:
1. Compile all production code
2. Verify all fallible APIs return `Result<T, E>` or `Option<T>`
3. Confirm `#[must_use]` annotations on non-Result fallible returns where appropriate

**Evidence**: Build log

### 3. Manual Spot-Check (Tertiary Layer)

**Approach**:
1. Random sample of production source files
2. Manual inspection to confirm no `.unwrap()`, `.expect()`, `panic!` in non-test code
3. Verify `#[cfg(test)]` module boundaries are correctly identified

**Evidence**: Audit notes

## Crates in Scope

| Crate | Files | Test Files | Production Files |
|-------|-------|------------|------------------|
| vb_core | ~50 | ~15 | ~35 |
| vb_expr | ~10 | ~2 | ~8 |
| vb_validate | ~30 | ~10 | ~20 |
| vb_compile | ~20 | ~5 | ~15 |
| vb_runtime | ~40 | ~15 | ~25 |
| vb_storage | ~30 | ~10 | ~20 |
| vb_ipc | ~10 | ~3 | ~7 |

## Verification Execution

### Phase 1: Grep Audit

```bash
# Run grep audit for all silent discard patterns
# Capture output to audit-grep-output.txt

grep -r '\.unwrap()' crates/*/src --include='*.rs' > audit-unwrap.txt
grep -r '\.expect' crates/*/src --include='*.rs' > audit-expect.txt
grep -r 'panic!' crates/*/src --include='*.rs' > audit-panic.txt

# Filter to production-only
grep -v '_tests' audit-unwrap.txt | grep -v '/tests/' | grep -v 'test_' > audit-unwrap-prod.txt
```

### Phase 2: Clippy Gate

```bash
cargo clippy --all-targets -- -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::result_expect -D clippy::unused_result
```

### Phase 3: Build Verification

```bash
cargo build --all-targets --all-features 2>&1 | tee build-log.txt
```

## Success Criteria

| Criterion | Threshold |
|-----------|-----------|
| Grep production .unwrap() matches | 0 |
| Grep production .expect() matches | 0 |
| Grep production panic! matches | 0 |
| Clippy violations (unwrap/expect/panic) | 0 |
| Build warnings (unused_must_use) | 0 |
| Build errors | 0 |

## Deliverables

- [x] `contract.md` — Audit scope, clauses, findings
- [x] `lean-contract.md` — Waiver of Lean obligations
- [x] `verification-layers.md` — Layer assignments
- [x] `proof-obligations.jsonl` — Machine-readable obligations
- [x] `traceability-matrix.jsonl` — Clause-to-evidence mapping
- [x] `martin-fowler-tests.md` — Test scenarios
- [ ] `audit-grep-output.txt` — Raw grep evidence (optional, can be captured inline)
- [ ] `clippy-report.txt` — Clippy output (optional, can be captured inline)

## Roles and Responsibilities

| Role | Responsibility |
|------|----------------|
| Contract Synthesizer (this bead) | Produce all contract artifacts |
| Independent Reviewer | Review contract bundle, approve or reject |
| CI Gate | Run clippy denial lint |

## Schedule

This is a State 1 → 1.5 artifact production bead. Timeline:
- State 1: Isolation setup (complete)
- State 1.5: Contract artifacts produced (this work)
- State 2: Independent review (pending)

---

**Test Plan Status**: COMPLETE — Verification strategy documented, all artifacts produced.
