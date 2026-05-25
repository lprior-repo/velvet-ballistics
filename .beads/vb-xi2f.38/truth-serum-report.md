# Truth Serum Report: vb-xi2f.38

**bead**: vb-xi2f.38
**date**: 2026-05-25
**auditor**: evidence-packaging agent (manual audit — truth-serum tool not available)
**context**: isolated workspace /home/lewis/src/vb-xi2f.38-ws

---

## Execution Context

- **truth-serum binary**: NOT FOUND in PATH
- **Active execution context**: manual audit by evidence-packaging agent
- **Source checkout**: /home/lewis/src/velvet-ballistics (HEAD: 0806ade88 vb-xi2f.5)
- **Bead commit**: a626cda0e (vb-xi2f.38, ancestor of HEAD)

---

## Command Evidence (Raw)

### Artifact Existence

```
$ test -s ".beads/vb-xi2f.38/delivery-scope.jsonl" && echo EXISTS || echo MISSING
EXISTS

$ test -s ".beads/vb-xi2f.38/contract.md" && echo EXISTS || echo MISSING
EXISTS

$ test -s ".beads/vb-xi2f.38/traceability-matrix.jsonl" && echo EXISTS || echo MISSING
EXISTS

$ test -s ".beads/vb-xi2f.38/proof-review.md" && echo EXISTS || echo MISSING
EXISTS

$ test -s ".beads/vb-xi2f.38/test-plan-review.md" && echo EXISTS || echo MISSING
MISSING

$ test -s ".beads/vb-xi2f.38/formal-verification-report.md" && echo EXISTS || echo MISSING
EXISTS

$ test -s ".beads/vb-xi2f.38/verification-ledger.jsonl" && echo EXISTS || echo MISSING
EXISTS

$ test -s ".beads/vb-xi2f.38/black-hat-review.md" && echo EXISTS || echo MISSING
MISSING

$ test -s ".beads/vb-xi2f.38/machine-gate-report.md" && echo EXISTS || echo MISSING
MISSING

$ test -s ".beads/vb-xi2f.38/regression-diff.md" && echo EXISTS || echo MISSING
MISSING
```

### JSONL Validity

```
$ jq -c . ".beads/vb-xi2f.38/delivery-scope.jsonl" >/dev/null 2>&1 && echo VALID || echo INVALID
VALID

$ jq -c . ".beads/vb-xi2f.38/traceability-matrix.jsonl" >/dev/null 2>&1 && echo VALID || echo INVALID
VALID

$ jq -c . ".beads/vb-xi2f.38/verification-ledger.jsonl" >/dev/null 2>&1 && echo VALID || echo INVALID
VALID
```

### Review Status Lines

```
$ rg -n '^STATUS: APPROVED$|^STATUS: PASS$' ".beads/vb-xi2f.38/proof-review.md" ".beads/vb-xi2f.38/formal-verification-report.md"
(no output — no matches found)
```

### Compilation Check

```
$ cargo build -p vb_compile 2>&1 | tail -20
error[E0277]: the trait bound `Option<Box<str>>: From<&str>` is not satisfied
    --> crates/vb_compile/src/ast/parse.rs:95:54
     95 |         path: trigger_str(value, "webhook", "path")?.into(),
     |                                                  ^^^^ the trait `From<&str>` is not implemented for `Option<Box<str>>`

error[E0277]: the trait bound `Option<Box<str>>: From<&str>` is not satisfied
    --> crates/vb_compile/src/ast/parse.rs:96:58
     96 |         method: trigger_str(value, "webhook", "method")?.into(),
     |                                                          ^^^^ the trait `From<&str>` is not implemented for `Option<Box<str>>`

cargo build: 2 errors, 0 warnings (2 crates)
```

### Moon CI Gate

```
$ moon ci 2>&1 | tail -30
 velvet-ballastics:lint-src | error: could not compile `vb_compile` (lib) due to 2 previous errors
 velvet-ballastics:lint-src (1s 18ms, ebda83bc)

Tasks: 12 completed (4 cached), 5 failed, 9 skipped
 Time: 51s 471ms
```

### Test Count

```
$ cargo test -p vb_compile 2>&1 | tail -5
test result: FAILED. 243 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.30s
```

### Missing Test File

```
$ test -f "crates/vb_compile/src/tests/digest_collect_tests.rs" && echo EXISTS || echo MISSING
MISSING
```

### Implementation Verification

```
$ grep -n 'StepPrimitive::Collect.*variable.*source.*pages.*items.*body' crates/vb_compile/src/compile/mod.rs
257:         vb_yaml::ast::StepPrimitive::Collect { variable, source, pages, items, body } => {

$ grep -n 'pub(super) fn digest_step_primitive' crates/vb_compile/src/mod_compile_lowering/part_05.rs
140: pub(super) fn digest_step_primitive(
```

The implementation fix IS present in source at:
- `crates/vb_compile/src/compile/mod.rs:257-271`
- `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-178`

---

## Truth Serum Audit Findings

### CRITICAL — Evidence Packaged Does Not Match Reality

| Claim (from artifacts) | Reality | Discrepancy |
|---|---|---|
| 309 tests passed including 18 digest_collect tests | 243 passed, 2 failed; digest_collect_tests.rs MISSING | User claim UNSUPPORTED by evidence |
| proof-review.md STATUS: APPROVED | proof-review.md STATUS: REJECTED (state 6) | Incorrect status |
| test-plan-review.md exists | MISSING | Artifact absent |
| black-hat-review.md exists | MISSING | Artifact absent |
| machine-gate-report.md exists | MISSING | Artifact absent |
| regression-diff.md exists | MISSING | Artifact absent |
| Source checkout compiles | HEAD (vb-xi2f.5) FAILS to compile | Build broken |
| moon ci PASS | moon ci lint-src FAILS | Gate failing |
| Kani/Verus BLOCKED_TOOLING waived | FW-001, FW-002 have approved_by: null | Waiver not approved |

### Anti-Hallucination Violations

1. **SUBAGENT SUMMARY AS EVIDENCE**: The user's context claim "309 tests passed including 18 digest_collect tests" was NOT verified by command evidence. The actual test run shows 243 passed, 2 failed. The `digest_collect_tests.rs` file does not exist.

2. **MISSING COMMAND OUTPUT**: `proof-evidence.md` lines 149-153 show proptest status as PENDING with no execution output. The formal-verification-report.md claims 290 tests passed, but `digest_collect_tests.rs` is MISSING from the source checkout.

3. **PATHS DO NOT EXIST**: `crates/vb_compile/src/tests/digest_collect_tests.rs` referenced throughout proof-evidence.md does not exist in the source checkout.

4. **REVIEW STATUS UNSUPPORTED**: `proof-review.md` explicitly shows `STATUS: REJECTED` at state 6, not APPROVED as required for landing.

5. **MOON CI GATE FAILING**: `moon ci` shows 5 tasks failed including `lint-src` due to compilation errors unrelated to this bead, but `moon ci` is the canonical gate per AGENTS.md.

---

## Summary

The evidence artifacts for bead vb-xi2f.38 are incomplete and contain material discrepancies:

1. 4 mandatory artifacts are missing (test-plan-review.md, black-hat-review.md, machine-gate-report.md, regression-diff.md)
2. The proof-review shows REJECTED, not APPROVED
3. The source checkout at HEAD does not compile
4. The moon ci canonical gate is failing
5. The user's claim of "309 tests passed" is not supported by command evidence
6. The `digest_collect_tests.rs` file referenced throughout proof-evidence does not exist

**truth-serum-audit: FAIL — evidence is incomplete and contains unsupported claims**
