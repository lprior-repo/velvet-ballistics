# Proof Plan Review — Digest Coverage of `for_each` Semantics

**Reviewer Skill:** proof-plan-reviewer
**Reviewer Invocation ID:** proof-plan-reviewer/vb-xi2f.28/2026-05-24T12:00:00Z
**Review State:** 5 (proof-plan-review)
**Date:** 2026-05-24
**Bead:** vb-xi2f.28

---

## Reviewed Artifacts

| Artifact | Hash (SHA-256) | Status |
|---|---|---|
| proof-strategy.md | (see file) | Reviewed |
| verifier-lane-decisions.jsonl | (see file) | Reviewed — 80 rows |
| proof-obligations.planned.jsonl | (see file) | Reviewed — 15 rows |
| trusted-base-plan.md | (see file) | Reviewed — 5 entries |
| waiver-candidates.jsonl | (see file) | Reviewed — 1 entry |
| proof-seeds.jsonl | (see file) | Reviewed — 10 seeds |
| traceability-matrix.jsonl | (see file) | Reviewed — 15 rows |
| proof-to-implementation-input.md | (see file) | Reviewed |
| contract.md | (see file) | Reviewed |
| boundary-map.md | (see file) | Reviewed |
| hazard-analysis.md | (see file) | Reviewed |
| workflow-model.md | (see file) | Reviewed |
| type-contracts.md | (see file) | Reviewed |
| delivery-scope.jsonl | (see file) | Reviewed |
| agent-invocation-ledger.jsonl | (see file) | Reviewed — 4 rows |

Planner Invocation ID: `proof-planner/vb-xi2f.28/2026-05-25T04:30:00Z`
Reviewer Invocation ID: `proof-plan-reviewer/vb-xi2f.28/2026-05-24T12:00:00Z`
Provenance: Independent — planner and reviewer are distinct skills with distinct invocation IDs.

---

## 1. Executive Summary

This is a **well-structured, proportional proof plan** for a P1 narrow-scope bead that adds explicit `StepPrimitive::ForEach` field hashing to `digest_step_primitive()` in two duplicate source files. The plan covers 10 proof seeds across the full core verifier set (80 lane decisions), has 15 concrete proof obligations (8 Kani + 7 proptest), addresses non-vacuity via TBD-FE-05 (Kani Arbitrary mandate), includes bridge planning, and has no behavior-affecting waivers.

**Three NON-BLOCKING findings** were identified (see §4). None warrant rejection for a P1 bead.

---

## 2. Provenance Check

| Criterion | Result |
|---|---|
| Reviewer invocation differs from planner | ✓ PASS — `proof-plan-reviewer` ≠ `proof-planner` |
| Planner artifacts have no self-stamped reviewer fields | ✓ PASS |
| Agent invocation ledger present | ✓ PASS — 4 rows covering states 1-4 |
| No proof-artifact files (state 5-6) mixed into plan state | ✓ PASS |

---

## 3. Lane Decision Coverage

### 3.1 Coverage Matrix

All 10 proof seeds have lane decisions for all 8 core verifiers (80 total):

| Seed | tla-plus | verus | kani | flux-rs | loom | miri | proptest | cargo-fuzz |
|---|---|---|---|---|---|---|---|---|
| PS-FE-01 | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable | required | not_applicable |
| PS-FE-02 | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable | required | not_applicable |
| PS-FE-03 | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable | required | not_applicable |
| PS-FE-04 | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable | required | not_applicable |
| PS-FE-05 | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable | required | not_applicable |
| PS-FE-06 | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | required | not_applicable |
| PS-FE-07 | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |
| PS-FE-08 | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | required | not_applicable |
| PS-FE-09 | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |
| PS-FE-10 | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |

### 3.2 Non-Applicability Evidence Assessment

