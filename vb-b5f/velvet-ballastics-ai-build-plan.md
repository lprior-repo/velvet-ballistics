# Velvet Ballastics AI Build Plan

Status: implementation handoff  
Audience: AI coding agents, runtime implementers, performance engineers, validators, compiler/runtime builders  
Product binary spelling: `velvet-ballastics`  
Rust crate/module spelling: `velvet_ballastics`

## 0. Prime Directive

`velvet-ballastics` is a full end-to-end, single-server, ultra-low-latency workflow orchestrator. It is not a web workflow server. It is not a JSON interpreter. It is not a YAML interpreter. YAML is only the human/AI authoring format. The runtime executes precompiled numeric state machines from preallocated memory, uses Fjall for embedded persistence, and exposes direct in-process and binary IPC entry points.

The final product must include all of the following. These are not optional:

1. Strict YAML language parser and source-mapped diagnostics.
2. Full schema and semantic validator.
3. Expression parser, type checker, bytecode compiler, and evaluator.
4. Slot compiler that maps references to `SlotIdx` or `AccessorIdx`.
5. Compact compiled IR format.
6. In-memory deterministic execution engine.
7. Native Rust action registry and dispatch by `ActionId`.
8. Full implementation of `set`, `do`, `choose`, `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, `ask`, `try_again`, `on_error`, `then`, and `finish`.
9. Shard-owned scheduler with bounded queues.
10. Fjall database storage for workflow snapshots, compiled IR, journal events, run headers, run snapshots, blobs, and indexes.
11. Compact binary journal using Postcard.
12. Recovery/replay from Fjall.
13. Binary trace ring and counters.
14. Direct Rust API submission.
15. Binary IPC submission.
16. Generated Rust workflow mode.
17. CLI for validate, compile, run, run-compiled, inspect, events, replay, bench-run, doctor.
18. Full tests, fuzz targets, property tests, and benchmarks.
19. Max-performance nightly build profile, PGO workflow, and benchmark gates.
20. CI lint gate that rejects unsafe, unwrap, expect, panic, unchecked indexing, unchecked slicing, unchecked casts, unchecked arithmetic, ignored `Result`, and unbounded resource behavior.

## 1. Non-Negotiable Rust Rules

First-party code must satisfy these rules in production paths:

- No `unsafe`.
- No `.unwrap()`.
- No `.expect()`.
- No `panic!`.
- No `todo!`.
- No `unimplemented!`.
- No `dbg!`.
- No unchecked indexing.
- No unchecked slicing.
- No unchecked numeric casts.
- No unchecked size/capacity/offset arithmetic.
- No ignored `Result`.
- No unbounded queues.
- No unbounded loops.
- No unbounded retries.
- No unbounded fanout.
- No unbounded pagination.
- No unbounded task spawning.
- No YAML interpretation during execution.
- No JSON in the runtime core.
- No HTTP in the runtime core.
- No dynamic string lookup for references during execution.
- No `HashMap<String, Value>` runtime state.
- No task-per-step scheduler.
- No text formatting inside hot execution loops.

Permitted dependency rule:

- First-party code is zero-unsafe.
- Third-party crates may contain internal unsafe only if audited, pinned, and on the dependency allowlist.
- Runtime-facing dependencies must be justified by measurable performance, correctness, or implementation-risk reduction.

## 2. Holzmann Rules Adapted To Velvet Ballastics

1. **Simple control flow:** Engine transitions are explicit `StepIdx -> StepIdx` transitions. No hidden graph mutation.
2. **No unbounded loops:** `for_each`, `collect`, `repeat`, retries, queues, traces, snapshots, and action fanout all require limits.
3. **No dynamic allocation in hot paths when avoidable:** Preallocate run frames, slots, queues, trace rings, expression stacks, and journal buffers in turbo mode.
4. **Short functions:** Hot functions target less than 60 lines. Complex validation is decomposed by validation phase.
5. **Assertions/contracts:** Debug assertions may verify compiler invariants. Runtime user errors return typed errors.
6. **Small scopes:** Minimize mutable state. Shards own state; no global mutable run map.
7. **Checked parameters/returns:** All parse, compile, eval, store, dispatch, queue, and scheduler functions return typed `Result`.
8. **Restricted macros:** No macro-hidden business logic. Codegen is explicit and tested.
9. **Restricted pointer complexity:** No first-party unsafe pointer work. Use numeric IDs and checked table access.
10. **Zero warnings/static analysis:** Clippy hard denies, dependency audits, Miri on pure crates, fuzzing on parsers/decoders.

## 3. Performance Architecture

The runtime flow is:

```text
YAML bytes
  -> strict YAML parser
  -> source-mapped AST
  -> schema validator
  -> semantic validator
  -> expression bytecode compiler
  -> slot compiler
  -> CompiledWorkflow IR
  -> optional generated Rust workflow module
  -> RunFrame from pool
  -> deterministic state-machine loop
  -> native ActionId dispatch / wait / ask / retry suspension
  -> compact binary journal to Fjall
  -> binary trace/counters
```

The hot path must look like:

```text
RunId dequeued from shard
  -> frame.pc read
  -> compiled node table access
  -> execute deterministic primitive
  -> write SlotIdx output
  -> advance pc
  -> continue until suspend or finish
```

The hot path must not contain:

```text
YAML parse
JSON parse
HTTP request handling
string reference lookup
HashMap<String, Value>
serde_json::Value
allocation per step
Tokio task per step
format!/println!/JSONL output
fsync per deterministic step unless strict profile demands it
```

## 4. Language Specification Summary

Create `docs/language-spec.md` with the title:

```text
# Velvet Ballastics Workflow Language v1
```

Canonical version:

```yaml
version: velvet-ballastics/v1
```

Required top-level fields:

```yaml
version:
name:
when:
steps:
```

Optional top-level fields:

```yaml
inputs:
vars:
secrets:
result:
examples:
```

### YAML Profile

Allowed:

- strings
- finite numbers
- booleans
- null
- lists
- objects
- comments

Rejected:

- duplicate keys
- anchors
- aliases
- merge keys
- custom tags
- binary scalars
- YAML 1.1 ambiguous boolean strings such as `yes`, `no`, `on`, `off`
- unknown top-level fields
- unknown step fields
- multiple YAML documents unless explicitly enabled by a future version

### Triggers

Mandatory v1 trigger support:

```yaml
when:
  manual: {}
