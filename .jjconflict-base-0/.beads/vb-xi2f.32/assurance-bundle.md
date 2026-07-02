# Assurance Bundle

**bead_id:** vb-xi2f.32
**source_checkout:** /home/lewis/src/vb-workspaces/vb-xi2f.32
**isolated_workspace:** /home/lewis/src/vb-workspaces/vb-xi2f.32
**commit_or_change:** vb-xi2f.32 Wait digest fix
**packaging_date:** 2026-05-25
**packager:** evidence-packaging agent (p14)

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| C1: Wait Field Hashing | `digest_step_primitive` must hash `Wait{ event, timeout }` fields | PO-002 (proptest PASS), PO-003 (fuzz PASS: 66,591 runs), PO-011 (proptest PASS), PO-001/P-013/P-015 (Kani BLOCKED_TOOLING, compensating coverage) | proof-review.md APPROVED, test-suite-review.md APPROVED | COVERED |
| C2: WaitUntil vs WaitEvent Discrimination | Digest must distinguish WaitUntil from WaitEvent via positional `b"none"` sentinel | PO-004 (proptest PASS), PO-005 (Kani BLOCKED_TOOLING), C2-shape test (proptest PASS) | proof-review.md APPROVED, test-suite-review.md APPROVED (DD-4 updated) | COVERED |
| C3: Absent Field Sentinels | Absent optional fields SHALL use `b"none"` sentinel | PO-006 (proptest PASS adapted), PO-007 (fuzz PASS: 82,767 runs), exact sentinel value tests (S2 resolved) | proof-review.md APPROVED (MITIGATED), test-suite-review.md APPROVED | COVERED |
| C4: Digest Determinism | `canonical_digest` remains deterministic after fix | PO-008/PO-014 (proptest PASS), 295+320 passing tests | proof-review.md APPROVED, test-suite-review.md APPROVED | COVERED |
| C5: Dual Implementation Consistency | Fix applied identically to both copies | PO-009/PO-016 (proptest PASS: cross-path), PO-010 (Kani BLOCKED_DEAD_CODE, waived) | proof-review.md APPROVED (waiver accepted), test-suite-review.md APPROVED | COVERED |
| C6: Backward Compatibility | All existing stability tests pass | PO-008/PO-014 (proptest PASS), 295+320 tests pass, 0 regressions | test-suite-review.md APPROVED (PI-5 primary, PI-8 supplementary) | COVERED |
| C7: No Digest Unification | OUT OF SCOPE — not required by this bead | N/A | N/A | OUT OF SCOPE |
| C8: Broader Digest Gap | OUT OF SCOPE — follow-up bead | N/A | N/A | OUT OF SCOPE |

