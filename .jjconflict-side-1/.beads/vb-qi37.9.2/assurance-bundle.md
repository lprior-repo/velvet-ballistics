# Assurance Bundle — vb-qi37.9.2

**bead_id**: vb-qi37.9.2
**source_checkout**: /home/lewis/src/Velvet-ballistics
**isolated_workspace**: /home/lewis/src/vb-qi37-9-2
**commit_or_change**: git worktree isolation

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| F64 add/sub/mul/neg finiteness | POST-001,POST-002,POST-003,POST-005 | Kani: 4 harnesses PASS; proptest: 38 f64 tests PASS | proof-review.md APPROVED | COVERED |
| F64 div by zero → NonFiniteFloat | POST-004,ERR-002 | Kani PO-002 PASS; proptest f64_div PASS | proof-review.md APPROVED | COVERED |
| I64 div by zero → DivisionByZero | ERR-002 | Kani PO-002 PASS | proof-review.md APPROVED | COVERED |
| NaN cannot enter system | INV-001,INV-003 | FiniteF64::new rejects NaN/Inf (14 proptest tests); Kani PO-001 PASS | domain-model-review.md APPROVED | COVERED |
| F64 comparison semantics | POST-006 | `f64_comparison_nan_yields_false` test PASS (State 12 repair) | black-hat-review.md APPROVED | COVERED |
| Stack bounds ≤ 64 | INV-004,POST-008 | stack_overflow 3 tests PASS | machine-gate-report.md PASS | COVERED |
| I64 overflow → IntegerOverflow | ERR-003 | integer_overflow 4 tests PASS | machine-gate-report.md PASS | COVERED |
| Clippy clean | all | clippy exit 0, 0 warnings | machine-gate-report.md PASS | COVERED |
| Build clean | all | cargo build exit 0 | machine-gate-report.md PASS | COVERED |
| Kani 7/7 harnesses | POST-001..007 | 639 checks, 0 failures | formal-verification-report.md APPROVED | COVERED |

---

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| PO-001 (F64 finiteness add/sub/mul/neg) | Kani | `cargo kani --package vb_expr` | 7 harnesses | PASS (7/7, 639 checks, 0 failed) | None |
| PO-002 (F64/0 → NonFiniteFloat) | Kani | `cargo kani --package vb_expr` | f64_div.rs | PASS | None |
| PO-003 (FiniteF64 ctor rejects NaN/Inf) | Proptest | `cargo test -p vb_core finite_f64` | 14 tests | PASS | None |
| PO-004 (FiniteF64 accepts all finite) | Proptest | `cargo test -p vb_core finite_f64_accepts` | 1 test | PASS | None |
| PO-005/006/007 (F64 arithmetic) | Proptest | `cargo test -p vb_expr f64` | 38 tests | PASS | None |
| PO-008 (F64/0 → NonFiniteFloat) | Proptest | `cargo test -p vb_expr f64_div` | div tests | PASS | None |
| PO-010 (NaN comparison) | Proptest | `cargo test -p vb_expr f64_comparison_nan_yields_false` | 1 test | PASS | None |
| PO-011 (stack bounds) | Proptest | `cargo test -p vb_expr stack_overflow` | 3 tests | PASS | None |
| PO-012 (I64 overflow) | Proptest | `cargo test -p vb_expr integer_overflow` | 4 tests | PASS | None |
| PO-014 (clippy) | Static scan | `cargo clippy -p vb_expr -p vb_core --lib --bins -- -D warnings` | — | PASS (exit 0) | None |
| PO-015 (build) | Static scan | `cargo build -p vb_expr -p vb_core` | — | PASS (exit 0) | None |
| WO-001 (fuzz) | Waiver | N/A | No harness | WAIVED | Formal waiver: serde roundtrip tests compensate |
| NO-001 (Miri) | Blocked | `forbid(unsafe_code)` | No unsafe code | blocked_tooling | cargo careful + Kani compensate |
| NO-002 (TLA+) | N/A | Pure computation | N/A | not_applicable | No temporal behavior |
| NO-003 (Verus) | N/A | Simple newtype | N/A | not_applicable | Kani+proptest sufficient |
| NO-004 (Flux) | N/A | No dependent types | N/A | not_applicable | Not needed |
| NO-005 (Loom) | N/A | Single-threaded eval | N/A | not_applicable | No concurrency |

---

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| vb_expr full suite | `cargo test -p vb_expr` | 339 tests | PASS |
| vb_core full suite | `cargo test -p vb_core` | 17 tests + 1 doctest | PASS |
| Kani formal verification | `cargo kani --package vb_expr` | 7/7 harnesses, 639 checks | PASS |
| Clippy strict gate | `cargo clippy -- -D warnings -D unsafe_code -D clippy::unwrap_used ...` | exit 0 | PASS |
| cargo build | `cargo build -p vb_expr -p vb_core` | exit 0 | PASS |
| NaN comparison test (State 12 repair) | `cargo test -p vb_expr f64_comparison_nan_yields_false` | 1 test | PASS |

---

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Contract Verification | contract-verification-review.md | APPROVED | All clauses traceable |
| Proof Review | proof-review.md | APPROVED | 7/7 Kani PASS; all LETHAL findings resolved |
| Test Plan Review | test-plan-review.md | VERDICT: APPROVED | 0 lethal, 0 major; proptest gap compensated by Kani |
| Formal Verification | formal-verification-report.md | APPROVED | All obligations PASS/WAIVED/blocked_tooling |
| Black-Hat Review | black-hat-review.md | APPROVED | All phases PASS; PO-010 NaN test confirmed |
| Machine Gate | machine-gate-report.md | PASS | clippy/build/kani/cargo-careful all PASS |

---

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| WO-001: Fuzz harness | No fuzz harness for FiniteF64 deserialization | proof-planner | N/A | Serde roundtrip tests + Kani PO-001 |
| NO-001: Miri | `forbid(unsafe_code)` on vb_expr and vb_core | State 4 | N/A | Kani 7 harnesses + cargo careful |
| DEFERRED_GLOBAL: vb_runtime build failure | chunk_001.rs missing | Outside scope | N/A | Not blocking vb_expr bead |

---

## Missing Artifact

| Artifact | Status | Impact |
|---|---|---|
| regression-diff.md | MISSING | No blocking impact. Black-hat reviewer APPROVED without it. All gates pass. |
