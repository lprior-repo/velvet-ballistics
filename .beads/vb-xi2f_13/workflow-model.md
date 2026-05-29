# Workflow Model: Nested Choose Lowering

## Bead Context

- **Bead ID:** vb-xi2f.13
- **Domain:** Lowering workflow — YAML choose AST → IR ChooseSlot with branch bodies.

---

## 1. State Machine Overview

The lowering workflow transforms a single `StepPrimitive::Choose` from the YAML AST into a collection of IR nodes: one `ChooseSlot` node plus zero or more body nodes per branch.

### Legal States

```
[1] VALIDATE_FANOUT     — check branches.len() ≤ 64
[2] VALIDATE_ROUTE      — check non-empty branches ∨ otherwise present
[3] RESOLVE_OTHERWISE   — resolve otherwise label → Option<StepIdx>
[4] LOWER_CONDITIONS    — convert each branch.when → SlotIdx (SlotBranch.condition)
[5] LOWER_BODIES        — for each branch, lower branch.steps → body nodes + assign target
[6] BUILD_CHOOSE_SLOT   — assemble CompiledNodeKind::ChooseSlot with branches and otherwise
[7] PUSH_NODES          — push ChooseSlot node + all body nodes to builder
[8] DONE                — success
```

### Error States

```
[E1] FANOUT_EXCEEDED    — branches.len() > 64
[E2] EMPTY_NO_FALLBACK  — branches.is_empty() && otherwise.is_none()
[E3] UNKNOWN_LABEL      — otherwise label not found in step_names
[E4] LABEL_OVERFLOW     — resolved otherwise index > u16::MAX
[E5] BODY_WIDTH_OVF     — body_width sum overflows usize
[E6] STEP_IDX_OVF       — generated StepIdx exceeds u16::MAX
[E7] SLOT_OVF           — slot allocation exceeds u16::MAX
[E8] UNSUPPORTED_PRIM   — body step uses unsupported primitive
```

---

## 2. Transition Diagram

```
                    ┌──────────────────┐
                    │  [1] VALIDATE    │
                    │     FANOUT       │
                    └────┬────────┬────┘
                         │        │
                    ≤64  │        │ >64
                         │        │
                    ┌────▼────┐   └──► [E1] FANOUT_EXCEEDED
                    │[2] VALID│
                    │  ROUTE  │
                    └────┬───┬┘
                         │   │
              ok         │   │ empty && no otherwise
                         │   │
                    ┌────▼┐  └──► [E2] EMPTY_NO_FALLBACK
                    │[3]   │
                    │RESOLV│──► [E3] UNKNOWN_LABEL
                    │OTHER │──► [E4] LABEL_OVERFLOW
                    └──┬───┘
                       │
                  ┌────▼───┐
                  │[4]     │──► (compile error if slot_from_text fails)
                  │LOWER   │
                  │CONDS   │
                  └──┬─────┘
                     │
                ┌────▼───────┐
                │[5] LOWER   │──► [E5..E8] body lowering errors
                │   BODIES   │
                └──┬─────────┘
                   │
              ┌────▼─────┐
              │[6] BUILD │
              │CHOOSE_SLT│
              └──┬───────┘
                 │
            ┌────▼─────┐
            │[7] PUSH  │
            │  NODES   │
            └──┬───────┘
               │
          ┌────▼────┐
          │[8] DONE │
          └─────────┘
```

---

## 3. Detailed Transition Descriptions

### T1: VALIDATE_FANOUT → [VALIDATE_FANOUT|FANOUT_EXCEEDED]
- **Guard:** `branches.len() ≤ 64`
- **On pass:** advance to VALIDATE_ROUTE
- **On fail:** `CompileError::PrimitiveLoweringLimitExceeded { primitive: "choose", field: "branches", value: branches.len(), limit: 64 }`
- **Idempotent:** pure check, no side effects

### T2: VALIDATE_ROUTE → [RESOLVE_OTHERWISE|EMPTY_NO_FALLBACK]
- **Guard:** `!branches.is_empty() || otherwise.is_some()`
- **On pass:** advance to RESOLVE_OTHERWISE
- **On fail:** `CompileError::Workflow(WorkflowError::EmptyBranchTable)`
- **Rationale:** An empty branch table with no `otherwise` can never route anywhere.

### T3: RESOLVE_OTHERWISE → [LOWER_CONDITIONS|error]
- **Input:** `otherwise: Option<&str>`
- **Action:** If `Some(label)`:
  1. Find `label` position in `step_names` via `.iter().position()`
  2. Convert position to `u16` → `StepIdx::new()`
  3. Wrap in `Some(StepIdx)`
- **Errors:**
  - `Some(label)` not found → `UnknownStepLabel { step: index, label }`
  - Position > `u16::MAX` → `PrimitiveLoweringLimitExceeded`