```

Mandatory IPC trigger support:

```yaml
when:
  ipc:
    name: issue_triage
```

HTTP/webhook is outside the runtime core and must not be implemented as the core ingress path.

### Step Primitives

Every step must have exactly one primitive:

```text
do, set, choose, for_each, together, collect, reduce, repeat, wait, ask, finish
```

Control/metadata fields:

```text
id, name, if, with, try_again, on_error, then
```

### IDs

Pattern:

```text
^[a-z][a-z0-9_]{0,63}$
```

Reserved:

```text
input, inputs, vars, secrets, steps, result, when, item, error,
true, false, null,
do, set, choose, for_each, together, collect, reduce, repeat,
wait, ask, try_again, on_error, then, finish
```

### References

Allowed roots:

```text
$input.x
$vars.x
$secrets.x
$step_id.x
$loop_name.x
$error.x
$attempt.x
$total.x
```

Compiler rule:

```text
All references are parsed, validated, type-checked, and compiled to numeric SlotIdx or AccessorIdx before execution.
```

Runtime rule:

```text
The runtime never resolves reference strings.
```

### Expressions

Operators:

```text
==, !=, >, >=, <, <=, and, or, not
```

Bounded arithmetic:

```text
+, -, *, /
```

Arithmetic rules:

- Operands must be finite numbers.
- Division by zero returns a typed runtime error.
- Non-finite results are rejected.
- `NaN`, `Infinity`, and `-Infinity` are invalid.
- Arithmetic is allowed in `set` and `reduce.set`.
- Arithmetic inside conditions must be governed by policy and tested.

Helpers:

```text
contains(value, needle)
starts_with(text, prefix)
ends_with(text, suffix)
has(object, key)
exists(path)
length(value)
empty(value)
append(list, value)
append_if(list, value, condition)
merge(object, object)
sum(list, field)
count(list)
unique(list)
```

Forbidden:

- JavaScript
- Python
- jq
- regex in v1
- network calls in expressions
- time/random functions
- user-defined functions
- loops inside expressions

### Validation Error Codes

Required error codes:

```text
DUPLICATE_KEY
FORBIDDEN_YAML_FEATURE
UNKNOWN_TOP_LEVEL_FIELD
UNKNOWN_STEP_FIELD
MISSING_REQUIRED_FIELD
INVALID_VERSION
INVALID_ID
RESERVED_ID
DUPLICATE_ID
MULTIPLE_STEP_PRIMITIVES
MISSING_STEP_PRIMITIVE
UNKNOWN_REFERENCE
FUTURE_REFERENCE
SECRET_NOT_DECLARED
DIRECT_RUNTIME_REFERENCE
INVALID_THEN_TARGET
CONTROL_FLOW_CYCLE
UNREACHABLE_STEP
INVALID_CHOOSE
INVALID_FOR_EACH
INVALID_TOGETHER
INVALID_COLLECT
INVALID_REDUCE
INVALID_REPEAT
INVALID_WAIT
INVALID_ASK
INVALID_FINISH
INVALID_RETRY
INVALID_ON_ERROR
SECRET_RESULT_LEAK
TYPE_MISMATCH
PAYLOAD_TOO_LARGE
LIMIT_REQUIRED
LIMIT_EXCEEDED
```

### Runtime Error Codes

Required runtime error codes:

```text
INPUT_MAPPING_FAILED
INPUT_TYPE_MISMATCH
SECRET_UNAVAILABLE
REFERENCE_MISSING
STEP_SKIPPED_REFERENCE
ACTION_FAILED
RETRY_EXHAUSTED
WAIT_TIMEOUT
ASK_TIMEOUT
FOR_EACH_ITEM_FAILED
TOGETHER_BRANCH_FAILED
COLLECT_LIMIT_REACHED
COLLECT_PAGE_FAILED
REDUCE_ITEM_FAILED
REPEAT_LIMIT_REACHED
RESULT_REFERENCE_MISSING
PAYLOAD_TOO_LARGE
QUEUE_FULL
IPC_FRAME_INVALID
IPC_PAYLOAD_TOO_LARGE
STORAGE_ERROR
REPLAY_DIVERGED
INTERNAL_INVARIANT_VIOLATION
```

## 5. Workspace Layout

```text
velvet-ballastics/
  Cargo.toml
  rust-toolchain.toml
  clippy.toml
  justfile
  deny.toml
  supply-chain/
    config.toml
  docs/
    language-spec.md
    ai-build-plan.md
    in-memory-runtime.md
    compiled-ir.md
    slot-value-model.md
    expression-engine.md
    yaml-compiler.md
    validation.md
    fjall-storage.md
    journal-format.md
    shard-scheduler.md
    action-abi.md
    binary-ipc.md
    generated-workflows.md
    binary-trace.md
    crash-recovery.md
    performance-contract.md
    benchmark-suite.md
  crates/
    vb_core/
    vb_yaml/
    vb_validate/
    vb_expr/
    vb_compile/
    vb_runtime/
    vb_storage/
    vb_ipc/
    vb_codegen/
    vb_cli/
  benches/
    parse_yaml.rs
    validate.rs
    compile_ir.rs
    expression.rs
    transitions.rs
    run_e2e.rs
    queues.rs
    storage_fjall.rs
    ipc.rs
    codegen.rs
  fuzz/
    fuzz_targets/
      yaml_events.rs
      expression.rs
      ipc_frame.rs
      journal_event.rs
      compiled_ir.rs
  tests/
    fixtures/
      valid/
      invalid/
      e2e/
