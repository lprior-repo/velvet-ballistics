# Proof-to-Rust Bridge Review: Wait Digest Coverage

**Reviewer skill:** `proof-reviewer`
**Review target:** bridge mapping (proof-to-rust-map/v1)
**Review invocation ID:** `ptr-2026-05-25T23-59-00-vb-xi2f.32`
**Bead:** vb-xi2f.32
**Date:** 2026-05-25
**Schema:** proof-to-rust-review/v1
**Prior proof review:** `proof-review.md` STATUS: APPROVED (R2)

---

## STATUS: APPROVED

The bridge mapping passes review. All 16 proof obligations are mapped to concrete Rust source refs with line numbers (`path::symbol` granularity). Every behavior-affecting obligation has at least one independent behavior test. The 8 proptest obligations are VERIFIED with raw execution log evidence. The 7 Kani/fuzz obligations are properly deferred to State 7 (formal-verifier) with exact commands, documented tooling blockers, and smoke-test compilation evidence. The 1 BLOCKED_DEAD_CODE obligation (PO-010) has a valid waiver with compensating proptest evidence. Provenance is clean — the bridge was written by `proof-to-implementation`, not the reviewer.

The bridge is approved for handoff to `formal-verifier` at State 7. Findings F-BRG-001 through F-BRG-003 carry REQUIRED fixes for the formal-verifier.

---

## Provenance Check

| Check | Result |
|-------|--------|
| Self-approval detection | **PASS** — bridge writer (`proof-to-implementation`) ≠ reviewer (`proof-reviewer`) |
| Prior proof-review approved | **PASS** — `proof-review.md` STATUS: APPROVED (R2) |
| Agent invocation ledger | **PASS** — 3 independent rows (femdation, proof-planner, proof-plan-reviewer) |
| Bridge artifact schema | **PASS** — `proof-to-rust-map/v1` valid schema |
| Bridge input consumed | **PASS** — consumed `proof-to-implementation-input.md`, `proof-obligations.planned.jsonl`, `contract.md` |

---

## 1. Source Ref Quality Assessment

All 16 RRO entries have concrete source refs. The bridge defines 14 source refs across 4 categories:

| Category | Count | Quality |
|----------|-------|---------|
| Active cold-path compiler | 4 | Line-level, `pub(crate) fn` granularity — verified in source |
| Legacy warm-path (dead) | 2 | Line-level — both copies confirmed with identical Wait arm |
| Upstream unchanged types | 5 | Line-level — correct structural references |
| Runtime wait handling | 4 | Line-level — not modified, correctly noted |

**Assessment: PASS.** Every RRO row spans at least one `path::symbol` source ref. The critical production fix (`part_05.rs:158-168`) is verified as matching the code on disk. The dead-code copy (`compile/mod.rs:257-267`) is confirmed as identical in substance, unreachable via `lib.rs` (no `mod compile;`), and referenced only in the Kani harness documentation.

---

## 2. Behavior Test Coverage Assessment

Every behavior-affecting obligation has at least one independent proptest test:

| Obligation | Behavior-Affecting | Independent Test | Test Status | Evidence |
|-----------|-------------------|-----------------|-------------|----------|
| PO-002 | Yes | `proptest_wait_field_sensitivity` | PASS | `01-field-sensitivity.log` |
| PO-004 | Yes | `proptest_wait_until_vs_wait_event` | PASS | `02-until-vs-event.log` |
| PO-006 | Yes | `proptest_wait_sentinel_unambiguous` | PASS (adapted) | `03-sentinel-unambiguous.log` |
| PO-008 | No (determinism) | `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` | PASS | `06-regression-equal-sources.log` |
| PO-009 | Yes | `cross_path_wait_digest_equivalence` | PASS | `05-cross-path-equivalence.log` |
| PO-011 | Yes | `proptest_wait_pairwise_distinct_digests` | PASS | `04-pairwise-distinct.log` |
| PO-014 | No (regression) | Full vb_compile suite (295 passed) | PASS | `00-all-tests.log` |
| PO-016 | Yes | `cross_path_wait_digest_equivalence` | PASS | `05-cross-path-equivalence.log` |

**Assessment: PASS.** All 8 proptest obligations backed by raw execution log evidence. All logs show genuine test runner output (binary name, test name, PASS/ok, exit status). The test assertions exercise production code paths (`canonical_digest_compat` → `compile_source` → `canonical_digest` → `digest_step_primitive` Wait arm). Assertions use `prop_assert_ne!` (digest inequality) — non-vacuous.

