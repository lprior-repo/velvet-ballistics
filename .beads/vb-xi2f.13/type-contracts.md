# Type Contracts: Nested Choose Lowering

## Bead Context

- **Bead ID:** vb-xi2f.13
- **Domain:** Type-level contracts for `choose` lowering across `vb_yaml`, `vb_compile`, and `vb_core` boundaries.
- **Checklist:** `type-contract-checklist.md` (newtypes, enum flags, Option→typestate, boundary parsing, semantic errors, pure-core isolation)

---

## 1. Newtype and Semantic Typing Checks

### TC1: `when` field cannot be primitive string in IR
- **Current:** `SlotBranch.condition: SlotIdx` — already numeric. ✓
- **YAML boundary:** `ChooseBranch.when: String` — must be resolved to `SlotIdx` at compile time.
- **Proof seed target:** The mapping `String → SlotIdx` via `slot_from_text` must always produce a valid `SlotIdx` or a compilation error. No string escapes into the IR.

### TC2: Branch body steps are typed `Vec<StepAst>`, not bare YAML
- **Current:** `ChooseBranch.steps: Vec<StepAst>` — already typed. ✓
- **Lowering output:** Each `StepAst` in the body must lower to a valid sequence of `CompiledNode` IR nodes.
- **Constraint:** Body steps must only contain the primitives supported by body lowering: `Set`, `Do`. The set of allowed primitives in branch bodies should match `emit_single_body_set`'s supported primitives (or be extended).

### TC3: `otherwise` label must be an enumerated step name, not a free string
- **Current:** `ChooseBranch.otherwise: Option<String>` — string at boundary.
- **Resolution:** `lower_canonical_choose` resolves it to `Option<StepIdx>` via `step_names` position lookup.
- **Proof seed target:** When `otherwise` is `Some(label)`, `step_names.iter().position()` must find `label`. Otherwise, `UnknownStepLabel` error must be emitted.

### TC4: Branch count stored as `usize`, validated against limit
- **Current:** Lowering validates `≤ 64`. IR uses `Box<[SlotBranch]>` unchecked at type level.
- **Improvement:** A `ChooseBranchCount` newtype wrapping `u8` would make the 0..64 bound representable at the type level, but this change spans `vb_yaml`, `vb_compile`, and `vb_core`. Out of scope for this bead.

### TC5: `SlotBranch.target` must point to a valid IR node
- **Current:** `StepIdx(u16)` — valid within 0..65535 for any workflow. The graph validator checks `target < nodes.len()`.
- **Body targeting:** Each branch's `target` now points to the **first** body node for that branch, not directly to the `next` step. The final body node falls through to `next` (or the `next` after all branch bodies).

---

## 2. Enum Replacements for Boolean/Flag Behavior

### TC6: Branch body emptiness is not a boolean flag
- **Current:** The compiler checks `if !branch.steps.is_empty()` and rejects. This is a gate, not a behavioral flag.
- **Fix:** Remove the gate. Branch body emptiness becomes a natural case: empty body → `target = next` (current behavior); non-empty body → `target = first_body_step` and lowering emits the body nodes.

### TC7: choose_width must not be boolean "1 or bust"
- **Current:** `choose_width` hardcodes `1`, assuming empty bodies.
- **Fix:** `choose_width(branches)` returns `1 + sum over branches of body_width(branch.steps, 0)`. The `+1` is the `ChooseSlot` node itself. The `0` overhead means branch bodies start at the next available slot (no prepended node per branch).

---

## 3. Option / Nullable Lifecycle → Explicit State

### TC8: `otherwise: Option<StepIdx>` is correct — no change needed
- **Semantics:** `None` means "no fallback; all-false is an error at runtime."
- **Alternative considered:** Adding an `EmptyBranchTable` typestate. Rejected — the validator already handles this at the graph level.

### TC9: Branch body lowering state
- The body-lowering function should explicitly track:
  - **Allocated slots per branch** (each body may produce output slots)
  - **Branch body start `StepIdx`** (the `target` for the `SlotBranch`)
  - **Fallthrough `StepIdx`** (the step after this branch's last body node; typically `next` from `lower_canonical_choose`)
- This is internal to the compiler lowering; no new public types needed.

---

## 4. Boundary Parsing Rules

### TC10: YAML `when` → `SlotIdx` is a boundary parse
- **Input:** YAML string `when` (e.g., `"auth_approved"`)
- **Parser:** `slot_from_text(text, index, "choose.branches[].when")`
- **Output:** `SlotIdx` or `CompileError`
- **Invariant:** The resolved `SlotIdx` must be a **boolean** slot at runtime. Currently unenforced at compile time. **Hazard.**

### TC11: Body step lowering is a nested boundary
- Body `StepAst` nodes contain their own `Set.value`, `Do.action`, `Do.input` fields that must be parsed from YAML strings to IR indices.
- The existing `emit_single_body_set` handles single-step bodies. Multi-step bodies require iteration over `branch.steps`, each lowered independently, with accumulated slot allocation.

---

## 5. Pure Core / Impure Shell Boundaries

| Layer | Component | Purity |
|---|---|---|
| YAML parsing | `vb_yaml` (saphyr) | **Impure** — reads raw YAML bytes |
| Validation | `vb_validate` (graph, nodes, targets) | **Pure** — operates on IR structures only |
| Lowering | `vb_compile::lower_canonical_choose` | **Pure** — maps `StepAst` to `CompiledNode` |
| Expression compilation | `vb_compile::expression` | **Pure** — lexer/parser for cold expressions |
| Runtime dispatch | `vb_core::replay_choose_slot` | **Pure** — reads slots, writes PC |
| Slot resolution | `vb_compile::slot_from_text` | **Pure** — maps string names to indices via local map |

### TC12: No I/O in lowering
- `lower_canonical_choose` must not perform I/O, time, network, or random operations.
- The lowering function produces `Vec<CompiledNode>` deterministically from `&[ChooseBranch]` and `step_names`. ✓ (already true)

---

## 6. Semantic Error Mapping

| YAML Error Surface | CompileError Variant | When |
|---|---|---|
| Branch count > 64 | `PrimitiveLoweringLimitExceeded { primitive: "choose", field: "branches", value, limit: 64 }` | `lower_canonical_choose` |
| Empty branches, no otherwise | `Workflow(EmptyBranchTable)` | `lower_canonical_choose` |
| Unknown `otherwise` label | `UnknownStepLabel { step, label }` | Label resolution |
| Body step too many (new) | `StepIndexOutOfRange { value }` | Body width overflow |
| Invalid body step primitive (new) | `UnsupportedStepPrimitive { step, primitive }` | Body step not `Set`/`Do` |
| Invalid `when` slot | `SlotIndexOutOfRange { value }` or expression error | `slot_from_text` |
| Body slot overflow (new) | `SlotIndexOutOfRange { value }` | Exceeded u16 slot count |

---

## 7. Typestate Design for Branch Body Lowering

The lowering function's internal typestate:

```
BodyLoweringState:
  [Initial] → computed `choose_width` reserves step range
  [PerBranch] → for each branch:
    if empty: target = next, skip body emission
    if non-empty: target = current_step_ptr
      for each step: lower step → push CompiledNode, advance step_ptr
  [Complete] → all branches' SlotBranch.target assigned
```

No external typestate — the layout precomputation (`canonical_layout`) already defines fixed-width step ranges, and the lowering fills them in.
