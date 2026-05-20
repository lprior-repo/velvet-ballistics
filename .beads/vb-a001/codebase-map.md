# Codebase Map: vb-a001 "runtime: repair for_each compiled parity"

## Bug Summary

A `for_each` YAML workflow passes `validate`, `verify`, `simulate`, and `compile` but `run-compiled` rejects the emitted postcard IR artifact with "compiled IR validation error" (unreachable node or backward edge). Interpreted `run` works correctly. The gap is in the YAML-to-IR lowering phase.

## Data Flow Diagram

```
YAML (for_each.yaml)
  │
  ▼
vb_yaml::parse_workflow_source()        [Parse: YAML → AST]
  │
  ▼
vb_compile::compile_workflow()          [Compile: AST → CompiledWorkflow]
  │
  ├── vb_compile::mod_compile_lowering::part_02::lower_canonical_for_each()
  │     └── Emits 4 compiled nodes: ForEachStart, body SetConst, ForEachNext, done
  │     └── BUG FIX: body SetConst next edge → ForEachNext (step offset 2)
  │
  ▼
WorkflowParts                          [Intermediate: deserializable struct]
  │
  ▼
postcard::to_allocvec()                [Serialize: WorkflowParts → bytes]
  │
  ▼
compiled IR (.vbir postcard)           [Artifact on disk]
  │
  ├── vb_cli::run::cmd_run()           [Path A: YAML → compile → run]
  │     └── Same compile path + vb_runtime execution
  │
  └── vb_cli::run::cmd_run_compiled()  [Path B: IR artifact → deserialization → validation → run]
        ├── postcard::from_bytes::<WorkflowParts>()
        ├── CompiledWorkflow::try_from_parts()  ← VALIDATION GATE
        │     ├── validate_reachability()
        │     ├── validate_forward_edges()
        │     └── validate_node() for each node
        └── run_compiled_workflow()
              └── vb_runtime::engine::drive_deterministic_full()
                    └── vb_runtime::engine::execute::execute_node_full()
                          └── ForEachStart / ForEachNext / ForEachJoin dispatch
```

## Node Index Layout for for_each.yaml

```
YAML steps (before for_each):   [0] set_input, [1] for_each
YAML steps (after for_each):    [2] set_body,  [3] done,  [4] finish

Compiled nodes:
  Node 0: ForEachStart { input=0, item_slot=1, limit=2, body=1, done=3 }
  Node 1: SetConst { value=ConstX }    next=Some(2)     ← FIX: was missing/misrouted
  Node 2: ForEachNext { item_slot=1, body=1, done=3 }
  Node 3: SetConst (body step)          next=Some(4)
  Node 4: Finish { result=0 }
```

## Reachability Graph

```
Entry (0: ForEachStart)
  ├─→ body: 1 (SetConst)          [next edge from ForEachStart body ref]
  └─→ done: 3 (body step)         [done edge from ForEachStart]

Node 1 (SetConst, body)
  └─→ next: 2 (ForEachNext)       [forward edge, ci=1, target=2]

Node 2 (ForEachNext)
  ├─→ body: 1 (SetConst)          [loop back-edge, reaches already-visited node]
  └─→ done: 3 (body step)         [done edge, ci=2, target=3]

Node 3 (body step SetConst)
  └─→ next: 4 (Finish)

Node 4 (Finish)
  └─→ (terminal)
```

All nodes reachable. Loop back-edge (2→1) is valid because reachability uses BFS with visited set, not forward-edge check.

## Forward Edge Validation

| Edge | From Node | To Node | ci | Target | Valid |
|------|-----------|---------|----|--------|-------|
| next | SetConst(1) | ForEachNext(2) | 1 | 2 | YES (2 > 1) |
| done | ForEachStart(0) | body_step(3) | 0 | 3 | YES (3 > 0) |
| done | ForEachNext(2) | body_step(3) | 2 | 3 | YES (3 > 2) |
| next | body_step(3) | Finish(4) | 3 | 4 | YES (4 > 3) |

Note: `body` references (ForEachStart.body→1, ForEachNext.body→1) bypass `validate_forward_target` because `validate_kind_edges` calls `validate_loop_done_only` which only validates `done`. Body references are validated through `validate_reachability` instead.

## Key Files

### Lowering (FIX APPLIED)
- **`crates/vb_compile/src/mod_compile_lowering/part_02.rs`** — `lower_canonical_for_each()` (line 143-194)
  - Emits ForEachStart node, body SetConst via `emit_single_body_set`, ForEachNext node
  - FIX: `emit_single_body_set` receives `Some(next_step)` where `next_step = checked_step_offset(id, 2, "for_each", "next")` — this is the ForEachNext step index

