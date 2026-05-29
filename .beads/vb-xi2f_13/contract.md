# Contract: vb-xi2f.13 — Nested Choose Primitive Body Lowering

## Bead Overview

- **Bead ID:** vb-xi2f.13
- **Title:** P0 bug — lower nested choose primitive bodies in compiler (vb_compile)
- **Domain:** `vb_compile` lowering — `lower_canonical_choose` and `choose_width`
- **Governing norm:** `velvet-ballistics-MASTER.md` — compiler pipeline, formal verification mandate

---

## 1. What This Contract Is About

The `lower_canonical_choose` function in `part_02.rs` currently **rejects** any `ChooseBranch` with a non-empty `steps` body via:

```rust
if !branch.steps.is_empty() {
    return Err(CompileErrors(vec![
        CompileError::UnsupportedStepPrimitive {
            step: index,
            primitive: "choose",
        },
    ]));
}
```

The IR types (`CompiledNodeKind::ChooseSlot`, `SlotBranch.target: StepIdx`) and the replay engine (`replay_choose_slot`) already support per-branch body targets. The missing piece is the compiler lowering: body steps must be lowered into IR nodes, and each branch's `SlotBranch.target` must point to the first body node of that branch (or the common "next" step for empty branches).

---

## 2. EARS Preconditions and Postconditions

### 2.1 `choose_width(branches) -> Result<usize, CompileError>` (MODIFIED)

**Precondition:** `branches` is a slice of `vb_yaml::ast::ChooseBranch`.

**Current behavior:**
```rust
pub(super) fn choose_width(_branches: &[vb_yaml::ast::ChooseBranch]) -> Result<usize, CompileError> {
    Ok(1)  // All branches must have empty bodies.
}
```

**Required behavior:**
- **WHEN** branches are non-empty, **THEN** the return value is `1 + sum of body_width(branch.steps, 0) for each branch`.
- **WHEN** all branches have empty bodies, **THEN** the return value is `1` (unchanged).
- **WHEN** `body_width` computation overflows, **THEN** return `Err(CompileError::StepIndexOutOfRange { value })`.

**Postcondition:** Returned `usize` is exactly `1 + SUM(branch_i: body_width(branch_i.steps, 0))`. Overhead `0` means no pre-body node is needed per branch — body steps start at the first available step after the `ChooseSlot` node.

### 2.2 `lower_canonical_choose(...) -> Result<(), CompileErrors>` (MODIFIED)

**Preconditions (retained):**
- `branches.len() ≤ 64`
- `!branches.is_empty() || otherwise.is_some()`
- If `otherwise` is `Some(label)`, `label` exists in `step_names`

**Preconditions (new):**
- Each branch's body steps must only contain `Set` or `Do` primitives
- Body step constants must be parseable to `ConstIdx`
- Body step slots must not overflow `u16`

**Current behavior (lines 244-260):**
```
1. Compute `next` as the target step (must exist: error if None)
2. For each branch: reject if `!branch.steps.is_empty()` → CompileError
3. All `SlotBranch.target` assigned to `next`
```

**Required behavior (replacing lines 242-293):**
```
1. Validate `next` is Some (no change: error if None)
2. For each branch:
   a. If branch.steps.is_empty():
      - slot_branch.target = next
   b. If !branch.steps.is_empty():
      - Compute body_start = current step pointer (starts at id + 1)
      - slot_branch.target = body_start
      - For each step in branch.steps:
        * Lower the step into CompiledNode(s)
        * Push nodes to builder
        * Advance current step pointer
      - Last body node's `next` field = common `next` (post-choose step)
3. Assemble SlotBranch array with all targets assigned
4. Call lower_choose(id, slot_branches, otherwise_target, builder)
5. Push ChooseSlot node to builder
```

**Postcondition 1 (emitted nodes):** Exactly `choose_width(branches)` nodes are emitted (including the `ChooseSlot` node and all body nodes).

**Postcondition 2 (target validity):** Every `SlotBranch.target` points to a valid `StepIdx` within the emitted node range.

**Postcondition 3 (body fallthrough):** For each branch with non-empty body, the last body node's `next` (or implicit fallthrough) reaches the common post-choose `next` step. Body nodes of branch N never fall through into body nodes of branch N+1.

**Postcondition 4 (slot correctness):** All `SlotBranch.condition` slots are recorded. All body output slots are recorded. No slot index collision exists between condition slots and body output slots.

**Postcondition 5 (no YAML in IR):** No `String` condition values appear in the emitted IR. All conditions are `SlotIdx`.

**Postcondition 6 (IR validation):** The emitted IR passes `vb_validate::shared::validate(parts)`.

### 2.3 Body Step Lowering Contract

**Precondition:** `steps: &[StepAst]` in a branch body is non-empty.

**Allowed primitives in body steps:**
- `StepPrimitive::Set { output, value }` — lower to `CompiledNodeKind::SetConst`
- `StepPrimitive::Do { action, input }` — lower to `CompiledNodeKind::Do`

**Forbidden primitives in body steps:**
- `Choose`, `ForEach`, `Together`, `Collect`, `Aggregate`, `Repeat`, `Wait`, `Ask`, `Finish`

