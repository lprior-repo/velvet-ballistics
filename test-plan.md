# Test Plan: Nested Reduce Body Lowering (vb-xi2f.24)

## Summary
- **Bead**: vb-xi2f.24
- **State**: 8 (test-planner)
- **Source checkout**: `/home/lewis/src/vb-workspaces/vb-xi2f.24`
- **Bridge review**: APPROVED (State 7)
- **Proof obligations mapped**: 32 (11 Kani, 6 Flux, 13 proptest, 2 fuzz, 5 Verus WAIVED)
- **Behaviors identified**: 48
- **Trophy allocation**: 18 unit / 20 integration / 3 e2e / 3 static / 4 proptest
- **Proptest invariants**: 13 (planned in bridge, tested via proptest crate)
- **Fuzz targets**: 2 (BLOCKED_TOOLING — musl+sanitizer)
- **Kani harnesses**: 11 (existing, refine-only)
- **Mutation threshold target**: >=90%

---

## 1. Behavior Inventory

### C1: Multi-Step Body Width Calculation
| B01 | `canonical_body_step_width()` returns correct width for Reduce with nested Set body (1) |
| B02 | `canonical_body_step_width()` returns correct width for Reduce with nested Do body (1) |
| B03 | `canonical_body_step_width()` returns correct width for Reduce with nested ForEach body |
| B04 | `canonical_body_step_width()` returns correct width for Reduce with N=2 Set steps (2) |
| B05 | `canonical_body_step_width()` returns correct width for Reduce with N=3 Set steps (3) |
| B06 | `canonical_body_step_width()` returns width for Reduce with mixed Set+Do body steps |
| B07 | `canonical_body_step_width()` returns width for nested Reduce (recursive) via `body_width(body, 3)` |
| B08 | `canonical_body_step_width()` returns Err(UnsupportedStepPrimitive) for Finish in body |
| B09 | `canonical_body_step_width()` returns Err(UnsupportedStepPrimitive) for Wait in body |
| B10 | `canonical_body_step_width()` returns Err(UnsupportedStepPrimitive) for Ask in body |
| B11 | `canonical_step_width(Reduce { body, .. })` equals `body_width(body, 3)` |

### C2: Width-Node Count Synchronization
| B12 | `body_width(body, 3)` equals `3 + emit_reduce_body_steps(body, ...).node_count` for N=1 body |
| B13 | `body_width(body, 3)` equals `3 + emit_reduce_body_steps(body, ...).node_count` for N=2 body |
| B14 | `body_width(body, 3)` equals `3 + emit_reduce_body_steps(body, ...).node_count` for N=3 body |
| B15 | Width-node parity holds when ForEach is in body (ForEach width > 1) |
| B16 | Width-node parity holds for nested Reduce in body |

### C3: Body Step Sequential Assignment
| B17 | `emit_reduce_body_steps` assigns sequential, non-overlapping StepIdx to body steps |
| B18 | First body step gets `id + 1` (body_step) |
| B19 | Second body step gets `body_step + canonical_body_step_width(step_0)` |
| B20 | All StepIdx values are strictly increasing and within `[body_step, next_step)` |
| B21 | No two body steps occupy the same StepIdx |
| B22 | Body steps do not overlap with ReduceNext or ReduceFinish |

### C4: Body Step Next-Link Chain
| B23 | First body step's next points to second body step (when N>1) |
| B24 | Last body step's next points to next_step (ReduceNext) |
| B25 | When N=1: body step's next points to next_step |
| B26 | Intermediate body step i's next points to body step i+1 |
| B27 | All next references are `Some(step)` (no dangling chains) |
| B28 | Chain integrity verified end-to-end via `compile_source()` |

### C5: ReduceStart/ReduceNext Body Reference
| B29 | `ReduceStart.body` equals `body_step = id + 1` regardless of body length |
| B30 | `ReduceNext.body` equals `body_step = id + 1` (same as ReduceStart) |
| B31 | `ReduceStart.done` equals `done = next_step + 1` |

### C6: ReduceFinish Position
| B32 | `ReduceFinish.id` equals `next_step + 1` |
| B33 | `ReduceFinish.id` is strictly after ReduceNext.id |
| B34 | `ReduceFinish.next` equals the parent aggregate's next step |

### C7: Single-Step Body Compatibility (REGRESSION)
| B35 | `body.len() == 1` Set: `emit_reduce_body_steps` produces same IR as `emit_single_body_set` |
| B36 | `body.len() == 1` Do: `emit_reduce_body_steps` produces same IR as `emit_single_body_set` |
| B37 | `body.len() == 1` ForEach: `emit_reduce_body_steps` produces same IR as `emit_single_body_set` |
| B38 | Existing single-step reduce workflow tests (PO-R1..R8) continue to pass |

### C8: Nested Reduce Semantics
| B39 | Reduce in body step 0 (non-last): receives next = next_body_step |
| B40 | Reduce in body step N-1 (last): receives next = Some(next_step) |
| B41 | Nested reduce produces correct 3-node ReduceStart/ReduceNext/ReduceFinish substructure |
| B42 | Nested reduce's body steps do not collide with outer reduce's body steps |
| B43 | Nested reduce compiles through `try_from_parts()` end-to-end |

### C9: Symbolic Diagnostics
| B44 | Empty reduce body → `StepFieldShape { field: "steps", .. }` with valid code |
| B45 | Non-Set/Do/ForEach body step → `UnsupportedStepPrimitive { .. }` with valid code |
| B46 | Overflowing body width → `StepIndexOutOfRange { .. }` with valid code |
| B47 | All error codes are registered in CODE_REGISTRY (not `INTERNAL_INVARIANT`) |

### C10: Deterministic Lowering
| B48 | Same multi-step reduce source produces identical IR across repeated compilations |
| B49 | Same multi-step reduce source produces identical digest across repeated compilations |

### C11: No Panic
| B50 | `emit_reduce_body_steps` never panics on any valid body (proptest) |
| B51 | `lower_canonical_aggregate` never panics on any valid reduce config |
| B52 | `canonical_body_step_width` never panics on any StepPrimitive variant |
| B53 | Checked arithmetic (`checked_add`, `checked_step_offset`) catches all overflows |

### C12: Empty Body Handling
| B54 | `emit_reduce_body_steps` with `body.len() == 0` returns `Err(StepFieldShape { field: "steps", .. })` |
| B55 | Empty body emits zero body nodes |
| B56 | Empty body does not produce a broken chain (rejected before chain construction) |

---

## 2. Trophy Allocation