```

## 6. Workspace Dependencies

Use current pinned versions after checking `cargo update -p` and dependency audit.

Required categories:

- `fjall`: embedded storage.
- `saphyr-parser`: strict YAML event parsing.
- `postcard`: compact binary encoding.
- `serde`: derive support for binary records only; not runtime JSON.
- `thiserror`: typed errors.
- `bytes`: payload/blob sharing.
- `arrayvec`: fixed expression stacks and bounded small buffers.
- `smallvec`: only when benchmarked.
- `compact_str`: compact text values.
- `crossbeam-queue`: bounded MPMC queues.
- `rtrb`: SPSC hot rings.
- `criterion`: local statistical benchmarks.
- `iai-callgrind`: CI-style instruction/cache benchmarks.
- `proptest`: property testing.
- `cargo-fuzz`: fuzzing.

## 7. Mandatory Crates And Responsibilities

### `vb_core`

Defines common IDs, values, compiled IR, run frame, deterministic engine, runtime limits, and core errors.

Files and functions:

```text
ids.rs
  WorkflowId(u32)
  WorkflowDigest([u8; 32])
  CompiledDigest([u8; 32])
  RunId(u64)
  StepIdx(u16)
  SlotIdx(u16)
  ExprIdx(u16)
  AccessorIdx(u16)
  ActionId(u16)
  BranchIdx(u16)
  AttemptIdx(u16)
  SeqNo(u64)
  as_usize() for each small index

value.rs
  FiniteNumber::new(f64) -> Result<FiniteNumber>
  SlotValue enum
  SymbolId
  TextRef
  BlobRef
  ListRef
  ObjectRef
  Taint enum
  SlotValue::type_name()
  SlotValue::is_truthy_strict() -> Result<bool>
  SlotValue::byte_len_estimate() -> usize

limits.rs
  RuntimeLimits
  check_yaml_size(bytes)
  check_steps(count)
  check_depth(depth)
  check_slots(count)
  check_output_size(bytes)
  check_result_size(bytes)
  check_loop_items(count)
  check_retry_attempts(count)
  check_queue_capacity(count)

compiled.rs
  CompiledWorkflow
  CompiledNode
  CompiledNodeKind
  CompiledSet
  CompiledDo
  CompiledChoose
  CompiledForEach
  CompiledTogether
  CompiledCollect
  CompiledReduce
  CompiledRepeat
  CompiledWait
  CompiledAsk
  CompiledFinish
  ExprProgram
  ExprOp
  AccessorProgram
  PathSegment
  ConstValue
  validate_compiled_invariants(&CompiledWorkflow) -> Result<()>

frame.rs
  RunFrame
  RunFramePool
  RunFrame::new
  RunFrame::reset
  RunFrame::pc
  RunFrame::set_pc
  RunFrame::read_slot
  RunFrame::write_slot
  RunFrame::copy_slot
  RunFrame::clear_slot
  RunFrame::read_taint
  RunFrame::write_taint
  RunFrame::mark_step_state
  RunFrame::step_state
  RunFramePool::with_capacity
  RunFramePool::checkout
  RunFramePool::release

engine.rs
  drive_until_suspend(workflow, frame, action_registry, journal, trace) -> Result<EngineSignal>
  execute_set(...)
  execute_choose(...)
  execute_finish(...)
  execute_for_each_control(...)
  execute_reduce_control(...)
  execute_repeat_control(...)
  execute_wait_control(...)
  execute_ask_control(...)
  prepare_do_action(...)
  apply_action_output(...)

errors.rs
  CoreError
  EngineError
  RuntimeErrorCode
  typed conversion helpers
