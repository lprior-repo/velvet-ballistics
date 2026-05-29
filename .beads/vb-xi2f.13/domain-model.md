# Domain Model: Nested Choose Lowering

## Bead Context

- **Bead ID:** vb-xi2f.13
- **Title:** P0 bug — lower nested choose primitive bodies in compiler (vb_compile)
- **Domain:** Compiler lowering — `vb_yaml` AST → `vb_core` IR via `vb_compile`
- **Severity:** P0 — the current lowering rejects non-empty `ChooseBranch.steps`, making nested choose workflows impossible to compile. The IR and replay engine already support per-branch body targets (`ChooseSlot` / `SlotBranch.target`), but the compiler refuses to emit them.

---

## 1. Ubiquitous Language

| Term | Definition |
|---|---|
| **Choose primitive** | A YAML-authored branching construct: `choose` with an ordered list of `when` branches and an optional `otherwise` fallback label. |
| **ChooseBranch** | A single `when` + `steps` pair in the YAML AST. `steps` may contain one or more `Set`/`Do` body steps. |
| **When condition** | A YAML string label (e.g. `"auth_approved"`) that names a boolean slot to be tested at runtime. |
| **Otherwise** | Optional label naming a fallback step when no branch condition is true. |
| **ChooseSlot** | The IR node (`CompiledNodeKind::ChooseSlot`) that dispatches based on pre-materialized boolean slot values. |
| **SlotBranch** | A `(condition: SlotIdx, target: StepIdx)` pair in the IR. `condition` is a boolean-slot index; `target` is the step to jump to when true. |
| **Branch body** | The sequence of step-ast nodes inside a `ChooseBranch.steps`. Currently required to be empty; the fix lifts this restriction. |
| **Condition materialization** | Translation of a YAML `when` string into a boolean `SlotIdx` via `slot_from_text`. Removes YAML condition strings from the IR — the IR is string-free. |
| **Body lowering** | Translation of branch `steps` into a linear sequence of IR nodes (`SetConst`, `Do`, etc.) interleaved with jumps to the `choose`'s "next" step after the body. |
| **Step layout** | The pre-computed mapping from each YAML step index to its IR `start: StepIdx` and `width: usize`. `choose_width` currently hardcodes `1` (must change to `1 + sum(body_width per branch)`). |
| **Fallthrough** | The existing behavior where all branches share a single `target` (the "next" step). Retained as a degenerate case when all branch bodies are empty. |

## 2. Domain Entities

### 2.1 YAML Domain (vb_yaml)

```
StepPrimitive::Choose {
    branches: Vec<ChooseBranch>,  // ordered, 0..64
    otherwise: Option<String>,    // step label string
}

ChooseBranch {
    when: String,                 // boolean slot name, resolved via step_names
    steps: Vec<StepAst>,          // 0..N body steps (currently forbidden >0)
}
```

### 2.2 IR Domain (vb_core)

```
CompiledNodeKind::ChooseSlot {
    branches: Box<[SlotBranch]>,  // ordered, 0..64
    otherwise: Option<StepIdx>,   // resolved label → numeric step, or None
}

SlotBranch {
    condition: SlotIdx,           // pre-materialized boolean slot
    target: StepIdx,              // first step of branch body, or next-step fallthrough
}
```

### 2.3 Compiler Domain (vb_compile)

```
lower_canonical_choose: AST ChooseBranch[] → IR ChooseSlot + body nodes
choose_width:                  AST ChooseBranch[] → layout width (must change from 1)
```

## 3. Invariants

### I1: No YAML strings in IR
The compiled `SlotBranch.condition` is **always** a numeric `SlotIdx`, never a YAML string. `when` strings are fully resolved during lowering. The "anti-hallucination" rule from the explorer is absolute: final IR must not carry YAML condition strings.

### I2: Branch fanout ≤ 64
`branches.len() ≤ 64`. Enforced at lowering time by `lower_canonical_choose` line 226.

### I3: Non-empty branch table requires otherwise
If `branches.is_empty()`, `otherwise` **must** be `Some`. Enforced by `WorkflowError::EmptyBranchTable`.

### I4: All branch targets valid
Every `SlotBranch.target` must point to an existing IR node within the compiled workflow. Enforced by `validate_slot_choose`.

### I5: Branch bodies allocate non-overlapping step ranges
When branch bodies are lowered, each branch's body nodes occupy a distinct, contiguous range of `StepIdx` values. These ranges must not overlap with other branches' ranges or with the `ChooseSlot` node's own step index.

### I6: Graph validation: edges forward
All branch `target` edges must point strictly forward (`target.as_usize() > ci`). Enforced by `validate_forward_target`.

### I7: Otherwise must point within bounds
When `otherwise` resolves from a label, the resolved `StepIdx` must be within `[0, nodes.len())`.

## 4. Illegal States (Made Unrepresentable by Type)

| Illegal State | How Prevented |
|---|---|
| More than 64 branches | `lower_canonical_choose` rejects with `PrimitiveLoweringLimitExceeded` |
| Empty branches with no otherwise | `validate_branch_route` → `WorkflowError::EmptyBranchTable` |
| YAML string in IR | `SlotBranch.condition: SlotIdx` (not `String`) |
| Non-boolean condition runtime | `replay_choose_slot` rejects non-`Bool` values |
| Backward branch edge | `validate_forward_target` rejects `target <= ci` |
| Missing otherwise at runtime with all-false branches | `replay_choose_slot` returns `Internal` error |

## 5. Open Design Questions

1. **Q1: Body step limits per branch.** Should branch body width have an independent limit distinct from the 64-branch fanout limit? The current `body_width` function computes aggregate width without per-branch caps. An independent per-branch step-count limit may be warranted to prevent pathological compilation.

2. **Q2: Nested choose within choose.** Can a branch body itself contain a `choose` primitive? The graph traversal would need to handle nested `ChooseSlot` nodes. The `push_loop_span` function does not currently track choose spans as loop spans — this is correct since choose is not a loop. But the reachability validation must traverse nested choose edges.

3. **Q3: choose_width recalculation.** `choose_width` must change from `Ok(1)` to `1 + sum of body_width(branch.steps)`. The question is whether branch body widths should be summed individually (each branch allocates its own width), or whether the total width is `1` (choose node) + `max_branch_body_width` (shared body region). Given that each `SlotBranch.target` points to a unique step, the layout allocator must give each branch a distinct starting step index, so individual summation is correct.

4. **Q4: `slot_from_text` for body steps.** When lowering body steps, the compiler must allocate fresh slots for each `Set` output within the body. The existing `emit_single_body_set` handles single `Set` or `Do` body steps — a generalization is needed for N-step bodies.