| Layer | Count | Behaviors | Rationale |
|-------|-------|-----------|-----------|
| **Unit / Calc** | 18 | B01-B11 (width calculation), B17-B22 (offset arithmetic), B29-B34 (node field assertions), B44-B47 (diagnostic codes), B48-B49 (digest determinism), B54-B56 (empty body) | Pure functions with no I/O — `canonical_body_step_width`, `body_width`, `checked_step_offset`. Test width arithmetic exhaustively with boundary values. Verify error variants are exact (not just `is_err()`). |
| **Integration** | 20 | B12-B16 (width-node parity), B23-B28 (chain integrity), B35-B43 (regression + nested reduce), B50-B53 (no panic via real lowering) | Component interactions using REAL builder/lowering — `emit_reduce_body_steps` with `SlotCompiler`, `lower_canonical_aggregate` with real builder, `compile_source()` end-to-end. No mocks. Verify IR structure via `CompiledWorkflow::try_from_parts()`. |
| **E2E** | 3 | B43 (nested reduce through try_from_parts), B48-B49 (full pipeline determinism) | Full workflow: YAML source → `compile_source()` → `try_from_parts()` → structural equality + digest match. |
| **Static Analysis** | 3 | Clippy zero-tolerance, `#![forbid(unsafe_code)]`, `#[must_use]` on width functions, destructured `StepPrimitive` match exhaustiveness in `emit_reduce_body_steps` | Compile-time checks: no unsafe, exhaustive match arms, no forbidden tokens. |
| **Proptest** | 13 (separate invocation) | Proof obligations PO-* (verification artifacts, not behavior tests) | Executed as separate proptest suite; planned in bridge, materialized at State 11-12. Behavior tests are independent of proptest harnesses. |

**Target ratios: ~38% unit, ~42% integration, ~6% e2e, ~6% static**

Deviation justification: The reduce body lowering pipeline has rich stateful interactions (builder state, sequential offset assignment, chain construction) that are best tested at the integration layer. The unit layer is thinner because the pure arithmetic functions (`body_width`, `canonical_body_step_width`) are small and well-bounded. The e2e layer is thin because the reduce lowering is exercised through `compile_source()`, which is already an integration-level entry point.

---

## 3. BDD Scenarios

### C1: Multi-Step Body Width Calculation

#### Behavior: B01 — `canonical_body_step_width` returns 1 for Set in body

```
fn canonical_body_step_width_returns_one_for_set_in_body():
    Given: a StepPrimitive::Set { value: "42" }
    When: canonical_body_step_width(&Set { .. }) is called
    Then: returns Ok(1)
```

#### Behavior: B02 — `canonical_body_step_width` returns 1 for Do in body

```
fn canonical_body_step_width_returns_one_for_do_in_body():
    Given: a StepPrimitive::Do { action: "1", input: "0" }
    When: canonical_body_step_width(&Do { .. }) is called
    Then: returns Ok(1)
```

#### Behavior: B03 — `canonical_body_step_width` returns ForEach width (including nested body)

```
fn canonical_body_step_width_returns_foreach_total_width():
    Given: a StepPrimitive::ForEach { body: [Set { value: "1" }] }
    When: canonical_body_step_width(&ForEach { .. }) is called
    Then: returns Ok(3)  // ForEach overhead=2 + body_width(body, 2)=1 → total 3
```

#### Behavior: B04 — `canonical_body_step_width` returns 2 for 2 Set steps in body

```
fn canonical_body_step_width_returns_two_for_two_set_steps_in_body():
    Given: a StepPrimitive::Reduce { body: [Set { .. }, Set { .. }] }
    When: canonical_body_step_width(&Reduce { .. }) is called
    Then: returns Ok(5)  // overhead=3 + body_width(body, 0)=2 → total 5 (nested Reduce body width)
```

#### Behavior: B05 — `canonical_body_step_width` returns 3 for 3 Set steps in body

```
fn canonical_body_step_width_returns_three_for_three_set_steps_in_body():
    Given: a StepPrimitive::Reduce { body: [Set, Set, Set] }
    When: canonical_body_step_width(&Reduce { .. }) is called
    Then: returns Ok(6)  // overhead=3 + body_width(body, 0)=3 → total 6
```

#### Behavior: B06 — `canonical_body_step_width` returns correct width for mixed Set+Do body

```
fn canonical_body_step_width_handles_mixed_set_do_body():
    Given: a StepPrimitive::Reduce { body: [Set, Do] }
    When: canonical_body_step_width(&Reduce { .. }) is called
    Then: returns Ok(5)  // overhead=3 + 1 + 1 → total 5
```

#### Behavior: B07 — `canonical_body_step_width` returns correct width for nested Reduce (recursive)

```
fn canonical_body_step_width_handles_nested_reduce_in_body():
    Given: a StepPrimitive::Reduce { body: [Reduce { body: [Set] }] }
    When: canonical_body_step_width(&Reduce { .. }) is called
    Then: returns Ok(8)  // outer overhead=3 + nested: body_width([Reduce{body:[Set]}], 3) = 3+body_width([Set],0)+body_width([],3)+? → need exact compute
```

#### Behavior: B08 — `canonical_body_step_width` rejects Finish in body

```
fn canonical_body_step_width_rejects_finish_in_body():
    Given: a StepPrimitive::Finish { result: 0 }
    When: canonical_body_step_width(&Finish { .. }) is called
    Then: returns Err(CompileError::UnsupportedStepPrimitive { primitive: "finish" })
```

#### Behavior: B09-B10 — rejects Wait/Ask in body

```
fn canonical_body_step_width_rejects_wait_in_body():
    Given: a StepPrimitive::Wait { .. }
    When: canonical_body_step_width(&Wait { .. }) is called
    Then: returns Err(CompileError::UnsupportedStepPrimitive { primitive: "wait" })

fn canonical_body_step_width_rejects_ask_in_body():
    Given: a StepPrimitive::Ask { .. }
    When: canonical_body_step_width(&Ask { .. }) is called
    Then: returns Err(CompileError::UnsupportedStepPrimitive { primitive: "ask" })
```

#### Behavior: B11 — `canonical_step_width(Reduce)` equals `body_width(body, 3)`

```
fn canonical_step_width_reduce_equals_body_width_plus_three():
    Given: a StepPrimitive::Reduce { body: [Set { .. }] }
    When: canonical_step_width(&Reduce { .. }) is called
    Then: result == body_width(&body, 3)
```

---

### C2: Width-Node Count Synchronization

#### Behavior: B12 — Width-node parity for N=1 body