```

### `vb_yaml`

Strict YAML parsing and source mapping.

Functions:

```text
parse_yaml_events(bytes, limits) -> Result<Vec<SpannedEvent>>
validate_yaml_profile(events) -> Result<()>
build_raw_ast(events) -> Result<RawYamlNode>
check_duplicate_keys(node) -> Result<()>
reject_forbidden_tags(events) -> Result<()>
reject_anchors_aliases(events) -> Result<()>
reject_merge_keys(node) -> Result<()>
source_for_node(node_id) -> SourceSpan
path_for_node(node_id) -> DiagnosticPath
parse_document(bytes, limits) -> Result<ParsedDocument>
```

Tests:

```text
minimal valid YAML
reject duplicate keys
reject anchors
reject aliases
reject merge key
reject custom tag
reject binary scalar
reject yes/no/on/off as booleans
reject multiple documents
source spans line/column correct
path mapping correct
size limit enforced
depth limit enforced
```

### `vb_validate`

Schema, semantic validation, references, control flow, type rules, diagnostics.

Functions:

```text
validate_document(parsed, registry, limits) -> Result<ValidatedWorkflow>
validate_top_level_fields(ast) -> Result<()>
validate_required_fields(ast) -> Result<()>
validate_version(ast) -> Result<()>
validate_name(ast) -> Result<()>
validate_trigger(ast) -> Result<TriggerSpec>
validate_inputs(ast) -> Result<InputSpecs>
validate_vars(ast) -> Result<VarSpecs>
validate_secrets(ast) -> Result<SecretSpecs>
validate_steps(ast) -> Result<StepSpecs>
validate_step_fields(step) -> Result<()>
validate_single_primitive(step) -> Result<PrimitiveKind>
validate_ids(steps) -> Result<IdTable>
validate_reserved_names(ids) -> Result<()>
validate_duplicate_ids(ids) -> Result<()>
validate_references(workflow) -> Result<ReferenceTable>
validate_no_future_references(workflow) -> Result<()>
validate_control_flow(workflow) -> Result<ControlFlowGraph>
validate_then_targets(workflow) -> Result<()>
validate_no_cycles(cfg) -> Result<()>
validate_reachability(cfg) -> Result<()>
validate_types(workflow, registry) -> Result<TypeTable>
validate_secret_taint(workflow) -> Result<TaintPlan>
validate_limits(workflow, limits) -> Result<()>
validate_action_contracts(workflow, registry) -> Result<()>
```

Tests:

```text
all validation error codes have tests
all diagnostics include code, path, source span, message
unknown top-level field
unknown step field
missing required field
invalid version
invalid ID
reserved ID
duplicate ID
multiple primitives
missing primitive
unknown reference
future reference
undeclared secret
invalid then target
cycle rejected
unreachable step rejected
choose no otherwise warning/error by policy
for_each missing at_once/per_second default policy
collect missing limits rejected
repeat missing limits rejected
retry times zero rejected
secret in result rejected
input default type mismatch
```

### `vb_expr`

Expression tokenizer, parser, AST, type checker, bytecode lowering, evaluator.

Functions:

```text
tokenize_expr(text, source_span) -> Result<Vec<Token>>
parse_expr(tokens) -> Result<ExprAst>
type_check_expr(ast, type_env) -> Result<ExprType>
compile_expr(ast, slot_env, const_pool) -> Result<ExprProgram>
eval_expr(program, frame, constants, accessors) -> Result<SlotValue>
eval_bool(program, frame, constants, accessors) -> Result<bool>
constant_fold(ast) -> ExprAst
compute_max_stack(program) -> u8
```

Tests:

```text
operator precedence
boolean logic
numeric comparisons
string comparisons
contains
starts_with
ends_with
has
exists
length
empty
arithmetic
division by zero returns error
non-finite rejected
type mismatch rejected
unknown helper rejected
function arity checked
malformed reference rejected
```

Property tests:

```text
bytecode max_stack is sufficient
constant folding preserves result
AST interpreter and bytecode evaluator agree for generated expressions
```

Fuzz:

```text
expression parser never panics
expression compiler never panics on arbitrary bytes
```

### `vb_compile`

Validated workflow to compiled IR.

Functions:

```text
compile_workflow(validated, action_registry, limits) -> Result<CompiledWorkflow>
allocate_slots(validated) -> Result<SlotLayout>
allocate_step_indices(validated) -> Result<StepIndexTable>
allocate_constants(validated) -> Result<ConstPool>
compile_inputs(validated, slots) -> Result<InputProgram>
compile_references(validated, slots) -> Result<ReferenceProgram>
compile_accessors(validated, slots) -> Result<AccessorTable>
compile_expressions(validated, slots, consts) -> Result<ExprTable>
lower_steps(validated, tables) -> Result<Box<[CompiledNode]>>
lower_set(step) -> Result<CompiledSet>
lower_do(step) -> Result<CompiledDo>
lower_choose(step) -> Result<CompiledChoose>
lower_for_each(step) -> Result<CompiledForEach>
lower_together(step) -> Result<CompiledTogether>
lower_collect(step) -> Result<CompiledCollect>
lower_reduce(step) -> Result<CompiledReduce>
lower_repeat(step) -> Result<CompiledRepeat>
lower_wait(step) -> Result<CompiledWait>
lower_ask(step) -> Result<CompiledAsk>
lower_finish(step) -> Result<CompiledFinish>
compute_workflow_digest(source) -> WorkflowDigest
compute_compiled_digest(compiled) -> CompiledDigest
```

Tests:

```text
slot layout deterministic
step layout deterministic
compiled digest deterministic
references compile to slots/accessors
constant pool deduplicates safely
minimal workflow compiles
set chain compiles
choose compiles
for_each compiles
together compiles
collect compiles
reduce compiles
repeat compiles
wait compiles
ask compiles
finish compiles
```

### `vb_storage`

Fjall storage, compact journal, snapshots, blob store, indexes, recovery.

Functions:

```text
FjallStore::open(path, config) -> Result<Self>
FjallStore::create_keyspaces() -> Result<()>
FjallStore::put_workflow_source(digest, bytes) -> Result<()>
FjallStore::get_workflow_source(digest) -> Result<Option<Bytes>>
FjallStore::put_compiled_ir(digest, compiled) -> Result<()>
FjallStore::get_compiled_ir(digest) -> Result<Option<CompiledWorkflow>>
FjallStore::put_run_header(header) -> Result<()>
FjallStore::get_run_header(run_id) -> Result<Option<RunHeader>>
FjallStore::append_journal_event(run_id, seq, event, persist_policy) -> Result<()>
FjallStore::read_journal_events(run_id, from_seq) -> Result<Vec<JournalEvent>>
FjallStore::write_snapshot(run_id, seq, snapshot, persist_policy) -> Result<()>
FjallStore::read_latest_snapshot(run_id) -> Result<Option<RunSnapshot>>
FjallStore::put_blob(digest, bytes) -> Result<()>
FjallStore::get_blob(digest) -> Result<Option<Bytes>>
FjallStore::index_run_status(run_id, status, time) -> Result<()>
FjallStore::recover_run(run_id) -> Result<RecoveredRun>
FjallStore::recover_all_active_runs() -> Result<Vec<RecoveredRun>>
```

Keyspaces:

```text
workflow_source
compiled_ir
run_header
run_event
run_snapshot
blob
index_status
index_workflow
```

Key encoding functions:

```text
key_workflow_source(digest) -> ArrayVec<u8, 33>
key_compiled_ir(digest) -> ArrayVec<u8, 33>
key_run_header(run_id) -> ArrayVec<u8, 9>
key_run_event(run_id, seq) -> ArrayVec<u8, 17>
key_run_snapshot(run_id, seq) -> ArrayVec<u8, 17>
key_blob(digest) -> ArrayVec<u8, 33>
key_index_status(status, timestamp, run_id) -> ArrayVec<u8, 18>
```

Tests:

```text
key order sorts by run_id then seq
journal append/read roundtrip
snapshot write/read roundtrip
workflow source write/read
compiled IR write/read
blob write/read
recover from events only
recover from snapshot plus tail events
corrupt event returns typed error
missing compiled IR returns typed error
strict persist mode can be invoked
journaled mode batches writes
```

Benchmarks:

```text
postcard_encode_journal_event
postcard_decode_journal_event
fjall_put_event_no_persist
fjall_put_event_group_commit
fjall_put_event_strict
fjall_read_1000_events
fjall_write_snapshot
fjall_read_snapshot
```

### `vb_runtime`

Runtime, shards, queues, actions, scheduler, trace, retry, wait, ask, loops.

Functions:

```text
Runtime::open(config, store, registry) -> Result<Self>
Runtime::load_workflow(source_bytes) -> Result<WorkflowHandle>
Runtime::compile_workflow(source_bytes) -> Result<CompiledHandle>
Runtime::register_compiled_workflow(compiled) -> Result<WorkflowId>
Runtime::submit_direct(workflow_id, input_frame) -> Result<RunId>
Runtime::submit_and_wait(workflow_id, input_frame, deadline) -> Result<RunResult>
Runtime::cancel(run_id) -> Result<()>
Runtime::inspect(run_id) -> Result<RunInspection>
Runtime::recover() -> Result<RecoveryReport>
Runtime::shutdown() -> Result<()>

