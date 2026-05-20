# Contract Specification: vb-a001 — runtime: repair for_each compiled parity

## Context

- **Feature:** Repair for_each compiled parity: ensure the compiled IR lowering emits correct
  `next` edges on the body SetConst node so that compiled execution reaches `RunFinished` instead
  of rejecting with "compiled IR validation error: unreachable node" or "backward edge".
- **Domain terms:**
  - `lower_canonical_for_each` — lowering function in `part_02.rs` that emits ForEachStart, body
    SetConst, ForEachNext, and done SetConst nodes.
  - `CompiledWorkflow::try_from_parts` — validation gate that runs reachability BFS, forward-edge
    checks, and kind-edge checks on deserialized WorkflowParts.
  - `validate_forward_target` — rejects any edge `target → ci` where `target ≤ ci`.
  - `validate_loop_done_only` — only validates the `done` edge of loop primitives; `body` edges
    bypass forward-edge check (validated via reachability BFS instead).
  - `drive_deterministic_full` — runtime loop that dispatches CompiledNodeKind variants and
    advances PC.
  - `RunFinished` — journal event emitted when a workflow completes successfully.
  - `cmd_run_compiled` — CLI path that deserializes postcard IR → validates via
    `CompiledWorkflow::try_from_parts` → executes.
  - `cmd_run` — CLI path that compiles YAML on-the-fly and executes in the same process.
  - Compiled parity — `run` and `run-compiled` must produce identical observable outcomes (exit
    code, journal events, slot values).
- **Assumptions:**
  - A1: The fix in `lower_canonical_for_each` (line 178: `emit_single_body_set` receives
    `Some(next_step)` where `next_step = checked_step_offset(id, 2, "for_each", "next")`) is
    correct. The explore agent verified this: the SetConst body node now gets `next=Some(ForEachNext)`
    instead of `next=None` or a misrouted edge.
  - A2: All 11,118 existing workspace tests pass with the fix applied.
  - A3: No API surface changes — no new public types, no signature changes, no new crates.
  - A4: The `body` edges of loop primitives bypass `validate_forward_target` and are instead
    validated via reachability BFS (which tolerates back-edges from ForEachNext→body).
  - A5: `emit_single_body_set` is the only code path that emits body SetConst nodes for for_each;
    its `next` parameter directly becomes the SetConst node's `next` field.
- **Open questions:**
  - OQ1: Does the `cmd_run` path also exercise the same `lower_canonical_for_each` lowering
    function? (Yes — `cmd_run` calls `compile_workflow` → `lower_canonical_step` →
    `lower_canonical_for_each`, so the same fix applies to both paths.)
  - OQ2: Does the journal replay contract require that `run-compiled` with `--durability full`
    produces the same `RunFinished` journal event as `run`? (Implied by compiled parity; no
    explicit durability test exists yet.)

## Preconditions

- **PRE-001 (Lowering):** `lower_canonical_for_each` must emit exactly 4 compiled nodes in
  index order: ForEachStart(0), body SetConst(1), ForEachNext(2), done SetConst(3), where
  `body_step = checked_step_offset(id, 1)`, `next_step = checked_step_offset(id, 2)`,
  `done = checked_step_offset(id, 3)`.
- **PRE-002 (Lowering):** The body SetConst node (index 1) must have `next = Some(ForEachNext)`
  where ForEachNext is at index 2. This is the fix: previously `next` was `None` or misrouted.
- **PRE-003 (Lowering):** The ForEachStart node must have `body = body_step` (index 1) and
  `done = done` (index 3). The ForEachNext node must have the same `body` and `done`.
- **PRE-004 (Lowering):** The done SetConst node must have `next = Some(Finish)`.
- **PRE-005 (Validation):** `validate_forward_edges` must accept the emitted IR graph without
  error. Specifically:
  - SetConst(1) → ForEachNext(2): forward (2 > 1) ✓
  - ForEachStart(0) → done SetConst(3): loop done only, passes `validate_loop_done_only` ✓
  - ForEachNext(2) → done SetConst(3): loop done only, passes `validate_loop_done_only` ✓
  - Done SetConst(3) → Finish(4): forward (4 > 3) ✓
- **PRE-006 (Validation):** `validate_reachability` must reach every node via BFS from entry.
  The loop back-edge ForEachNext(2) → body SetConst(1) is valid because reachability uses a
  visited set and does not enforce forward-only on body edges.
- **PRE-007 (Runtime):** `for_each_start` must read input list, emit first item to `item_slot`,
  write tail to output slot, and jump to `body`. Empty list → jump to `done`.
- **PRE-008 (Runtime):** `for_each_next` must read iterator list, emit first item to output,
  write tail to iterator slot, and jump to `body`. Empty iterator → jump to `done`.

## Postconditions

- **POST-001 (Compiled parity — happy path):** For any valid for_each YAML workflow,
  `cmd_run` and `cmd_run_compiled` must both:
  - Exit with code 0.
  - Produce a `RunFinished` journal event with matching `run` ID.
  - Produce identical slot values at the workflow's output.
- **POST-002 (Compiled parity — empty list):** For a for_each workflow where the input list is
  empty, both `cmd_run` and `cmd_run_compiled` must emit a `RunFinished` event and skip all
  body iterations.
