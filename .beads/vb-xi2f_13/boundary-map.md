# Boundary Map: Nested Choose Lowering

## Bead Context

- **Bead ID:** vb-xi2f.13
- **Domain:** Boundary analysis for choose lowering across crate boundaries.

---

## 1. Architecture: Hexagonal Core/Shell

```
┌─────────────────────────────────────────────────────────────┐
│                     IMPERATIVE SHELL                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐ │
│  │  vb_yaml     │  │  vb_runtime  │  │  external I/O    │ │
│  │  (Parsing)   │  │  (Replay)    │  │  (network, etc.) │ │
│  └──────┬───────┘  └──────┬───────┘  └──────────────────┘ │
│         │                 │                                 │
│         ▼                 ▼                                 │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              PURE / FUNCTIONAL CORE                   │  │
│  │                                                       │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │  │
│  │  │  vb_compile  │  │  vb_core     │  │ vb_validate │ │  │
│  │  │  (Lowering)  │  │  (IR + IDs)  │  │ (Graph)     │ │  │
│  │  └──────────────┘  └──────────────┘  └────────────┘ │  │
│  │                                                       │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Boundary 1: vb_yaml → vb_compile (YAML AST Boundary)

**Artifacts crossing:**

| Direction | Type | Description |
|---|---|---|
| vb_yaml → vb_compile | `vb_yaml::ast::ChooseBranch` | YAML authoring branch with `when: String` and `steps: Vec<StepAst>` |
| vb_yaml → vb_compile | `String` (otherwise label) | Step label for fallback target |

**Parsing rules:**
- `when` string is NOT a condition expression — it is a slot name that resolves to `SlotIdx`
- `steps: Vec<StepAst>` may contain `Set` or `Do` step primitives
- `otherwise: Option<String>` is an optional step label, NOT a step index

**Validation at boundary:**
- `lower_canonical_choose` is the boundary handler
- All YAML strings are resolved to numeric indices before IR emission
- No YAML strings cross into `vb_core::CompiledNodeKind`

**Anti-hallucination rule:**
> "Final IR must not carry YAML condition strings" — enforced by `SlotBranch.condition: SlotIdx` type.

---

## 3. Boundary 2: vb_compile → vb_core (IR Emission Boundary)

**Artifacts crossing:**

| Direction | Type | Description |
|---|---|---|
| vb_compile → vb_core | `CompiledNode { kind: ChooseSlot { ... } }` | The dispatch node |
| vb_compile → vb_core | `SlotBranch { condition: SlotIdx, target: StepIdx }` | Per-branch routing |
| vb_compile → vb_core | `CompiledNode[]` (body nodes) | `SetConst` and `Do` nodes for branch bodies |
| vb_compile → vb_core | `ConstValue[]` | Constants used in body `Set` steps |

**Invariants at boundary:**
1. `SlotBranch.condition` is always a valid `SlotIdx` (recorded via `builder.record_slot`)
2. `SlotBranch.target` is always a valid `StepIdx` within the emitted node range
3. Branch body nodes are sequenced contiguously after the `ChooseSlot` node
4. Total node count matches `choose_width(branches)` precomputation

**Post-boundary validation:**
- `vb_validate::shared::validate(parts)` checks all slot references against `slot_count`
- `validate_slot_choose` checks all `SlotBranch.target` against `nodes.len()`
- `validate_choice` (graph) checks all targets are forward edges
- `validate_reachability` ensures all body nodes are reachable

---

## 4. Boundary 3: vb_core → vb_runtime (Replay Boundary)

**Artifacts crossing:**

| Direction | Type | Description |
|---|---|---|
| vb_core → vb_runtime | `CompiledWorkflow` | Fully validated workflow with `ChooseSlot` + body nodes |

**Runtime handler:**
- `replay_choose_slot(run, branches, otherwise)` at `vb_core/src/replay/choose.rs`
- Already supports per-branch `target: StepIdx` — **no runtime changes needed**

**Runtime invariants:**
1. All `SlotBranch.condition` slots must be initialized and contain `Bool` values
2. At least one branch must be true OR `otherwise` must be `Some`
3. Body nodes are stepped through linearly after the branch target is resolved

---

## 5. Slot Allocation Boundary

**Slot allocation during lowering:**

```
slot_from_text("when_string") → SlotIdx (condition slots)
              
For each branch body with Set steps:
    builder.record_slot(SlotIdx::new(next_available))
    
All slots are recorded in the same SlotCompiler instance (global slot namespace).

Post-condition: slot_count matches (number of condition slots + body output slots).
```

**Constraint:** Condition slots and body output slots share the same `u16` index space. The total must not exceed `u16::MAX`.

---

## 6. Crate Responsibility Matrix

| Crate | Role | Choose-specific responsibility |
|---|---|---|
| `vb_yaml` | YAML parsing and AST | Define `ChooseBranch { when, steps }` type |
| `vb_yaml` | Name validation | Validate step IDs in branch bodies |
| `vb_compile` | Lowering pipeline | `lower_canonical_choose`: resolve strings, lower bodies |
| `vb_compile` | Layout computation | `choose_width`: precompute total node count |
| `vb_compile` | Expression compilation | Compile body `Set` values to `ConstIdx` |
| `vb_compile` | Slot compiler | Allocate condition + body output slots |
| `vb_core` | IR types | Define `ChooseSlot`, `SlotBranch`, `CompiledNodeKind` |
| `vb_core` | Validation | Graph, nodes, targets, reachability checks |
| `vb_core` | Replay | `replay_choose_slot`: runtime dispatch |
| `vb_validate` | Shared validation | Invoked by `compile_source` before returning `CompiledWorkflow` |

---

## 7. What Must NOT Change

- `vb_core::CompiledNodeKind::ChooseSlot` — already supports `SlotBranch.target` per branch
- `vb_core::replay_choose_slot` — already uses per-branch targets
- `vb_core::validation::nodes::validate_slot_choose` — already checks per-branch slots and targets
- `vb_core::validation::graph::validate_choice` — already validates per-branch forward edges
- `vb_yaml::ast::ChooseBranch` — already has `steps: Vec<StepAst>`

---

## 8. What MUST Change

| File | Function | Change |
|---|---|---|
| `part_01.rs:117-122` | `choose_width` | Return `1 + sum(body_width(branch.steps, 0))` instead of `Ok(1)` |
| `part_02.rs:216-293` | `lower_canonical_choose` | Replace the "reject non-empty body" gate with body lowering logic |
| `part_02.rs:251-259` | `lower_canonical_choose` (body rejection) | **Remove** the `UnsupportedStepPrimitive` branch. Replace with per-branch body lowering. |

**No changes needed in:**
- `vb_core` (all types and runtime logic already support the IR shape)
- `vb_yaml` (AST already supports `steps` in branches)
- `vb_validate` (already validates per-branch targets)