Shard::new(id, config) -> Result<Shard>
Shard::run_loop() -> Result<()>
Shard::drain_commands() -> Result<Progress>
Shard::drive_ready_runs() -> Result<Progress>
Shard::drive_one_run(run_id) -> Result<EngineSignal>
Shard::resume_action(run_id, step, output) -> Result<()>
Shard::process_timers(now) -> Result<Progress>
Shard::process_waits(now) -> Result<Progress>
Shard::process_asks(now) -> Result<Progress>
Shard::write_trace(event) -> Result<()>

ActionRegistry::new() -> Self
ActionRegistry::register_builtin(action_id, contract, handler) -> Result<()>
ActionRegistry::resolve_name(name) -> Result<ActionId>
ActionRegistry::contract(action_id) -> Result<ActionContract>
ActionRegistry::dispatch_sync(action_id, frame, ctx) -> Result<ActionOutcome>
ActionRegistry::dispatch_async(action_id, input, ctx) -> Result<ActionTicket>
```

Scheduler tests:

```text
submit returns run ID
queue full returns QUEUE_FULL
run assigned to deterministic shard
set-only workflow completes synchronously
workflow suspends at do action
resume action completes workflow
wait suspends and resumes
ask suspends and resumes
cancel terminal transition
shutdown drains or rejects according to policy
```

### `vb_ipc`

Binary IPC; no HTTP, no JSON.

Functions:

```text
encode_frame(command, buffer) -> Result<usize>
decode_frame(bytes) -> Result<IpcFrame>
validate_frame_header(header) -> Result<()>
read_frame(stream, buffer, limits) -> Result<IpcFrame>
write_frame(stream, frame) -> Result<()>
IpcServer::bind(path, runtime) -> Result<Self>
IpcServer::run() -> Result<()>
IpcClient::connect(path) -> Result<Self>
IpcClient::submit_run(workflow_id, input_ref) -> Result<RunId>
IpcClient::cancel(run_id) -> Result<()>
IpcClient::inspect(run_id) -> Result<RunInspection>
```

Tests:

```text
valid frame roundtrip
bad magic rejected
bad version rejected
oversized payload rejected
short frame rejected
partial reads handled
unknown command rejected
submit via IPC completes run
IPC queue full returns typed error
```

Benchmarks:

```text
ipc_frame_encode
ipc_frame_decode
ipc_submit_to_finish
```

### `vb_codegen`

Generated Rust workflow mode.

Functions:

```text
emit_workflow_module(compiled, names, writer) -> Result<()>
emit_constants(compiled, writer) -> Result<()>
emit_drive_function(compiled, writer) -> Result<()>
emit_step_function(node, writer) -> Result<()>
emit_expr_function(expr, writer) -> Result<()>
emit_accessor_function(accessor, writer) -> Result<()>
emit_action_call(node, writer) -> Result<()>
format_generated_code(path) -> Result<()>
compile_generated_fixture(path) -> Result<()>
```

Tests:

```text
generated code compiles
generated set workflow matches IR mode
generated choose workflow matches IR mode
generated expression workflow matches IR mode
generated do workflow matches IR mode with fake actions
generated code contains no unsafe/unwrap/expect/panic tokens
generated code has stable output for same input
```

Benchmark:

```text
ir_vs_generated_rust_1_step
ir_vs_generated_rust_1000_steps
```

### `vb_cli`

CLI binary. Human diagnostics are allowed; runtime core remains binary/in-memory.

Commands:

```text
velvet-ballastics validate <workflow.yaml>
velvet-ballastics compile <workflow.yaml> --emit ir --out <file.vbir>
velvet-ballastics compile <workflow.yaml> --emit rust --out <file.rs>
velvet-ballastics run <workflow.yaml> --input-bin <input.vbin> --durability <mode>
velvet-ballastics run-compiled <workflow.vbir> --input-bin <input.vbin> --durability <mode>
velvet-ballastics ipc-serve --socket <path> --db <path>
velvet-ballastics inspect <run_id> --db <path>
velvet-ballastics events <run_id> --db <path>
velvet-ballastics replay <run_id> --db <path>
velvet-ballastics bench-run <workflow.yaml>
velvet-ballastics doctor --db <path>
```

CLI output can be text and binary. No JSON contract in v1.

## 8. Build Order

### Phase 0: Repository, toolchain, lints, CI skeleton

Deliver:

- Workspace layout.
- `rust-toolchain.toml`.
- Workspace lints.
- `deny.toml`.
- `justfile`.
- Empty crates.
- CI script.

Acceptance:

```bash
cargo +nightly fmt --all -- --check
cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings
cargo +nightly test --workspace --all-features
```

### Phase 1: Core IDs, errors, limits, values

Deliver:

- `vb_core::ids`
- `vb_core::errors`
- `vb_core::limits`
- `vb_core::value`

Tests:

- finite number accepts finite values
- finite number rejects NaN/Infinity
- index conversions are safe
- limit checks return typed errors
- `SlotValue::type_name` correct
- taint enum roundtrips through postcard

### Phase 2: Strict YAML event parser

Deliver:

- Saphyr event parser wrapper.
- Strict profile validation.
- Source maps.
- Raw AST.

Tests:

- valid minimal manual workflow
- valid IPC workflow
- duplicate keys rejected
- anchors rejected
- aliases rejected
- merge rejected
- custom tags rejected
- binary scalars rejected
- ambiguous boolean strings rejected where profile requires
- source spans available
- YAML size limit enforced
- nesting limit enforced

Fuzz:

- arbitrary YAML bytes never panic parser wrapper

### Phase 3: Language AST model

Deliver:

- Typed AST structs for top-level workflow.
- Step AST with primitive enum.
- Input/vars/secrets AST.
- Expression string holders with source spans.

Tests:

- minimal AST constructed
- all primitives parsed
- unknown fields preserved only as diagnostics, not silently ignored

### Phase 4: Schema validator

Deliver:

- Top-level validation.
- Step field validation.
- Primitive count validation.
- ID validation.
- Trigger validation.
- Inputs/vars/secrets validation.

Tests:

- every validation code in schema category
- all diagnostics include source span and path

### Phase 5: Reference and control-flow validator

Deliver:

- ID table.
- Reference table.
- Future reference rejection.
- CFG builder.
- Forward-only `then` validation.
- Cycle and reachability checks.

Tests:

- unknown reference
- future reference
- duplicate ID
- invalid then target
- backward then rejected
- jump into nested scope rejected
- jump out of nested scope rejected
- unreachable step rejected
- terminal step reachability checked

### Phase 6: Type validator and taint validator

Deliver:

- Deep type model.
- Input schema validation.
- Action contract schema validation.
- Expression type environment.
- Secret taint rules.

Tests:

- scalar type mismatch
- list without `of` rejected unless `list<any>`
- object extra allow/reject
- optional field
- default type mismatch
- null only where allowed
- secret output leak rejected

### Phase 7: Expression engine

Deliver:

- Lexer.
- Parser.
- AST.
- Type checker.
- Bytecode compiler.
- Evaluator.
- Fixed-stack path if possible.

Tests:

- operators
- helpers
- arithmetic
- precedence
- error paths
- non-finite numbers
- division by zero
- stack underflow impossible for compiled programs

Bench:

- simple equality
- numeric compare
- boolean chain
- arithmetic reducer expression

### Phase 8: Compiled IR and slot compiler

Deliver:

- Slot layout.
- Accessor layout.
- Constant pool.
- Expression table.
- Node lowering.
- Compiled digest.

Tests:

- deterministic slot layout
- deterministic compiled digest
- direct references become slots
- nested references become accessors only where not flattened
- all primitive lowerers covered

Bench:

- compile 10 steps
- compile 1000 steps

### Phase 9: Deterministic in-memory engine MVP

Deliver:

- RunFrame.
- Engine loop.
- `set`.
- `choose`.
- `finish`.
- In-memory input frame builder.

Tests:

- one-step set/finish
- 1000 set chain
- choose first branch wins
- choose otherwise
- no match returns runtime error
- missing slot returns typed error

Bench:

- transition set
- transition choose
- run 1 step
- run 1000 steps

### Phase 10: Fjall storage foundation

Deliver:

- Fjall open/init.
- Keyspaces.
- Key encoders.
- Workflow source storage.
- Compiled IR storage.
- Journal event encoding.
- Run header storage.
- Snapshot storage.
- Blob storage.

Tests:

- key order
- put/get workflow
- put/get compiled IR
- append/read events
- snapshot roundtrip
- blob roundtrip
- corrupted postcard typed error

Bench:

- encode event
- append event without strict persist
- append event strict persist
- read event stream

### Phase 11: Runtime durability profiles

Deliver:

- `volatile`.
- `snapshot`.
- `journaled`.
- `strict`.
- Policy mapping to Fjall writes.

Tests:

- volatile completes run without writes
- journaled records events
- strict persists accepted before returning
- snapshot writes at configured interval
- crash simulator can recover from journal/snapshot

### Phase 12: Native action registry

Deliver:

- `ActionId` name resolver at compile time.
- Action contracts.
- Sync native action dispatcher.
- Fake built-ins: `memory.echo`, `memory.add`, `memory.fail`, `memory.sleep_tick`.
- `do` step execution and resume path.

Tests:

- action contract validation
- known action dispatch
- unknown action rejected at validation
- action failure triggers typed runtime error
- action output stored in slots
- action retry bounded
- idempotency policy enforced

Bench:

- sync action dispatch
- do-step suspend/resume

### Phase 13: Shard scheduler and queues

Deliver:

- Runtime object.
- Shard object.
- Bounded submit queues.
- Run frame pool.
- Run routing.
- Drive loop.
- Cancellation.
- Shutdown.

Tests:

- submit returns run ID
- queue full returns `QUEUE_FULL`
- run routes to deterministic shard
- run completes through shard
- cancellation terminal state
- no run leaves terminal state
- shutdown behavior deterministic

Bench:

- submit-to-start
- submit-to-finish
- queue latency
- shard throughput

### Phase 14: Full primitive implementation

Deliver all primitives:

- `for_each`
- `together`
- `collect`
- `reduce`
- `repeat`
- `wait`
- `ask`
- `try_again`
- `on_error`
- `then`
- `finish`

Tests:

- for_each empty list
- for_each output order preserved
- for_each at_once respected
- for_each per_second respected by virtual clock
- for_each fail-fast
- together fast failure
- together after_all failure
- together collect mode
- collect page limit
- collect item limit
- collect time limit
- collect cursor progression
- reduce accumulator result
- reduce partial accumulator on error
- repeat until success
- repeat limit reached
- wait duration
- wait until timestamp
- wait event with timeout
- ask pending
- ask answer validated
- ask timeout
- try_again attempts counted correctly
- on_error then branch
- on_error set replacement output
- handler failure records original and handler error

Bench:

- for_each no-op 10k
- together no-op 100 branches
- reduce numeric 10k
- repeat 100 attempts virtual

### Phase 15: Binary trace and counters

Deliver:

- Atomic counters.
- Binary trace event.
- Bounded trace ring.
- Trace modes: off, counters, binary.
- Trace dump decoder CLI.

Tests:

- trace off does not write events
- trace ring bounded
- full ring policy enforced
- decoder roundtrip

Bench:

- trace off vs counters vs binary ring

### Phase 16: Binary IPC

Deliver:

- Frame protocol.
- Unix socket server.
- Unix socket client.
- Submit/cancel/inspect.
- Payload limits.

Tests:

- frame roundtrip
- bad magic
- bad version
- oversized payload
- partial frame
- submit via IPC completes run

Bench:

- encode/decode frame
- IPC submit-to-finish

### Phase 17: CLI end-to-end

Deliver commands:

- `validate`
- `compile --emit ir`
- `compile --emit rust`
- `run`
- `run-compiled`
- `ipc-serve`
- `inspect`
- `events`
- `replay`
- `doctor`
- `bench-run`

Tests:

- CLI validate valid/invalid
- compile IR writes file
- run workflow completes
- run-compiled completes
- inspect returns run state
- events prints event list
- replay works after restart
- doctor catches missing db

### Phase 18: Generated Rust mode

Deliver:

- Rust emitter.
- Generated workflow crate/module.
- Generated expression functions.
- Generated step functions.
- Build integration.
- Equivalence tests versus IR mode.

Tests:

- generated code compiles
- generated code passes clippy hard denies
- generated set/choose/do/loops match IR mode
- generated code token scan rejects unsafe/unwrap/expect/panic

Bench:

- IR vs generated, 1 step
- IR vs generated, 1000 steps
- IR vs generated, expression-heavy

### Phase 19: Recovery and replay

Deliver:

- Recovery from Fjall.
- Run journal replay.
- Snapshot plus tail replay.
- Workflow snapshot binding.
- Action side-effect replay policy.
- Replay command.

Tests:

- recover accepted queued run
- recover running deterministic run
- recover waiting run
- recover asking run
- recover after action started before completion
- completed steps do not rerun
- side-effecting action not silently rerun
- replay divergence detected

### Phase 20: Performance hardening

Deliver:

- `maxperf` profile.
- PGO script.
- `target-cpu=native` build script.
- Full benchmark suite.
- Regression thresholds.
- Flamegraph/samply workflow notes.

Tests:

- benchmarks compile
- benchmark fixtures valid

Acceptance:

- report p50/p95/p99 for core engine transitions
- report allocations per run
- report queue p99
- report Fjall append/read p99
- report IPC submit-to-finish p99
- report generated vs IR

## 9. End-to-End Acceptance Tests

Create fixtures:

```text
tests/fixtures/e2e/minimal_set.yaml
tests/fixtures/e2e/choose.yaml
tests/fixtures/e2e/do_memory_echo.yaml
tests/fixtures/e2e/for_each.yaml
tests/fixtures/e2e/together.yaml
tests/fixtures/e2e/collect.yaml
tests/fixtures/e2e/reduce.yaml
tests/fixtures/e2e/repeat.yaml
tests/fixtures/e2e/wait.yaml
tests/fixtures/e2e/ask.yaml
tests/fixtures/e2e/retry.yaml
tests/fixtures/e2e/on_error.yaml
tests/fixtures/e2e/full_workflow.yaml
```

Required E2E tests:

```text
parse -> validate -> compile -> store compiled IR in Fjall
parse -> validate -> compile -> run volatile -> succeeds
parse -> validate -> compile -> run journaled -> events exist in Fjall
compiled IR file -> run-compiled -> succeeds
IPC submit -> run completes
restart runtime -> recover waiting run
restart runtime -> recover journaled run
replay run -> deterministic result matches
IR mode result == generated Rust result
```

## 10. Mandatory Benchmarks

```text
parse_yaml_small
parse_yaml_1mb
validate_minimal
validate_1000_steps
compile_ir_minimal
compile_ir_1000_steps
expr_eq_symbol
expr_number_compare
expr_boolean_chain
expr_arithmetic
slot_read
slot_write
slot_copy
transition_set
transition_choose_2
transition_choose_100
transition_finish
run_noop_1
run_noop_10
run_noop_1000
run_set_chain_1000
run_choose_heavy
for_each_noop_10000
together_noop_100
reduce_numeric_10000
postcard_encode_event
postcard_decode_event
fjall_append_event_no_persist
fjall_append_event_strict
fjall_read_1000_events
arrayqueue_push_pop
rtrb_push_pop
shard_submit_to_start
shard_submit_to_finish
ipc_frame_encode
ipc_frame_decode
ipc_submit_to_finish
ir_vs_generated_1000
trace_off_vs_binary
```

## 11. Fuzz Targets

```text
fuzz_targets/yaml_events.rs
  Input: arbitrary bytes
  Assert: parser wrapper never panics; errors are typed

