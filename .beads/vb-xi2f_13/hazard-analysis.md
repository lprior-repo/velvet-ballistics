# Hazard Analysis: Nested Choose Lowering

## Bead Context

- **Bead ID:** vb-xi2f.13
- **Domain:** What can go wrong with nested choose body lowering.

---

## 1. Temporal Hazards

### H1: Layout/Width Mismatch (SEVERITY: CRITICAL)
- **Description:** `choose_width` computes `N` total nodes, but the lowering emits `M` nodes (M ≠ N). The layout precomputation and body lowering must produce identical step counts.
- **Risk:** IR validation failure (graph edges point to non-existent or wrong steps). Worst case: silent corruption where targets land on adjacent branches' body nodes.
- **Mitigation:** Post-condition check in `lower_canonical_choose` that `node_count_emitted == choose_width(branches)`. Use the precomputed `CanonicalStepLayout.width` as the authoritative source.
- **Proof seed:** PS-TEMPORAL-001

### H2: Branch Body Steps Interleaving (SEVERITY: HIGH)
- **Description:** When multiple branches have non-empty bodies, the bodies are emitted sequentially in the IR. Branch N+1's body starts immediately after branch N's body ends. If the replay engine executes branch N's body and the final body node has `next` pointing to the wrong target (e.g., skipping to branch N+1's body instead of the post-choose `next` step), the execution path is wrong.
- **Risk:** A matched branch executes its body, then falls through into the next branch's body.
- **Mitigation:** Each branch body's last node must set `next` to the common post-choose `next` step, NOT to the next branch's body start.
- **Proof seed:** PS-TEMPORAL-002

### H3: Otherwise Target Amidst Branch Bodies (SEVERITY: MEDIUM)
- **Description:** The `otherwise` target points to a step that lies within a branch body range rather than after all branch bodies.
- **Risk:** An otherwise-matched execution re-enters a body range from the wrong starting point.
- **Mitigation:** `otherwise` must always target a step after all branch body nodes. Enforced by `validate_forward_target` which checks `target > ci` but does not check that `target` is not within another branch's span. Additional per-branch span validation may be needed.
- **Proof seed:** PS-TEMPORAL-003

### H4: Body Width Overflow (SEVERITY: LOW)
- **Description:** `body_width` computation overflows `usize` when summing large numbers of body steps.
- **Risk:** Integer overflow in layout computation could produce a small `StepIdx` that collides with other steps.
- **Mitigation:** `checked_add` is already used in `body_width`. The existing `canonical_layout` also uses `checked_add`. Both must continue to use checked arithmetic.
- **Proof seed:** PS-ARITH-001

---

## 2. Invariant Hazards

### H5: SlotIndex Reuse (SEVERITY: CRITICAL)
- **Description:** Two different branches allocate the same `SlotIdx` for their body output slots, causing data corruption at runtime.
- **Risk:** Branch A writes to slot 5; branch B also writes to slot 5. The replay engine reads stale data.
- **Mitigation:** `SlotCompiler` allocates slots globally. As long as `record_slot` is called for every body output slot, no reuse occurs. The invariant `slot_count == number of unique recorded slots` must hold.
- **Proof seed:** PS-INVARIANT-001

### H6: Condition Slot Overwritten by Body (SEVERITY: HIGH)
- **Description:** A body `Set` step within a branch writes to a slot that is also used as a `SlotBranch.condition` for the same or another branch.
- **Risk:** Branch B's body overwrites branch A's condition slot, corrupting the condition value before branch B's condition is evaluated.
- **Mitigation:** Condition slots and body output slots must be disjoint. The `slot_from_text` function maps condition names to pre-existing slot indices from the workflow's named outputs. Body `Set` steps allocate fresh output slots via `builder.record_slot`. As long as condition slot names and body output names are distinct in the YAML source, this hazard is avoided. Validate that condition slot indices != body output slot indices.
- **Proof seed:** PS-INVARIANT-002

### H7: StepIdx Overflow (SEVERITY: LOW)
- **Description:** Body lowering produces more than `u16::MAX` nodes, causing `StepIdx::new()` to wrap or fail.
- **Risk:** Node IDs are corrupted, validation rejects the workflow.
- **Mitigation:** `StepIdx::new(val)` requires `val: u16`. The layout computation uses `step_idx(cursor)` which returns `Result<StepIdx, CompileError>` and checks for overflow. `CompileError::StepIndexOutOfRange` catch fires before any nodes are emitted.
- **Proof seed:** PS-ARITH-002

### H8: Branch Fanout Evasion (SEVERITY: LOW)
- **Description:** The 64-branch fanout limit is enforced at `lower_canonical_choose` but could be bypassed if branches are emitted via a different code path.
- **Risk:** Could theoretically produce a `ChooseSlot` node with more than 64 branches, violating the budget/performance contract.
- **Mitigation:** The fanout limit is also checked in `lower_choose` (part_06.rs line 26) which is the only path to creating a `ChooseSlot`. It is checked in both places, providing defense in depth.
- **Proof seed:** PS-FANOUT-001

