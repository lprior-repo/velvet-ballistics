# Proof Plan Review — Section 16 Symbolic Diagnostic Codes

**Reviewer Skill**: proof-plan-reviewer
**Reviewer Invocation ID**: ppr-vb-xi2f10-20260525T054500Z-c81e7d
**Review State**: APPROVED
**Bead**: vb-xi2f.10
**Phase**: State 4 — Proof Plan Review (Re-Review After Repair)
**Date**: 2026-05-25

---

## 1. Review Summary

This is a re-review of the proof plan for Section 16 diagnostic codes after repair of 3 CRITICAL and 1 HIGH finding from the previous review (ppr-vb-xi2f10-20260525T034500Z-f3a91).

All four blocking findings from the previous review have been successfully repaired:

1. **PO-015** cross-crate harness relocated from `vb_core/kani/` to `workspace_tests/kani/` with correct `--crate workspace_tests`.
2. **PO-013** Kani unwind bound raised from 10 to 100, covering the full 90-entry registry scan in `DiagnosticCode::symbolic_code()`.
3. **Agent invocation ledger** now contains a complete proof-planner invocation row with invocation_id `pp-vb-xi2f10-20260525T035355Z-8a3f2e`, input/output artifact hashes, and timestamps — distinct from the reviewer invocation.
4. **Proof coverage matrix** proptest count corrected from (11) to (10).

Two additional MEDIUM findings from the original review were also repaired:
- Traceability matrix schema version corrected from `proof-seed/v1` to `traceability/v1`.
- (The empty evidence refs in chained VLD rows remain as an optional improvement.)

The proof plan is structurally sound: 20 proof seeds, 160 verifier lane decisions (20 seeds × 8 verifiers), 28 schema-compliant proof obligations, well-justified non-applicability for TLA+/Verus/Flux/Loom/Miri, and a documented trusted base with 6 entries. The plan is precise enough for proof-writer to proceed.

**STATUS: APPROVED**

---

## 2. Repair Verification

### 2.1 CRITICAL Fixes Verified

| Finding | Original Defect | Repair | Verified |
|---------|----------------|--------|----------|
| F-001 (was E_KANI_ASSUMPTION_VACUITY) | PO-015 harness in `vb_core/kani/` referencing `vb_runtime`/`vb_storage` types — crate dependency deadlock | PO-015 `artifact` → `crates/workspace_tests/kani/kani_error_types_code.rs`, `command` → `cargo kani --harness kani_error_types_symbolic_code --crate workspace_tests` | ✅ |
| F-002 (was E_KANI_ASSUMPTION_VACUITY) | PO-013 `unwind=10` insufficient for 90-entry registry linear scan — vacuous proof risk | PO-013 `unwind` → 100, `model_bounds` updated with "For DiagnosticCode::symbolic_code(): 90-entry registry scan (unwind=100 covers with margin)" | ✅ |
| F-003 (was E_INVOCATION_LEDGER_MISSING) | No proof-planner invocation row in agent-invocation-ledger.jsonl | Ledger sequence 2 added: `invocation_id=pp-vb-xi2f10-20260525T035355Z-8a3f2e`, `skill=proof-planner`, `state=4`, 12 input artifacts with hashes, 7 output artifacts with hashes, `started_at`, `completed_at`, `status=completed` | ✅ |

### 2.2 HIGH Fix Verified

| Finding | Original Defect | Repair | Verified |
|---------|----------------|--------|----------|
| F-004 (was E_SCHEMA_REFERENCE_FORK) | Coverage matrix §2 claimed "PO-016–PO-026 (11)" proptest; PO-022 is cargo-fuzz | Changed to "PO-016–PO-021, PO-023–PO-026 (10)" with footnote | ✅ |

### 2.3 MEDIUM Fixes Verified

| Finding | Original Defect | Repair | Verified |
|---------|----------------|--------|----------|
| F-006 (was E_SCHEMA_REFERENCE_FORK) | Traceability matrix used `schema_version: proof-seed/v1` | Changed to `schema_version: traceability/v1` | ✅ |
| F-005 (was E_LANE_DECISION_WEAK) | 98 not_applicable VLD rows with empty evidence refs | Not repaired (optional per repair guide). Non-blocking. | Remains unfixed |

---

## 3. Artifacts Reviewed