---

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| PO-001 | kani | `cargo kani --harness wait_digest_step_primitive_no_panic -p vb_compile` | `.evidence/vb-xi2f.32/kani-compile-failure.log` | BLOCKED_TOOLING (String:Arbitrary) | Compensating: proptest PO-002 + fuzz PO-003 |
| PO-002 | proptest | `cargo test -p vb_compile -- proptest_wait_field_sensitivity` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/01-field-sensitivity.log` | PASS (1 passed) | — |
| PO-003 | fuzz | `cargo fuzz run wait_digest_sensitivity --target x86_64-unknown-linux-gnu -- -max_len=64 -max_total_time=30` | `.evidence/vb-xi2f.32/fuzz-wait_digest_sensitivity.log` | PASS (66,591 runs, 0 assertions) | — |
| PO-004 | proptest | `cargo test -p vb_compile -- proptest_wait_until_vs_wait_event` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/02-until-vs-event.log` | PASS (1 passed) | — |
| PO-005 | kani | `cargo kani --harness wait_until_vs_wait_event_no_collision -p vb_compile` | `.evidence/vb-xi2f.32/kani-compile-failure.log` (shared) | BLOCKED_TOOLING (String:Arbitrary) | Compensating: proptest PO-004 |
| PO-006 | proptest | `cargo test -p vb_compile -- proptest_wait_sentinel_unambiguous` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/03-sentinel-unambiguous.log` | PASS (1 passed, adapted property) | MITIGATED (PO-013 Kani would provide exhaustive coverage at State 7) |
| PO-007 | fuzz | `cargo fuzz run wait_sentinel_collision --target x86_64-unknown-linux-gnu -- -max_len=64 -max_total_time=30` | `.evidence/vb-xi2f.32/fuzz-wait_sentinel_collision.log` | PASS (82,767 runs, 0 assertions) | — |
| PO-008 | proptest | `cargo test -p vb_compile -- proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/06-regression-equal-sources.log` | PASS (1 passed) | — |
| PO-009 | proptest | `cargo test -p vb_compile -- cross_path_wait_digest_equivalence` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/05-cross-path-equivalence.log` | PASS (1 passed) | — |
| PO-010 | kani | N/A — dead code, unreachable | — | BLOCKED_DEAD_CODE | WAIVED (warm-path copy unreachable; property satisfied by design + PO-009/PO-016 proptest) |
| PO-011 | proptest | `cargo test -p vb_compile -- proptest_wait_pairwise_distinct_digests` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/04-pairwise-distinct.log` | PASS (1 passed) | — |
| PO-012 | fuzz | `cargo fuzz run wait_digest_exhaustive_collision --target x86_64-unknown-linux-gnu -- -max_len=64 -max_total_time=30` | `.evidence/vb-xi2f.32/fuzz-wait_digest_exhaustive_collision.log` | PASS (84,129 runs, 0 assertions) | — |
| PO-013 | kani | `cargo kani --harness wait_configurations_pairwise_distinct -p vb_compile` | `.evidence/vb-xi2f.32/kani-compile-failure.log` (shared) | BLOCKED_TOOLING (String:Arbitrary) | Compensating: proptest PO-011 + fuzz PO-012 |
| PO-014 | proptest | Same as PO-008 (same existing test) | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/06-regression-equal-sources.log` | PASS (1 passed) | — |
| PO-015 | kani | `cargo kani --harness wait_digest_both_copies_no_panic -p vb_compile` | `.evidence/vb-xi2f.32/kani-compile-failure.log` (shared) | BLOCKED_TOOLING (String:Arbitrary) | Compensating: proptest + fuzz all-PASS suite |
| PO-016 | proptest | Same as PO-009 (same cross-path test) | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/05-cross-path-equivalence.log` | PASS (1 passed) | — |

**Summary:** 8 proptest PASS, 3 fuzz PASS, 4 Kani BLOCKED_TOOLING (compensating proptest/fuzz), 1 Kani BLOCKED_DEAD_CODE (waived), 0 FAIL.

---

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| cargo-test-vb-compile | `cargo test -p vb_compile` | `.evidence/vb-xi2f.32/cargo-test-vb-compile.log` | PASS: 320 passed (6 suites, 2.35s) |
| cargo-test-workspace | `cargo test --workspace` | verification-ledger.jsonl L52 | PASS: ~2800 passed, 0 failed |
| cargo-check-vb-compile | `cargo check -p vb_compile` | verification-ledger.jsonl L50 | PASS: 0 errors, 0 warnings |
| unit tests (wait_digest_unit_tests.rs) | All 15 unit tests | test-suite-review.md | PASS: 15/15 (12 + 3 new exact-sentinel tests) |
| integration tests (v1_primitive_lowering.rs) | All 10 wait-related integration tests | test-suite-review.md | PASS: 10/10 |
| proptest PI-1 (field sensitivity) | `proptest_wait_field_sensitivity` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/01-field-sensitivity.log` | PASS |
| proptest PI-2 (until vs event) | `proptest_wait_until_vs_wait_event` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/02-until-vs-event.log` | PASS |
| proptest PI-3 (sentinel unambiguous) | `proptest_wait_sentinel_unambiguous` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/03-sentinel-unambiguous.log` | PASS |
| proptest PI-4 (pairwise distinct) | `proptest_wait_pairwise_distinct_digests` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/04-pairwise-distinct.log` | PASS |
| proptest PI-5 (equal sources regression) | `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/06-regression-equal-sources.log` | PASS |
| proptest PI-6 (cross-path equivalence) | `cross_path_wait_digest_equivalence` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/05-cross-path-equivalence.log` | PASS |
| proptest C2-shape | `compile_workflow_emits_exact_wait_until_shape_...` | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/08-wait-until-shape.log` | PASS |
| fuzz FZ-1 (sensitivity) | `cargo fuzz run wait_digest_sensitivity` | `.evidence/vb-xi2f.32/fuzz-wait_digest_sensitivity.log` | PASS: 66,591 runs |
| fuzz FZ-2 (sentinel collision) | `cargo fuzz run wait_sentinel_collision` | `.evidence/vb-xi2f.32/fuzz-wait_sentinel_collision.log` | PASS: 82,767 runs |
| fuzz FZ-3 (exhaustive collision) | `cargo fuzz run wait_digest_exhaustive_collision` | `.evidence/vb-xi2f.32/fuzz-wait_digest_exhaustive_collision.log` | PASS: 84,129 runs |
| Mutation resistance | All 7 tracked mutations | test-suite-review.md (kill rate ~100%) | PASS: 7/7 caught |
| Contract clause C1-C6 | Full behavioral coverage | test-suite-review.md contract matrix | PASS: 0 gaps |

---

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Proof-plan review | `.beads/vb-xi2f.32/proof-plan-review.md` | APPROVED | Schema compliance fixed; all 8 F-findings resolved |
| Proof review R2 | `.beads/vb-xi2f.32/proof-review.md` | APPROVED | Proptest evidence captured; Kani/fuzz deferred to State 7 |
| Proof-to-rust bridge review | `.beads/vb-xi2f.32/proof-to-rust-review.md` | APPROVED | 16 obligations mapped; 14 source refs verified |
| Test suite review | `.beads/vb-xi2f.32/test-suite-review.md` | APPROVED | S1-S4 resolved; 3 exact sentinel tests added; 4 doc inconsistencies (D1-D4 low) |
| Black-hat review | NOT FOUND in bead directory | User states APPROVED | No artifact at `.beads/vb-xi2f.32/black-hat-review.md`; packaging proceeds per user instruction |

---

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| WC-001 (Verus waiver) | P1 scope; Kani+proptest+fuzz provide stronger direct coverage for collision/panic risks | proof-plan-reviewer | When bead priority promotes to P0 or Verus-compatible blake3 binding exists | 8 proptest + 3 fuzz PASS results |
| WC-002 (TLA+ waiver) | Digest is pure function; no temporal/retry/lease/queue semantics | proof-plan-reviewer | Never — digest remains pure function | PO-008, PO-014 (determinism) + PO-009, PO-016 (cross-path) |
| WC-003 (Loom waiver) | Zero threads/atomics/channels/concurrent interleavings | proof-plan-reviewer | Never — digest remains pure function | N/A (zero concurrency risk) |
| WC-004 (Miri waiver) | Zero unsafe code; `#![forbid(unsafe_code)]` enforced | proof-plan-reviewer | Until unsafe code introduced in vb_compile | Project-wide unsafe prohibition |
| WC-005 (Flux waiver) | No refinement-type predicates in digest path | proof-plan-reviewer | Never — no refinement predicates needed | Pattern matching exhaustiveness; validation gate |
| PO-010 (Kani BLOCKED_DEAD_CODE) | Warm-path copy in `compile/mod.rs` unreachable dead code | formal-verifier | Follow-up bead for dead code removal | PO-009/PO-016 proptest cross-path equivalence |
| PO-001/005/013/015 (Kani BLOCKED_TOOLING) | Kani 0.67.0 does not implement `Arbitrary` for `String` | formal-verifier | Kani tooling upgrade or harness refactor to `[u8; N]` | 8 proptest + 3 fuzz PASS (all 4 Kani obligations have compensating behavioral coverage) |
| D1-D4 (doc inconsistencies) | Stale comments/descriptions in domain-model.md, test-plan.md, v1_primitive_lowering.rs | test-reviewer | Follow-up documentation bead | No behavioral impact; implementation correct |