```
fn width_node_parity_single_step_body():
    Given: a reduce body [Set { value: "1" }] and overhead=3
    When: body_width(body, 3) is computed
    And: emit_reduce_body_steps(body, body_step, idx, slot, next, builder) is executed
    Then: body_width result equals (3 + builder.node_count())
```

#### Behavior: B13 — Width-node parity for N=2 body

```
fn width_node_parity_two_step_body():
    Given: a reduce body [Set { .. }, Do { .. }] and overhead=3
    When: body_width(body, 3) is computed
    And: emit_reduce_body_steps emits body nodes
    Then: width equals (3 + nodes_emitted)
```

#### Behavior: B14 — Width-node parity for N=3 body

```
fn width_node_parity_three_step_body():
    Given: a reduce body [Set, Set, Set] and overhead=3
    When: body_width(body, 3) is computed
    And: emit_reduce_body_steps emits body nodes
    Then: width equals (3 + nodes_emitted)
```

#### Behavior: B15 — Width-node parity with ForEach in body

```
fn width_node_parity_with_foreach_in_body():
    Given: a reduce body [ForEach { body: [Set] }] and overhead=3
    When: body_width(body, 3) is computed
    And: emit_reduce_body_steps emits body nodes
    Then: width equals (3 + nodes_emitted)
    And: ForEach width (3) is accounted for, not just 1 step
```

#### Behavior: B16 — Width-node parity with nested Reduce in body

```
fn width_node_parity_with_nested_reduce_in_body():
    Given: a reduce body [Reduce { body: [Set] }] and overhead=3
    When: body_width(body, 3) is computed
    And: emit_reduce_body_steps emits body nodes
    Then: width equals (3 + nodes_emitted)
```

---

### C3: Body Step Sequential Assignment

#### Behavior: B17-B22 — Sequential, non-overlapping StepIdx values

```
fn emit_reduce_body_steps_assigns_sequential_distinct_step_indices():
    Given: a reduce body [Set, Set, Set] and body_step = StepIdx(10)
    When: emit_reduce_body_steps(body, body_step, idx, slot, next, builder) is executed
    Then: three nodes are emitted
    And: node[0].id == StepIdx(10) (first body step)
    And: node[1].id == StepIdx(11) (second body step, after Set width=1)
    And: node[2].id == StepIdx(12) (third body step)
    And: all IDs are distinct
    And: all IDs < next_step (ReduceNext sits after all body steps)

fn emit_reduce_body_steps_offsets_skip_for_foreach_body_step():
    Given: a reduce body [ForEach { body: [Set] }, Set]
    And: body_step = StepIdx(10)
    When: emit_reduce_body_steps body step dispatch runs
    Then: ForEach occupies offsets 10..13 (4 steps: ForEachStart + Set + ForEachCheck + ForEachFinish)
    And: the Set after ForEach starts at StepIdx(13) not StepIdx(11)

fn emit_reduce_body_steps_no_slot_collision_with_reduce_next():
    Given: a reduce body [Set, Set] and body_step = StepIdx(10)
    And: next_step = body_step + body_width(body, 0) = StepIdx(12)
    When: emit_reduce_body_steps emits body nodes
    Then: last body node at StepIdx(11)
    And: StepIdx(12) is not used by any body node (reserved for ReduceNext)
```

---

### C4: Body Step Next-Link Chain

#### Behavior: B23-B28 — Correct next-link chains

```
fn body_step_next_chain_first_to_second_when_multi_step():
    Given: a reduce body [Set, Do]
    When: emit_reduce_body_steps emits body nodes
    Then: node[0].next == Some(StepIdx(body_step + 1))
    And: node[0].next is not None

fn body_step_next_chain_last_links_to_next_step():
    Given: a reduce body [Set, Do] and next_step = Some(StepIdx(12))
    When: emit_reduce_body_steps emits body nodes
    Then: node[1].next == Some(StepIdx(12))  // last body step → ReduceNext

fn body_step_next_chain_single_step_links_to_next_step():
    Given: a reduce body [Set] and next_step = Some(StepIdx(12))
    When: emit_reduce_body_steps emits body node
    Then: node[0].next == Some(StepIdx(12))

fn body_step_next_chain_integrity_full_body():
    Given: a reduce body [Set, Set, Set] and next_step = Some(StepIdx(15))
    When: emit_reduce_body_steps emits body nodes
    Then: node[i].next == Some(node[i+1].id) for i in 0..2
    And: node[2].next == Some(next_step)
    And: no next field is None (no dangling chains)
```

---

### C5-C6: ReduceStart/ReduceNext/ReduceFinish References

```
fn reduce_start_and_reduce_next_both_point_to_body_step():
    Given: a reduce with body [Set, Set] lowered through lower_canonical_aggregate
    When: inspecting the emitted IR nodes
    Then: the ReduceStart node's body field == body_step
    And: the ReduceNext node's body field == body_step (same)
    And: ReduceStart.done == next_step + 1

fn reduce_finish_id_is_next_step_plus_one():
    Given: body_step=10, body_width(body, 0)=2 → next_step=12
    When: lower_canonical_aggregate emits ReduceFinish
    Then: ReduceFinish.id == StepIdx(13)  // next_step + 1

fn reduce_finish_next_is_parent_aggregate_next():
    Given: a reduce embedded in workflow with parent_next = Some(StepIdx(20))
    When: lower_canonical_aggregate(nxt=parent_next) compiles
    Then: ReduceFinish.next == Some(StepIdx(20))
```

---

### C7: Single-Step Body Compatibility (REGRESSION)

```
fn emit_reduce_body_steps_single_set_equivalent_to_emit_single_body_set():
    Given: a body with exactly 1 Set step
    When: emit_reduce_body_steps(body, id, diag, slot, next, builder_a) is called
    And: emit_single_body_set(body, id, diag, slot, next, builder_b, false) is called
    Then: both return Ok(())
    And: builder_a.node_count() == builder_b.node_count()
    And: builder_a.nodes[0].id == builder_b.nodes[0].id
    And: builder_a.nodes[0].next == builder_b.nodes[0].next
    And: builder_a.nodes[0].kind matches builder_b.nodes[0].kind

fn emit_reduce_body_steps_single_do_equivalent_to_emit_single_body_set():
    Given: a body with exactly 1 Do step
    When: both dispatchers are called with same parameters
    Then: both emit identical IR

fn emit_reduce_body_steps_single_foreach_equivalent_to_emit_single_body_set():
    Given: a body with exactly 1 ForEach step (body: [Set])
    When: both dispatchers are called with same parameters
    Then: both emit identical IR (ForEach → 4 nodes with same structure)

fn existing_po_r1_through_r8_tests_pass():
    Given: existing reduce unit and digest tests (tests.rs:524-730)
    When: tests are run after emit_reduce_body_steps implementation
    Then: all 8 reduce digest tests (PO-R1..PO-R8) continue to pass
    And: collect vs aggregate collision test (PO-R8) continues to pass
```