| Artifact | Lines/Rows | Schema Compliance | Status |
|----------|-----------|-------------------|--------|
| `contract.md` | 167 | N/A (prose) | Unchanged from prior review |
| `proof-strategy.md` | 166 | N/A (prose) | Unchanged from prior review |
| `verifier-lane-decisions.jsonl` | 160 rows | `verifier-lane-decision/v1` ✓ | Unchanged. 2 reference errors noted (see §6.2) |
| `proof-obligations.planned.jsonl` | 28 rows | `proof-obligation/v1` ✓ | **REPAIRED**: PO-013, PO-015 fixed |
| `proof-seeds.jsonl` | 20 rows | `proof-seed/v1` ✓ | Unchanged |
| `traceability-matrix.jsonl` | 45 rows | `traceability/v1` | **REPAIRED**: schema tag fixed |
| `trusted-base-plan.md` | 164 | Markdown (not JSONL) | Unchanged. Adequate for State 4 |
| `waiver-candidates.jsonl` | 1 row | `waiver-candidate/v1` ✓ | Unchanged. Valid non-behavior waiver |
| `proof-coverage-matrix.md` | 103 | N/A (prose) | **REPAIRED**: count corrected to (10) |
| `agent-invocation-ledger.jsonl` | 2 rows | `agent-invocation/v1` | **REPAIRED**: planner row added |
| `proof-plan-repair-guide.md` | 203 | N/A (prose) | Reference only — prior review guide |

**Artifact Hashes (reviewed revisions)**:

| Artifact | SHA-256 |
|----------|---------|
| `contract.md` | `a6bf55ff...39877386` |
| `proof-strategy.md` | `5aef599a...3f86169b` |
| `verifier-lane-decisions.jsonl` | `56c6813f...378eed3d` |
| `proof-obligations.planned.jsonl` | `2190d222...67d76d23` |
| `proof-seeds.jsonl` | `d19919ba...7e94786` |
| `traceability-matrix.jsonl` | `e9c6c28a...11626dc6d` |
| `trusted-base-plan.md` | `d77b9fb0...010c7fc7` |
| `waiver-candidates.jsonl` | `6c4cd592...7d415251` |
| `proof-coverage-matrix.md` | `5cfa8daf...4afefa6127` |
| `agent-invocation-ledger.jsonl` | `7119a327...d58be742e9` |

---

## 4. Lane Decision Coverage

### 4.1 Seed × Verifier Matrix

All 20 seeds have full 8-verifier lane decisions (160 rows = 20 × 8):

| Verifier | Required | Not Applicable | Total |
|----------|----------|----------------|-------|
| `kani` | 16 | 4 | 20 |
| `proptest` | 13 | 7 | 20 |
| `cargo-fuzz` | 1 | 19 | 20 |
| `tla-plus` | 0 | 20 | 20 |
| `verus` | 0 | 20 | 20 |
| `flux-rs` | 0 | 20 | 20 |
| `loom` | 0 | 20 | 20 |
| `miri` | 0 | 20 | 20 |

**Total required lane decisions**: 30 (previous review: 27 — planner added 3 more required VLD rows during repair).

### 4.2 Non-Applicability Assessment (Re-confirmed)

- **TLA+** (20 not_applicable): Correctly justified. No temporal behavior, queues, retries, leases, or distributed protocols. Evidence: `boundary-map.md#2.1`, `workflow-model.md#Section7`, `hazard-analysis.md#Section5`.
- **Verus** (20 not_applicable): Correctly justified for P1 diagnostic infrastructure. Invariants are const-validated data consistency. Kani + proptest provide adequate defense-in-depth. Evidence: `codebase-map.md#Section3`, `type-contracts.md#Section11`.
- **Flux** (20 not_applicable): Correctly justified. `SymbolicCode` is a simple newtype over `&'static str`; validation is const lookup — no complex refinement types. Evidence: `type-contracts.md#Section1, Section3`.
- **Loom** (20 not_applicable): Correctly justified. No concurrency primitives, atomics, channels, locks, or async code. `CODE_REGISTRY` is const static. Evidence: `hazard-analysis.md#Section5`.
- **Miri** (20 not_applicable): Correctly justified. No unsafe code, FFI, raw pointers, or provenance concerns. `#![forbid(unsafe_code)]` applied. Evidence: `hazard-analysis.md#Section6`.

All non-applicability decisions are well-supported with concrete evidence refs in the canonical VLD rows (VLD-003 through VLD-007). 98 of 130 chained rows have empty evidence refs but the chain is traceable through direct references (e.g., "See VLD-003").

### 4.3 Required Lane Adequacy