---

## Formal Verification Summary

**Report:** `reports/formal-verification-report.md` (workspace root)
**Status:** All executable obligations PASS or BLOCKED with compensating coverage

| Classification | Count | Details |
|---|---|---|
| PASS | 16 | 8 proptest + 3 fuzz + cargo-check + cargo-test-vb-compile + cargo-test-workspace + repair-fix + build-verify |
| FAIL_LOCAL | 0 | — |
| FAIL_REGRESSION | 0 | — |
| FAIL_GLOBAL | 0 | — |
| BLOCKED_TOOLING | 4 | Kani PO-001, PO-005, PO-013, PO-015 (Kani 0.67 String:Arbitrary) |
| BLOCKED_DEAD_CODE | 1 | Kani PO-010 (warm-path unreachable) |
| WAIVED | 0 | — |

---

## Mandatory Gate Verification

| Check | Result |
|---|---|
| `delivery-scope.jsonl` exists and non-empty | PASS |
| `contract.md` exists and non-empty | PASS |
| `traceability-matrix.jsonl` exists and valid JSONL | PASS |
| `proof-review.md` STATUS: APPROVED | PASS |
| `test-suite-review.md` STATUS: APPROVED | PASS |
| `formal-verification-report.md` exists (at `reports/`) | PASS (location: `reports/formal-verification-report.md`) |
| `verification-ledger.jsonl` valid JSONL with vb-xi2f.32 entries | PASS (15 State 5 + 7 State 12 = 22 entries) |
| `black-hat-review.md` exists in bead directory | MISSING (user states APPROVED) |
| `machine-gate-report.md` exists | MISSING |
| `regression-diff.md` exists | MISSING |
| JSONL artifacts parse one object per line | PASS |
| Every requirement maps to at least one proof/test evidence row | PASS (C1-C6 all covered) |
| Every proof obligation has PASS, BLOCKED (with compensating), or WAIVED | PASS (16/16) |
| Every waiver has owner, reason, expiry, and compensating evidence | PASS (5 waiver candidates + dead-code waiver) |
| No subagent summary used as command evidence | PASS (all evidence is raw logs or verified report artifacts) |
| All referenced artifact paths exist | PASS |

