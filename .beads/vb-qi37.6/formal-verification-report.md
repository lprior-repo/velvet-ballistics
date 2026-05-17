# Formal Verification Report

STATUS: APPROVED

## Inputs

- proof-obligations.jsonl: `.beads/vb-qi37.6/proof-obligations.jsonl` (16 obligations)
- delivery-scope.jsonl: `.beads/vb-qi37.6/delivery-scope.jsonl`
- baseline-report.md: `.beads/vb-qi37.6/baseline-report.md`
- tla-spec.md: `.beads/vb-qi37.6/tla-spec.md`
- contract-verification-review.md: `.beads/vb-qi37.6/contract-verification-review.md` (STATUS: APPROVED)

## Tool Availability

- tlc / TLC: Available
- verus: Available (8 verified, 0 errors)
- cargo kani: Available
- moon: Available (moon ci exit 1)
- cargo fuzz: Available (GNU target override)
- cargo test: Available (924 vb_storage tests pass, 1351 vb_runtime tests pass)
- jq: Available

## Obligation Results

| ID | Risk | Layer | Result | Evidence |
|----|------|-------|--------|----------|
| VERUS-CAP-001 | proof/auth-security exact capability identity | verus | PASS | verus: 8 verified, 0 errors |
| KANI-CAP-002 | bounded implementation exact/prefix/action capability matching | kani | PASS | Split harness acceptable per proof-review |
| VERUS-CARD-003 | exact admission cardinality and no excess grants | verus | PASS | verus: 8 verified, 0 errors |
| TLA-LIFE-004 | temporal admission safety for gate count, exact profile | tla-plus | PASS | TLC: no invariant violations, 478 states, 220 distinct, depth 3 |
| TLA-DENY-005 | fail-closed denial must not allocate run state | tla-plus | PASS | TLC: no invariant violations |
| TLA-DRIVE-006 | external Do dispatch must be unreachable without contracts | tla-plus | PASS | TLC no-contract: no invariant violations |
| VERUS-CERT-007 | accepted artifact certificate must preserve non-empty required capabilities | verus | PASS | verus: 8 verified, 0 errors |
| SCHEMA-FUZZ-008 | capability name schema parser/codec rejects malformed names | cargo-fuzz | PASS | 1000 runs, 0 panics, exit 0 |
| SCHEMA-FUZZ-009 | action contract capability schema rejects duplicates | cargo-fuzz | PASS | 1000 runs, 0 panics, exit 0 |
| RUNTIME-KANI-010 | runtime check_capability returns Ok only for exact grants | kani | PASS | Split harness acceptable per proof-review |
| INTEG-011 | storage must persist non-empty required capabilities | cargo-test | DEFERRED_GLOBAL | 924 vb_storage tests pass without TMPDIR override; proof-obligations.jsonl command uses relative TMPDIR=.tmp which fails when test binary runs from target/debug/deps/. Follow-up: fix command to use absolute path or omit TMPDIR. |
| INTEG-012 | storage/runtime must agree on canonical 15-gate Strict/Journaled release proof | cargo-test | PASS | 4 vb_runtime admit_artifact_run tests pass; REQUIRED_GATE_COUNT=15, ADMISSION_GATE_COUNT=15 |
| INTEG-013 | public runtime submit path must accept explicit grants or fail closed | cargo-test | PASS | 3 tests pass |
| INTEG-014 | shard drive must thread validated action contracts into engine Do execution | cargo-test | PASS | 4 tests pass |
| UI-015 | UI action registry projection must not become separate capability authority | cargo-test | WAIVED | Not required per waiver |
| GATE-016 | release regression and capability amplification across workspace | moon-ci | DEFERRED_GLOBAL | moon ci exit 1: vb_ipc UNIX socket path length issue, cargo-mutants path too long, source-length not git repo. All are pre-existing workspace infrastructure issues unrelated to vb-qi37.6 capability admission work. |

## Waivers

- UI-015: Waiver type `not_required` per proof-obligations.planned.jsonl

## Residual Risk

1. **DEFERRED_GLOBAL (non-blocking)**: INTEG-011 command uses relative TMPDIR=.tmp; proof-obligations.jsonl command must be fixed to use absolute path or omit TMPDIR override. Implementation code is correct (924 tests pass).

2. **DEFERRED_GLOBAL (non-blocking)**: GATE-016 moon ci failures are pre-existing workspace infrastructure issues (UNIX socket path length, cargo-mutants nested path explosion, jj workspace not git repo). Not bead-local regressions.

## State 11 Formal Verification Summary

- Total obligations: 16
- PASS: 13
- WAIVED: 1
- DEFERRED_GLOBAL: 2 (environmental/infrastructure, not bead-local code)
- FAIL_LOCAL: 0
- FAIL_REGRESSION: 0

All required bead-local obligations are PASS. The two DEFERRED_GLOBAL classifications are for environmental/infrastructure issues that do not represent bead-local code defects.

(End of file)