### Validation
- **`crates/vb_core/src/workflow/mod.rs`** — `CompiledWorkflow::try_from_parts()` (line ~730)
  - Calls: `validate_entry`, `validate_node` (per node), `validate_reachability`, `validate_forward_edges`
  - `validate_reachability` (line 1353-1422): BFS from entry, marks all reachable nodes
  - `validate_forward_edges` (line 1529-1551): checks forward-only edges + loop nesting
  - `validate_kind_edges` (line 1555-1616): for ForEachStart/ForEachNext → `validate_loop_done_only` (only validates done edge)
  - `collect_node_targets` (line 1426-1492): ForEachStart/ForEachNext → body + done targets

### Runtime Execution
- **`crates/vb_runtime/src/primitives/for_each.rs`** — `for_each_start()`, `for_each_next()`, `for_each_join()`
  - `for_each_start`: checks input list, returns `Continue` with item or `Done` if empty
  - `for_each_next`: reads item, returns `Continue` or `Done`
  - `for_each_join`: marks iteration complete, outputs result
- **`crates/vb_runtime/src/engine/execute.rs`** — dispatch for `ForEachStart/Next/Join` (line ~56-90)
- **`crates/vb_runtime/src/engine/drive.rs`** — `drive_deterministic_full` loop
- **`crates/vb_runtime/src/engine/signal.rs`** — `runtime_from_core` signal conversion

### CLI
- **`crates/vb_cli/src/run.rs`** — `cmd_run()`, `cmd_run_compiled()`, `map_runtime_inputs()`
  - `cmd_run_compiled`: deserializes postcard → WorkflowParts → `CompiledWorkflow::try_from_parts()`
- **`crates/vb_cli/tests/ir_artifact_admission.rs`** — regression test `run_compiled_for_each_corpus_artifact_reaches_runtime_semantics`
- **`crates/vb_cli/src/cli_postcard.rs`** — Postcard header validation (INV-005, POST-007)

### Corpus
- **`fuzz/corpus/vb_f04l_yaml_compiler_compile/for_each.yaml`** — 7-step workflow with `for_each` primitive

## Risk Analysis

### LOW RISK
- **Minimal scope**: Fix is in lowering only, no validation or runtime changes
- **Existing tests**: All 11118 workspace tests pass with fix
- **No API changes**: No new public APIs, no signature changes

### MEDIUM RISK
- **Control-flow graph topology**: Fix changes reachability graph; must ensure loop back-edge is correct
- **Validation gate**: `CompiledWorkflow::try_from_parts` must accept valid for_each IR without weakening other checks
- **E2E parity**: `run` and `run-compiled` must agree for all for_each workflows

### NOT YET VERIFIED
- Valid list input envelope (for_each with actual list data executes to completion)
- Durable journal replay emits `RunFinished` for successful for_each run

## Test Matrix

| Test | Path | Input | Expected |
|------|------|-------|----------|
| run_compiled_for_each_corpus_artifact_reaches_runtime_semantics | YAML → compile → run + run-compiled | /dev/null | Same exit code, no IR validation error |
| run_compiled_accepts_valid_handcrafted_ir_artifact | run-compiled | I64(42) | exit 0, "run completed" |
| run_compiled_rejects_handcrafted_unreachable_node_ir | run-compiled | empty | exit failure, "not reachable" |
| run_compiled_rejects_handcrafted_backward_edge_ir | run-compiled | empty | exit failure, "backward edge" |
| for_each_start_returns_continue_when_list_has_items | runtime | list input | Continue |
| for_each_start_returns_done_when_list_is_empty | runtime | empty list | Done |
| for_each_iteration_drains_all_items_sequentially | runtime | 3-item list | 3 iterations, then Done |

## Verification Commands

```bash
# Unit / integration tests
cargo test -p vb_cli --test ir_artifact_admission
cargo test -p vb_runtime for_each
cargo test -p vb_compile lower

# Full CI
moon ci

# Manual for_each run
target/debug/velvet-ballastics run fuzz/corpus/vb_f04l_yaml_compiler_compile/for_each.yaml --input-bin /dev/null
target/debug/velvet-ballastics compile --emit postcard --out /tmp/for_each.vbir fuzz/corpus/vb_f04l_yaml_compiler_compile/for_each.yaml
target/debug/velvet-ballastics run-compiled /tmp/for_each.vbir --input-bin /dev/null

# Journal replay (when proven)
target/debug/velvet-ballastics run for_each.yaml --input-bin list_input.bin --durability full --db /tmp/vb-journal
target/debug/velvet-ballastics inspect /tmp/vb-journal
```