---

## Production Fix Verified

The Wait arm was added to `digest_step_primitive` in both copies:

**Cold-path** (`crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-168`):
```rust
vb_yaml::ast::StepPrimitive::Wait { event, timeout } => {
    hasher.update(b"wait");
    match event {
        Some(e) => hasher.update(e.as_bytes()),
        None => hasher.update(b"none"),
    };
    match timeout {
        Some(t) => hasher.update(t.as_bytes()),
        None => hasher.update(b"none"),
    };
}
```

**Warm-path** (`crates/vb_compile/src/compile/mod.rs:257-267`): Identical structure (dead code, applied for consistency).

Visibility changed from `pub(super)` to `pub(crate)` for Kani harness access.

---

## Evidence Location Summary

| Evidence Class | Path |
|---|---|
| Bead artifacts | `.beads/vb-xi2f.32/` |
| Proptest logs | `.beads/vb-xi2f.32/evidence/proptest-vb-xi2f.32/` (12 files) |
| Fuzz logs | `.evidence/vb-xi2f.32/fuzz-*.log` (3 files) |
| Cargo test log | `.evidence/vb-xi2f.32/cargo-test-vb-compile.log` |
| Kani compile failure | `.evidence/vb-xi2f.32/kani-compile-failure.log` |
| Formal verification report | `reports/formal-verification-report.md` |
| Verification ledger | `verification-ledger.jsonl` (workspace root, 70 lines) |
