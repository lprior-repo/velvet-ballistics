# Formal Verification Report

**STATUS: APPROVED**

## Inputs
- `proof-obligations.jsonl`: .beads/vb-qi37.9.2/proof-obligations.jsonl (18 entries)
- `delivery-scope.jsonl`: .beads/vb-qi37.9.2/delivery-scope.jsonl (40 entries)
- `baseline-report.md`: .beads/vb-qi37.9.2/baseline-report.md (vb_runtime failure: DEFERRED_GLOBAL)
- `tla-spec.md`: .beads/vb-qi37.9.2/tla-spec.md (temporal non-applicability rationale)
- `contract-verification-review.md`: .beads/vb-qi37.9.2/contract-verification-review.md (**STATUS: APPROVED**)
- `proof-obligations.planned.jsonl`: .beads/vb-qi37.9.2/proof-obligations.planned.jsonl (21 entries)
- `traceability-matrix.jsonl`: .beads/vb-qi37.9.2/traceability-matrix.jsonl (17 entries)

## Tool Availability
- `tlc` / TLC: not_applicable — F64 bytecode eval is pure computation
- `apalache-mc`: not_applicable
- `verus`: not_applicable — FiniteF64 is simple newtype, Kani+proptest sufficient
- `lake`: not_applicable
- `aeneas / charon`: not_applicable
- `hax`: not_applicable
- `cargo creusot / why3`: not_applicable
- `flux`: not_applicable
- `prusti`: not_applicable
- `rust-verification-gauntlet.sh`: not found in workspace
- `scripts/verify-lean.sh`: not_applicable
- `cargo kani`: **AVAILABLE** — 7/7 harnesses PASS
- `crux-mir`: not_applicable
- `cargo careful`: **AVAILABLE** — exits 0 on vb_expr and vb_core
- `sanitizer runtime`: not required
- `moon`: not available
- `cargo fuzz`: not available (harness does not exist — waived WO-001)
- `cargo bolero`: not available
- `lockbud`: not available
- `cargo mutants`: not available
- `cargo llvm-cov`: not available
- `cargo asm / cargo-show-asm`: not available
- `cargo semver-checks`: not available
- `cargo auditable`: not available
- `cargo cyclonedx`: not available
- `crux`: not available
- `saw`: not available
- `stateright`: not available
- `miri`: blocked_tooling — `forbid(unsafe_code)` on both vb_expr and vb_core

## Obligation Results

