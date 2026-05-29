# Codebase Map: vb-xi2f.13 — Lower Nested Choose Primitive Bodies

## Bead Context
- **Bead**: vb-xi2f.13 (P0 bug)
- **Title**: "P0: lower nested choose primitive bodies"
- **Split from**: vb-xi2f.3
- **Phase**: compiler nested-body lowering
- **Public API touched**: `compile_source` behavior only

## Summary
`lower_canonical_choose` currently rejects choose branches that contain body steps (`steps` field), making nested choose primitives impossible. The YAML AST already supports `ChooseBranch.steps: Vec<StepAst>`, parsed correctly by `parse_choose`, but the lowering path at `part_02.rs` explicitly rejects non-empty bodies (lines 251-259). The fix must accept branch body steps, lower them into the node graph, with each branch getting its own target `StepIdx`.

---

## 1. Affected Source Files

### 1.1 vb_yaml — AST Types & Parsing (NO CHANGES EXPECTED)
| File | Purpose | Line(s) |
|------|---------|---------|
| `crates/vb_yaml/src/ast/types.rs` | `ChooseBranch` type with `steps: Vec<StepAst>` | 334-339 |
| `crates/vb_yaml/src/ast/types.rs` | `StepPrimitive::Choose { branches, otherwise }` | 244-249 |
| `crates/vb_yaml/src/ast/types.rs` | `StepAst` type (`id`, `primitive`) | 201-218 |
| `crates/vb_yaml/src/ast/parse_steps.rs` | `parse_choose` — parses branches + steps via `parse_body_steps` | 175-191 |
| `crates/vb_yaml/src/ast/parse_steps.rs` | `parse_body_steps` — generic body step parser | 258-267 |

**Status**: AST layer is complete. No changes needed here.

### 1.2 vb_compile — Lowering (BUG LOCATION — REQUIRES CHANGES)
| File | Purpose | Line(s) | Risk |
|------|---------|---------|------|
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | `compile_source` entry point, `choose_width` (hardcoded to 1), `canonical_layout` | 16-60, 99, 117-122 | **HIGH** — `choose_width` must account for body step widths |
| `crates/vb_compile/src/mod_compile_lowering/part_02.rs` | `lower_canonical_choose` — **THE BUG**: rejects non-empty body steps, forces all branch targets to same next step | 216-294 (esp. 251-259) | **HIGH** — core lowering fix |
| `crates/vb_compile/src/mod_compile_lowering/part_04.rs` | `emit_single_body_set` — model for how to lower single-body steps (used by for_each) | 213-310 | **REFERENCE ONLY** — pattern to follow |
| `crates/vb_compile/src/mod_compile_lowering/part_06.rs` | `lower_choose` — builds `ChooseSlot` node from slot branches | 19-50 | **MEDIUM** — already supports per-branch targets |
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | `canonical_step_width`, `canonical_body_step_width` | 86-146 | **MEDIUM** — layout computation |
| `crates/vb_compile/src/mod_compile_lowering.rs` | Module root (re-exports) | 1-53 | **LOW** |

### 1.3 vb_compile — Public API
| File | Purpose | Line(s) |
|------|---------|---------|
| `crates/vb_compile/src/compile/mod.rs` | `compile_source` (public), `lower_choose` (public re-export) | 25-27, 350-372 |
| `crates/vb_compile/src/lib.rs` | Public re-exports including `compile_source`, `lower_choose` | 105-116 |

### 1.4 vb_core — IR Types (NO CHANGES EXPECTED)
| File | Purpose | Line(s) |
|------|---------|---------|
| `crates/vb_core/src/nodes.rs` | `CompiledNodeKind::ChooseSlot` variant | 55-60 |
| `crates/vb_core/src/workflow/mod.rs` | `SlotBranch` (`condition: SlotIdx`, `target: StepIdx`) | 264-270 |
| `crates/vb_core/src/workflow/mod.rs` | `ExprBranch` (`condition: ExprIdx`, `target: StepIdx`) | 256-261 |
| `crates/vb_core/src/workflow/mod.rs` | `CompiledNodeKind::ChooseSlot` duplicate in workflow enum | 597-602 |
| `crates/vb_core/src/replay/choose.rs` | `replay_choose_slot` — runtime branch dispatch | 12-58 |
| `crates/vb_core/src/engine/step.rs` | Engine dispatch to `choose::choose_slot_branch` | 64-67 |

**Status**: IR layer supports per-branch targets. No changes needed.

---

## 2. Existing Tests

### 2.1 Choose Lowering Unit Tests (vb_compile)
| Test | File | Line | Coverage |
|------|------|------|----------|
| `lower_canonical_choose_accepts_two_branches` | `.../mod_compile_lowering/tests.rs` | 523-551 | Two empty-body branches compile |
| `lower_canonical_choose_rejects_unknown_otherwise_label` | `.../mod_compile_lowering/tests.rs` | 553-583 | Unknown otherwise → error |
| `lower_canonical_choose_rejects_65_branches` | `.../mod_compile_lowering/tests.rs` | 585-613 | 65 branches → fanout error |

**Gap**: NO tests for non-empty branch body steps exist.