fuzz_targets/expression.rs
  Input: arbitrary bytes
  Assert: tokenizer/parser/compiler never panic

fuzz_targets/ipc_frame.rs
  Input: arbitrary bytes
  Assert: decoder never panics; length checks enforced

fuzz_targets/journal_event.rs
  Input: arbitrary bytes
  Assert: postcard decode failure is typed; no panic

fuzz_targets/compiled_ir.rs
  Input: arbitrary bytes
  Assert: decode/validate_compiled_invariants never panic
```

## 12. Property Tests

```text
expression_constant_folding_preserves_result
expression_bytecode_matches_ast_interpreter
compiled_digest_stable_for_same_input
slot_layout_stable_for_same_workflow
journal_replay_is_deterministic
snapshot_plus_tail_equals_full_journal_replay
for_each_output_order_matches_input_order
together_output_order_matches_yaml_order
retry_attempt_count_never_exceeds_limit
collect_never_exceeds_page_item_time_limits
no_terminal_state_transitions_back_to_running
secret_taint_never_enters_result
```

## 13. CLI Commands And Required Functions

```text
validate:
  read_workflow_file
  parse_document
  validate_document
  print_diagnostics_text

compile --emit ir:
  read_workflow_file
  parse_document
  validate_document
  compile_workflow
  encode_compiled_ir_postcard
  write_output_file