---

### C8: Nested Reduce Semantics

```
fn nested_reduce_in_body_step_0_gets_next_body_step_as_next():
    Given: a reduce body [Reduce { body: [Set] }, Set]
    When: emit_reduce_body_steps dispatches body step 0 (nested reduce)
    Then: lower_canonical_aggregate is called with next = Some(second_body_step_id)

fn nested_reduce_in_last_body_step_gets_next_step_as_next():
    Given: a reduce body [Set, Reduce { body: [Set] }]
    When: emit_reduce_body_steps dispatches body step 1 (nested reduce, last)
    Then: lower_canonical_aggregate is called with next = Some(next_step)

fn nested_reduce_produces_correct_three_node_substructure():
    Given: a reduce with body [Reduce { variable: "inner", input: "items", initial: "0", body: [Set { value: "1" }] }]
    When: the full workflow is lowered through compile_source
    Then: the IR contains ReduceStart(inner) + Set(body) + ReduceNext(inner) + ReduceFinish(inner) within the outer body
    And: inner reduce's node IDs do not collide with outer reduce's node IDs

fn nested_reduce_compiles_through_try_from_parts():
    Given: a YAML source with nested reduce
    When: compile_source(source) is called
    Then: result is Ok(CompiledWorkflow)
    And: CompiledWorkflow::try_from_parts passes
```

---

### C9: Symbolic Diagnostics

```
fn reduce_empty_body_returns_step_field_shape_with_field_steps():
    Given: a reduce body with 0 steps
    When: emit_reduce_body_steps(body_empty, id, idx, slot, next, builder) is called
    Then: returns Err(CompileErrors { .. })
    And: the error is CompileError::StepFieldShape { field: "steps", .. }
    And: error.code() returns Some(code) where code is registered in CODE_REGISTRY

fn reduce_non_set_body_step_returns_unsupported_step_primitive():
    Given: a reduce body step of type Finish
    When: emit_reduce_body_steps dispatches the body step
    Then: returns Err(CompileError::UnsupportedStepPrimitive { primitive: "finish", .. })
    And: error.code() returns Some(code) registered in CODE_REGISTRY

fn reduce_width_overflow_returns_step_index_out_of_range():
    Given: a reduce body with enough steps to overflow u16::MAX in width calculation
    When: body_width is computed or emit_reduce_body_steps runs
    Then: returns Err(CompileError::StepIndexOutOfRange { .. })
    And: error.code() returns Some(code) registered in CODE_REGISTRY
```

---

### C10: Deterministic Lowering

```
fn same_multi_step_reduce_source_produces_identical_ir():
    Given: a YAML source with multi-step reduce body
    When: compile_source(source) is called twice
    Then: both results are Ok
    And: workflow_a.nodes == workflow_b.nodes (structurally identical)
    And: workflow_a.digest == workflow_b.digest

fn same_multi_step_reduce_source_produces_identical_digest():
    Given: a reduce workflow with multi-step body
    When: compile_source(source) is called 3 times
    Then: digest_a == digest_b == digest_c
```

---

### C11: No Panic

```
fn emit_reduce_body_steps_never_panics():
    Given: valid and invalid body configurations (tested via proptest)
    When: emit_reduce_body_steps is called
    Then: always returns Result<(), CompileErrors> — never panics

fn lower_canonical_aggregate_never_panics():
    Given: valid reduce configurations
    When: lower_canonical_aggregate is called
    Then: always returns Result — never panics
```

---

### C12: Empty Body Handling

```
fn empty_body_rejected_before_any_nodes_emitted():
    Given: a reduce body with 0 steps
    When: emit_reduce_body_steps(body=[], body_step, idx, slot, next, builder) is called
    Then: returns Err immediately (no nodes emitted)
    And: builder.node_count() == 0 (no orphaned nodes)
    And: builder.slot_count() is unchanged

fn empty_body_does_not_produce_broken_chain():
    Given: a reduce with 0 body steps
    When: lowered through lower_canonical_aggregate
    Then: fails before ReduceNext is emitted (rejected in body dispatcher)
    And: no partially-constructed ReduceStart orphan exists
```

**Important design note**: The empty-body semantics may differ between the multi-step dispatcher and `emit_single_body_set`. The single-step dispatcher rejects `body.len() == 0` (it requires exactly 1). The multi-step dispatcher must also reject `body.len() == 0`, but via the explicit error path, not via the `len() != 1` guard. See domain-model open question 2 — need explicit domain decision on whether `body.len() == 0` is a compile-time error.

---

## 4. Proptest Invariants

### PO-001: Width-Node Parity (RRO-001, existing: `reduce_body_width_parity.rs`)
- **Invariant**: For any `Vec<StepAst>` body (1..6 steps, valid primitives: Set, Do, ForEach), `body_width(body, 3) == 3 + emit_reduce_body_steps(body, ...).node_count`.
- **Strategy**: Generate random valid reduce bodies. Compute width. Execute multi-step dispatcher. Assert count equality.
- **Anti-invariant**: Empty body produces predictable rejection without node emission.

### PO-002: Offset Monotonicity (RRO-006, existing: `reduce_body_offset_monotonic.rs`)
- **Invariant**: All StepIdx values emitted by `emit_reduce_body_steps` are distinct, strictly increasing, and within `[body_step, next_step)`.
- **Strategy**: Generate random body sequences. Execute dispatcher. Collect all emitted StepIdx. Verify monotonic + distinct + in-range.

### PO-003: Chain Integrity (RRO-015, existing: `reduce_body_chain_integrity.rs`)
- **Invariant**: For any valid body (1..6 steps), `step[i].next == Some(step[i+1].id)` for i < N-1, and `step[N-1].next == Some(next_step)`. No broken or dangling links.
- **Strategy**: Generate random bodies. Execute dispatcher. Walk the emitted chain. Assert correctness.

### PO-004: Single-Step Regression (RRO-022, existing: `reduce_single_step_regression.rs`)
- **Invariant**: For body.len() == 1, `emit_reduce_body_steps` and `emit_single_body_set` produce structurally identical IR (same node count, same IDs, same next links, same slot assignments).
- **Strategy**: Generate random single-step bodies (Set, Do, ForEach). Execute both dispatchers with same inputs. Assert structural equality.
- **Note**: BLOCKED until `emit_reduce_body_steps` is implemented. Tests using only `emit_single_body_set` + width verification can run now.