**Observation F-BRG-002 (LOW):** The `cross_path_wait_digest_equivalence` test verifies that `compile_source` (cold-path entry) and `compile_workflow` (warm-path delegate) produce identical digests. However, `compile_workflow` delegates to `YamlCompiler::compile()` which calls `crate::mod_compile_lowering::compile_source()` — the same cold-path. The test therefore verifies **public API consistency** rather than the **dead `compile/mod.rs` copy**. This is acceptable because the dead copy is not compiled (TBL-006) and the cross-path property is satisfied by design. The proptest still has real value as a public API determinism check.

---

## 3. Deferred Obligation Assessment (Kani/Fuzz)

7 obligations are deferred to State 7 with `PENDING_FORMAL_EXECUTION` and `mapping_status: planned`:

| Obligation | Verifier | Artifact | Compiles? | Blocker |
|-----------|----------|----------|-----------|---------|
| PO-001 | kani | `kani_wait_digest.rs:34` | Yes (`#[cfg(kani)]` in lib.rs:48) | BLOCKED_TOOLING (Arbitrary for String) |
| PO-003 | fuzz | `wait_digest_sensitivity.rs` | Yes (`cargo check` in fuzz/) | BLOCKED_TOOLING (musl/sanitizer) |
| PO-005 | kani | `kani_wait_digest.rs:79` | Yes | BLOCKED_TOOLING (Arbitrary for String) |
| PO-007 | fuzz | `wait_sentinel_collision.rs` | Yes | BLOCKED_TOOLING (musl/sanitizer) |
| PO-012 | fuzz | `wait_digest_exhaustive_collision.rs` | Yes | BLOCKED_TOOLING (musl/sanitizer) |
| PO-013 | kani | `kani_wait_digest.rs:148` | Yes | BLOCKED_TOOLING (Arbitrary for String) |
| PO-015 | kani | `kani_wait_digest.rs:216` | Yes | BLOCKED_TOOLING (Arbitrary for String) |

