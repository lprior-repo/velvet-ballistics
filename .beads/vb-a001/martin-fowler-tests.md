# Martin Fowler Test Plan: vb-a001 — runtime: repair for_each compiled parity

## Happy Path Tests

### test_for_each_run_compiled_corpus_artifact_reaches_runtime_semantics
**Given:** the for_each.yaml corpus artifact (`fuzz/corpus/vb_f04l_yaml_compiler_compile/for_each.yaml`) with `/dev/null` input
**When:** both `run` and `run-compiled` are executed
**Then:**
- `run` exits 0 with "run completed" in stdout
- `run-compiled` exits 0 with "run completed" in stdout
- Both paths produce a `RunFinished` journal event
- Both paths produce identical journal event sequences

### test_for_each_run_compiled_with_list_input
**Given:** a for_each.yaml workflow with a non-empty list input (`--input-bin list_input.bin`)
**When:** both `run` and `run-compiled` are executed
**Then:**
- Both exit 0
- Body step executes exactly `N` times where `N = Len(input_list)`
- Both emit `RunFinished` with matching `run` ID
- Slot values at the workflow output match between paths

### test_for_each_run_compiled_with_single_item_list
**Given:** a for_each.yaml workflow with a single-item list input
**When:** both `run` and `run-compiled` are executed
**Then:**
- Body step executes exactly 1 time
- Both exit 0
- `RunFinished` emitted by both paths

### test_for_each_run_compiled_accepts_valid_handcrafted_ir
**Given:** a handcrafted valid for_each postcard IR artifact (4 nodes, correct edges)
**When:** `run-compiled` is executed with the artifact
**Then:**
- `CompiledWorkflow::try_from_parts` succeeds (no validation error)
- Execution completes with "run completed"
- `RunFinished` event emitted

## Error Path Tests

### test_run_compiled_rejects_unreachable_node_ir
**Given:** a handcrafted postcard IR with an unreachable node (node index 5 referenced but not reachable from entry 0)
**When:** `run-compiled` is executed
**Then:**
- `CompiledWorkflow::try_from_parts` returns `Err(WorkflowError::UnreachableNode { step: 5 })`
- Exit code is non-zero
- stderr contains "not reachable" or "unreachable node"

### test_run_compiled_rejects_backward_edge_ir
**Given:** a handcrafted postcard IR where a node's `next` edge points to a lower-indexed node (e.g., node 3 → node 1)
**When:** `run-compiled` is executed
**Then:**
- `CompiledWorkflow::try_from_parts` returns `Err(WorkflowError::BackwardEdge { from: 3, to: 1 })`
- Exit code is non-zero
- stderr contains "backward edge"

### test_run_compiled_rejects_improper_loop_nesting
**Given:** a handcrafted IR with nested for_each where inner done ≤ outer done
**When:** `run-compiled` is executed
**Then:**
- `CompiledWorkflow::try_from_parts` returns `Err(WorkflowError::ImproperLoopNesting { ... })`
- Exit code is non-zero

### test_run_compiled_rejects_malformed_postcard
**Given:** garbage bytes as postcard input
**When:** `run-compiled` is executed
**Then:**
- `postcard::from_bytes` fails with a deserialization error
- Exit code is non-zero

### test_for_each_start_returns_done_when_list_empty
**Given:** a for_each workflow where the input slot contains an empty list
**When:** `for_each_start` is called
**Then:**
- Returns `jump_to(done)` (not `Continue`)
- Output slot written with empty list
- Body step is NOT executed

### test_for_each_start_returns_continue_when_list_has_items
**Given:** a for_each workflow where the input slot contains a non-empty list
**When:** `for_each_start` is called
**Then:**
- Returns `Continue`
- Item slot contains first item
- Output slot contains remaining tail list
- Body step is executed next

### test_for_each_iteration_drains_all_items_sequentially
**Given:** a for_each workflow with a 3-item list input
**When:** the workflow runs (interpreted or compiled)
**Then:**
- Body step executes exactly 3 times
- Each iteration receives the correct item (item 1, then item 2, then item 3)
- After the 3rd iteration, `for_each_next` returns Done
- `RunFinished` event emitted

## Edge Case Tests

### test_for_each_with_empty_input_list
**Given:** for_each with input list = `[]`
**When:** both `run` and `run-compiled` execute
**Then:**
- Body step is NEVER executed
- `for_each_start` immediately jumps to done
- `RunFinished` emitted by both paths
- Output slot contains empty list

### test_for_each_at_once_limit_1
**Given:** for_each with `at_once: 1` (no parallelism)
**When:** both `run` and `run-compiled` execute
**Then:**
- Items are processed sequentially (one at a time)
- Results match `at_once: 2` (same items, same order)

### test_for_each_at_once_limit_2
**Given:** for_each with `at_once: 2`
**When:** both `run` and `run-compiled` execute
**Then:**
- Items are processed (order may differ from at_once: 1)
- Both paths produce the same set of items in the output

### test_for_each_minimum_node_count
**Given:** a for_each workflow that produces the minimum 4 nodes (ForEachStart + body SetConst + ForEachNext + done SetConst)
**When:** compiled and run
**Then:**
- `CompiledWorkflow::try_from_parts` accepts the IR
- Execution completes successfully