**Body lowering algorithm:**
```
for (step_idx, step) in branch.steps.iter().enumerate():
    match &step.primitive:
        Set { output, value } =>
            1. Parse value → ConstIdx via body_constant_index
            2. Allocate output slot via builder.record_slot
            3. Emit CompiledNode { id: current_step_ptr, kind: SetConst { value: const_idx }, next: see below }
            4. Advance current_step_ptr
        Do { action, input } =>
            1. Parse action → ActionId (u16)
            2. Parse input → SlotIdx (u16) via slot_from_text or string→int parse
            3. Emit CompiledNode { id: current_step_ptr, kind: Do { action, input }, next: see below }
            4. Advance current_step_ptr
        _ => return Err(CompileErrors(vec![UnsupportedStepPrimitive]))

    // next assignment:
    if step_idx == branch.steps.len() - 1:
        node.next = common_next  // last body step → post-choose next
    else:
        node.next = Some(current_step_ptr)  // non-last step → next body step
```

### 2.4 Backward Compatibility

**WHEN** all branch bodies are empty, **THEN** the behavior is **identical** to the current behavior:
- `choose_width` returns `1`
- `SlotBranch.target` = `next` for all branches
- No body nodes emitted

---

## 3. Non-Goals (Out of Scope)

1. Nested choose within choose body steps (choose inside a branch body)
2. Support for primitives other than `Set` and `Do` in branch bodies
3. Per-branch step count limits independent of the 64-branch fanout limit
4. Compile-time boolean slot type validation
5. Changes to `vb_yaml` AST types or `vb_core` IR types
6. Changes to the replay engine or runtime dispatch

---

## 4. Implementation Contract

### File: `crates/vb_compile/src/mod_compile_lowering/part_01.rs`

**Function: `choose_width` (line 117-122)**

Replace body with:
```rust
pub(super) fn choose_width(
    branches: &[vb_yaml::ast::ChooseBranch],
) -> Result<usize, CompileError> {
    // 1 for the ChooseSlot node + sum of body widths across all branches
    let mut total: usize = 1;
    for branch in branches {
        total = total
            .checked_add(body_width(&branch.steps, 0)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: total })?;
    }
    Ok(total)
}
```

### File: `crates/vb_compile/src/mod_compile_lowering/part_02.rs`

**Function: `lower_canonical_choose` (lines 216-293)**

Replace with implementation that:
1. Retains fanout validation (lines 226-235)
2. Retains empty-branch-table validation (lines 236-241)
3. Retains `next` validation (lines 244-250)
4. **Removes** the non-empty body rejection (lines 251-259)
5. Retains `otherwise` resolution (lines 262-283)
6. **Adds** body lowering:
   - Track `current_step = id + 1` (or use layout to determine each branch's body start)
   - For each branch:
     - `slot_from_text(&branch.when, ...)` → condition `SlotIdx`
     - If `branch.steps.is_empty()`: `target = next`
     - Else: `target = current_step`; lower each body step; advance; last body step → `next`
   - Assemble `SlotBranch { condition, target }`
7. Calls `lower_choose(id, slot_branches, otherwise_target, builder)`
8. Returns `Ok(())`

---

## 5. Acceptance Criteria

| # | Criterion | Verification Method |
|---|---|---|
| AC1 | `choose_width` returns correct count for non-empty branch bodies | Unit test: single branch with 2 Set steps → width = 3 (1 choose + 2 body) |
| AC2 | `choose_width` returns 1 for all-empty branches | Unit test: existing behavior preserved |
| AC3 | `lower_canonical_choose` emits `ChooseSlot` + body nodes | Integration: compile YAML with nested choose body |
| AC4 | Each `SlotBranch.target` points to correct body start | Assert: branch[0].target == id + 1; branch[1].target == id + 1 + width(branch[0]) |
| AC5 | Last body step falls through to `next`, not adjacent branch body | Assert: body_node[N-1].next == Some(common_next) |
| AC6 | All condition slots recorded via `builder.record_slot` | Assert: `builder.slot_count()` matches expected |
| AC7 | IR passes `vb_validate::shared::validate` | Assert: `validate(parts).is_ok()` |
| AC8 | Empty-body case produces identical IR to current behavior | Regression test: compile same YAML, compare node count and structure |
| AC9 | No YAML strings in IR | Assert: all `SlotBranch.condition` fields are `SlotIdx` values |
| AC10 | Fanout limit, route validation, and label resolution preserved | Existing tests pass unchanged |

---

## 6. Verification Strategy

| Layer | Tool | What to Verify |
|---|---|---|
| **Unit** | `#[test]` | `choose_width` computation, body lowering for single/multi-step bodies |
| **Unit** | `#[test]` | Error paths: body overflow, invalid primitive, constant parse failure |
| **Integration** | `#[test]` | End-to-end: YAML with choose bodies → `compile_source` → valid `CompiledWorkflow` |
| **Integration** | `#[test]` | Replay: compiled choose + body → `replay_choose_slot` + body node execution |
| **Regression** | `#[test]` | Empty branches still compile identically |
| **Proof** | Kani | `choose_width` arithmetic correctness (no overflow, exact match) |
| **Proof** | proptest | Layout/emission parity: random branch configurations, assert width matches node count |
| **Proof** | Flux | Body lowering slot preservation: slot_count matches allocations |

---

## 7. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Layout/width mismatch | Medium | Critical | Post-condition assertion; proptest |
| Body interleaving wrong | Medium | High | Exhaustive body-edge test |
| Slot collision | Low | Critical | Single SlotCompiler instance; global slot tracking |
| Regression of empty-body case | Low | High | Regression test suite |
| Compile-time performance regression | Low | Low | Acceptable; layout precomputation amortized |