### PO-005: ForEach Width Advancement (RRO-012, existing: `reduce_nested_foreach_layout.rs`)
- **Invariant**: After emitting a ForEach body step with internal body, the accumulated offset advances by the full ForEach width (3+body_width), not by 1.
- **Strategy**: Generate bodies containing ForEach steps. Verify offset advancement matches `canonical_body_step_width(ForEach{..})`.

### PO-006: Nested Next Correctness (RRO-018, existing: `reduce_nested_next.rs`)
- **Invariant**: Nested Reduce at position i gets correct next: if i == last → `Some(next_step)`, else → `Some(next_body_step)`.
- **Strategy**: Generate bodies with nested Reduce at varying positions. Verify dispatch context's next parameter.

### PO-007: Width Overflow (RRO-009, existing: `reduce_body_width_overflow.rs`)
- **Invariant**: `body_width` returns `Err(StepIndexOutOfRange)` when cumulative width exceeds u16::MAX, and never panics.
- **Strategy**: Generate deeply nested bodies with large widths. Assert graceful overflow → error, no panic.

### PO-008: Empty Body (RRO-023, existing: `reduce_empty_body.rs`)
- **Invariant**: `body.len() == 0` always produces `Err(StepFieldShape { field: "steps", .. })` with zero nodes emitted.
- **Strategy**: Generate empty bodies with varying other parameters. Assert consistent rejection.

### PO-009: No Panic (RRO-025, existing: `reduce_lowering_no_panic.rs`)
- **Invariant**: Random StepAst trees never cause panics in `emit_reduce_body_steps` or `lower_canonical_aggregate`. All errors propagate via `Result`.
- **Strategy**: Generate random reduce body trees (including nested, deeply nested). Execute lowering. Assert no panic.

### PO-010: Diagnostic Code Validity (RRO-028, existing: `reduce_diagnostic_codes.rs`)
- **Invariant**: All CompileErrors produced by reduce lowering have valid registered SymbolicCode values.
- **Strategy**: Generate error-triggering configurations. Verify each error's `.code()` returns `Some(code)` and code is in expected set.

### PO-011: Digest Determinism (RRO-031, existing: `reduce_digest_determinism.rs`)
- **Invariant**: Same WorkflowSource produces identical canonical_digest across repeated compilations.
- **Strategy**: Generate random reduce workflows. Compile 3x each. Assert digest equality.

### PO-012: try_from_parts Success (RRO-004, existing: `reduce_multi_step_try_from_parts.rs`)
- **Invariant**: Multi-step reduce bodies compile through the full pipeline and pass `CompiledWorkflow::try_from_parts` validation.
- **Strategy**: Generate random valid multi-step reduce workflows. Execute full compile_source pipeline. Assert Ok.

### PO-013: Collision Safety (RRO-032, existing: `reduce_together_collision.rs`)
- **Invariant**: After vb-xi2f.22 (nested together) merge, both reduce and together multi-step bodies compile correctly. No regressions in either bead's test suite.
- **Strategy**: Cross-bead integration: generate both reduce and together workflows. Compile both. Assert no collisions.

---

## 5. Fuzz Targets

### FZ-001: Reduce Lowering Panic Fuzz (PO-NOPANIC-FUZZ-001)
- **Target**: `compile_source()` (full YAML→IR pipeline)
- **Input type**: arbitrary YAML bytes (up to 64 KiB)
- **Risk**: Panic on malformed reduce body steps, integer overflow in step offset computation, IndexOutOfBounds on mismatched width/lowering, infinite loop in nested body dispatch.
- **Corpus seeds**:
  - Valid reduce with 1 Set body step
  - Valid reduce with 2 body steps
  - Valid nested reduce (reduce in reduce body)
  - Reduce with empty body
  - Reduce with non-Set/Do/ForEach body
  - Reduce with deeply nested ForEach
  - Reduce with max u16 body steps
  - Truncated YAML (parse failure, should not panic)
- **Status**: BLOCKED_TOOLING — musl+sanitizer incompatibility. Compensating coverage: Kani (PO-NOPANIC-KANI-001) + proptest (PO-NOPANIC-PROP-001).
- **Fuzz target file**: `fuzz/fuzz_targets/reduce_lowering_panic.rs`

### FZ-002: Diagnostic Code Fuzz (PO-DIAGNOSTIC-FUZZ-001)
- **Target**: `compile_source()` (full pipeline)
- **Input type**: arbitrary YAML bytes
- **Risk**: Missing or unregistered SymbolicCode on error paths, None code on valid error, INTERNAL_INVARIANT sentinel rather than specific code.
- **Corpus seeds**:
  - Reduce with empty body (→ StepFieldShape code)
  - Reduce with unsupported primitive in body (→ UnsupportedStepPrimitive code)
  - Reduce with overflow width (→ StepIndexOutOfRange code)
- **Status**: BLOCKED_TOOLING. Kani+proptest provide compensating coverage.
- **Fuzz target file**: `fuzz/fuzz_targets/reduce_diagnostic_codes.rs`

---

## 6. Kani Harnesses

All 11 Kani harnesses are pre-existing in `crates/vb_compile/src/mod_compile_lowering/`. The test planner documents them for completeness; they are executed as part of the verification lane, not the behavior test lane.

| Harness | Property | Production Binding | Command |
|---------|----------|-------------------|---------|
| `check_reduce_body_width_parity` | Width-node parity bounded | body_width, emit_single_body_set | `cargo kani -p vb_compile --harness check_reduce_body_width_parity --unwind 16` |
| `check_reduce_body_offset_distinctness` | StepIdx distinctness | emit_single_body_set, checked_step_offset | `cargo kani -p vb_compile --harness check_reduce_body_offset_distinctness --unwind 16` |
| `check_reduce_body_chain_integrity` | Next-link chain integrity | emit_single_body_set, checked_step_offset | `cargo kani -p vb_compile --harness check_reduce_body_chain_integrity --unwind 16` |
| `check_reduce_nested_next_correctness` | Nested reduce next-field | emit_single_body_set, lower_canonical_aggregate | `cargo kani -p vb_compile --harness check_reduce_nested_next_correctness --unwind 16` |
| `check_reduce_empty_body_rejection` | Empty body rejection | lower_canonical_aggregate | `cargo kani -p vb_compile --harness check_reduce_empty_body_rejection` |
| `check_reduce_single_step_equivalence` | Regression equivalence | emit_single_body_set (*emit_reduce_body_steps blocked*) | `cargo kani -p vb_compile --harness check_reduce_single_step_equivalence --unwind 8` |
| `check_reduce_foreach_width_advance` | ForEach width advancement | canonical_body_step_width, emit_single_body_set | `cargo kani -p vb_compile --harness check_reduce_foreach_width_advance --unwind 16` |
| `check_reduce_lowering_no_panic` | Pipeline panic-freedom | lower_canonical_aggregate, canonical_body_step_width | `cargo kani -p vb_compile --harness check_reduce_lowering_no_panic --unwind 16` |
| `check_reduce_error_diagnostic_codes` | Symbolic code validity | CompileError::code | `cargo kani -p vb_compile --harness check_reduce_error_diagnostic_codes --unwind 16` |
| `check_reduce_multi_step_try_from_parts` | E2E try_from_parts | lower_canonical_aggregate, CompileWorkflow::try_from_parts | `cargo kani -p vb_compile --harness check_reduce_multi_step_try_from_parts --unwind 16` |
| `check_reduce_body_width_overflow` | Width overflow detection | body_width | `cargo kani -p vb_compile --harness check_reduce_body_width_overflow --unwind 32` |