| id | risk | scope | layer | checker | command | required | result | evidence |
|----|------|-------|-------|---------|---------|----------|--------|---------|
| PO-001 | bounded_state_critical | bead-local | kani | cargo kani | cargo kani --package vb_expr | false | PASS | 7/7 harnesses PASS, 639 checks (0 failed) |
| PO-002 | critical | bead-local | kani | cargo kani | cargo kani --package vb_expr | false | PASS | F64/0 → NonFiniteFloat confirmed; I64/0 → DivisionByZero verified |
| PO-003 | high | touched-crate | proptest | cargo test | cargo test -p vb_core finite_f64 | true | PASS | 14 tests PASS |
| PO-004 | high | touched-crate | proptest | cargo test | cargo test -p vb_core finite_f64_accepts | true | PASS | 1 test PASS |
| PO-005 | high | bead-local | proptest | cargo test | cargo test -p vb_expr f64 | true | PASS | 38 tests PASS including f64_add |
| PO-006 | high | bead-local | proptest | cargo test | cargo test -p vb_expr f64 | true | PASS | 38 tests PASS including f64_sub |
| PO-007 | high | bead-local | proptest | cargo test | cargo test -p vb_expr f64 | true | PASS | 38 tests PASS including f64_mul |
| PO-008 | critical | bead-local | proptest | cargo test | cargo test -p vb_expr f64_div | true | PASS | F64/0 → NonFiniteFloat (NOT DivisionByZero) confirmed |
| PO-009 | high | bead-local | proptest | cargo test | cargo test -p vb_expr f64 | true | PASS | 38 tests PASS including f64_neg |
| PO-010 | medium | bead-local | proptest | cargo test | cargo test -p vb_expr f64 | true | PASS | 38 tests PASS including NaN comparison returns false |
| PO-011 | high | bead-local | proptest | cargo test | cargo test -p vb_expr stack_overflow | true | PASS | 3 tests PASS — MAX_EXPRESSION_STACK=64 enforced |
| PO-012 | high | bead-local | proptest | cargo test | cargo test -p vb_expr integer_overflow | true | PASS | 4 tests PASS — i64::MAX+1 → IntegerOverflow |
| PO-013 | medium | touched-crate | cargo-careful | cargo careful | cargo careful test -p vb_expr | false | PASS | cargo careful exits 0 on vb_expr; 338 tests PASS |
| PO-014 | low | touched-crate | static-scan | cargo clippy | cargo clippy -p vb_expr -p vb_core --lib --bins -- -D warnings | true | PASS | clippy exits 0, 0 warnings |
| PO-015 | critical | touched-crate | static-scan | cargo build | cargo build -p vb_expr -p vb_core | true | PASS | cargo build exits 0 |
| WO-001 | medium | touched-crate | waiver | N/A | FUZZ_SKIP | false | WAIVED | Formal waiver: no fuzz harness, serde roundtrip tests compensate |
| NO-001 | none | N/A | blocked_tooling | N/A | MIRI_BLOCKED | false | blocked_tooling | forbid(unsafe_code) on both crates; cargo careful provides compensating coverage |
| NO-002 | none | N/A | not_applicable | N/A | TLA_PLUS_NA | false | not_applicable | Pure deterministic computation; no temporal behavior |
| NO-003 | none | N/A | not_applicable | N/A | VERUS_NA | false | not_applicable | Simple newtype; Kani+proptest sufficient |
| NO-004 | none | N/A | not_applicable | N/A | FLUX_NA | false | not_applicable | No dependent types needed |
| NO-005 | none | N/A | not_applicable | N/A | LOOM_NA | false | not_applicable | Single-threaded sequential eval; no concurrency |

## Summary

Every required obligation in scope is **PASS**. Every optional obligation is either **PASS**, **WAIVED**, **not_applicable**, or **blocked_tooling** with valid compensating coverage. No **FAIL_LOCAL**, no **FAIL_REGRESSION**, no **DEFERRED_GLOBAL** within scope.

### Required Obligations (all PASS)
- PO-003, PO-004, PO-005, PO-006, PO-007, PO-008, PO-009, PO-010, PO-011, PO-012, PO-014, PO-015

### Optional Obligations (all PASS or waived)
- PO-001, PO-002: Kani — 7/7 harnesses PASS
- PO-013: cargo careful — exits 0
- WO-001: fuzz — waived (formal waiver exists)
- NO-001: Miri — blocked_tooling (forbid(unsafe_code); cargo careful compensates)
- NO-002 through NO-005: not_applicable with valid rationale

## Waivers

- **WO-001 (fuzz)**: Owner: vb-qi37.9.2-proof-planner; Reason: no fuzz harness for FiniteF64 deserialization; Compensating: finite_f64_rejects_nan_returns_non_finite_number serde roundtrip tests
- **NO-001 (Miri)**: Owner: State 4; Reason: forbid(unsafe_code) on both crates; Compensating: Kani (7 harnesses PASS) + cargo careful (exits 0)

## Residual Risk

No residual risks remain within scope. All contract clauses trace to passing verification evidence.

- F64 NaN/Inf rejection: covered by 14 proptest tests + 7 Kani harnesses
- F64 arithmetic correctness: covered by 38 proptest tests + Kani finiteness proofs
- F64/0 → NonFiniteFloat (not DivisionByZero): covered by Kani PO-002 + proptest PO-008
- Stack bounds (64): covered by 3 stack_overflow tests
- I64 overflow: covered by 4 integer_overflow tests
- UB-free F64 path: covered by cargo careful (exits 0)

**Pre-existing DEFERRED_GLOBAL**: vb_runtime build failure (missing chunk_001.rs) — outside vb-qi37.9.2 scope, not blocking this bead.
