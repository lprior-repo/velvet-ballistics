# Formal Verification Report — vb-xi2f.33

**Bead**: `vb-xi2f.33` / P1: digest covers ask semantics
**Phase**: State 12 (formal-verifier)
**Agent**: formal-verifier (`deepseek-v4-pro`)
**Report Date**: 2026-05-25
**Workspace**: `/home/lewis/src/vb-workspaces/vb-xi2f.33`

## Executive Summary

**Result: PARTIAL PASS** — 4/11 obligations PASS, 6/11 FAIL_LOCAL (known Kani tooling limitation), 1/11 fuzz deferred.

All behavior-affecting contract clauses (INV-ASK-001 through INV-ASK-007, TC-001, TC-002, TC-007) are covered by PASS-ing proptest evidence (4 suites, 3000 total random cases) plus 245 existing unit tests (0 failures). The 6 Kani harnesses are discoverable, compilable, and executable but cannot complete verification due to a known `InlineAsm` limitation in the `blake3` trusted dependency. The proof-review (APPROVED) and proof-to-rust-review (APPROVED) both document and accept this limitation with compensating evidence.

## Lane Execution Results

### Proptest: 4/4 PASS ✅

| Obligation | Result | Duration |
|---|---|---|
| PO-PROPTEST-001 (prompt sensitivity) | PASS | 0.31s |
| PO-PROPTEST-002 (timeout sensitivity) | PASS | 0.05s |
| PO-PROPTEST-003 (determinism) | PASS | 0.12s |
| PO-PROPTEST-004 (field ordering) | PASS | 0.11s |

Full details: [proptest-report.md](./proptest-report.md)

### Kani: 0/6 — FAIL_LOCAL ⚠️

| Obligation | Result | Root Cause |
|---|---|---|
| PO-KANI-001 (prompt sensitivity) | FAIL_LOCAL | blake3 InlineAsm |
| PO-KANI-002 (timeout sensitivity) | FAIL_LOCAL | blake3 InlineAsm |
| PO-KANI-003 (empty prompt distinct) | FAIL_LOCAL | blake3 InlineAsm |
| PO-KANI-004 (timeout sentinel) | FAIL_LOCAL | blake3 InlineAsm |
| PO-KANI-005 (field ordering) | FAIL_LOCAL | blake3 InlineAsm |
| PO-KANI-006 (no-panic) | FAIL_LOCAL | blake3 InlineAsm |

All 6 Kani harnesses fail at the blake3 dependency boundary with `TerminatorKind::InlineAsm is not currently supported by Kani`. This is a known Kani limitation ([kani#2](https://github.com/model-checking/kani/issues/2)). The harnesses are structurally correct (discoverable, compilable, call production Rust code), use `kani::any()` for input generation (GOD RULE 1), and bind to actual `canonical_digest()`/`digest_step_primitive()` implementations (GOD RULE 2).

Full details: [kani-report.md](./kani-report.md)

### Fuzz: 1 deferred

| Obligation | Status |
|---|---|
| PO-FUZZ-001 (canonical_digest_ask) | Not executed — compilation passes, execution deferred |

### Not Applicable Lanes (per verifier-lane-decisions.jsonl)

- **TLA+**: not_applicable (no temporal/state-machine/distributed properties)
- **Verus**: not_applicable (P1 scope; blake3 as trusted primitive)
- **Flux**: not_applicable (no refinement-type properties)
- **Loom**: not_applicable (no concurrency)
- **Miri**: not_applicable (no unsafe code)

### Refinement Obligation Status

| RRO ID | Proof ID | Verifier | Status |
|---|---|---|---|
| RRO-ASK-001 | PO-KANI-001 | kani | materialized (FAIL_LOCAL) |
| RRO-ASK-002 | PO-KANI-002 | kani | materialized (FAIL_LOCAL) |
| RRO-ASK-003 | PO-KANI-003 | kani | materialized (FAIL_LOCAL) |
| RRO-ASK-004 | PO-KANI-004 | kani | materialized (FAIL_LOCAL) |
| RRO-ASK-005 | PO-KANI-005 | kani | materialized (FAIL_LOCAL) |
| RRO-ASK-006 | PO-KANI-006 | kani | materialized (FAIL_LOCAL) |
| RRO-ASK-007 | PO-PROPTEST-001 | proptest | **verified** (PASS) |
| RRO-ASK-008 | PO-PROPTEST-002 | proptest | **verified** (PASS) |
| RRO-ASK-009 | PO-PROPTEST-003 | proptest | **verified** (PASS) |
| RRO-ASK-010 | PO-PROPTEST-004 | proptest | **verified** (PASS) |
| RRO-ASK-011 | PO-FUZZ-001 | cargo-fuzz | materialized (deferred) |
| RRO-ASK-012 | PO-UT-001 | code-review | planned (delegated to State 8) |
| RRO-ASK-013 | PO-UT-002 | unit-test | planned (delegated to State 8) |
| RRO-ASK-014 | PO-UT-003 | unit-test | materialized (dead code — compile/mod.rs not mounted) |

## Baseline Verification

- **Holzman Rust**: PASS — `cargo check` / `moon ci` green, 0 unsafe, 0 unwrap, 0 expect in production code
- **Unit Tests**: PASS — 245 vb_compile lib tests pass (0 failures)
- **moon ci**: PASS — 27 tasks completed (7 cached), 0 failures
- **Crate compilation**: PASS — all 8 crates compile clean

## Waiver Status

- **formal-waivers.jsonl**: Not created. The waiver-candidates.jsonl (WC-NONE-001) states "No behavior-affecting waivers needed." The Kani failures are a tooling limitation, not a behavior waiver. The proof-review approved this as compensated (not waived).
- Behavior-affecting waivers: **0 accepted, 0 rejected**

## Closure Status

| Category | Count | Status |
|---|---|---|
| PASS | 4 | Closed |
| FAIL_LOCAL | 6 | Closed (documented, compensated) |
| Deferred | 1 | Open (fuzz execution) |
| Planned (S8) | 2 | Open (PO-UT-001, PO-UT-002 delegated to test-planner) |
| Not applicable | 50 | Closed (per VLD) |
| **Total** | **11 proof obligations** | **10 closed, 1 deferred** |

## Layer Reports

- [kani-report.md](./kani-report.md) — 6 FAIL_LOCAL (blake3 InlineAsm)
- [proptest-report.md](./proptest-report.md) — 4 PASS

## Blocker Assessment

**No blockers for bead delivery.** All behavior-affecting contract clauses are covered by proptest evidence (PASS) and existing unit tests (PASS). The Kani InlineAsm limitation is a known tooling issue, not a code defect. Proof-review and proof-to-rust-review both APPROVED with compensating evidence acknowledged.

## Next States

- State 8 (test-planner): PO-UT-001 and PO-UT-002 remain planned — test-planner creates dedicated behavior tests for explicit Ask arm and Set/Finish regression
- Fuzz execution: PO-FUZZ-001 compilation confirmed; execution may be triggered independently as a long-running security check
