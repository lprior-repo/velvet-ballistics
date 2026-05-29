# Proof Strategy: vb-xi2f.13 — Nested Choose Primitive Body Lowering

## Bead Context
- **Bead ID:** vb-xi2f.13
- **Title:** P0 bug — lower nested choose primitive bodies in compiler
- **Domain:** `vb_compile` lowering — `lower_canonical_choose` and `choose_width`
- **Scope:** Two-function fix: `choose_width` (part_01.rs) and `lower_canonical_choose` (part_02.rs)
- **Files changed:** `crates/vb_compile/src/mod_compile_lowering/part_01.rs`, `crates/vb_compile/src/mod_compile_lowering/part_02.rs`
- **Files NOT changed:** `vb_core` (IR types + replay), `vb_yaml` (AST types + parsing), `vb_validate` (validation)

## Risk Classification

| Risk Category | Applicable | Rationale |
|---|---|---|
| **Temporal / State-Machine** | YES | Layout/width mismatch (H1), body interleaving (H2), otherwise span (H3), emission parity (PS-EMISSION-PARITY) |
| **Rust-Local Invariant** | YES | Slot uniqueness (H5), condition/body slot disjointness (H6), fanout limits (H8) |
| **Bounded State** | YES | Branch fanout ≤64, body widths bounded by u16::MAX, all arithmetic checked |
| **Refinement / Type-State** | YES | Boolean slot type invariant (H9), slot count refinement after lowering |
| **Concurrency** | NO | Choose is sequential (H11); no threads, atomics, or channels touched |
| **Unsafe / UB** | NO | No unsafe code in affected files; lowering is safe Rust |
| **Untrusted Input** | YES | YAML `when` string injection (H12), deep nesting (H13), parse boundaries |
| **Dependency / Supply-Chain** | NO | No dependency files changed; vb_core, vb_yaml, vb_validate unchanged |
| **Performance** | LOW | Body node bloat (H14) is deferred out of scope |
| **Release-Critical** | YES | P0 bug fix; `compile_source` is public API; empty-body regression risk |

## Verifier Lane Selection

| Lane | Decision | Seeds Covered | Rationale |
|---|---|---|---|
| **Kani** | REQUIRED | PS-TEMPORAL-001..003, PS-ARITH-001..002, PS-INVARIANT-001..002, PS-FANOUT-001, PS-TYPE-001, PS-LIVENESS-001, PS-EMISSION-PARITY, PS-YAML-FREE-IR | Primary lane. Compiler lowering is a pure function over bounded inputs (≤64 branches, u16-limited step/slot counts). Kani's bounded model checking is the best fit for verifying choose_width arithmetic, body lowering correctness, slot invariants, and emission parity. 13 behavior-affecting seeds demand Kani harnesses. |
| **Verus** | REQUIRED | PS-TYPE-001 | Formal spec of boolean slot type invariant. The hazard (H9) is a HIGH-severity runtime behavior risk where a non-boolean slot at runtime causes Internal error. A Verus spec can model the invariant that slot_from_text resolves to a boolean slot. Required for this seed only; overkill for the arithmetic/emission seeds. |
| **Flux** | REQUIRED | PS-INVARIANT-001, PS-INVARIANT-002 | Refinement types for slot count monotonicity and condition/body slot disjointness. Flux annotations on `record_slot` and `lower_canonical_choose` can express the post-condition `slot_count_after == slot_count_before + body_output_slots`. |
| **Proptest** | REQUIRED | PS-TEMPORAL-001, PS-TEMPORAL-002, PS-TEMPORAL-003, PS-EMISSION-PARITY, PS-INPUT-002 | Random branch configuration generation verifies emission parity and layout correctness across the valid input space. Proptest shrinking gives minimal counterexamples for width mismatch bugs. |
| **cargo-fuzz** | REQUIRED | PS-INPUT-001, PS-INPUT-002 | Fuzz hostile YAML input at compile boundary. YAML `when` strings and deeply nested choose bodies must be resilient against malformed input. Required for parse-boundary defense-in-depth. |
| **TLA+** | NOT APPLICABLE | None | The lowering is a compile-time pure function, not a temporal workflow/protocol. No retries, leases, queues, recovery, or distributed coordination. The workflow-model.md state machine is a sequential computation pipeline. |
| **Loom** | NOT APPLICABLE | PS-CONCURRENCY-001 | PS-CONCURRENCY-001 is explicitly `behavior_affecting: false`. Choose dispatch is sequential. No threads, atomics, channels, async shutdown touched. Hazard analysis H11 confirms choose has no process-level concurrency hazard. |
| **Miri** | NOT APPLICABLE | None | No unsafe code, FFI, raw pointers, or provenance-sensitive operations in affected files. All lowering is safe Rust. |

## Proof Coverage Summary

- **Total proof seeds:** 15
- **Behavior-affecting seeds:** 14 (all except PS-CONCURRENCY-001)
- **Kani obligations planned:** 14 (covers all behavior-affecting seeds)
- **Verus obligations planned:** 1 (PS-TYPE-001)
- **Flux obligations planned:** 2 (PS-INVARIANT-001, PS-INVARIANT-002)
- **Proptest obligations planned:** 5
- **cargo-fuzz obligations planned:** 2
- **Waiver candidates:** 2 (PS-CONCURRENCY-001 Loom, PS-TYPE-001 completeness)

## Trusted Base

The lowering fix relies on these pre-existing artifacts that do NOT change:
1. `vb_core::SlotCompiler` — global slot allocation with monotonic `slot_count`
2. `vb_core::StepIdx::new(u16)` — bounds-checked step index constructor
3. `vb_core::validation::graph::validate_choice` — forward-edge validation
4. `vb_validate::shared::validate` — IR integrity validation
5. `vb_compile::slot_from_text` — when-string → SlotIdx resolution (unchanged)
6. `vb_compile::lower_choose` — ChooseSlot node assembly (unchanged)
7. `vb_compile::checked_step_offset` — bounds-checked step arithmetic
8. `vb_yaml::parse_choose` — YAML parsing of ChooseBranch with steps

See `trusted-base-plan.md` for detailed trust boundaries.

## Implementation Bridge Notes

The proof-to-implementation bridge (`proof-to-implementation-input.md`) maps each proof obligation to:
- Exact Rust source file and function
- Required behavior test (unit, integration, regression)
- Expected evidence command and output patterns

Key bridge invariants:
1. Every `SlotBranch.target` must be a valid forward `StepIdx`
2. `choose_width` return value must match actual node count emitted
3. Body edge nodes must chain to `common_next`, not adjacent branch body
4. Condition slots and body output slots must be disjoint