### test_for_each_max_steps_boundary
**Given:** a for_each workflow at the max step boundary (u16::MAX nodes)
**When:** compiled
**Then:**
- Compilation succeeds (no overflow in StepIdx arithmetic)
- Validation accepts the IR

### test_for_each_limit_exceeded
**Given:** a for_each with `at_once: 2` but input list has 5 items (limit = 2, items = 5)
**When:** run
**Then:**
- Runtime returns `EngineError::IterationLimitExceeded { resource: "for_each_limit" }`
- Exit code is non-zero
- Body step executes at most `limit` times

## Contract Verification Tests

### test_precondition_lowering_emits_exactly_4_nodes
**Given:** any valid for_each YAML with body steps
**When:** `lower_canonical_for_each` is called
**Then:**
- Exactly 4 nodes are appended to `builder.nodes`
- Node 0: `ForEachStart { input, item_slot, limit, body, done }`
- Node 1: `SetConst { ... }` with `next = Some(ForEachNext)`
- Node 2: `ForEachNext { iterator_slot, body, done }`
- Node 3: `SetConst { ... }` with `next = Some(Finish)`

### test_precondition_body_setconst_next_is_foreachnext
**Given:** the lowered nodes from `lower_canonical_for_each`
**When:** inspecting node 1 (body SetConst)
**Then:**
- `node.next = Some(StepIdx(2))` (ForEachNext index)
- `StepIdx(2) > StepIdx(1)` (forward edge invariant)

### test_postcondition_artifact_roundtrip_succeeds
**Given:** for_each.yaml compiled to postcard
**When:** postcard is deserialized → `CompiledWorkflow::try_from_parts`
**Then:**
- Returns `Ok(workflow)` (no error)
- `workflow.nodes.len() == 4`

### test_invariant_all_nodes_reachable
**Given:** the compiled IR from `lower_canonical_for_each`
**When:** `validate_reachability` runs BFS from entry
**Then:**
- All 4 nodes are marked visited
- No `UnreachableNode` error returned

### test_invariant_no_backward_edges
**Given:** the compiled IR from `lower_canonical_for_each`
**When:** `validate_forward_edges` runs
**Then:**
- All `next` edges satisfy `target > source`
- All `done` edges satisfy `target > source`
- No `BackwardEdge` error returned

### test_invariant_compiled_parity
**Given:** any valid for_each YAML and list input
**When:** `run` and `run-compiled` are both executed
**Then:**
- `run.journal == compiled.journal` (event-for-event equality)
- `run.exit_code == compiled.exit_code == 0`
- `run.output_slots == compiled.output_slots`

## Given-When-Then Scenarios

### Scenario 1: Basic compiled parity — happy path
**Given:** `for_each.yaml` corpus artifact with `/dev/null` input
**Given:** workflow has 1 for_each body step (SetConst output=seen, value=1)
**When:** `velvet-ballistics run for_each.yaml --input-bin /dev/null` executes
**When:** `velvet-ballistics compile --emit postcard --out /tmp/for_each.vbir for_each.yaml`
**When:** `velvet-ballistics run-compiled /tmp/for_each.vbir --input-bin /dev/null` executes
**Then:**
- Both commands exit 0
- Both produce "run completed" in stdout
- Both emit `RunFinished` journal event with same run ID
- Body step executes exactly 2 times (at_once=2)

### Scenario 2: Compiled parity — empty list
**Given:** a for_each.yaml with input slot pointing to an empty list
**When:** `run` executes
**When:** `run-compiled` executes
**Then:**
- Body step executes 0 times
- Both emit `RunFinished`
- Output slot contains empty list

### Scenario 3: Compiled parity — multi-item list
**Given:** a for_each.yaml with input slot pointing to `[I64(1), I64(2), I64(3)]`
**When:** `run` executes
**When:** `run-compiled` executes
**Then:**
- Body step executes 3 times
- Each iteration body receives the correct item
- Both emit `RunFinished` with identical journal events
- Output slot contains ordered results `[1, 2, 3]`

### Scenario 4: Validation gate — unreachable node rejection
**Given:** a manually crafted postcard IR with an isolated node (not reachable from entry)
**When:** `run-compiled` executes with the artifact
**Then:**
- `try_from_parts` returns `Err(UnreachableNode { step: <index> })`
- Exit code is non-zero
- Error message mentions "unreachable" or "not reachable"

### Scenario 5: Validation gate — backward edge rejection
**Given:** a manually crafted postcard IR with a `next` edge pointing backward (e.g., node 3 → node 1)
**When:** `run-compiled` executes with the artifact
**Then:**
- `try_from_parts` returns `Err(BackwardEdge { from: 3, to: 1 })`
- Exit code is non-zero
- Error message mentions "backward edge"

### Scenario 6: Durability — run with journal replay
**Given:** `for_each.yaml` with `--durability full --db /tmp/vb-journal`
**When:** `run` executes successfully
**When:** `inspect /tmp/vb-journal` is called
**Then:**
- Journal contains `RunAccepted` at seq=0
- Journal contains slot/step events for each body iteration
- Journal contains `RunFinished` at final seq
- Replay from journal reproduces `RunFinished` event