---

## 3. Refinement Hazards (Runtime Behavior)

### H9: Boolean Slot Type Mismatch (SEVERITY: HIGH)
- **Description:** A `SlotBranch.condition` is compiled to a `SlotIdx` that holds a non-boolean value at runtime (e.g., `I64`, `String`).
- **Risk:** `replay_choose_slot` returns `ReplayError::Internal { reason: "choose_slot condition is not boolean" }`.
- **Mitigation:** Currently no compile-time type tracking for slot value types. The `when` string maps to a slot name, but the slot's runtime type is not validated at compile time. **This is an existing gap, not introduced by this bead.**
- **Proof seed:** PS-TYPE-001

### H10: All-Branches-False with Missing Otherwise (SEVERITY: MEDIUM)
- **Description:** At runtime, all branch conditions evaluate to `false` and `otherwise` is `None`.
- **Risk:** `replay_choose_slot` returns `Internal` error, which may fail the entire run.
- **Mitigation:** This is a valid error state documented in the replay engine. The workflow author is responsible for providing an `otherwise` when all-false is possible. The compiler does not enforce liveness — this is a semantic workflow design decision.
- **Proof seed:** PS-LIVENESS-001

---

## 4. Concurrency Hazards

### H11: Slot Race During Body Execution (SEVERITY: LOW — not applicable to choose)
- **Description:** In a `together` (parallel) context, body steps within choose branches might race if choose appears inside parallel branches.
- **Risk:** Data races on slot writes.
- **Analysis:** `ChooseSlot` is a sequential dispatch node — at runtime, only one branch body executes. The replay engine is single-threaded per workflow run. No concurrency hazard exists for choose nodes themselves. If choose appears inside `Together` branches, the together isolation protects slot writes per branch. **No process-level concurrency hazard.**
- **Proof seed:** PS-CONCURRENCY-001

---

## 5. Hostile Input Hazards

### H12: YAML Injection via `when` String (SEVERITY: MEDIUM)
- **Description:** A malicious `when` string could name a non-existent slot or a slot outside the workflow's slot namespace, causing runtime errors.
- **Risk:** Denial of service at compile time (expected) or runtime slot access violation.
- **Mitigation:** `slot_from_text` fails closed for unknown slot names. Runtime `SlotNotAvailable` error is handled gracefully by the replay engine.
- **Proof seed:** PS-INPUT-001

### H13: Excessively Deep Body Nesting (SEVERITY: LOW)
- **Description:** A choose branch body contains another choose, which contains another choose, etc., leading to exponential node count growth.
- **Risk:** Stack overflow in the compiler or runtime. Denial of service.
- **Mitigation:** YAML depth limit (`DepthLimit`) caps total nesting depth. Node count limit (`NodeLimit`) caps total nodes. Both are enforced at the parser level in `vb_yaml`.
- **Proof seed:** PS-INPUT-002

---

## 6. Performance Hazards

### H14: Body Node Bloat (SEVERITY: LOW)
- **Description:** A branch body with dozens of steps generates many IR nodes, increasing compile time and runtime traversal cost.
- **Risk:** Acceptable — this is an authoring choice. The layout precomputation ensures compile-time resources are bounded.
- **Mitigation:** Per-branch step count limits could be added as a future refinement. Not in scope for this bead.

---

## 7. Summary of Hazard Coverage

| Hazard ID | Category | Severity | Covered By |
|---|---|---|---|
| H1 | Temporal / Layout Mismatch | CRITICAL | PS-TEMPORAL-001 |
| H2 | Temporal / Interleaving | HIGH | PS-TEMPORAL-002 |
| H3 | Temporal / Otherwise Span | MEDIUM | PS-TEMPORAL-003 |
| H4 | Arithmetic / Overflow | LOW | PS-ARITH-001 |
| H5 | Invariant / Slot Reuse | CRITICAL | PS-INVARIANT-001 |
| H6 | Invariant / Condition Overwrite | HIGH | PS-INVARIANT-002 |
| H7 | Arithmetic / StepIdx | LOW | PS-ARITH-002 |
| H8 | Fanout / Evasion | LOW | PS-FANOUT-001 |
| H9 | Type / Boolean Slot | HIGH | PS-TYPE-001 |
| H10 | Liveness / No Otherwise | MEDIUM | PS-LIVENESS-001 |
| H11 | Concurrency (N/A) | LOW | PS-CONCURRENCY-001 |
| H12 | Hostile Input / When | MEDIUM | PS-INPUT-001 |
| H13 | Hostile Input / Depth | LOW | PS-INPUT-002 |
| H14 | Performance / Bloat | LOW | (Deferred) |
