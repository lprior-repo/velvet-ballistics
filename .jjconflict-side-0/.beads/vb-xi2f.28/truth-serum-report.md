# Truth Serum Report — vb-xi2f.28

**Audited Bundle:** `.beads/vb-xi2f.28/assurance-bundle.md`
**Auditor:** evidence-packaging agent + truth-serum skill
**Timestamp:** 2026-05-26
**Workspace:** /home/lewis/src/vb-workspaces/vb-xi2f.28

---

## 🔬 Execution Evidence

All commands run in the active execution context at `/home/lewis/src/vb-workspaces/vb-xi2f.28`.

### Gate 1: Artifact Existence

All 15 bundle-referenced artifacts checked with `test -s`. Results:

| Artifact | Status |
|---|---|
| `.beads/vb-xi2f.28/contract.md` (161 lines) | ✅ EXISTS |
| `.beads/vb-xi2f.28/traceability-matrix.jsonl` (15 lines) | ✅ EXISTS |
| `.beads/vb-xi2f.28/delivery-scope.jsonl` (26 lines) | ✅ EXISTS |
| `.beads/vb-xi2f.28/proof-review.md` (326 lines) | ✅ EXISTS |
| `.beads/vb-xi2f.28/proof-to-rust-review.md` (401 lines) | ✅ EXISTS |
| `.beads/vb-xi2f.28/test-suite-review.md` (257 lines) | ✅ EXISTS |
| `.beads/vb-xi2f.28/rust-refinement-obligations.jsonl` (15 lines) | ✅ EXISTS |
| `.beads/vb-xi2f.28/agent-invocation-ledger.jsonl` (8 lines) | ✅ EXISTS |
| `.beads/vb-xi2f.28/proof-evidence.md` (221 lines) | ✅ EXISTS |
| `formal-verification-report.md` (213 lines) | ✅ EXISTS |
| `verification-ledger.jsonl` (70 lines) | ✅ EXISTS |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` (248 lines) | ✅ EXISTS |
| `crates/vb_compile/src/compile/mod.rs` | ❌ **DOES NOT EXIST** |
| `crates/vb_compile/src/lib.rs` (75 lines) | ✅ EXISTS |
| `crates/vb_compile/tests/proptest_digest_foreach.rs` (538 lines) | ✅ EXISTS |
| `crates/vb_yaml/src/ast/types.rs` (406 lines) | ✅ EXISTS |

**CRITICAL FINDING**: `crates/vb_compile/src/compile/mod.rs` (path A) does not exist. The `compile/` directory does not exist at all under `crates/vb_compile/src/`. The formal-verification-report.md (lines 162-163) and proof-to-rust-review.md (§2.2) both claim to have inspected this file at specific line numbers, but it is not present on disk.

### Gate 2: JSONL Validation

All 5 JSONL files validate as parseable JSON:

```bash
$ jq -c . .beads/vb-xi2f.28/delivery-scope.jsonl >/dev/null && echo PASS → PASS
$ jq -c . .beads/vb-xi2f.28/traceability-matrix.jsonl >/dev/null && echo PASS → PASS
$ jq -c . .beads/vb-xi2f.28/rust-refinement-obligations.jsonl >/dev/null && echo PASS → PASS
$ jq -c . .beads/vb-xi2f.28/agent-invocation-ledger.jsonl >/dev/null && echo PASS → PASS
$ jq -c . verification-ledger.jsonl >/dev/null && echo PASS → PASS
```

### Gate 3: Build Compilation

```bash
$ rtk cargo build -p vb_compile -p vb_yaml
cargo build (1 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
EXIT: 0
```

✅ Build passes.

### Gate 4: Clippy (Zero Warnings Gate)

```bash
$ rtk cargo clippy -p vb_compile -- -D warnings
cargo clippy: No issues found
EXIT: 0
```

✅ Clippy passes with zero warnings.

### Gate 5: Strict Panic-Surface Gate

```bash
$ rtk cargo clippy -p vb_compile -- \
  -D warnings -D clippy::unwrap_used -D clippy::expect_used \
  -D clippy::panic -D clippy::todo -D clippy::unimplemented \
  -D clippy::dbg_macro -D clippy::indexing_slicing
cargo clippy: No issues found
EXIT: 0
```

✅ No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg_macro`, or `indexing_slicing` in vb_compile production code.

### Gate 6: ForEach Arm Verification (Contract §2.1)

Verified in `crates/vb_compile/src/mod_compile_lowering/part_05.rs` lines 155-170:

```rust
vb_yaml::ast::StepPrimitive::ForEach {
    variable, input, at_once, body,
} => {
    hasher.update(b"for_each");
    hasher.update(b":variable:");
    hasher.update(variable.as_bytes());
    hasher.update(b":input:");
    hasher.update(input.as_bytes());
    hasher.update(b":at_once:");
    let limit = at_once.unwrap_or(1);
    hasher.update(&limit.to_le_bytes());
    hasher.update(b":body:");
    for step in body {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(hasher, &step.primitive);
    }
}
```

✅ All four fields hashed with `:` delimiters. `at_once.unwrap_or(1)` used. Body steps recursively hashed with step IDs. Matches contract §2.1 exactly.

### Gate 7: Proptest Execution (Independent Verification)

All 7 proptest obligations executed individually with 500 cases each:

```bash
$ PROPTEST_CASES=500 cargo test -p vb_compile --test proptest_digest_foreach -- proptest_foreach_input_variation_changes_digest
test result: ok. 1 passed, 8 filtered out (1 suite, 0.09s)    → PASS

$ PROPTEST_CASES=500 cargo test -p vb_compile --test proptest_digest_foreach -- proptest_foreach_at_once_variation_changes_digest
test result: ok. 1 passed, 8 filtered out (1 suite, 0.11s)    → PASS

$ PROPTEST_CASES=500 cargo test -p vb_compile --test proptest_digest_foreach -- proptest_foreach_variable_variation_changes_digest
test result: ok. 1 passed, 8 filtered out (1 suite, 0.09s)    → PASS

$ PROPTEST_CASES=500 cargo test -p vb_compile --test proptest_digest_foreach -- proptest_foreach_body_variation_changes_digest
test result: ok. 1 passed, 8 filtered out (1 suite, 0.11s)    → PASS

$ PROPTEST_CASES=500 cargo test -p vb_compile --test proptest_digest_foreach -- proptest_foreach_digest_deterministic
test result: ok. 1 passed, 8 filtered out (1 suite, 0.07s)    → PASS

$ PROPTEST_CASES=500 cargo test -p vb_compile --test proptest_digest_foreach -- proptest_foreach_nonregression_set_finish
test result: ok. 1 passed, 8 filtered out (1 suite, 0.08s)    → PASS

$ PROPTEST_CASES=500 cargo test -p vb_compile --test proptest_digest_foreach -- proptest_foreach_nonregression_set_sensitivity
test result: ok. 1 passed, 8 filtered out (1 suite, 0.04s)    → PASS
```

✅ All 7 proptest obligations PASS (independently verified). 3,500 total diversified input combinations.

### Gate 8: Full Test Suite

```bash
$ rtk cargo test -p vb_compile -p vb_yaml
cargo test: 559 passed (9 suites, 2.54s)
EXIT: 0
```

✅ Full test suite passes (559 tests, no failures, no skips).

### Gate 9: Kani Harness Compilation

```bash
$ rtk cargo kani --harness kani_foreach_delimiter_byte_not_in_yaml_id -p vb_compile
error[E0433]: failed to resolve: could not find `assume` in `kani`
  --> crates/vb_compile/src/mod_compile_lowering/kani_proofs/kani_digest_foreach_delimiter.rs:58:11
   |
58 |     kani::assume!(concat_a != concat_b);
   |           ^^^^^^ could not find `assume` in `kani`
warning: unused import: `StepAst`
help: kani::assume is not a macro, but a function, try to remove `!`

error: could not compile `vb_compile` (lib) due to 1 previous error; 4 warnings emitted
EXIT: 101
```

❌ **FAIL**: Kani harnesses do not compile with kani 0.67.0. The harness uses `kani::assume!(...)` (macro syntax) which does not exist in this Kani version. The error message says `kani::assume is not a macro, but a function, try to remove '!'`. Additionally, `StepAst` is imported but unused in several harness files.

**This contradicts the formal-verification-report.md claim that PO-K-FE-10 H1+H2 were VERIFIED with "VERIFICATION:- SUCCESSFUL (0 of 37 failed, 0.016s)."**

The formal-verification-report evidence for Kani is unverifiable. The reported Kani verification results cannot be reproduced.

### Gate 10: Production Assertion Scan

```bash
$ grep -rn 'assert!\|assert_eq!\|assert_ne!\|unreachable!' \
  crates/vb_compile/src/mod_compile_lowering/part_05.rs
(no matches)
```

✅ No production assertions in the modified code.

### Gate 11: Irreproducible Claims Audit

| Formal Verification Report Claim | Independent Verification | Match? |
|---|---|---|
| `crates/vb_compile/src/compile/mod.rs` inspected at lines 257-271 | File does not exist (no `compile/` directory) | ❌ **UNVERIFIABLE** |
| Kani H1 VERIFIED SUCCESSFUL (0 of 37 failed) | Compilation error: `kani::assume!` not found | ❌ **UNVERIFIABLE** |
| Kani H2 VERIFIED SUCCESSFUL (0 of 37 failed) | Compilation error: `kani::assume!` not found | ❌ **UNVERIFIABLE** |
| Proptest 7 tests PASS (500 cases each) | Re-run: 7 tests PASS (500 cases each) | ✅ **CONFIRMED** |
| Build passes `cargo build -p vb_compile -p vb_yaml` | Re-run: EXIT 0 (0.30s) | ✅ **CONFIRMED** |
| Full test suite passes | Re-run: 559 passed (2.54s) | ✅ **CONFIRMED** |
| ForEach arm in `part_05.rs:158-172` | Found at lines 155-170 (slightly different line nos) | ✅ **CONFIRMED** |
| `lib.rs` re-exports at lines 66-67 | Verified at lines 66-67 | ✅ **CONFIRMED** |

---

## 🫂 Empathetic User Review

From the perspective of a developer consuming this bead's deliverable, the core functional claim holds: **changing any ForEach field (input, at_once, variable, body) changes the workflow digest**. The ForEach arm in `digest_step_primitive` is clean, well-structured, and follows the established delimiter convention. All proptest tests pass consistently.

The digest computation remains deterministic — five recompiles of the same source always produce the same digest. Non-ForEach primitives (Set, Finish) are unaffected.

**Friction points**:
- The Kani harnesses don't compile. A developer trying to run `cargo kani` defense-in-depth will hit compilation errors immediately.
- The formal-verification-report references a file (`compile/mod.rs`) that doesn't exist, which could confuse maintainers following the report.

---

## 🕵️ Skeptical QA Review

### CRITICAL FINDINGS

**FINDING TS-01 (HIGH): Formal Verification Report Contains Unverifiable Kani Claims**

The `formal-verification-report.md` §4.1 claims Kani harnesses H1 and H2 were "VERIFIED SUCCESSFUL (0 of 37 failed)" with specific timings (0.016s, 0.017s). Independent execution reveals these harnesses fail to compile:

- **Root cause**: `kani::assume!()` macro syntax used in `kani_digest_foreach_delimiter.rs:58`. Kani 0.67.0 treats `assume` as a function, not a macro.
- **Impact**: The 2/15 VERIFIED Kani claims cannot be independently reproduced. All Kani evidence in the formal-verification-report is unverifiable.

**Mitigation**: The 7 proptest obligations independently verified in this audit cover the same P0 behavior claims (AC-FE-01 through AC-FE-05, AC-FE-08). The delimiter collision resistance claim (INV-FE-02) — the specific property the Kani harnesses were proving — is now backed only by code review and proptest, not by exhaustive byte-level verification.

**FINDING TS-02 (MEDIUM): Formal Verification Report References Nonexistent File**

The `formal-verification-report.md` §8 claims source reference verification for `crates/vb_compile/src/compile/mod.rs` at lines 243-276 and 257-271. The `crates/vb_compile/src/compile/` directory does not exist. This means:
- Path A (duplicate code) never existed in this workspace
- AC-FE-06 (dual-path equivalence) is trivially satisfied — there is only one path
- The formal-verification-report's source reference table contains at least one hallucinated entry

**Mitigation**: The absence of a second path simplifies correctness. AC-FE-06 is automatically satisfied.

### NON-CRITICAL OBSERVATIONS

- **Missing `.beads/vb-xi2f.28/black-hat-review.md`**: The user instruction states "Black-hat APPROVED WITH CONDITIONS" but no artifact exists at the expected location.
- **Missing `test-plan-review.md`**: `test-suite-review.md` covers similar ground with APPROVED status.
- **6 of 10 required per-skill artifacts are not in `.beads/vb-xi2f.28/`** as required by the evidence-packaging skill gate.
- **Proptest uses hardcoded step IDs** (`id: "s"`, `id: "f"`) in body step strategies — ID variation within ForEach bodies is not tested.

### EXECUTION EVIDENCE SUMMARY

| Gate | Command | Result | Evidence Type |
|---|---|---|---|
| Build | `cargo build -p vb_compile -p vb_yaml` | PASS (EXIT 0) | Direct |
| Clippy | `cargo clippy -p vb_compile -- -D warnings` | PASS (EXIT 0) | Direct |
| Panic surface | `cargo clippy -p vb_compile -- -D clippy::unwrap_used ...` | PASS (EXIT 0) | Direct |
| Proptest (7 tests) | `PROPTEST_CASES=500 cargo test ...` | PASS (EXIT 0, 7/7) | Direct |
| Full test suite | `cargo test -p vb_compile -p vb_yaml` | PASS (EXIT 0, 559 passed) | Direct |
| Kani H1/H2 | `cargo kani --harness ...` | FAIL (EXIT 101, compilation error) | Direct |
| Source ref verification | File existence + content grep | 13/14 confirmed, 1 nonexistent | Direct |
| JSONL validation | `jq -c . *.jsonl` | PASS (5/5) | Direct |
| ForEach arm review | `grep -n -A 15 'StepPrimitive::ForEach' part_05.rs` | CONFIRMED at lines 155-170 | Direct |

---

## 🚀 Mandated Improvements

1. **Fix Kani harness compilation**: Replace `kani::assume!(...)` with `kani::assume(...)` in all harness files. Remove unused `StepAst` imports. Re-run Kani verification after fixing.
2. **Correct formal-verification-report.md**: Remove or annotate the nonexistent `crates/vb_compile/src/compile/mod.rs` source references. The `compile/` directory does not exist in this workspace.
3. **Correct verification-ledger.jsonl**: Lines 58 and 60 claim Kani PASS for delimiter harnesses. These should be reclassified as FAIL_LOCAL or updated after harness fixes.
4. **File missing artifacts or explain gap**: `black-hat-review.md` is missing from `.beads/vb-xi2f.28/`. Either file it or document why the bead directory structure differs from the skill expectation.
5. **Fix type-contracts.md drift**: §3.3 specifies None→0u32 but implementation uses unwrap_or(1)→1u32. Documentation contradicts reality.
6. **Add step ID variation to proptest body strategies**: `set_step_strategy()` and `finish_step_strategy()` hardcode `id: "s"` and `id: "f"`. Random ID generation would improve coverage for `step.id.as_bytes()` hashing.

---

## Final Truth Serum Assessment

| Category | Verdict |
|---|---|
| **Core functional correctness** | ✅ PROVEN (proptest 7/7 PASS, independently verified) |
| **Regression safety** | ✅ PROVEN (full suite 559 PASS, Set/Finish non-regression 500 cases PASS) |
| **Kani defense-in-depth** | ❌ UNVERIFIABLE (harnesses fail compilation in kani 0.67.0) |
| **Formal report accuracy** | ⚠️ PARTIAL (source references reference nonexistent file; Kani evidence irreproducible) |
| **Artifact completeness** | ⚠️ PARTIAL (6/10 per-skill artifacts missing from bead directory) |
| **Production panic surface** | ✅ CLEAN (zero unwrap/expect/panic in vb_compile) |
| **Overall** | **APPROVED WITH CORRECTIONS** — The core behavioral claims are independently verified via proptest. Kani evidence is unverifiable but the proptest compensating evidence is sufficient for P0 claims. |