**Important**: The `check_reduce_single_step_equivalence` harness has commented-out TODO blocks for comparing `emit_reduce_body_steps` vs `emit_single_body_set`. When `emit_reduce_body_steps` is implemented (as part of this bead), the harness must be uncommented and re-executed. Currently the harness validates the contract (body_width correctness) and the reference implementation (emit_single_body_set behavior).

---

## 7. Mutation Checkpoints

### Critical Mutations to Survive

| Mutation Target | Test That Must Catch It |
|----------------|------------------------|
| `canonical_body_step_width` — remove Reduce arm from match | `canonical_body_step_width_handles_reduce` (unit B07) + width-parity integration tests |
| `canonical_body_step_width` — return Ok(1) for Reduce instead of body_width | `canonical_body_step_width_returns_correct_for_nested_reduce` (unit B07) + offset monotonicity tests |
| `canonical_body_step_width` — return Ok(0) for Reduce (under-count) | width-node parity tests (integration B12-B16) |
| `emit_reduce_body_steps` — emit body step at offset 0 instead of 1 | first body step gets `body_step + 0` test (integration B17-B18) |
| `emit_reduce_body_steps` — skip step advancement (assign same ID twice) | offset monotonicity proptest (PO-002) |
| `emit_reduce_body_steps` — omit next-link assignment | chain integrity tests (integration B23-B28) |
| `emit_reduce_body_steps` — last step's next = None instead of Some(next_step) | `body_step_next_chain_last_links_to_next_step` (integration B24) |
| `emit_reduce_body_steps` — emit wrong primitive kind for Set/Do/ForEach dispatch | single-step regression equivalence tests (integration B35-B37) |
| `emit_reduce_body_steps` — accept empty body silently (return Ok with 0 nodes) | `empty_body_rejected_before_any_nodes_emitted` (unit B54-B56) |
| `lower_canonical_aggregate` — swap body_step for next_step in ReduceStart.body | `reduce_start_and_reduce_next_both_point_to_body_step` (integration B29-B30) |
| `lower_canonical_aggregate` — miscompute done_step (done = next_step instead of next_step + 1) | `reduce_finish_id_is_next_step_plus_one` (integration B32) |
| `body_width` — swap `checked_add` for `+` (allow overflow panic) | width overflow tests (unit B53) + proptest PO-007 |
| `canonical_body_step_width` — add Finish to accepted set (over-admit) | `canonical_body_step_width_rejects_finish_in_body` (unit B08) |
| `canonical_body_step_width` — remove one of Set/Do/ForEach from match (under-accept) | `canonical_body_step_width_returns_one_for_set_in_body` (unit B01-B02) |
| `checked_step_offset` — return Ok with wrong offset | offset monotonicity + chain integrity tests |
| `emit_reduce_body_steps` — swap last vs non-last next assignment logic | nested reduce dispatch tests (integration B39-B40) |
| `lower_canonical_aggregate` — omit reduce next computation (body_step = id instead of id+1) | `reduce_finish_id_is_next_step_plus_one` (integration B32) + width-node parity |

### Threshold: >=90% mutation kill rate
- Target: `cargo mutants --package vb_compile --files "src/mod_compile_lowering/part_01.rs" "src/mod_compile_lowering/part_04.rs"`

---

## 8. Combinatorial Coverage Matrix

### Unit: Width Calculation (`part_01.rs`)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| happy: Set width | Set { value: "1" } | Ok(1) | unit |
| happy: Do width | Do { action: "0", input: "0" } | Ok(1) | unit |
| happy: ForEach width (empty body) | ForEach { body: [] } | Ok(2) (overhead only) | unit |
| happy: ForEach width (1 Set) | ForEach { body: [Set] } | Ok(3) (overhead 2 + body_width 1) | unit |
| happy: Reduce width (1 Set body) | Reduce { body: [Set] } | Ok(body_width(body,3)) | unit |
| happy: Reduce width (2 Set body) | Reduce { body: [Set, Set] } | Ok(body_width(body,3)) | unit |
| happy: Reduce width (3 Set body) | Reduce { body: [Set, Set, Set] } | Ok(body_width(body,3)) | unit |
| happy: Reduce width (mixed Set+Do) | Reduce { body: [Set, Do] } | Ok(body_width(body,3)) | unit |
| happy: Reduce width (nested Reduce) | Reduce { body: [Reduce{..}] } | Ok(body_width(body,3)) | unit |
| happy: width = step_width equivalence | Reduce { .. } | canonical_step_width(Reduce) == body_width(body, 3) | unit |
| error: Finish in body | Finish { .. } | Err(UnsupportedStepPrimitive { primitive: "finish" }) | unit |
| error: Wait in body | Wait { .. } | Err(UnsupportedStepPrimitive { primitive: "wait" }) | unit |
| error: Ask in body | Ask { .. } | Err(UnsupportedStepPrimitive { primitive: "ask" }) | unit |
| error: Together in body | Together { .. } | Err(UnsupportedStepPrimitive { primitive: "together" }) | unit |
| error: Choose in body | Choose { .. } | Err(UnsupportedStepPrimitive { primitive: "choose" }) | unit |
| error: Collect in body | Collect { .. } | Err(UnsupportedStepPrimitive { primitive: "collect" }) | unit |
| error: Repeat in body | Repeat { .. } | Err(UnsupportedStepPrimitive { primitive: "repeat" }) or Ok(body_width) | unit |
| boundary: empty body (Reduce) | Reduce { body: [] } | Ok(3) (overhead only) | unit |
| boundary: max width (many nested Set) | deeply nested body | Ok(65535) or Err(StepIndexOutOfRange) for overflow | unit |
| boundary: overflow at u16::MAX+1 | sum of step widths > 65535 | Err(StepIndexOutOfRange { .. }) | unit |
| invariant: width parity | any valid body | body_width == 3 + node_count | proptest |