- **POST-003 (Validation rejection):** `cmd_run_compiled` must reject postcard IR artifacts that:
  - Contain unreachable nodes → `WorkflowError::UnreachableNode`.
  - Contain backward edges in `next` or `done` fields → `WorkflowError::BackwardEdge`.
  - Have improper loop nesting → `WorkflowError::ImproperLoopNesting`.
- **POST-004 (Artifact round-trip):** `compile --emit postcard` output, when deserialized via
  `postcard::from_bytes::<WorkflowParts>()` and passed to `CompiledWorkflow::try_from_parts()`,
  must succeed (no `Err`). This is the core compiled-parity invariant.

## Invariants

- **INV-001 (Lowering invariant):** `lower_canonical_for_each` always emits exactly 4 nodes in
  ascending index order with correct Kind types. No extra nodes, no missing nodes, no ordering
  violations.
- **INV-002 (Edge invariant):** Every node emitted by `lower_canonical_for_each` has at most one
  `next` edge, and every `next` edge points to a node with strictly higher index than the source.
  (Loop back-edges are only on `body` fields, not on `next`.)
- **INV-003 (Validation invariant):** `CompiledWorkflow::try_from_parts` never accepts an IR graph
  where any node is unreachable from the entry via BFS traversal of all edge types (next, on_error,
  kind targets).
- **INV-004 (Runtime invariant):** `drive_deterministic_full` terminates for any valid compiled
  workflow. Termination is guaranteed because: (a) for_each has a finite `limit`, (b) each
  ForEachNext drains one item, and (c) forward edges strictly advance or loop within bounded
  iteration count.
- **INV-005 (Parity invariant):** For all valid for_each inputs, `cmd_run` and `cmd_run_compiled`
  produce the same journal event sequence (same events, same ordering, same values).

## Error Taxonomy

- `WorkflowError::UnreachableNode { step }` — raised when reachability BFS cannot reach a node.
  The fix ensures body SetConst(1) IS reachable (ForEachStart.body→1, ForEachNext.body→1).
- `WorkflowError::BackwardEdge { from, to }` — raised when `target.as_usize() ≤ ci`. The fix
  ensures body SetConst.next = ForEachNext(2) where 2 > 1, so no backward edge.
- `WorkflowError::ImproperLoopNesting` — raised when inner loop's done ≤ outer loop's done.
  Not affected by this fix (single-level for_each).
- `WorkflowError::EmptyNodes` — raised when workflow has zero nodes. Not affected.
- `EngineError::IterationLimitExceeded` — runtime limit guard. Not affected by lowering fix.
- `EngineError::InternalInvariantViolation` — should never occur for valid IR. The fix ensures
  IR is valid so the guard's `reason: "for_each item_count checked nonzero"` is provably unreachable.

## Contract Signatures (relevant surfaces)

```rust
// Lowering (FIX APPLIED)
fn lower_canonical_for_each(
    index: usize,
    id: StepIdx,
    input: &str,
    at_once: Option<u32>,
    body: &[StepAst],
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors>
// Post: emits exactly 4 nodes, body SetConst.next = Some(ForEachNext)

// Validation (unchanged)
impl CompiledWorkflow {
    pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError>
    // Post: Ok iff reachability + forward_edges + kind_edges all pass
}

// Runtime (unchanged)
fn for_each_start(...) -> Result<EngineSignal, EngineError>
fn for_each_next(...) -> Result<EngineSignal, EngineError>
fn drive_deterministic_full(...) -> Result<RunResult, EngineError>
```

## Verus-Owned Clauses

- **INV-002 (Edge invariant):** The body SetConst's `next` edge from `lower_canonical_for_each`
  is provably `Some(ForEachNext_step)` where `ForEachNext_step > body_step`. This is a
  Rust-local pure property of the lowering function and can be expressed as a Verus postcondition
  on `lower_canonical_for_each`'s effect on `SlotCompiler.nodes`.
- **INV-004 (Runtime termination):** The termination of `drive_deterministic_full` for valid
  for_each workflows is a Rust-local state-machine property. It can be expressed as a Verus loop
  invariant on the drive loop with a decreasing measure (remaining items in iterator list +
  for_each limit counter).

## TLA+-Owned Clauses

- **INV-005 (Parity invariant):** The compiled-parity property is a temporal/state-over-time
  behavior claim. It requires a TLA+ model of the `run` vs `run-compiled` execution paths,
  showing that both paths produce the same journal event sequence. The TLA+ model abstracts
  the slot store and expression evaluation, focusing on node traversal order and journal events.
- **INV-004 (Termination):** The termination of the for_each loop is a temporal liveness property
  (`□ ◇ RunFinished`) that can be verified in TLA+ with a bounded iteration count.

## Theorem-Owned Clauses

- None. The Rust-local pure properties (INV-001, INV-002) are expressible in Verus. No tiny
  theorem kernel beyond Verus is needed.

## Non-goals

- NG-001: Performance benchmarking of compiled vs interpreted execution throughput (not claimed
  by this bead).
- NG-002: Parallel for_each execution (at_once > 1 is supported but not tested for correctness
  beyond the basic happy path).
- NG-003: Journal replay semantics beyond RunFinished emission (the bead scope is parity, not
  durability guarantees).
- NG-004: Collect, Reduce, Repeat — other loop primitives (only for_each is in scope).