| Risk | Lane | Obligations | Adequate? |
|------|------|------------|-----------|
| Bounded state, invariants, determinism | Kani | 15 (PO-001–PO-015) | ✅ All bounds verified |
| Enumeration, property testing, contract parity | proptest | 10 (PO-016–PO-026) | ✅ 1000+ cases per test |
| Hostile input (serde) | cargo-fuzz | 1 (PO-022) | ✅ 100k runs, 4096-byte payloads |
| Mutation resistance | cargo-mutants | 1 (PO-027) | ✅ 4 packages, 300s timeout |
| CI aggregation | moon-ci | 1 (PO-028) | ✅ 1800s timeout |

---

## 5. Obligation Review

### 5.1 Schema Compliance

All 28 obligations use `schema_version: "proof-obligation/v1"` with all required fields present. No legacy aliases (`layer`, `checker`, `claim`) detected. All commands are well-specified with harness names, crate targets, and tool flags.

### 5.2 Obligation Coverage by Seed

| Seed | Obligations | Status |
|------|------------|--------|
| PS-001 | PO-001 (Kani), PO-016 (proptest) | Covered |
| PS-002 | PO-002 (Kani), PO-016 (proptest), PO-023 (proptest) | Covered |
| PS-003 | PO-002 (Kani), PO-016 (proptest), PO-023 (proptest) | Covered |
| PS-004 | PO-003 (Kani) | Covered |
| PS-005 | PO-017 (proptest) | Covered |
| PS-006 | PO-004 (Kani), PO-018 (proptest) | Covered |
| PS-007 | PO-005 (Kani), PO-019 (proptest) | Covered |
| PS-008 | PO-006 (Kani) | Covered |
| PS-009 | PO-020 (proptest) | Covered |
| PS-010 | PO-007 (Kani, waiver candidate) | Covered |
| PS-011 | PO-008 (Kani) | Covered |
| PS-012 | PO-009 (Kani), PO-021 (proptest), PO-022 (fuzz) | Covered |
| PS-013 | PO-010 (Kani), PO-023 (proptest) | Covered |
| PS-014 | PO-011 (Kani), PO-023 (proptest) | Covered (see §6.2 note) |
| PS-015 | PO-012 (Kani), PO-023 (proptest) | Covered |
| PS-016 | PO-024 (proptest) | Covered |
| PS-017 | PO-013 (Kani) — **unwind=100, verified** | Covered |
| PS-018 | PO-014 (Kani) | Covered |
| PS-019 | PO-015 (Kani, workspace_tests), PO-025 (proptest) | Covered |
| PS-020 | PO-026 (proptest) | Covered |

### 5.3 Verifier Distribution

| Verifier | Count | Obligations |
|----------|-------|-------------|
| Kani | 15 | PO-001–PO-015 |
| proptest | 10 | PO-016–PO-021, PO-023–PO-026 |
| cargo-fuzz | 1 | PO-022 |
| cargo-mutants | 1 | PO-027 |
| moon-ci | 1 | PO-028 |
| **Total** | **28** | |

---

## 6. Findings

### 6.1 Resolved (Previously CRITICAL/HIGH)

All 4 blocking findings from the previous review (ppr-vb-xi2f10-20260525T034500Z-f3a91) are resolved:

- **F-001** (CRITICAL, PO-015 cross-crate harness): REPAIRED — harness relocated to `workspace_tests`.
- **F-002** (CRITICAL, PO-013 low unwind): REPAIRED — unwind raised to 100.
- **F-003** (CRITICAL, missing planner ledger): REPAIRED — invocation row added.
- **F-004** (HIGH, coverage matrix proptest count): REPAIRED — count corrected to (10).

### 6.2 New Finding — VLD PO Reference Error (MEDIUM, Non-Blocking)

**F-008**: `verifier-lane-decisions.jsonl` VLD-098 and VLD-106 reference PO-021 (serde roundtrip, C-SYM-5) instead of PO-023 (registry consistency, C-REG-3/4/5).

- **VLD-098** (PS-013 proptest not_applicable): `decision_reason` says "Covered by PO-021 (registry category + non-zero + uniqueness...)" — should reference PO-023.
- **VLD-106** (PS-014 proptest required): `required_obligation_ids` is `["PO-021"]` — should be `["PO-023"]`.

Both lane decisions are functionally correct (proptest coverage exists for registry properties via PO-023). The reference error is documentation-level only. The correct obligation PO-023 exists and covers C-REG-3, C-REG-4, and C-REG-5.

**Action**: Update VLD-098 `decision_reason` and VLD-106 `required_obligation_ids` to reference PO-023 before proof-writer consumes these rows. Non-blocking for plan approval.

### 6.3 Remaining (Carried Forward, Non-Blocking)