### 2.2 Choose Replay Unit Tests (vb_core)
| Test | File | Line | Coverage |
|------|------|------|----------|
| `choose_slot_true_branch_advances_pc` | `crates/vb_core/src/replay/choose.rs` | 119-133 | True branch → target |
| `choose_slot_false_branch_falls_through_to_otherwise` | `crates/vb_core/src/replay/choose.rs` | 136-150 | False → otherwise |
| `choose_slot_out_of_bounds_slot_returns_slot_not_available` | `crates/vb_core/src/replay/choose.rs` | 153-164 | OOB slot error |
| `choose_slot_uninitialized_slot_returns_slot_not_available` | `crates/vb_core/src/replay/choose.rs` | 167-179 | Uninit error |
| `choose_slot_non_boolean_condition_returns_internal_error` | `crates/vb_core/src/replay/choose.rs` | 182-198 | Non-bool → error |
| `choose_slot_no_otherwise_and_all_false_returns_internal_error` | `crates/vb_core/src/replay/choose.rs` | 201-217 | No otherwise + all false → error |
| `choose_slot_set_pc_failure_returns_slot_to_replay_err` | `crates/vb_core/src/replay/choose.rs` | 220-236 | PC OOB error |

### 2.3 Integration/E2E Choose Tests
| File | Coverage |
|------|----------|
| `crates/vb_core/src/engine/tests/integration_choose.rs` | Engine-level ChooseSlot test fixtures |
| `crates/workspace_tests/tests/gate_10_node_tests.rs` | Validation gate tests for Choose/ChooseSlot |
| `crates/workspace_tests/benches/velvet_ballistics.rs` | Benchmark fixtures with choose |
| `crates/workspace_tests/tests/fixtures/pgo/choose_true.yaml` | Fixture YAML (simple, non-nested) |
| `crates/workspace_tests/tests/fixtures/valid/3step_choose.yaml` | Valid 3-step choose fixture (referenced in `phase0_scaffold_test.rs:586`) |

### 2.4 Kani Verification
| File | Coverage |
|------|----------|
| `crates/vb_compile/src/mod_compile_lowering/kani_vb_awhr.rs` | Otherwise label resolution (PO-002) |

---

## 3. Lowering Path for Choose/Otherwise

### Current Flow (broken for nested bodies)
```
compile_source (part_01.rs:16)
  → canonical_layout → choose_width → hardcoded 1
  → lower_canonical_step (part_02.rs:17)
    → lower_canonical_choose (part_02.rs:216)
      → REJECTS non-empty branch.steps (lines 251-259) ← BUG
      → Resolves otherwise label → StepIdx (lines 262-283)
      → Builds SlotBranch { condition, target } where ALL branches share same target ← BUG
      → lower_choose (part_06.rs:19) → ChooseSlot node
```

### Anti-Hallucination Constraint
Final IR already carries boolean `SlotIdx` conditions, not YAML strings. The `slot_from_text` function (part_05.rs:29) parses integer-strings into `SlotIdx`, ensuring YAML condition strings never leak into IR. This constraint is already satisfied.

### Required Lowering Changes
1. **`choose_width`** (part_01.rs:117): Must compute `overhead(1) + sum of body_width for each branch's body steps`. Model: `together_width` already does this for together.
2. **`lower_canonical_choose`** (part_02.rs:216): Must accept branch body steps, allocate body step indices via `checked_step_offset`, emit body nodes with proper chaining, and set per-branch targets.
3. **Body lowering pattern**: Follow `emit_single_body_set` (part_04.rs:213) which emits `Set` or `Do` body nodes. Choose branches should use the same pattern.
4. **Layout**: `canonical_step_names` already handles multi-width steps correctly; `lower_canonical_choose` needs to produce the right node count.

---

## 4. Key Functions

| Symbol | File | Purpose |
|--------|------|---------|
| `compile_source` | `part_01.rs:16` | Public entry point — compiles AST→IR |
| `lower_canonical_choose` | `part_02.rs:216` | **BUGGY** — rejects non-empty body steps |
| `lower_choose` | `part_06.rs:19` | Builds `ChooseSlot` node |
| `choose_width` | `part_01.rs:117` | Returns width=1 always — **BUGGY** |
| `emit_single_body_set` | `part_04.rs:213` | Reference pattern for single-body lowering |
| `slot_from_text` | `part_05.rs:29` | Converts integer-string condition to `SlotIdx` |
| `slot_idx_for_step` | `part_11.rs:226` | Converts step index to `SlotIdx` |
| `checked_step_offset` | `part_05.rs:11` | Bounds-checked step index arithmetic |
| `validate_branch_route` | `part_08.rs:129` | Empty-branch + missing-otherwise validation |
| `parse_choose` | `parse_steps.rs:175` | YAML parse for choose (already supports steps) |
| `replay_choose_slot` | `replay/choose.rs:12` | Runtime branch dispatch |

---

## 5. Open Questions

1. **Evidence file**: `.evidence/vb-xi2f-nested-choose.toml` referenced in bead context was NOT FOUND in isolated workspace. UNKNOWN.
2. **Max body depth**: Should nested choose branches be restricted to single-step bodies (like `emit_single_body_set` enforces) or support arbitrary-depth nested steps?
3. **otherwise label on nested choose**: Should nested choose in a branch body support its own `otherwise` label? The branch body is emitted as a linear sequence; the branch's last step chains to `next` (the step after the choose).
4. **Choose within choose branch**: Can a choose branch body itself contain a choose? This would require recursive lowering support.

## 6. Recommended Downstream Owners

| Agent | State | Purpose |
|-------|-------|---------|
| `rust-contract` | State 3 | Model the domain contract for nested choose lowering |
| `proof-planner` | State 4 | Plan proofs for no-panic, branch-target correctness, YAML-string-free IR |
| `proof-writer` / `kani` | State 5 | Kani harnesses for choose lowering with body steps |
| `test-planner` / `test-writer` | State 5 | Unit tests: nested choose success, invalid condition, missing otherwise, no-match |
| `holzman-rust` | State 8 | Implement the fix in `part_01.rs` and `part_02.rs` |
| `formal-verifier` | State 9 | Execute proof and test obligations |
| `black-hat-reviewer` | State 10 | Review contract parity, fanout limits, body-step validation |