### Unit: Error Diagnostics (`mod_compile_errors`)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| happy: StepFieldShape code | body.len() == 0 | .code() is Some(registered) | unit |
| happy: UnsupportedStepPrimitive code | Finish in body | .code() is Some(registered) | unit |
| happy: StepIndexOutOfRange code | width overflow | .code() is Some(registered) | unit |
| error: no INTERNAL_INVARIANT | any valid error source | .code() != INTERNAL_INVARIANT sentinel | unit |

### Integration: Multi-Step Body Lowering

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| happy: N=1 Set body | body = [Set { value: "1" }] | Ok: 3+1=4 total nodes, chain correct | integration |
| happy: N=1 Do body | body = [Do { action: "0", input: "0" }] | Ok: 3+1=4 total nodes | integration |
| happy: N=1 ForEach body | body = [ForEach { body: [Set] }] | Ok: 3+3=6 total nodes (ForEach = 3 nodes overhead+body) | integration |
| happy: N=2 Set body | body = [Set, Set] | Ok: 3+2=5 total nodes, chain: 0→1→next | integration |
| happy: N=3 Set body | body = [Set, Set, Set] | Ok: 3+3=6 total nodes | integration |
| happy: mixed Set+Do+ForEach | body = [Set, Do, ForEach { body: [Set] }] | Ok: 3+1+1+3=8 total nodes, chain correct | integration |
| happy: nested Reduce (non-last) | body = [Reduce { body: [Set] }, Set] | Ok: chain: nested_start→nested_body→nested_next→nested_finish→outer_set→next | integration |
| happy: nested Reduce (last) | body = [Set, Reduce { body: [Set] }] | Ok: chain: outer_set→nested_start→nested_body→nested_next→nested_finish→next_step | integration |
| happy: ForEach with body in reduce body | ForEach with 2 Set body steps | ForEach width advancement correct (5 not 3) | integration |
| error: empty body | body = [] | Err(StepFieldShape { field: "steps", .. }) | integration |
| error: non-Set/Do/ForEach body step | body = [Finish { .. }] | Err(UnsupportedStepPrimitive { .. }) | integration |
| error: width overflow | deeply nested ForEach | Err(StepIndexOutOfRange { .. }) | integration |
| regression: single-step equivalence | body.len() == 1 | emit_reduce_body_steps == emit_single_body_set | integration |
| regression: PO-R1..R8 pass | existing reduce tests | all 8 digest tests pass | integration |
| invariant: sequential IDs | any valid body | IDs: monotonic, distinct, < next_step | integration + proptest |
| invariant: chain integrity | any valid body | step[i].next == Some(step[i+1].id), last→next_step | integration + proptest |
| invariant: no panic | any body tree | returns Result, never panics | integration + proptest |

### E2E: Full Pipeline

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| happy: nested reduce through try_from_parts | YAML → compile_source → try_from_parts | Ok(CompiledWorkflow) | e2e |
| happy: deterministic output | same YAML twice | identical IR + digest | e2e |
| error: invalid body through full pipeline | YAML with empty body | CompileErrors with valid code | e2e |

---

## 9. Test File Allocation

### New Behavior Test Files to Create

| File | Crate | Test Type | Behaviors Covered |
|------|-------|-----------|-------------------|
| `crates/vb_compile/src/mod_compile_lowering/tests.rs` (extend) | vb_compile | unit | B01-B11 (width), B17-B22 (offsets), B54-B56 (empty body) — add to existing reduce section |
| `crates/vb_compile/src/mod_compile_lowering/tests.rs` (extend) | vb_compile | integration | B23-B28 (chain), B29-B34 (node fields), B35-B43 (regression + nested) |
| `crates/vb_compile/tests/v1_primitive_lowering.rs` (extend) | vb_compile | integration/e2e | B12-B16 (width-node parity), B43 (try_from_parts), B48-B49 (determinism), multi-step reduce workflows |
| `crates/vb_compile/tests/digest_field_coverage.rs` (extend) | vb_compile | integration/proptest | B49 (digest determinism for multi-step reduce) |
| `crates/vb_compile/src/tests/error_variant_tests.rs` (extend) | vb_compile | unit | B44-B47 (diagnostic code validity for reduce-specific errors) |

### Existing Proptest/Verification Files (no changes to behavior tests)

| File | Status |
|------|--------|
| `crates/vb_compile/src/proptest_body_dispatcher.rs` | Existing — covers emit_single_body_set behavior. No changes needed. |
| `crates/vb_compile/src/proptest_error_parity.rs` | Existing — covers error parity for non-Set variants. No changes. |
| `crates/vb_compile/src/proptest_collect.rs` | Existing — covers collect behavior. No changes. |
| `crates/vb_compile/src/mod_compile_lowering/proptest_nested_foreach.rs` | Existing — ForEach proptest. No changes. |
| `verification/proptest/vb_compile/reduce_*.rs` | Verification artifacts (13 files) — planned, materialized at State 11-12. |

### Kani Harnesses (existing, verify-only)

| File | Status |
|------|--------|
| `crates/vb_compile/src/mod_compile_lowering/kani_reduce_regression.rs` | Existing — TODO blocks for `emit_reduce_body_steps`. Must uncomment when implemented. |
| `crates/vb_compile/src/mod_compile_lowering/kani_reduce_nopanic.rs` | Existing — bound to emit_single_body_set. Must extend to emit_reduce_body_steps after implementation. |
| `crates/vb_compile/src/mod_compile_lowering/kani_reduce_empty.rs` | Existing — uses emit_single_body_set as reference. Add multi-step path. |
| `crates/vb_compile/src/mod_compile_lowering/kani_reduce_diagnostics.rs` | Existing — covers error code validity. No changes needed. |
| Other 7 Kani harness files | Existing — verification artifacts. Re-run at State 11-12 post-implementation. |

---

## 10. Implementation Sequencing (for test-writer)

The implementation is NOT yet done — `emit_reduce_body_steps` does not exist. Tests should be written in phases:

### Phase 1: Tests that CAN pass NOW (pre-implementation)

1. **Width calculation unit tests** (B01-B11): `canonical_body_step_width` for Set, Do, ForEach already works. Tests for Reduce/Collect/Together/Repeat/Choose MUST FAIL initially (TDD red), then pass after `canonical_body_step_width` is widened.