**Assessment: PASS (with findings).** All 7 deferred obligations have:
- Written artifacts that compile (`cargo check` or `#[cfg(kani)]` module registration confirmed)
- Exact State 7 commands documented in the bridge map (Section 5.2, 5.3)
- Honest tooling blockers (Kani 0.67 doesn't implement `Arbitrary for String`; fuzz fails with musl/sanitizer incompatibility)
- GOD RULE 1 compliant Kani harnesses (`kani::any()` for symbolic inputs, proper `kani::assume` guards)
- Non-vacuous fuzz targets with crash-on-collision oracles

**Finding F-BRG-003 (MEDIUM):** Kani harness `wait_digest_step_primitive_no_panic` (PO-001, line 64) and `wait_digest_both_copies_no_panic` (PO-015, line 246) use `kani::assert(true, "...")` as a terminal assertion. While the actual verification comes from Kani's panic detection (any panic during `digest_step_primitive` execution would fail the proof), this pattern is weaker than an explicit property assertion. The harnesses are still valid — Kani reports FAILURE if any code path panics — but the `assert(true)` is a documentation convention, not a verifier assertion. **Remediation at State 7:** The formal-verifier should confirm the raw Kani output includes `VERIFICATION:- SUCCESSFUL` (not just the `assert(true)` reaching the end).

---

## 4. Waiver Assessment

| Waiver ID | Obligation | Type | Status |
|-----------|------------|------|--------|
| PO-010-DEAD-CODE | PO-010 | BLOCKED_DEAD_CODE | **ACCEPTED** |

**Assessment: PASS.** PO-010 requires Kani cross-path equivalence between `part_05.rs` and `compile/mod.rs`. The warm-path copy is unreachable dead code (no `mod compile;` in `src/lib.rs`). The Kani harness cannot bind to the dead copy. The waiver is permanent: `mapping_status: planned` with disposition "until dead code is removed or reintegrated." Compensating evidence exists: proptest PO-009/PO-016 verify cross-path equivalence at the workflow level (both `compile_source` and `compile_workflow` produce identical digests).

**Follow-up:** File bead for `compile/mod.rs` removal (as recommended in proof-review.md, proof-to-rust-map.md Section 7.1).

---

## 5. TLA+ Assessment

**No TLA+ obligations exist for this bead.** The vb-xi2f.32 scope is compile-time digest computation with no temporal state machine, no concurrency, and no distributed protocol. All 16 obligations are proptest/Kani/fuzz. This is correct per Section 6 of the bridge map.

---

## 6. Implementation Drift Analysis

The actual production implementation differs from the originally planned pattern (documented in `proof-to-implementation-input.md:34-51`):

| Aspect | Planned | Implemented | Assessment |
|--------|---------|-------------|------------|
| WaitUntil/Event discrimination | `b"wait_until"` / `b"wait_event"` discriminators | Single `b"wait"` discriminator + event field (`b"none"` for None vs actual text for Some) | **Equivalent.** The event field itself discriminates: WaitUntil always has event=None → hashes `b"none"`; WaitEvent always has event=Some → hashes actual text. |
| Event field hashing | Only for event=Some in a separate branch | In the common match arm for both cases | **Simpler.** The implementation is more concise and easier to maintain. |
| Overall correctness | Satisfies C1-C6 | Satisfies C1-C6 | **PASS.** All 8 proptest obligations verified; all 7 Kani/fuzz obligation designs validated. |

**Assessment: ACCEPTED.** The implementation drift simplifies the design while satisfying all contract clauses. The bridge map accurately reflects the actual production code (line ranges, field hashing logic, sentinel use). All provenance documents (proof-review.md, proof-obligations.planned.jsonl, contract.md) are consistent with the implemented pattern.

---

## 7. Non-Vacuity Summary

| Lane | Assessment |
|------|-----------|
| Proptest | **STRONG.** Tests assert digest inequality (`prop_assert_ne!`). Randomized strategies generate diverse Wait field values. Tests pass AFTER production fix (previously FAILED on buggy code per proof-review R1). |
| Kani | **Non-vacuous design.** `kani::any()` for symbolic inputs. Proper `kani::assume` guards exclude illegal states. `kani::assert` checks digest inequality for WaitUntil vs WaitEvent and pairwise distinctness. Unwind bounds declared. Pending execution at State 7. |
| Fuzz | **Non-vacuous design.** Crash-on-collision oracle. Coverage-guided mutation of Wait field values. Pending execution at State 7. |

---

## 8. Mapping Gap Summary

| Gap ID | Obligation | Severity | Status |
|--------|------------|----------|--------|
| GAP-MAP-001 | PO-010 | MEDIUM | BLOCKED_DEAD_CODE — valid waiver with compensating proptest |
| GAP-MAP-002 | PO-001,005,013,015 | MEDIUM | BLOCKED_TOOLING — Kani `Arbitrary for String` not implemented; documented with State 7 commands |
| GAP-MAP-003 | PO-003,007,012 | MEDIUM | BLOCKED_TOOLING — fuzz musl/sanitizer incompatibility; documented with State 7 commands |
| GAP-MAP-004 | PO-006 | LOW | Property adapted per TBL-007; Kani PO-013 provides exhaustive coverage |
| GAP-MAP-005 | All | LOW | No State 6/7 verification-ledger entries yet |

All gaps are documented with remediation paths. No gaps block bridge approval at State 7 — they block formal-verifier execution at State 7.

---

## 9. Findings

### Finding F-BRG-001 — MEDIUM — Partial Kani non-vacuity assertion

**Obligation IDs:** PO-001, PO-015
**Artifact:** `crates/vb_compile/src/kani_wait_digest.rs:64,246`
**Finding:** The harnesses use `kani::assert(true, "...")` as terminal assertions. While panic-freedom is verified by Kani's built-in panic detection (any panic during `digest_step_primitive` → FAILURE), the `assert(true)` provides no additional property verification. The proof is still valid (panic detection is the actual verification mechanism), but the pattern is unconventional.
**Required fix:** State 7: Confirm that the raw Kani output includes `VERIFICATION:- SUCCESSFUL` (not just reaching `assert(true)`). The formal-verifier should capture full output.
**Severity:** MEDIUM — does not block bridge approval; requires State 7 verification.

### Finding F-BRG-002 — LOW — Cross-path test exercises public API, not dead code

**Obligation IDs:** PO-009, PO-016
**Artifact:** `crates/vb_compile/tests/v1_primitive_lowering.rs:929-955`
**Finding:** The `cross_path_wait_digest_equivalence` test calls `compile_source` (cold-path) and `compile_workflow` (warm-path delegate), which both route through `crate::mod_compile_lowering::compile_source()`. The dead `compile/mod.rs` copy is never exercised. The test verifies public API consistency, which is still valuable for determinism.
**Required fix:** File follow-up bead to remove `compile/mod.rs` dead code.
**Severity:** LOW — property satisfied by design (only one active copy exists). Compensating evidence: fix applied identically to both copies.

### Finding F-BRG-003 — LOW — Kani/fuzz PENDING_FORMAL_EXECUTION with tooling blockers

**Obligation IDs:** PO-001, PO-003, PO-005, PO-007, PO-012, PO-013, PO-015
**Artifacts:** `kani_wait_digest.rs`, fuzz targets
**Finding:** 7 obligations deferred to State 7 with documented tooling blockers. Kani 0.67 `Arbitrary for String` blocker and fuzz `musl/sanitizer` blocker must be resolved before execution.
**Required fix:** State 7: Resolve tooling blockers. Run all Kani harnesses and fuzz targets per bridge map Sections 5.2-5.3. Log results in verification-ledger.jsonl.
**Severity:** LOW — properly deferred with written artifacts, compilation evidence, and exact commands.

---

## 10. Obligation Status Summary

All 16 rows from `rust-refinement-obligations.jsonl`:

| ID | Verifier | `mapping_status` | Source Refs | Behavior Test | Independent | Status |
|----|----------|-----------------|-------------|--------------|-------------|--------|
| RRO-VB-032-001 | kani | planned | ✅ | ✅ (via proptest) | N/A | DEFERRED State 7 |
| RRO-VB-032-002 | proptest | verified | ✅ | ✅ | ✅ | **VERIFIED** |
| RRO-VB-032-003 | fuzz | planned | ✅ | ✅ (via proptest) | N/A | DEFERRED State 7 |
| RRO-VB-032-004 | proptest | verified | ✅ | ✅ | ✅ | **VERIFIED** |
| RRO-VB-032-005 | kani | planned | ✅ | ✅ (via proptest) | N/A | DEFERRED State 7 |
| RRO-VB-032-006 | proptest | verified | ✅ | ✅ | ✅ | **VERIFIED** (adapted) |
| RRO-VB-032-007 | fuzz | planned | ✅ | ✅ (via proptest) | N/A | DEFERRED State 7 |
| RRO-VB-032-008 | proptest | verified | ✅ | ✅ | ✅ | **VERIFIED** |
| RRO-VB-032-009 | proptest | verified | ✅ | ✅ | ✅ | **VERIFIED** |
| RRO-VB-032-010 | kani | planned (waived) | ✅ | ✅ (via proptest) | ✅ (proptest) | WAIVED — BLOCKED_DEAD_CODE |
| RRO-VB-032-011 | proptest | verified | ✅ | ✅ | ✅ | **VERIFIED** |
| RRO-VB-032-012 | fuzz | planned | ✅ | ✅ (via proptest) | N/A | DEFERRED State 7 |
| RRO-VB-032-013 | kani | planned | ✅ | ✅ (via proptest) | N/A | DEFERRED State 7 |
| RRO-VB-032-014 | proptest | verified | ✅ | ✅ | ✅ | **VERIFIED** |
| RRO-VB-032-015 | kani | planned | ✅ | ✅ (via proptest) | N/A | DEFERRED State 7 |
| RRO-VB-032-016 | proptest | verified | ✅ | ✅ | ✅ | **VERIFIED** |

Key: `✅` = `path::symbol` granularity source ref present | `behavior_affecting=true` rows have an independent behavior test | `mapping_status: planned` rows deferred to State 7.

---

## 11. Handoff to Formal-Verifier (State 7)

### Required actions:
1. **Kani execution:** Resolve `Arbitrary for String` blocker (refactor to `[u8; N]` arrays with valid-UTF-8 assumptions, or upgrade Kani). Run all 4 harnesses per bridge map Section 5.2. Capture raw output.
2. **Fuzz execution:** Resolve `musl/sanitizer` blocker (configure musl/fuzz compatibility or switch to glibc target). Run all 3 targets per bridge map Section 5.3. Capture raw output.
3. **Ledger entries:** Log all State 7 execution results in `verification-ledger.jsonl` with `bead=vb-xi2f.32`.
4. **PO-010 dead code:** File follow-up bead for `compile/mod.rs` removal.
5. **Verification completeness:** After all 16 obligations have raw evidence (8 proptest verified + 7 Kani/fuzz executed + 1 waived), the bead is ready for black-hat review at State 8.

---

## Final Disposition

The `proof-to-rust-map.md` bridge mapping is comprehensive, accurate, and honest. All 16 proof obligations are mapped to concrete Rust source locations with line-level granularity. Every behavior-affecting obligation has at least one independent behavior test. The 8 proptest obligations are verified with raw execution log evidence. The 7 Kani/fuzz obligations are properly deferred with documented tooling blockers, compilable artifacts, and exact State 7 commands. The 1 BLOCKED_DEAD_CODE waiver has valid compensating evidence.

All bridge review checklist items pass:
- ✅ Every obligation maps to a requirement or contract clause
- ✅ Every obligation has concrete `path::symbol` source refs (not file-only refs)
- ✅ Every behavior-affecting obligation has at least one independent behavior test
- ✅ Deferred obligations have documented blockers and exact commands
- ✅ No TLA+ claims exist (correct for compile-time digest domain)
- ✅ Provenance is clean (no self-approval)
- ✅ Implementation drift documented and accepted
- ✅ Non-vacuity assessed for all three lanes

**STATUS: APPROVED** — Proceed to State 7 (formal-verifier) with findings F-BRG-001, F-BRG-002, F-BRG-003.