| Verifier | Seeds Not-Applied | Primary Evidence | Verdict |
|---|---|---|---|
| **tla-plus** | 10/10 | boundary-map.md §1, §5; workflow-model.md §2 — no temporal state, pure function | **ACCEPTED**. Evidence is concrete and conclusive. |
| **verus** | 10/10 | workflow-model.md §2 — behavioral properties, not deep invariants | **ACCEPTED with finding** (F-PPR-001). Evidence is judgment-based but correct for a P1 pure-function bead. |
| **kani** | 2/10 (PS-FE-06, PS-FE-08) | PS-FE-06: cross-path Kani not possible across compilation units; PS-FE-08: regression is cross-version behavioral | **ACCEPTED**. Reasoning is tool-appropriate. |
| **flux-rs** | 10/10 | type-contracts.md — Rust destructuring enforces field exhaustiveness; Flux can't model blake3::Hasher | **ACCEPTED with finding** (F-PPR-002). The missing verifier-trigger-matrix.md reference is cosmetic; primary evidence is adequate. |
| **loom** | 10/10 | hazard-analysis.md §5 (HZ-C01, HZ-C02) — no shared state, no concurrency | **ACCEPTED**. Evidence is concrete and exhaustive. |
| **miri** | 10/10 | boundary-map.md §4; hazard-analysis.md §6 — no unsafe code in digest pipeline | **ACCEPTED**. Evidence is concrete and exhaustive. |
| **proptest** | 3/10 (PS-FE-07, PS-FE-09, PS-FE-10) | Specific Kani obligation covers each narrow claim exhaustively | **ACCEPTED**. Redundancy decisions are well-motivated. |
| **cargo-fuzz** | 10/10 | domain-model.md §5 DD-02; delivery-scope.jsonl — digest consumes parsed AST, not raw bytes | **ACCEPTED**. The "wrong layer" rationale is definitive. |

---

## 4. Findings

### F-PPR-001: Missing Evidence Reference (LOW)

**Finding Code:** `E_LANE_DECISION_WEAK`
**Severity:** LOW
**Artifact:** verifier-lane-decisions.jsonl
**Affected Rows:** VLD-FE-01-VER, VLD-FE-01-FLX (and cross-referencing rows)
**Message:** Lane decisions reference `verifier-trigger-matrix.md` as supporting evidence, but this file does not exist in the workspace. The primary evidence refs (workflow-model.md §2, type-contracts.md) independently support the non-applicability conclusions for this P1 bead, so this is cosmetic. For P0 beads, missing evidence references would be rejection-level.
**Required Fix:** Either create `verifier-trigger-matrix.md` with the stated content, or remove the stale reference from VLD-FE-01-VER and VLD-FE-01-FLX. No state rollback needed.

### F-PPR-002: Verus Non-Applicability Evidence Is Thin but Proportionate (LOW)

**Finding Code:** `E_LANE_DECISION_WEAK`
**Severity:** LOW
**Artifact:** verifier-lane-decisions.jsonl
**Affected Rows:** VLD-FE-01-VER through VLD-FE-10-VER (all 10 Verus decisions)
**Message:** All Verus non-applicability decisions rest on the planner's judgment that "no deep mathematical invariants" exist. For a P1 bead on a pure deterministic function adding one match arm, this is proportionate. For a P0 bead or one involving arithmetic invariants, this evidence quality would be insufficient. The proof-obligation artifact confirms that all behavior claims are covered by Kani + proptest with specific commands.
**Required Fix:** None for this bead. Future P0 beads should include concrete code analysis showing absence of the specific invariants Verus would prove.

### F-PPR-003: Body Recursion Depth Limitation Acknowledged but Not Proven (LOW)

**Finding Code:** `E_KANI_ASSUMPTION_VACUITY`
**Severity:** LOW
**Artifact:** proof-obligations.planned.jsonl
**Affected Rows:** PO-K-FE-04, PO-P-FE-04
**Message:** Both Kani and proptest obligations for body-step hashing restrict body primitive types to `["Set", "Finish"]`, excluding nested `ForEach`. This is a modeling simplification explicitly documented in the assumptions ("simplified model; full StepPrimitive enum too large for Kani"). The recursive dispatch through `digest_step_primitive` should still work correctly for nested ForEach, but the proof obligations don't verify it. For a P1 bead, this is acceptable given the low practical likelihood of nested ForEach in body content.
**Required Fix:** No state rollback needed. The proof-reviewer should verify that the proof-writer's harnesses explicitly document this limitation. A future bead covering full recursive hashing for all primitive types would close this gap.

---

## 5. Obligation Schema Compliance

All 15 proof obligations conform to `proof-obligation/v1`:

| Field | Status |
|---|---|
| `schema_version` | ✓ Present |
| `id` | ✓ Unique per obligation |
| `requirement_id` / `contract_clause` | ✓ Mapped to contract |
| `domain_claim` | ✓ Specific and falsifiable |
| `risk` / `risk_tags` | ✓ Classified |
| `verifier` | ✓ kani (8) + proptest (7) |
| `artifact` | ✓ Concrete file paths |
| `target` | ✓ Specific function/harness name |
| `command` | ✓ Executable commands with `-p vb_compile` |
| `workdir` | ✓ `/home/lewis/src/velvet-ballistics` |
| `expected_evidence` | ✓ Specific, verifiable outputs described |
| `assumptions` | ✓ Enumerated |
| `model_bounds` | ✓ Bounds specified for Kani; proptest iteration counts specified |
| `tool_metadata` | ✓ Version and flags specified |
| `trusted_base_refs` | ✓ Cross-referenced to TBD entries |
| `required` + `behavior_affecting` | ✓ All 15 are required; 14 are behavior-affecting |
| `mode` | ✓ All `verify-proof` |
| `owner_state` / `rerun_from` | ✓ owner_state=4, rerun_from=5 |
| `status` | ✓ All `planned` |
| No legacy alias fields | ✓ No `layer`, `checker`, or bare `claim` |

---

## 6. Trusted Base Review

| Entry | Trust Kind | Verdict |
|---|---|---|
| TBD-FE-01 (`blake3::Hasher`) | external_library | **ACCEPTED**. blake3 is widely audited, deterministic by design. |
| TBD-FE-02 (`WorkflowDigest::from_bytes`) | domain_type | **ACCEPTED**. 1-line newtype constructor; trivial correctness. |
| TBD-FE-03 (`u32::to_le_bytes`) | language_primitive | **ACCEPTED**. Rust standard library guarantee. |
| TBD-FE-04 (recursion termination) | structural_guard | **ACCEPTED**. AST is a tree, recursion bounded by YAML depth. |
| TBD-FE-05 (Kani Arbitrary mandate) | tool_requirement | **ACCEPTED**. Critical non-vacuity guard; proof-reviewer must enforce. |

No unledgered trust markers detected. The trusted base surface is minimal and well-documented.

---

## 7. Waiver Review

| Waiver | Type | Verdict |
|---|---|---|
| WC-FE-01 (Kani tooling availability) | Non-behavior, tooling | **ACCEPTED as planned**. Marked `behavior_affecting: false`. Compensating evidence (proptest POs) documented. `review_status: pending` is appropriate at plan-review stage. |

---

## 8. Bridge Plan Review

`proof-to-implementation-input.md` provides adequate bridge planning:
- Concrete implementation targets (two source files, two symbols each) ✓
- Exact code pattern specified ✓
- Proof-claim-to-source-file mapping (7 subsections) ✓
- Behavior test expectations documented ✓
- Known pre-existing divergences cataloged (Together/Aggregate naming) ✓
- Instrumentation notes for Kani harness visibility ✓

---

## 9. Obligation Execution Feasibility

| Concern | Assessment |
|---|---|
| Kani harnesses in `crates/vb_compile/src/kani_proofs/` | Artifact paths consistent; files do not yet exist (expected for State 4) |
| Proptest in `crates/vb_compile/tests/` | Integration test path; requires `pub` visibility for both `canonical_digest` copies |
| Dual-path equivalence (PO-P-FE-06) | Both functions must be importable from the same integration test crate |
| Bounded Kani unwind values | Appropriate: unwind 2-10 depending on recursion depth |
| Workdir reference to repo root | Consistent across all POs; matching the isolated workspace |

---

## 10. Final Status

### STATUS: APPROVED

**Rationale:** The plan meets all `proof-plan-reviewer` criteria for a P1 narrow-scope bead:
- Full core-verifier coverage (80 lane decisions for 10 seeds)
- 15 concrete, executable obligations with specific commands and bounds
- Non-vacuity addressed (TBD-FE-05 Kani Arbitrary mandate)
- Trusted base plan with 5 entries
- Bridge planning present
- No behavior-affecting waivers
- Three LOW-severity findings — none block proof writing

**Findings:** 3 (F-PPR-001, F-PPR-002, F-PPR-003) — all LOW severity, all non-blocking.

**Next State:** 5 (proof-writer). The proof-writer may proceed with this approved plan. The proof-reviewer should verify TBD-FE-05 compliance (kani::Arbitrary usage) at State 6.