2. **Error rejection tests** (B08-B10, B44-B47): Tests for Finish/Wait/Ask in body produce `UnsupportedStepPrimitive` — may already pass if match arm is covered, or may fail if the match is not yet widened.

3. **Digest determinism tests** (B48-B49): Can test with single-step reduce bodies (already supported).

### Phase 2: Tests that MUST FAIL initially, then PASS after implementation

4. **Width-node parity tests** (B12-B16): Require `emit_reduce_body_steps` to exist. Must FAIL initially with compile error. Will PASS after implementation.

5. **Sequential assignment tests** (B17-B22): Require `emit_reduce_body_steps`. FAIL initially.

6. **Chain integrity tests** (B23-B28): Require `emit_reduce_body_steps`. FAIL initially.

7. **Single-step regression equivalence** (B35-B38): Require both dispatchers to exist. The Kani harness has the comparison pattern. FAIL initially.

8. **Nested reduce tests** (B39-B43): Require full multi-step body dispatch, including recursive `lower_canonical_aggregate` calls. FAIL initially.

9. **Empty body rejection in multi-step** (B54-B56): Requires `emit_reduce_body_steps` to exist. May be identical behavior to `emit_single_body_set` (rejects len != 1), but the error message may differ. FAIL initially.

### Phase 3: Proptest (verification lane, State 11-12)

10. All 13 proptest invariants are verification artifacts executed separately from behavior tests.

---

## 11. Test Naming Convention

All test function names follow the pattern:
```
fn [subject]_[outcome]_when_[condition]()
```

Examples for this bead:
- `fn canonical_body_step_width_returns_one_for_set_in_body()`
- `fn emit_reduce_body_steps_assigns_sequential_distinct_step_indices()`
- `fn body_step_next_chain_last_links_to_next_step_parameter()`
- `fn nested_reduce_in_last_body_step_receives_next_step_as_next_parameter()`
- `fn empty_body_rejected_with_step_field_shape_before_any_nodes_emitted()`

---

## 12. Anti-Patterns to Reject

- ❌ `result.is_ok()` without asserting the node count or structural properties → **REJECT**
- ❌ `result.is_err()` without asserting the exact error variant and field → **REJECT**
- ❌ Test named `test_reduce_body()` instead of `reduce_body_compiles_n_steps_when_body_has_n_set_steps()` → **REJECT**
- ❌ Single test covering both width calculation and chain integrity (split by behavior) → **REJECT**
- ❌ Logic (loops, conditionals) inside test bodies to generate test cases → **REJECT** (use proptest strategies)
- ❌ Mocking the `SlotCompiler` or `compile_source()` — use real implementations → **REJECT**
- ❌ `sleep()` inside tests → **REJECT** (redundant — this is pure computation)
- ❌ Test that passes when `emit_reduce_body_steps` is deleted → **REJECT** (mutation survivor)

---

## Open Questions

1. **Q1: Empty body semantics.** The multi-step dispatcher contract says reject `body.len() == 0` with `StepFieldShape`. The single-step dispatcher rejects `body.len() != 1`. Should the multi-step dispatcher use the same error variant and message? *Recommendation: Use `StepFieldShape { field: "steps", expected: "at least one body step" }` for clarity — different expected message than single-step's "exactly one set step".*

2. **Q2: Primitive whitelist for reduce body.** Which primitives beyond Set, Do, ForEach should `emit_reduce_body_steps` support? The domain model mentions Reduce (nested), but Together/Collect/Repeat/Choose are undecided. *Recommendation: Start with what `canonical_body_step_width` supports (Set, Do, ForEach, Reduce). Add Together/Collect/Repeat/Choose incrementally via separate beads. Document the whitelist in a const array for testability.*

3. **Q3: Recursive reduce depth limit.** Should there be a maximum nesting depth for reduce-in-reduce to prevent compile-time stack overflow? *Recommendation: Rely on the existing `VbU16Max` bound (body_width cannot exceed 65535) as the implicit depth limiter. A 65535-node body would naturally limit depth.*

4. **Q4: Kani regression harness unblocking.** `kani_reduce_regression.rs` has commented-out TODO blocks that depend on `emit_reduce_body_steps` existing. Should this bead's test plan include tests that exercise the TODO comparisons? *Yes — the single-step equivalence behavior tests (B35-B38) directly verify the same contract. The Kani harness is verification-lane, not test-lane.*

5. **Q5: Canonical primitive name for reduce in error messages.** `canonical_primitive_name()` in `part_05.rs:107` maps Reduce to a name string. Is it `"reduce"` or `"aggregate"`? This affects expected values in `UnsupportedStepPrimitive { primitive: ... }` assertions. *Recommendation: Verify the current value via existing test assertions and use that exact string in new test assertions. The vb-xi2f.37 bead resolved this — check its resolution.*

6. **Q6: Test visibility into `emit_reduce_body_steps`.** The new function will likely be `pub(super)` like `emit_single_body_set`. For in-crate tests in `mod_compile_lowering/tests.rs`, this is fine. For external crate tests (e.g., `workspace_tests`), the function needs `pub` or `pub(crate)` visibility. *Recommendation: Make `emit_reduce_body_steps` `pub(crate)` to match existing pattern (`part_04::*` is `pub(crate)` use).*

---

## References

- **Proof-to-rust bridge**: `proof-to-rust-map.md` — 32 obligations mapped, 13 behavior test refs planned
- **RRO file**: `rust-refinement-obligations.jsonl` — 32 rows, all `mapping_status: planned`
- **Contract**: `.beads/vb-xi2f.24/contract.md` — 12 clauses (C1-C12), 8 acceptance gates (G1-G8)
- **Domain model**: `.beads/vb-xi2f.24/domain-model.md` — ubiquitous language, value objects, forbidden states
- **Codebase map**: `.beads/vb-xi2f.24/codebase-map.md` — pipeline overview, implementation strategy
- **Production code**:
  - `crates/vb_compile/src/mod_compile_lowering/part_01.rs` — width calculation
  - `crates/vb_compile/src/mod_compile_lowering/part_04.rs` — `lower_canonical_aggregate`, `emit_single_body_set`
  - `crates/vb_compile/src/mod_compile_lowering/part_12.rs` — `checked_step_offset`
  - `crates/vb_compile/src/mod_compile_lowering/part_14.rs` — `emit_choose_branch_body` (reference pattern)
  - `crates/vb_compile/src/mod_compile_lowering/tests.rs` — existing reduce tests (PO-R1..R8)
  - `crates/vb_compile/tests/v1_primitive_lowering.rs` — existing integration reduce tests