compile --emit rust:
  read_workflow_file
  parse_document
  validate_document
  compile_workflow
  emit_workflow_module
  write_output_file

run:
  read_workflow_file
  parse_document
  validate_document
  compile_workflow
  open_runtime
  register_compiled_workflow
  build_input_frame_from_binary
  submit_and_wait
  print_run_summary

run-compiled:
  read_compiled_file
  decode_compiled_ir
  validate_compiled_invariants
  open_runtime
  register_compiled_workflow
  build_input_frame_from_binary
  submit_and_wait

ipc-serve:
  open_runtime
  load_compiled_workflows
  bind_unix_socket
  accept_binary_frames
  dispatch_ipc_commands

inspect:
  open_fjall_store
  read_run_header
  read_latest_snapshot
  read_journal_events
  print_inspection_text

events:
  open_fjall_store
  read_journal_events
  decode_events
  print_events_text

replay:
  open_fjall_store
  recover_run
  replay_with_policy
  print_replay_report

doctor:
  check_db_path
  check_fjall_open
  check_keyspaces
  check_disk_space
  check_ipc_socket
  check_compiled_workflows

bench-run:
  read_workflow_file
  parse_validate_compile
  run_repeated
  print_latency_summary
```

## 14. CI And Validation Gate

Required commands:

```bash
cargo +nightly fmt --all -- --check

