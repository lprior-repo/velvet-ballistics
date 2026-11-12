# Domain Model Review: `run --step` Type and State Machine

## Type Model Analysis

### Newtype Wrappers (Make Illegal States Unrepresentable)

The following newtype wrappers exist in `vb_core/src/ids/mod.rs`:

- `StepIdx(u16)` — index into compiled workflow node array. `new(value)` is public; the inner value is opaque. The constructor does not validate bounds, but all public APIs that use `StepIdx` perform bounds checking before dereferencing.
- `SlotIdx(u16)` — index into frame slot array. Same construction discipline.
- `RunId(u64)` — run identifier; constructed from nanoseconds or explicit value.
- `ExprIdx`, `ConstIdx`, `ActionId`, `AccessorIdx`, `SymbolId`, `ListId`, `ObjectId`, `BlobId`, `EventSeq`, `SeqNo` — all follow the same newtype pattern.

**Scott Wlaschin principle applied**: The raw numeric types are never exposed in public signatures. All indices are wrapped, making it impossible to accidentally pass a `u16` step index where a `SlotIdx` is expected. The compiler enforces the difference.

### StepState Enum (Type-Safe Step Lifecycle)

```rust
pub enum StepState {
    Pending,    // Not yet entered
    Running,    // Currently executing
    Succeeded,  // Completed successfully
    Failed,     // Error during execution
    Skipped,    // Skipped by control flow
    Waiting,    // Suspended on wait primitive
    Asking,     // Suspended on ask primitive
    Cancelled,  // Cancelled externally
}
```

**Design evaluation**:
- `Pending` is the initial state for all steps — enforced by `RunFrame::new` which initializes all entries to `Pending`.
- `Running` is transient — set by `run.mark_running()` before `execute_node`, cleared to `Succeeded`/`Failed`/`Waiting`/`Asking` by `mark_step_after_signal()` after execution.
- The enum is `non_exhaustive` — new variants can be added without breaking downstream code.
- There is no `Stopped` or `Aborted` variant distinct from `Failed`/`Cancelled`. If a distinction is needed in future, the type model accommodates it.
- The transition predicate `is_valid_step_state_transition(current, new)` is delegated to `vb_proof_kernels::step_state::is_valid_transition` — a thin proof kernel shim.

### EngineSignal Enum (Execution Outcome)

```rust
pub enum EngineSignal {
    Continue,                 // Step succeeded, PC advanced
    Finished(SlotValue, Taint), // Workflow finished with result
    StepBudgetExhausted,      // Budget depleted (not used in --step)
    AwaitingAction,           // Suspended on Do node
    AwaitingWait,             // Suspended on WaitUntil/WaitEvent
    AwaitingAsk,              // Suspended on Ask
}
```

**Design evaluation**:
- Each variant corresponds to a distinct caller action: continue, finish, resume action, resume wait, resume ask. There are no "orphan" variants.
- `Finished` carries a `SlotValue` and `Taint` — the result of the workflow. This is appropriate for CLI output.
- `StepBudgetExhausted` is not expected in single-step mode (budget is conceptually 1), but it is correct to surface it if encountered.
- All variants are `PartialEq + Eq + Clone + Debug` — serializable to JSON.

### SlotValue Enum (Runtime Data)

```rust
pub enum SlotValue {
    I64(i64),
    Bool(bool),
    Null,
    Object(ObjectId),
    List(ListId),
    FiniteF64(FiniteF64),
}
```

**Design evaluation**:
- No raw `f64` — `FiniteF64` rejects NaN and infinities at construction time, making illegal float states unrepresentable.
- Object and List use opaque handles (`ObjectId`, `ListId`) rather than inline values — the actual object/list data lives in `ValueStore`. This is a Parse Don't Validate pattern: the handle is always valid by construction; the store provides the lookup.
- `postcard` serialization is derived — `SlotValue` implements `Serialize` and `Deserialize`.

### Taint Enum (Secret Propagation)

```rust
pub enum Taint {
    Clean = 0,
    DerivedFromSecret = 1,
    Secret = 2,
}
```

**Design evaluation**:
- Monotonic lattice under join (Clean < DerivedFromSecret < Secret). `join_taint(a, b)` returns the maximum — enforced by the `join_taint()` function.
- u8 repr allows direct comparison via discriminant. The discriminant ordering matches the security lattice (0 = least sensitive, 2 = most sensitive).
- Taint is always written alongside a slot value via `write_slot_with_taint()` — no separate taint-only write path.

### DurabilityMode (Parse-Don't-Validate Boundary)

```rust
pub enum DurabilityMode {
    Strict,
    Journaled,
    None,
}
```