- **F-005** (MEDIUM): 98 not_applicable VLD rows with empty `non_applicability_evidence_refs`. Traceable through "See VLD-XXX" chains. Non-blocking.
- **F-007** (LOW): Trusted-base plan uses Markdown tables. JSONL conversion required before State 12.

---

## 7. Waiver Assessment

**WVR-PS010-ALLOC** (PO-007, PS-010): Zero-allocation proof with Kani alloc stubs.

| Criterion | Assessment |
|-----------|------------|
| `behavior_affecting: false` | ✅ Valid non-behavior performance invariant |
| `boundary_proof` | ✅ Static type analysis (Copy types, `repr(transparent)` newtypes) |
| `compensating_evidence` | ✅ Source audit + Criterion benchmark + type-level assurance |
| `expiry: 2026-12-31` | ✅ Reasonable re-evaluation timeline |
| `review_status: pending` | ✅ Correct for State 4 |

**Verdict**: Waiver candidate is valid. No behavior-affecting waiver issues. If Kani alloc stubs prove infeasible in State 5, the compensating evidence provides adequate assurance.

---

## 8. Trusted Base Assessment

6 trusted base entries (TBL-001 through TBL-006) identified, covering all 28 obligations:

| TBL | Kind | Behavior Affecting | Compensation |
|-----|------|--------------------|--------------|
| TBL-001 (rustc const-eval) | external_body | true | Runtime proptest PO-023 |
| TBL-002 (serde framework) | external_body | true | Fuzz PO-022 + Kani PO-009 |
| TBL-003 (thiserror derive) | external_body | true | Existing 834-line test + proptest |
| TBL-004 (Kani alloc stubs) | stub | false | Static analysis + Criterion benchmark |
| TBL-005 (existing test fixtures) | trusted | true | Mutation testing PO-027 |
| TBL-006 (master contract doc) | external_body | true | Inline golden data, visible deviations |

All entries have compensating evidence. The cross-reference table in trusted-base-plan.md §3 correctly maps assumptions to obligations.

**Trusted base acceptance gate**: Pending (requires review in States 6 and 12 per trusted-base-plan.md §6).

---

## 9. Non-Vacuity Assessment

- **Kani enumeration obligations** (PO-001–PO-012, PO-015): Concrete enumeration over registry entries and error variants. Inherently non-vacuous.
- **PO-013 (determinism)**: Uses `kani::any` for arbitrary error values. With `unwind=100`, the full `DiagnosticCode::symbolic_code()` registry scan path is explored. Non-vacuous.
- **PO-014 (constructor invariant)**: Enumeration over all ~90 SymbolicCode values. Non-vacuous.
- **Proptest obligations**: Enumeration-based (not random generation in most cases). PO-016, PO-021 use 1000 random cases with shrinking. Non-vacuous.
- **Fuzz obligation** (PO-022): 100k coverage-guided runs. Non-vacuous.

**Verdict**: No vacuity risks. All obligations are concrete, bounded, and verifiable.

---

## 10. Bridge Planning Assessment

`proof-to-implementation-input.md` maps proof domains to Rust source targets, behavior tests, and refinement harness refs. It references the repaired PO-015 artifact location. Adequate for proof-writer consumption.

---

## 11. Review Provenance

| Field | Value |
|-------|-------|
| **Reviewer invocation ID** | `ppr-vb-xi2f10-20260525T054500Z-c81e7d` |
| **Planner invocation ID** | `pp-vb-xi2f10-20260525T035355Z-8a3f2e` |
| **Previous reviewer invocation ID** | `ppr-vb-xi2f10-20260525T034500Z-f3a91` |
| **Reviewer differs from planner** | ✅ Confirmed |
| **Planner invocation independent** | ✅ Complete ledger entry with hashes and timestamps |

---

## 12. Verdict

**STATUS: APPROVED**

The proof plan for Section 16 symbolic diagnostic codes has been repaired and is now precise enough for proof-writer (State 5). All three critical defects from the previous review are resolved:

1. PO-015 harness correctly placed in `workspace_tests` (no cross-crate import deadlock).
2. PO-013 Kani unwind bound correctly set to 100 (no vacuous proof risk).
3. Agent invocation ledger complete with planner invocation row (provenance verifiable).

One new MEDIUM finding (VLD-098/VLD-106 PO reference error) was identified during re-review. The lane decisions are functionally correct and the correct obligation (PO-023) exists. The reference correction should be applied before proof-writer consumes the VLD rows but does not block plan approval.

The plan may proceed to proof-writer with 28 obligations across 5 verifiers, well-justified non-applicability for TLA+/Verus/Flux/Loom/Miri, documented trusted base, and valid non-behavior waiver for zero-allocation proof.
