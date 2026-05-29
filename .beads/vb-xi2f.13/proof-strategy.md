# Proof Strategy: vb-xi2f.13 — Nested Choose Primitive Body Lowering

**Planner invocation:** proof-planner-vb-xi2f.13-20260529-001
**Bead ID:** vb-xi2f.13
**Artifact version:** 1
**Generated:** 2026-05-29T00:00:00Z

---

## 1. Overview

The `lower_canonical_choose` function in `vb_compilepart_02.rs` currently rejects any `ChooseBranch` with a non-empty `steps` body. The fix lifts this restriction by lowering body steps (`Set`, `Do` primitives) into IR nodes, assigning each branch a distinct `SlotBranch.target` pointing to the first body node, and ensuring the last body node's `next` field falls through to the common post-choose step.

This proof strategy covers 15 proof seeds spanning temporal/layout, arithmetic, invariant, type, liveness, concurrency, hostile-input, emission-parity, and anti-hallucination claims. The plan produces 23 concrete proof obligations across Kani (12), Verus (4), Flux (3), proptest (2), and cargo-fuzz (2), with 36 verifier-lane-decision rows covering 14 behavior-affecting plus 1 non-behavior-affecting seed.

All behavior-affecting seeds receive the default Rust profile (Verus, Kani, Flux, proptest) as required lane decisions, except where `not_applicable` is justified by concrete contract exclusions or type-system guarantees. Conditional lanes (TLA+, cargo-fuzz) are added only where risk tags mandate them.

---

## 2. Verifier Lane Profile

### 2.1 Default Profile (applied to all behavior-affecting seeds)

| Verifier | Role | Scope |
|---|---|---|
| **Kani** | Bounded model checking — panic-freedom, overflow, error/rejection claims, structural invariants | Primary harness vehicle for all 14 behavior-affecting seeds |
| **Verus** | Rust-local pure invariants — functional correctness of `choose_width`/`lower_canonical_choose` parity, arithmetic proofs, body fallthrough | Core functional properties (PS-EMISSION-PARITY, PS-TEMPORAL-001/002/003, PS-ARITH-001) |
| **Flux** | Refinement types — slot count monotonicity, width bounds, StepIdx range, slot disjointness | Invariant properties expressible as refinements (PS-INVARIANT-001/002, slot bounds) |
| **proptest** | Property-based testing — random branch configurations, body step sequences, layout/emission parity | Broad input space coverage (PS-EMISSION-PARITY, PS-INPUT-001, PS-TEMPORAL-001) |

### 2.2 Conditional Profile (risk-tag driven)

| Verifier | Trigger Risk Tags | Seeds |
|---|---|---|
| **TLA+** | `temporal` | PS-TEMPORAL-001/002/003 — **not_applicable**: these are compile-time layout invariants, not runtime temporal protocols. Verified by Kani/Verus/proptest at the Rust level. |
| **cargo-fuzz** | `hostile-input` | PS-INPUT-001/002 — **required** |
| **Loom** | `concurrency` | PS-CONCURRENCY-001 — **not_applicable**: choose dispatch is sequential per workflow run; seed is non-behavior-affecting |

### 2.3 Not Applicable With Evidence

| Seed | Verifier | Evidence |
|---|---|---|
| PS-TYPE-001 | All 4 defaults | Contract Non-Goals item 4 explicitly excludes compile-time boolean slot type validation. Hazard H9 is documented; runtime `replay_choose_slot` already rejects non-bool condition values. This is a deferred concern → waiver candidate WC-001. |
| PS-YAML-FREE-IR | Verus, Flux, proptest | The property "no YAML strings in IR" is a type-system guarantee (`SlotBranch.condition: SlotIdx`, not `String`). Verus/Flux refinement proofs are unnecessary; Kani harness confirms type safety. |
| PS-CONCURRENCY-001 | All default + Loom | Seed is non-behavior-affecting. Choose dispatch is single-threaded per run. No concurrency hazard exists in the current model. |

---

## 3. Obligation Summary

**Total planned obligations:** 23

| Verifier | Count | Obligation IDs |
|---|---|---|
| Kani | 12 | PO-KANI-001 through PO-KANI-012 |
| Verus | 4 | PO-VERUS-001 through PO-VERUS-004 |
| Flux | 3 | PO-FLUX-001 through PO-FLUX-003 |
| proptest | 2 | PO-PROPTEST-001, PO-PROPTEST-002 |
| cargo-fuzz | 2 | PO-FUZZ-001, PO-FUZZ-002 |

---

## 4. Trusted Base

The implementation relies on components outside the scope of `lower_canonical_choose`:

1. **SlotCompiler** — External stateful component. Trusted to allocate unique `SlotIdx` values monotonically. PS-INVARIANT-001 provides Kani verification of slot uniqueness post-lowering.
2. **body_width** — Pure helper function. Trusted for correct `checked_add` usage. PS-ARITH-001 provides Kani overflow verification.
3. **step_idx** — Conversion helper. Trusted for overflow checking. PS-ARITH-002 covers bounds.
4. **lower_choose** (part_06.rs) — Trusted to emit correctly-formed `ChooseSlot` nodes from valid `SlotBranch` arrays. Called after all validation passes.
5. **vb_validate::shared::validate** — Graph validator. Trusted to catch IR invariants missed by the compiler. Operates independently at IR level.