**Design evaluation**:
- Constructed from CLI argument strings via `DurabilityMode::from_str` (not shown, but exists in args.rs as `parse_durability`). Invalid strings result in a parse error before any execution.
- The CLI layer enforces `PRE-001: durability == None` as an explicit gate before calling `step_once`. This is a "parse at the boundary, then enforce in the domain" pattern.
- `DurabilityMode` is `Copy + Eq + PartialEq` — cheap to pass by value.

### OutputFormat (Parse-Don't-Validate Boundary)

```rust
pub enum OutputFormat {
    Text,
    Json,
    Jsonl,
}
```

**Design evaluation**:
- Default is `Text`. Invalid format strings result in a parse error, caught before execution.
- Json and Jsonl variants carry no additional data — the output format is a pure control flag.
- `OutputFormat` implements `Serialize` via postcard/serde — can be included in JSON output directly.

## Parse-Don't-Validate Boundaries

1. **StepTarget parsing** (`parse_optional_step` in args.rs):
   - `step_id: u16` — parsed with `parse::<u16>()`. Validated by `compiled.node(step_idx).is_some()` in `cmd_run_step`.
   - `step_input: PathBuf` — existence validated by `read_file()` before `decode_step_inputs()`.
   - The `StepTarget` struct carries raw parsed values; validation is deferred to the domain layer.

2. **Workflow compilation** (`compile_bytes` in app_impl.rs):
   - Raw bytes → `CompiledWorkflow` via `vb_compile::compile_workflow`. Compile errors are collected and reported.
   - The compiled artifact is self-consistent by construction (the compiler enforces structural validity).

3. **Step input decoding** (`decode_step_inputs` in app_impl.rs):
   - Raw bytes → `Box<[SlotValue]>` via `postcard::from_bytes`. Decode errors are caught and reported as `StepInputDecodeError`.

## Workflow State Machine for Step Execution

The single-step execution is a pure function with no state machine (no loop, no continuation). However, the frame's step states collectively form a state machine across multiple `step_once` calls in a full workflow run:

```
Pending → Running → Succeeded   (normal completion)
Pending → Running → Failed      (error during execution)
Pending → Running → Waiting    (suspended on wait)
Pending → Running → Asking     (suspended on ask)
Pending → Running → Skipped    (jump over this step)
Pending → Cancelled            (external cancellation)
```

For single-step `run --step`, only the first transition per invocation matters. The state after the step reflects the `EngineSignal`:

| EngineSignal        | StepState after |
|--------------------|-----------------|
| Continue           | Succeeded       |
| Finished           | Succeeded       |
| AwaitingAction     | Running         |
| AwaitingWait       | Waiting         |
| AwaitingAsk        | Asking          |
| StepBudgetExhausted| Running         |
| Err                | Failed          |

This mapping is the core invariant **INV-002** of the contract.

## Delta Reporting Design

The delta model requires before/after snapshots of:
- **pc**: `StepIdx` — the program counter before and after the step
- **slots**: `Box<[Option<SlotValue>]>` — each slot's value before and after
- **taint**: `Box<[Taint]>` — each slot's taint before and after
- **states**: `Box<[StepState]>` — each step's state before and after

Implementation approach:
1. Capture `frame.pc()`, `frame.slots.clone()`, `frame.taint.clone()`, `frame.states.clone()` before calling `step_once()`
2. Call `step_once()` which mutates the frame in place
3. Compute diffs: for each index, if before != after, record a delta entry
4. Serialize delta in the requested `OutputFormat`

This is a pure data transformation with no additional error surface. The delta computation is infallible (always produces a result) and the output is bounded by the frame's slot/step counts.

## Type Gaps and Risk Notes

- **Risk: structured-output-gap** — `print_step_result()` currently outputs text only. JSON/JSONL variants need to be added. The `OutputFormat` enum already exists; the gap is in the implementation of `print_step_result_json()` and `print_step_result_jsonl()`.
- **Risk: delta-reporting** — No before/after frame snapshot mechanism exists. The frame mutation is in-place, so "before" must be captured explicitly before calling `step_once()`. This is straightforward but must be implemented.
- **Risk: durability-gates** — The current implementation already rejects non-None durability (line 1476-1478 in app_impl.rs). This is correct behavior per the contract.
- **Risk: typed-errors** — `EngineError` variants are rich and descriptive. The gap is only in the CLI output formatting (JSON/JSONL error serialization), not in the error type itself.
- **Invariant enforcement**: `is_valid_step_state_transition()` delegates to `vb_proof_kernels::step_state::is_valid_transition`. This is the proof kernel boundary. If the proof kernel is correct, the runtime predicate is sound. The proof kernel is out of scope for this bead.