### T4: LOWER_CONDITIONS → [LOWER_BODIES|error]
- **Action:** For each `branch` in `branches`:
  1. Call `slot_from_text(&branch.when, index, "choose.branches[].when")`
  2. Build `SlotBranch { condition, target: PLACEHOLDER }`
  3. Store `SlotBranch` in a `Vec`
- **Note:** `target` is placeholder at this stage; assigned during LOWER_BODIES.
- **Error:** If `slot_from_text` fails for any branch.

### T5: LOWER_BODIES → [BUILD_CHOOSE_SLOT|error] **(NEW — this bead's core fix)**

**Precondition:** `step_width = choose_width(branches)` was precomputed by `canonical_layout`, and the `id` (starting `StepIdx`) has been allocated.

**Current behavior (broken):** All branches must have empty bodies; any non-empty body is rejected.

**Required behavior:**
1. Let `current_step = id.checked_add(1)` (the step after the `ChooseSlot` node)
2. For each `SlotBranch` (originally from LOWER_CONDITIONS):
   - If `branch.steps.is_empty()`:
     - `slot_branch.target = next` (the step after all branch bodies)
   - If `!branch.steps.is_empty()`:
     - `slot_branch.target = current_step`
     - For each `step` in `branch.steps`:
       - Lower `step` into one or more `CompiledNode`s, each with `id = current_step`
       - Push each node to `builder`
       - Advance `current_step` by the node count for this step
     - The **last** body node for this branch gets `next` set as its `next` field (or the per-step next depends on layout)
3. All `SlotBranch.target` values are now assigned
4. Continue to BUILD_CHOOSE_SLOT

**Constraints:**
- Branch body nodes must not overlap with other branches' nodes
- The total step consumption must match `choose_width(branches) - 1`
- `body_width` calculation must be deterministic and exact

### T6: BUILD_CHOOSE_SLOT → [PUSH_NODES]
- **Action:** Call `lower_choose(id, slot_branches, otherwise, builder)`
- **Output:** `CompiledNode { id, kind: CompiledNodeKind::ChooseSlot { branches, otherwise } }`
- **Validation:** `lower_choose` internally calls `validate_branch_route` and `record_slot` for each condition.

### T7: PUSH_NODES → [DONE]
- **Action:** Push `ChooseSlot` node to builder (as a single-element `Vec` or directly)
- **Note:** Body nodes were already pushed during T5.
- **Postcondition:** Builder now contains all nodes for this choose step.

### T8: DONE
- **Terminal state.** `lower_canonical_choose` returns `Ok(())`.

---

## 4. Guards

| Guard | Check | Source |
|---|---|---|
| G1: Fanout limit | `branches.len() <= 64` | `lower_canonical_choose` line 226 |
| G2: Route viability | `!branches.is_empty() \|\| otherwise.is_some()` | `lower_canonical_choose` line 237 |
| G3: Otherwise label reference | `step_names` contains `otherwise` label | `lower_canonical_choose` line 263 |
| G4: Body width within u16 | `choose_width` return value fits in `u16` | `part_01.rs` layout computation |
| G5: Slot count within u16 | All slots from `record_slot` remain in range | `SlotCompiler` |
| G6: Node count within u16 | All generated nodes' ids remain in range | `SlotCompiler::push_node` |

---

## 5. Outcomes

| Outcome | Terminal | Description |
|---|---|---|
| `Ok(())` | Yes | Choose lowered successfully with all branches and body nodes |
| `Err(CompileErrors(vec![...]))` | Yes | Compilation failed; one of E1-E8 |

---

## 6. Commands and Events (Compilation Model)

The lowering does not produce runtime events — it produces IR data. However, for completeness:

| Command | Actor | Description |
|---|---|---|
| `lower_canonical_step(choose)` | `compile_source` | Dispatches to `lower_canonical_choose` |
| `slot_from_text(when)` | Compiler utility | Resolves a when-string to a SlotIdx |
| `builder.push_node(node)` | `SlotCompiler` | Appends an IR node |
| `builder.record_slot(slot)` | `SlotCompiler` | Registers a slot index |

---

## 7. Temporal Hazards in Lowering Workflow

| Hazard | Manifestation | Mitigation |
|---|---|---|
| **Layout/width mismatch** | `choose_width` returns N, but lowering emits M nodes (M ≠ N) | Precompute layout exactly; validate post-condition that `step_ptr - startid == choose_width - 1` |
| **Target confusion** | `SlotBranch.target` points to wrong step after body node insertion | Assign targets after all body nodes are allocated, using the precomputed branch body starts from layout |
| **Slot collision** | Two branches allocate the same `SlotIdx` for different purposes | `SlotCompiler` tracks all allocated slots globally; body lowering reuses the same builder |
| **Step index overflow** | Body lowering generates so many nodes that `StepIdx` overflows u16 | Rejected at layout time: `canonical_layout` fails with `StepIndexOutOfRange` |
| **Non-deterministic ordering** | Branch body steps compile in different order on different runs | Branches are processed in order; deterministic by design |