See `trusted-base-plan.md` for full `trusted-base-ledger/v1` rows.

---

## 5. Waiver Strategy

Two non-behavior-affecting waiver candidates are planned:

| Waiver | Seed | Reason |
|---|---|---|
| WC-001 | PS-TYPE-001 | Boolean slot type tracking excluded by contract Non-Goals item 4. Runtime safety net: `replay_choose_slot` rejects non-bool conditions. |
| WC-002 | PS-INPUT-002 (partial) | Deep-nesting fuzz deferred to `vb_yaml` parser-level fuzzing. Compensated by YAML `DepthLimit` enforcement at parse time. |

See `waiver-candidates.jsonl` for full `waiver-candidate/v1` rows.

---

## 6. Bridge Strategy

Every Kani and Verus obligation targets production code functions (`choose_width`, `lower_canonical_choose`, `body_width`, `slot_from_text`, `lower_choose`, `replay_choose_slot`). Harnesses must use `kani::Arbitrary`/`kani::any()` — no hardcoded structural inputs per GOD RULE #1.

Proof-to-implementation bridge mapping is prepared in `proof-to-implementation-input.md`:
- Each proof claim is mapped to exact Rust source refs
- Independent behavior tests are specified per acceptance criteria (AC1-AC10)
- Refinement harness refs for Kani harnesses are documented
- Exact evidence commands are recorded

---

## 7. Non-Vacuity Constraints

Per GOD RULES:
- **No hardcoded Kani shapes** (Rule 1): Harnesses must use `kani::Arbitrary` for `ChooseBranch`, `StepAst`, and associated structures.
- **No vacuum Verus proofs** (Rule 2): Verus `spec fn` models must bind to production `exec fn` via `requires`/`ensures`.
- **No blinded loop** (Rule 4): If verification exposes a flaw, fix the implementation, not the proof harness.

---

## 8. Acceptance Criteria Coverage

| AC | Obligations | Seeds |
|---|---|---|
| AC1 (choose_width correct count) | PO-KANI-001, PO-VERUS-001, PO-PROPTEST-001 | PS-TEMPORAL-001, PS-EMISSION-PARITY |
| AC2 (empty branches → 1) | PO-KANI-001, PO-PROPTEST-001 | PS-EMISSION-PARITY |
| AC3 (emit ChooseSlot + body) | PO-KANI-002, PO-PROPTEST-001 | PS-TEMPORAL-002, PS-EMISSION-PARITY |
| AC4 (target points to body start) | PO-KANI-003, PO-VERUS-002 | PS-TEMPORAL-002 |
| AC5 (last body → next) | PO-KANI-003, PO-VERUS-002 | PS-TEMPORAL-002 |
| AC6 (all condition slots recorded) | PO-KANI-006, PO-FLUX-001 | PS-INVARIANT-001 |
| AC7 (IR passes validate) | PO-KANI-012, PO-PROPTEST-001 | PS-YAML-FREE-IR, PS-EMISSION-PARITY |
| AC8 (empty-body regression) | PO-KANI-002, PO-PROPTEST-001 | PS-EMISSION-PARITY |
| AC9 (no YAML strings in IR) | PO-KANI-012 | PS-YAML-FREE-IR |
| AC10 (fanout/route/label preserved) | PO-KANI-008, PO-KANI-009 | PS-FANOUT-001, PS-LIVENESS-001 |

---

## 9. Risk Coverage Summary

| Hazard | Severity | Covered By |
|---|---|---|
| H1 Layout/Width Mismatch | CRITICAL | PO-KANI-001, PO-VERUS-001, PO-PROPTEST-001, PO-FLUX-003 |
| H2 Branch Body Interleaving | HIGH | PO-KANI-003, PO-VERUS-002 |
| H3 Otherwise Target Span | MEDIUM | PO-KANI-004, PO-VERUS-003 |
| H4 Body Width Overflow | LOW | PO-KANI-005, PO-VERUS-004 |
| H5 SlotIndex Reuse | CRITICAL | PO-KANI-006, PO-FLUX-001 |
| H6 Condition Slot Overwrite | HIGH | PO-KANI-007, PO-FLUX-002 |
| H7 StepIdx Overflow | LOW | PO-KANI-005, PO-VERUS-004 |
| H8 Branch Fanout Evasion | LOW | PO-KANI-008 |
| H9 Boolean Slot Type Mismatch | HIGH | Waiver WC-001 (deferred) |
| H10 All-Branches-False | MEDIUM | PO-KANI-009 |
| H11 Concurrency (N/A) | LOW | PS-CONCURRENCY-001 → not_applicable |
| H12 YAML Injection via when | MEDIUM | PO-KANI-010, PO-FUZZ-001 |
| H13 Excessive Body Nesting | LOW | PO-FUZZ-002 |
| H14 Performance Bloat | LOW | Deferred (non-behavior) |