cargo +nightly clippy \
  --workspace \
  --all-targets \
  --all-features \
  -- \
  -D warnings \
  -D clippy::unwrap_used \
  -D clippy::expect_used \
  -D clippy::panic \
  -D clippy::panic_in_result_fn \
  -D clippy::todo \
  -D clippy::unimplemented \
  -D clippy::dbg_macro \
  -D clippy::indexing_slicing \
  -D clippy::string_slice \
  -D clippy::get_unwrap \
  -D clippy::arithmetic_side_effects \
  -D clippy::as_conversions \
  -D clippy::let_underscore_must_use

cargo +nightly test --workspace --all-features
cargo +nightly nextest run --workspace --all-features
cargo +nightly doc --workspace --all-features --no-deps
cargo audit
cargo deny check
cargo geiger
cargo vet
cargo machete
```

Miri on pure crates:

```bash
cargo +nightly miri test -p vb_core
cargo +nightly miri test -p vb_expr
cargo +nightly miri test -p vb_compile
```

Bench compile gate:

```bash
cargo +nightly bench --no-run
```

## 15. Max Performance Build

Profiles:

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"

[profile.maxperf]
inherits = "release"
lto = "fat"
codegen-units = 1
debug = false
```

Build:

```bash
cargo +nightly build --profile maxperf

RUSTFLAGS="-C target-cpu=native" \
cargo +nightly build --profile maxperf
```

PGO flow:

```bash
rm -rf /tmp/velvet-ballastics-pgo

RUSTFLAGS="-Cprofile-generate=/tmp/velvet-ballastics-pgo" \
cargo +nightly build --profile maxperf

./target/maxperf/velvet-ballastics bench-run tests/fixtures/e2e/minimal_set.yaml
./target/maxperf/velvet-ballastics bench-run tests/fixtures/e2e/full_workflow.yaml
./target/maxperf/velvet-ballastics bench-run tests/fixtures/e2e/reduce.yaml

LLVM_PROFDATA="$(rustc +nightly --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata"

"$LLVM_PROFDATA" merge \
  -o /tmp/velvet-ballastics-pgo/merged.profdata \
  /tmp/velvet-ballastics-pgo

RUSTFLAGS="-Cprofile-use=/tmp/velvet-ballastics-pgo/merged.profdata -Cllvm-args=-pgo-warn-missing-function" \
cargo +nightly build --profile maxperf
```

## 16. AI Agent Acceptance Contract

Every AI implementation PR must output:

```text
1. Phase implemented.
2. Files changed.
3. New public functions/types.
4. Error model.
5. Resource bounds.
6. Allocation behavior.
7. Hot-path behavior.
8. Fjall persistence behavior if touched.
9. Tests added.
10. Benchmarks added.
11. Commands run.
12. Remaining follow-up work.
```

Automatic rejection:

```text
uses unsafe
uses unwrap/expect/panic/todo/unimplemented/dbg
unchecked indexing/slicing
unchecked arithmetic/casts
ignored Result
unbounded queue/loop/retry/fanout
YAML interpreted at runtime
JSON inserted into runtime core
HTTP inserted into runtime core
HashMap<String, Value> runtime state
one task per step
no tests for new code
speed claim without benchmark
```

## 17. First AI Task

Task:

```text
Implement Phase 0 and Phase 1.
```

Scope:

```text
- Create workspace.
- Add toolchain/lints.
- Implement vb_core ids/errors/limits/value.
- Add tests.
- Add clippy gate.
```

Do not implement YAML yet until Phase 1 core passes.

Expected result:

```bash
cargo +nightly fmt --all -- --check
cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings
cargo +nightly test --workspace --all-features
```

## 18. Final Definition Of Done

`velvet-ballastics` is done when:

```text
1. Every language primitive validates, compiles, runs, persists, recovers, and replays.
2. Direct API can submit and complete a workflow.
3. Binary IPC can submit and complete a workflow.
4. Fjall stores workflow source, compiled IR, run headers, event journals, snapshots, blobs, and indexes.
5. Recovery from Fjall works after restart.
6. Generated Rust mode compiles and matches IR mode outputs.
7. All validation diagnostics include stable codes and source locations.
8. All runtime failures are typed and graceful.
9. All resource limits are enforced.
10. All queues are bounded.
11. No first-party unsafe/unwrap/expect/panic exists.
12. Full test suite, property tests, fuzz targets, and benchmarks exist.
13. `maxperf` and PGO workflow are documented and tested.
14. Benchmarks report transition latency, submit-to-finish latency, Fjall write latency, queue latency, IPC latency, and generated-vs-IR speed.
15. The runtime core contains no HTTP and no JSON.
```

