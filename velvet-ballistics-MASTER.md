# velvet-ballastics — Master Implementation Document

**Status:** implementation handoff — single source of truth for all AI coding agents
**Audience:** AI coding agents, runtime implementers, performance engineers
**Product binary:** `velvet-ballastics`
**Rust crate/module:** `velvet_ballastics`
**Language version:** `velvet-ballastics/v1`
**Bead tracking:** all work is tracked as beads in the local Dolt database at `.beads/dolt/` (rig: `velvet-ballistics`, database: `velvet_ballistics`); use `bd` commands to create, advance, and close beads

---

## 0. Prime Directive

`velvet-ballastics` is a full end-to-end, single-server, ultra-low-latency workflow orchestrator written in Rust nightly. It is **not** a web workflow server. It is **not** a JSON interpreter. It is **not** a YAML interpreter. YAML is only the human/AI authoring format. The runtime executes precompiled numeric state machines from preallocated memory, uses Fjall for embedded persistence, and exposes direct in-process and binary IPC entry points.

The final product includes every item below. **None are optional:**

1. Strict YAML language parser with source-mapped diagnostics.
2. Full schema and semantic validator.
3. Expression parser, type checker, bytecode compiler, and evaluator.
4. Slot compiler that maps all references to `SlotIdx` or `AccessorIdx`.
5. Compact compiled IR format.
6. In-memory deterministic execution engine.
7. Native Rust action registry and dispatch by `ActionId`.
8. Full implementation of: `set`, `do`, `choose`, `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, `ask`, `try_again`, `on_error`, `then`, `finish`.
9. Shard-owned scheduler with bounded queues.
10. Fjall database storage for workflow snapshots, compiled IR, journal events, run headers, blobs, and indexes.
11. Compact binary journal using Postcard.
12. Recovery/replay from Fjall.
13. Binary trace ring and counters.
14. Direct Rust API submission.
15. Binary IPC submission.
16. Generated Rust workflow mode.
17. CLI: `validate`, `compile`, `run`, `run-compiled`, `inspect`, `events`, `replay`, `bench-run`, `doctor`.
18. Full tests, fuzz targets, property tests, and benchmarks.
19. Max-performance nightly build profile, PGO workflow, and benchmark gates.
20. CI lint gate that rejects unsafe, unwrap, expect, panic, unchecked indexing, unchecked slicing, unchecked casts, unchecked arithmetic, ignored Result, and unbounded resource behavior.

---

## 1. Non-Negotiable Rust Rules

First-party code (everything under `crates/` that is not a third-party dependency) must satisfy these on every PR:

- **No `unsafe`.**
- **No `.unwrap()`.**
- **No `.expect()`.**
- **No `panic!`.**
- **No `todo!`, `unimplemented!`, `dbg!`.**
- **No unchecked indexing** (`[i]` without bounds check).
- **No unchecked slicing.**
- **No unchecked numeric casts** (`as` without validation).
- **No unchecked size/capacity/offset arithmetic.**
- **No ignored `Result`** (every `Result` must be handled or explicitly propagated).
- **No unbounded queues, loops, retries, fanout, pagination, or task spawning.**
- **No YAML interpretation during execution.**
- **No JSON in the runtime core** (serde is allowed only for binary/data schema derivation).
- **No HTTP in the runtime core.**
- **No dynamic string lookup for references during execution.**
- **No `HashMap<String, Value>` runtime state.**
- **No task-per-step scheduler.**
- **No text formatting inside hot execution loops** (`format!`, `println!`, `eprintln!` in hot paths).

**Dependency rule:** Third-party crates may contain internal unsafe only if audited, pinned, and on the dependency allowlist. Runtime-facing dependencies must be justified by measurable performance, correctness, or implementation-risk reduction.

---

## 2. Holzmann Rules Adapted to velvet-ballastics

1. **Simple control flow:** Engine transitions are explicit `StepIdx → StepIdx`. No hidden graph mutation.
2. **No unbounded loops:** `for_each`, `collect`, `repeat`, retries, queues, traces, snapshots, and action fanout all require explicit limits.
3. **No dynamic allocation in hot paths when avoidable:** Preallocate run frames, slots, queues, trace rings, expression stacks, and journal buffers in turbo mode.
4. **Short functions:** Hot functions target <60 lines. Complex validation is decomposed by phase.
5. **Assertions/contracts:** Debug assertions may verify compiler invariants. Runtime user errors return typed errors.
6. **Small scopes:** Minimize mutable state. Shards own state; no global mutable run map.
7. **Checked parameters/returns:** All parse, compile, eval, store, dispatch, queue, and scheduler functions return typed `Result`.
8. **Restricted macros:** No macro-hidden business logic. Codegen is explicit and tested.
9. **Restricted pointer complexity:** No first-party unsafe pointer work. Use numeric IDs and checked table access.
10. **Zero warnings/static analysis:** Clippy hard denies, dependency audits, Miri on pure crates, fuzzing on parsers/decoders.

---

## 3. Performance Architecture

### Runtime pipeline

```
YAML bytes
  → strict YAML event parser (Saphyr)
  → source-mapped AST
  → schema validator
  → semantic validator
  → expression bytecode compiler
  → slot compiler
  → CompiledWorkflow IR
  → optional generated Rust workflow module
  → RunFrame from pool
  → deterministic state-machine loop
  → native ActionId dispatch / wait / ask / retry suspension
  → compact binary journal to Fjall
  → binary trace / counters
```

### Hot path

```
RunId dequeued from shard
  → frame.pc read
  → compiled node table access
  → execute deterministic primitive
  → write SlotIdx output
  → advance pc
  → continue until suspend or finish
```

### Hot path must NOT contain

```
YAML parse · JSON parse · HTTP request handling · string reference lookup
HashMap<String, Value> · serde_json::Value · allocation per step
Tokio task per step · format!/println!/JSONL output
fsync per deterministic step unless strict profile demands it
```

---

## 4. Language Specification

**Title:** Velvet Ballastics Workflow Language v1
**Canonical version string:** `velvet-ballastics/v1`

### Required top-level fields

```yaml
version: velvet-ballastics/v1   # must be exact
name: < workflow name >
when: < trigger >
steps: < step list >
```

### Optional top-level fields

```yaml
inputs:   # input schema declarations
vars:     # static non-secret constants
secrets:  # required secret bindings (no literal values)
result:   # final output mapping
examples: # executable test fixtures
```

### YAML Profile

**Allowed:** strings, finite numbers, booleans, null, lists, objects, comments.

**Rejected:** duplicate keys, anchors, aliases, merge keys, custom tags, binary scalars, YAML 1.1 ambiguous booleans (`yes`, `no`, `on`, `off`), unknown top-level fields, unknown step fields, multiple documents.

### Triggers

**v1 MVP:** `manual` only.

```yaml
when:
  manual: {}
```

**Later:** IPC trigger:

```yaml
when:
  ipc:
    name: issue_triage
```

HTTP/webhook is outside the runtime core — adapter, not core.

### Step Primitives

Every step has **exactly one** primitive: `set`, `do`, `choose`, `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, `ask`, `finish`.

Control/metadata fields (not primitives): `id`, `name`, `if`, `with`, `try_again`, `on_error`, `then`.

### IDs

Pattern: `^[a-z][a-z0-9_]{0,63}$`

Reserved names: `input`, `inputs`, `vars`, `secrets`, `steps`, `result`, `when`, `item`, `error`, `true`, `false`, `null`, `do`, `set`, `choose`, `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, `ask`, `try_again`, `on_error`, `then`, `finish`.

### References

Allowed roots: `$input.x`, `$vars.x`, `$secrets.x`, `$step_id.x`, `$loop_name.x`, `$error.x`, `$attempt.x`, `$total.x`.

**Compiler rule:** All references are parsed, validated, type-checked, and compiled to `SlotIdx` or `AccessorIdx` before execution.

**Runtime rule:** The runtime **never resolves reference strings**.

### Expressions

**Operators:** `==`, `!=`, `>`, `>=`, `<`, `<=`, `and`, `or`, `not`

**Bounded arithmetic:** `+`, `-`, `*`, `/`

Arithmetic rules: operands must be finite numbers; division by zero returns typed runtime error; NaN/Infinity/-Infinity are invalid; arithmetic allowed in `set` and `reduce.set`.

**Helpers:** `contains`, `starts_with`, `ends_with`, `has`, `exists`, `length`, `empty`, `append`, `append_if`, `merge`, `sum`, `count`, `unique`.

**Forbidden:** JavaScript, Python, jq, regex in v1, network calls, time/random functions, user-defined functions, loops inside expressions.

### Validation Error Codes (required)

```
DUPLICATE_KEY · FORBIDDEN_YAML_FEATURE · UNKNOWN_TOP_LEVEL_FIELD
UNKNOWN_STEP_FIELD · MISSING_REQUIRED_FIELD · INVALID_VERSION
INVALID_ID · RESERVED_ID · DUPLICATE_ID · MULTIPLE_STEP_PRIMITIVES
MISSING_STEP_PRIMITIVE · UNKNOWN_REFERENCE · FUTURE_REFERENCE
SECRET_NOT_DECLARED · DIRECT_RUNTIME_REFERENCE · INVALID_THEN_TARGET
CONTROL_FLOW_CYCLE · UNREACHABLE_STEP · INVALID_CHOOSE · INVALID_FOR_EACH
INVALID_TOGETHER · INVALID_COLLECT · INVALID_REDUCE · INVALID_REPEAT
INVALID_WAIT · INVALID_ASK · INVALID_FINISH · INVALID_RETRY · INVALID_ON_ERROR
SECRET_RESULT_LEAK · TYPE_MISMATCH · PAYLOAD_TOO_LARGE · LIMIT_REQUIRED
LIMIT_EXCEEDED
```

### Runtime Error Codes (required)

```
INPUT_MAPPING_FAILED · INPUT_TYPE_MISMATCH · SECRET_UNAVAILABLE
REFERENCE_MISSING · STEP_SKIPPED_REFERENCE · ACTION_FAILED · RETRY_EXHAUSTED
WAIT_TIMEOUT · ASK_TIMEOUT · FOR_EACH_ITEM_FAILED · TOGETHER_BRANCH_FAILED
COLLECT_LIMIT_REACHED · COLLECT_PAGE_FAILED · REDUCE_ITEM_FAILED
REPEAT_LIMIT_REACHED · RESULT_REFERENCE_MISSING · PAYLOAD_TOO_LARGE
QUEUE_FULL · IPC_FRAME_INVALID · IPC_PAYLOAD_TOO_LARGE · STORAGE_ERROR
REPLAY_DIVERGED · INTERNAL_INVARIANT_VIOLATION
```

---

## 5. Performance Contract

```
1. YAML is never interpreted during run execution.
2. JSON is never used for internal runtime values.
3. HTTP is not part of the runtime core.
4. Workflows are parsed, validated, and compiled before accepting runs.
5. Each run binds to an immutable compiled workflow snapshot.
6. References compile to numeric SlotIdx or AccessorIdx.
7. Steps compile to numeric StepIdx transitions.
8. Expressions compile to bytecode or generated Rust functions.
9. Actions compile to numeric ActionId dispatch.
10. Deterministic steps run synchronously until an async/suspend boundary.
11. The runtime does not spawn one task per step.
12. Run state is shard-owned.
13. Queues are bounded.
14. Memory is preallocated in turbo mode.
15. Fjall persistence happens through compact binary records.
16. Observability is counters or binary trace by default.
17. Human-readable output is diagnostic, not hot-path.
18. Every hot path has benchmarks.
```

---

## 6. Core Rust Types

All live under `crates/vb_core/src/`. Every file starts with `#![forbid(unsafe_code)]`.

### `ids.rs` — Compact numeric wrappers

```rust
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkflowId(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RunId(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StepIdx(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SlotIdx(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExprIdx(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActionId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccessorIdx(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConstIdx(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SeqNo(pub u64);

impl StepIdx {
    #[must_use] pub fn as_usize(self) -> usize { usize::from(self.0) }
}
impl SlotIdx {
    #[must_use] pub fn as_usize(self) -> usize { usize::from(self.0) }
}
impl ExprIdx {
    #[must_use] pub fn as_usize(self) -> usize { usize::from(self.0) }
}
impl AccessorIdx {
    #[must_use] pub fn as_usize(self) -> usize { usize::from(self.0) }
}
impl ConstIdx {
    #[must_use] pub fn as_usize(self) -> usize { usize::from(self.0) }
}

impl RunId {
    pub const ZERO: Self = Self(0);
    #[must_use] pub fn as_u64(self) -> u64 { self.0 }
}
impl StepIdx {
    pub const ZERO: Self = Self(0);
}
impl SlotIdx {
    pub const ZERO: Self = Self(0);
}
```

### `value.rs` — Runtime values and taint

```rust
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Taint {
    Clean,
    Secret,
    DerivedFromSecret,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SlotValue {
    Null,
    Bool(bool),
    I64(i64),
    Text(Box<str>),
    Bytes(bytes::Bytes),
}

impl SlotValue {
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null    => "null",
            Self::Bool(_) => "boolean",
            Self::I64(_)  => "number",
            Self::Text(_)  => "text",
            Self::Bytes(_) => "bytes",
        }
    }

    /// Returns true only for Bool(true). All other values are false.
    #[must_use]
    pub fn is_true(&self) -> bool {
        matches!(self, Self::Bool(true))
    }
}
```

### `errors.rs` — Typed errors

```rust
#![forbid(unsafe_code)]

use crate::ids::{ExprIdx, SlotIdx, StepIdx};
use thiserror::Error;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("invalid program counter: {step:?}")]
    InvalidProgramCounter { step: StepIdx },

    #[error("missing next step for {step:?}")]
    MissingNextStep { step: StepIdx },

    #[error("slot index out of bounds: {slot:?}")]
    SlotOutOfBounds { slot: SlotIdx },

    #[error("expression index out of bounds: {expr:?}")]
    ExprOutOfBounds { expr: ExprIdx },

    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: &'static str, found: &'static str },

    #[error("division by zero")]
    DivisionByZero,

    #[error("non-finite number is not allowed")]
    NonFiniteNumber,

    #[error("queue full")]
    QueueFull,

    #[error("resource limit exceeded: {resource}")]
    ResourceLimitExceeded { resource: &'static str },

    #[error("allocation failed")]
    AllocationFailed,
}
```

### `compiled.rs` — Compiled workflow IR

```rust
#![forbid(unsafe_code)]

use crate::ids::{ActionId, AccessorIdx, ConstIdx, ExprIdx, SlotIdx, StepIdx};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledWorkflow {
    pub workflow_id: u32,
    pub nodes: Box<[CompiledNode]>,
    pub expressions: Box<[ExprProgram]>,
    pub accessors: Box<[AccessorProgram]>,
    pub constants: Box<[ConstValue]>,
    pub slot_count: u16,
    pub first_step: StepIdx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledNode {
    pub id: StepIdx,
    pub output: Option<SlotIdx>,
    pub next: Option<StepIdx>,
    pub kind: CompiledNodeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompiledNodeKind {
    SetConst(CompiledSetConst),
    Copy(CompiledCopy),
    Choose(CompiledChoose),
    Finish(CompiledFinish),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSetConst {
    pub value: ConstIdx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledCopy {
    pub source: SlotIdx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledChoose {
    pub branches: Box<[CompiledBranch]>,
    pub otherwise: Option<StepIdx>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledBranch {
    pub condition: SlotIdx,
    pub target: StepIdx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledFinish {
    pub result: SlotIdx,
}

// Expression bytecode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExprProgram {
    pub ops: Box<[ExprOp]>,
    pub max_stack: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExprOp {
    LoadSlot(SlotIdx),
    LoadConst(u16),
    LoadAccessor(AccessorIdx),
    Eq, NotEq, Gt, Gte, Lt, Lte,
    And, Or, Not,
    Add, Sub, Mul, Div,
    Length, Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessorProgram {
    pub root: SlotIdx,
    pub path: Box<[PathSegment]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathSegment {
    Field(Box<str>),
    Index(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstValue {
    Null,
    Bool(bool),
    I64(i64),
    Text(Box<str>),
}
```

### `frame.rs` — Run frame

```rust
#![forbid(unsafe_code)]

use crate::errors::{CoreError, CoreResult};
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Pending, Running, Succeeded, Failed,
    Skipped, Waiting, Asking, Cancelled,
}

pub struct RunFrame {
    run_id: RunId,
    pc: StepIdx,
    executed: u64,
    states: Box<[StepState]>,
    slots: Box<[Option<SlotValue>]>,
    taint: Box<[Taint]>,
}

impl RunFrame {
    pub fn new(run_id: RunId, first_step: StepIdx, step_count: usize, slot_count: usize) -> Self {
        Self {
            run_id,
            pc: first_step,
            executed: 0,
            states: vec![StepState::Pending; step_count].into_boxed_slice(),
            slots: vec![None; slot_count].into_boxed_slice(),
            taint: vec![Taint::Clean; slot_count].into_boxed_slice(),
        }
    }

    #[must_use] pub fn run_id(&self) -> RunId { self.run_id }
    #[must_use] pub fn pc(&self) -> StepIdx { self.pc }
    #[must_use] pub fn executed(&self) -> u64 { self.executed }

    pub fn set_pc(&mut self, pc: StepIdx) { self.pc = pc; }

    pub fn read_slot(&self, slot: SlotIdx) -> CoreResult<&SlotValue> {
        self.slots
            .get(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })?
            .as_ref()
            .ok_or(CoreError::SlotOutOfBounds { slot })
    }

    pub fn write_slot(&mut self, slot: SlotIdx, value: SlotValue) -> CoreResult<()> {
        self.slots
            .get_mut(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })?
            .replace(value);
        Ok(())
    }

    pub fn copy_slot(&mut self, dst: SlotIdx, src: SlotIdx) -> CoreResult<()> {
        let value = self.read_slot(src)?.clone();
        self.write_slot(dst, value)
    }

    pub fn read_taint(&self, slot: SlotIdx) -> CoreResult<Taint> {
        *self.taint
            .get(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })
    }

    pub fn write_taint(&mut self, slot: SlotIdx, taint: Taint) -> CoreResult<()> {
        *self.taint
            .get_mut(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })? = taint;
        Ok(())
    }

    pub fn mark_running(&mut self, step: StepIdx) {
        if let Some(s) = self.states.get_mut(step.as_usize()) {
            *s = StepState::Running;
        }
    }

    pub fn mark_succeeded(&mut self, step: StepIdx) {
        if let Some(s) = self.states.get_mut(step.as_usize()) {
            *s = StepState::Succeeded;
        }
    }

    pub fn mark_failed(&mut self, step: StepIdx) {
        if let Some(s) = self.states.get_mut(step.as_usize()) {
            *s = StepState::Failed;
        }
    }

    pub fn step_state(&self, step: StepIdx) -> Option<StepState> {
        self.states.get(step.as_usize()).copied()
    }
}
```

### `engine.rs` — Deterministic engine

```rust
#![forbid(unsafe_code)]

use crate::compiled::{CompiledChoose, CompiledNodeKind, CompiledWorkflow, ExprProgram};
use crate::errors::{CoreError, CoreResult};
use crate::frame::RunFrame;
use crate::ids::StepIdx;
use crate::value::SlotValue;

#[derive(Debug, Clone, PartialEq)]
pub enum EngineSignal {
    Continue,
    AwaitingAction,
    Finished(SlotValue),
    StepBudgetExhausted,
}

pub struct StepBudget(u64);

impl StepBudget {
    pub fn new(n: u64) -> Self { Self(n) }
    pub fn decrement(&mut self) -> bool { self.0 = self.0.saturating_sub(1); self.0 > 0 }
    #[must_use] pub fn remaining(&self) -> u64 { self.0 }
}

/// Drive deterministic nodes until finish, action boundary, or budget exhaustion.
pub fn drive_deterministic(
    workflow: &CompiledWorkflow,
    frame: &mut RunFrame,
    budget: &mut StepBudget,
) -> CoreResult<EngineSignal> {
    loop {
        let pc = frame.pc();

        // Budget check before every transition
        if !budget.decrement() {
            return Ok(EngineSignal::StepBudgetExhausted);
        }

        let node = workflow
            .nodes
            .get(pc.as_usize())
            .ok_or(CoreError::InvalidProgramCounter { step: pc })?;

        frame.mark_running(pc);

        match &node.kind {
            CompiledNodeKind::SetConst(op) => {
                let value = frame.read_slot(SlotIdx(0)).ok()
                    .and_then(|_| {
                        workflow.constants.get(op.value.as_usize()).cloned()
                    })
                    .unwrap_or(SlotValue::Null);
                if let Some(out) = node.output {
                    frame.write_slot(out, value)?;
                }
                let next = node.next.ok_or(CoreError::MissingNextStep { step: pc })?;
                frame.set_pc(next);
                frame.mark_succeeded(pc);
            }

            CompiledNodeKind::Copy(op) => {
                if let Some(out) = node.output {
                    frame.copy_slot(out, op.source)?;
                }
                let next = node.next.ok_or(CoreError::MissingNextStep { step: pc })?;
                frame.set_pc(next);
                frame.mark_succeeded(pc);
            }

            CompiledNodeKind::Choose(choose) => {
                let next = eval_choose(workflow, frame, choose)?;
                frame.set_pc(next);
                frame.mark_succeeded(pc);
            }

            CompiledNodeKind::Finish(op) => {
                let result = frame.read_slot(op.result)?.clone();
                frame.mark_succeeded(pc);
                return Ok(EngineSignal::Finished(result));
            }
        }
    }
}

fn eval_choose(
    workflow: &CompiledWorkflow,
    frame: &RunFrame,
    choose: &CompiledChoose,
) -> CoreResult<StepIdx> {
    for branch in choose.branches.iter() {
        let cond = frame.read_slot(branch.condition)?;
        if cond.is_true() {
            return Ok(branch.target);
        }
    }
    choose.otherwise.ok_or(CoreError::ResourceLimitExceeded {
        resource: "choose_without_match",
    })
}

/// Execute one node (single-step mode for testing).
pub fn step_once(
    workflow: &CompiledWorkflow,
    frame: &mut RunFrame,
) -> CoreResult<EngineSignal> {
    let mut budget = StepBudget::new(1);
    drive_deterministic(workflow, frame, &mut budget)
}
```

---

## 7. Fjall Storage Design

### Keyspaces

```
workflow_source   immutable YAML source by digest
compiled_ir      compiled workflow IR by digest
run_header       run metadata
run_event        compact binary event journal
run_snapshot     compact binary run snapshots
blob             large step outputs / input blobs
index_status     status/time indexes
index_workflow   workflow/run indexes
```

### Binary key format

Prefix bytes + big-endian numeric IDs. No string keys on hot paths.

```
[0x01][workflow_digest_32]                    → workflow_source
[0x02][compiled_digest_32]                    → compiled_ir
[0x10][run_id_u64_be]                          → run_header
[0x11][run_id_u64_be][seq_u64_be]             → run_event
[0x12][run_id_u64_be][seq_u64_be]             → run_snapshot
[0x20][blob_digest_32]                        → blob
[0x30][state_u8][timestamp_u64_be][run_id_u64_be] → index_status
```

### Journal events

```rust
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use vb_core::ids::{ActionId, RunId, StepIdx};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JournalEvent {
    RunAccepted { run: RunId, workflow_digest: [u8; 32] },
    RunStarted { run: RunId },
    StepStarted { run: RunId, step: StepIdx, attempt: u16, action: Option<ActionId> },
    StepSucceeded { run: RunId, step: StepIdx, output_blob: Option<[u8; 32]> },
    StepFailed { run: RunId, step: StepIdx, code: u16 },
    RunSucceeded { run: RunId, result_blob: Option<[u8; 32]> },
    RunFailed { run: RunId, code: u16 },
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    InputMappingFailed = 1,
    ReferenceMissing = 2,
    ActionFailed = 3,
    RetryExhausted = 4,
    QueueFull = 5,
    PayloadTooLarge = 6,
    Internal = 255,
}
```

### Durability profiles

| Profile | Behavior |
|---------|----------|
| `volatile` | no Fjall writes during run |
| `snapshot` | periodic async snapshots |
| `journaled` | compact events queued to Fjall writer, group commit |
| `strict` | critical events synchronously persisted before acknowledgement |

---

## 8. Runtime/Shard Design

Each shard owns:

- bounded inbound command queue (`crossbeam_queue::ArrayQueue`)
- run frame pool
- timer wheel (for `wait`/`ask`/`repeat`)
- action completion queue
- binary trace ring
- local metrics

**No global `Arc<Mutex<RunState>>`.** A run belongs to exactly one shard. Deterministic steps run **synchronously** inside the shard loop. Only action/wait/ask/retry/fanout boundaries suspend execution.

```rust
#![forbid(unsafe_code)]

use crossbeam_queue::ArrayQueue;
use vb_core::errors::{CoreError, CoreResult};
use vb_core::ids::RunId;

#[derive(Debug, Clone, Copy)]
pub enum ShardCommand {
    Submit { run: RunId },
    Resume { run: RunId },
    Cancel { run: RunId },
    Stop,
}

pub struct Shard {
    id: u16,
    inbound: Arc<ArrayQueue<ShardCommand>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardProgress {
    MadeProgress,
    Idle,
    Stopped,
}
```

---

## 9. Binary IPC Protocol

Fastest path is direct in-process Rust API. IPC for external local processes is binary.

### Frame wire format

```
magic:       u32 = 0x56424C54  ("VBLT")
version:     u16
command:     u16
correlation: u64
payload_len: u32
payload:     postcard-encoded bytes
```

### Forbidden on IPC

```
HTTP ingress · JSON routing · unbounded channels · blocking producer admission
```

---

## 10. Generated Rust Workflow Mode

The fastest execution mode. `velvet-ballastics compile workflow.yaml --emit rust --out generated/issue_triage.rs`

Generated code rules — same as first-party:

```
no unsafe · no unwrap · no expect · no panic
no unchecked indexing · no JSON · no runtime string reference resolution
```

Shape:

```rust
pub fn drive_generated(
    frame: &mut vb_core::frame::RunFrame,
) -> vb_core::errors::CoreResult<vb_core::engine::EngineSignal> {
    loop {
        match frame.pc() {
            StepIdx(0) => { step_0_title(frame)?; frame.set_pc(StepIdx(1)); }
            StepIdx(1) => { return Ok(vb_core::engine::EngineSignal::AwaitingAction); }
            StepIdx(2) => { return step_2_finish(frame); }
            step => { return Err(vb_core::errors::CoreError::InvalidProgramCounter { step }); }
        }
    }
}
```

---

## 11. Library Choices

| Library | Purpose | Version |
|---------|---------|---------|
| `fjall` | embedded storage, LSM-tree | 3.1.x |
| `saphyr-parser` | strict YAML event parsing | 0.0.6 |
| `postcard` | compact binary encoding | 1.x |
| `serde` | derive for binary records only | 1.x |
| `thiserror` | typed errors | 2.x |
| `bytes` | payload/blob sharing | 1.x |
| `arrayvec` | fixed-capacity expression stacks, bounded buffers | 0.7.x |
| `compact_str` | compact text values | 0.9.x |
| `crossbeam-queue` | bounded MPMC queues | 0.3.x |
| `rtrb` | SPSC lock-free ring buffer | 0.3.x |
| `criterion` | local statistical benchmarks | 0.8.x |
| `iai-callgrind` | CI-style instruction/cache benchmarks | 0.16.x |
| `proptest` | property testing | 1.x |
| `cargo-fuzz` | fuzzing | — |

---

## 12. The 24 Design Decisions (Short Form)

```
1.  YAML is authoring only.
2.  Runtime executes CompiledWorkflow, not YAML.
3.  The fastest mode emits Rust code.
4.  Runtime values are SlotValue, not JSON.
5.  References are SlotIdx/AccessorIdx, not strings.
6.  Steps are StepIdx transitions, not YAML objects.
7.  Actions are ActionId dispatch, not string lookup.
8.  The engine loop is synchronous until suspension.
9.  No task per step.
10. Run state is shard-owned.
11. Queues are bounded.
12. Memory is preallocated in turbo mode.
13. Fjall persists compact binary records.
14. Postcard encodes internal journal/snapshot records.
15. Observability is counters/binary trace, not formatted strings.
16. HTTP/JSON are optional adapters, never the core.
17. Benchmarks decide every speed claim.
18. No unsafe in first-party code.
19. No unwrap/expect/panic in production paths.
20. Every queue/fanout/retry/loop has an explicit limit.
21. Diagnostics include stable codes and source locations.
22. Crash recovery replays from Fjall, not YAML.
23. The CLI is binary-first; JSON is a cold adapter.
24. Beads track every unit of work in Dolt.
```

---

## 13. First Implementation Target

A 3-step workflow for the first working demo:

```yaml
version: velvet-ballastics/v1
name: minimal_fast
when:
  manual: {}
inputs:
  label: text
steps:
  - id: copy_label
    set:
      value: $input.label
  - id: route
    choose:
      - if: $copy_label.value == "urgent"
        steps:
          - id: urgent
            set:
              kind: urgent
        result:
          kind: $urgent.kind
      - otherwise: true
        steps:
          - id: normal
            set:
              kind: normal
        result:
          kind: $normal.kind
result:
  kind: $route.kind
```

First benchmark goals:

```
run_noop_1_step · run_noop_10_steps · run_noop_1000_steps
transition_set · transition_choose
```

Only after this is fast: actions, Fjall, IPC, generated Rust.

---

## 14. Workspace Structure

```
velvet-ballastics/
  Cargo.toml
  rust-toolchain.toml
  clippy.toml
  justfile
  deny.toml
  supply-chain/
    config.toml
  crates/
    vb_core/           # IDs, values, compiled IR, frame, engine, errors, limits
    vb_yaml/           # Saphyr event parser, strict profile, source maps
    vb_validate/      # Schema, semantic, references, control flow, types, diagnostics
    vb_expr/           # Lexer, parser, AST, type checker, bytecode, evaluator
    vb_compile/        # Slot layout, lowering, constants, accessors, workflow compiler
    vb_runtime/        # Runtime, shards, queues, scheduler, actions, trace
    vb_storage/        # Fjall store, journal, keys, snapshot, blob store, recovery
    vb_ipc/            # Binary frame protocol, Unix socket, shared memory
    vb_codegen/        # Rust workflow emitter
    velvet_ballastics/ # CLI binary
  benches/
    parse_yaml.rs · validate.rs · compile_ir.rs · expression.rs
    transitions.rs · run_e2e.rs · queues.rs · storage_fjall.rs
    ipc.rs · codegen.rs
  fuzz/
    fuzz_targets/
      yaml_events.rs · expression.rs · ipc_frame.rs
      journal_event.rs · compiled_ir.rs
  tests/
    fixtures/
      valid/ · invalid/ · e2e/
```

**Existing vs. plan mismatch:** The current repo has `crates/vb-compiler/` (monolith), `crates/vb-core/`, `crates/vb-ipc/`, `crates/vb-storage/`, `crates/velvet-ballastics/`. The plan calls for the expanded crate map above. The existing scaffold must be **rebaselined** before Phase 2 begins: split `vb-compiler` into `vb_yaml`, `vb_validate`, `vb_expr`, `vb_compile`, and ensure `vb_core` absorbs the `CompiledWorkflow`/`RunFrame`/engine code. All existing types must be renamed/aligned to match this document's type shapes.

---

## 15. Workspace Cargo.toml

```toml
[workspace]
members = [
  "crates/vb_core",
  "crates/vb_yaml",
  "crates/vb_validate",
  "crates/vb_expr",
  "crates/vb_compile",
  "crates/vb_runtime",
  "crates/vb_storage",
  "crates/vb_ipc",
  "crates/vb_codegen",
  "crates/velvet_ballastics",
]
resolver = "2"

[workspace.package]
edition = "2024"
license = "MIT OR Apache-2.0"
rust-version = "1.91"
version = "0.1.0"

[workspace.dependencies]
thiserror = "2"
serde = { version = "1", default-features = false, features = ["derive", "alloc"] }
postcard = { version = "1", default-features = false, features = ["alloc"] }
bytes = "1"
arrayvec = "0.7"
smallvec = "1"
compact_str = "0.9"
saphyr-parser = "0.0.6"
fjall = "3.1"
crossbeam-queue = "0.3"
rtrb = "0.3"
parking_lot = "0.12"
criterion = "0.8"
iai-callgrind = "0.16"
proptest = "1"

[workspace.lints.rust]
unsafe_code = "forbid"
unused_must_use = "deny"
unreachable_pub = "deny"
rust_2018_idioms = "deny"

[workspace.lints.clippy]
correctness = "deny"
suspicious = "deny"
perf = "deny"
complexity = "deny"
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
panic_in_result_fn = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
indexing_slicing = "deny"
string_slice = "deny"
get_unwrap = "deny"
arithmetic_side_effects = "deny"
as_conversions = "deny"
let_underscore_must_use = "deny"
await_holding_lock = "deny"

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

[profile.bench]
inherits = "release"
debug = true
lto = "thin"
codegen-units = 1
```

---

## 16. Toolchain

```toml
# rust-toolchain.toml
[toolchain]
channel = "nightly-2026-04-28"
profile = "minimal"
components = ["rustfmt", "clippy", "rust-src", "miri", "llvm-tools-preview"]
targets = ["x86_64-unknown-linux-gnu"]
```

---

## 17. Implementation Phases

### Phase 0: Repository, toolchain, lints, CI skeleton

**Deliver:** workspace layout, `rust-toolchain.toml`, workspace lints, `deny.toml`, `justfile`, empty placeholder crates, CI script.

**Acceptance:**

```bash
cargo +nightly fmt --all -- --check
cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings
cargo +nightly test --workspace --all-features
```

### Phase 1: Core IDs, errors, limits, values

**Deliver:** `vb_core::ids`, `vb_core::errors`, `vb_core::limits`, `vb_core::value`

**Tests:** finite number accepts/rejects correctly, index conversions safe, limit checks typed, `SlotValue::type_name` correct, taint roundtrips through postcard.

### Phase 2: Strict YAML event parser

**Deliver:** `vb_yaml` — Saphyr event parser wrapper, strict profile validation, source maps, raw AST.

**Tests:** minimal valid workflow, duplicate keys rejected, anchors/aliases/merge/custom-tags/binary-scalars rejected, YAML 1.1 booleans rejected, source spans line/column correct, size/depth limits enforced.

**Fuzz:** arbitrary YAML bytes never panic parser wrapper.

### Phase 3: Language AST model

**Deliver:** Typed AST structs for top-level workflow, step primitive enum, input/vars/secrets AST, expression string holders with source spans.

### Phase 4: Schema validator

**Deliver:** `vb_validate` — top-level, step field, primitive count, ID, trigger, inputs/vars/secrets validation.

**Tests:** every validation code, all diagnostics include code/path/source-span/message.

### Phase 5: Reference and control-flow validator

**Deliver:** ID table, reference table, future reference rejection, CFG builder, forward-only `then` validation, cycle and reachability checks.

### Phase 6: Type validator and taint validator

**Deliver:** Deep type model, input schema validation, action contract schema validation, expression type environment, secret taint rules.

### Phase 7: Expression engine

**Deliver:** `vb_expr` — lexer, parser, AST, type checker, bytecode compiler, evaluator. Fixed-stack path when possible.

**Bench:** expression_eq, numeric_compare, boolean_chain, arithmetic reducer.

### Phase 8: Compiled IR and slot compiler

**Deliver:** `vb_compile` — slot layout, accessor layout, constant pool, expression table, node lowering, compiled digest.

**Bench:** compile 10 steps, compile 1000 steps.

### Phase 9: Deterministic in-memory engine MVP

**Deliver:** `RunFrame`, engine loop, `set`, `choose`, `finish`, in-memory input frame builder.

**Bench:** transition_set, transition_choose, run 1/10/1000 steps.

### Phase 10: Fjall storage foundation

**Deliver:** Fjall open/init, keyspaces, key encoders, workflow source, compiled IR, journal event encoding, run header, snapshot, blob storage.

**Bench:** postcard_encode/decode_event, fjall append (no-sync / strict / group), fjall read 1000 events.

### Phase 11: Runtime durability profiles

**Deliver:** `volatile`, `snapshot`, `journaled`, `strict` policy mapping to Fjall writes.

### Phase 12: Native action registry

**Deliver:** ActionId name resolver at compile time, action contracts, sync native dispatcher, fake built-ins (`memory.echo`, `memory.add`, `memory.fail`, `memory.sleep_tick`), `do` step execution and resume path.

### Phase 13: Shard scheduler and queues

**Deliver:** Runtime, shard objects, bounded submit queues, run frame pool, run routing, drive loop, cancellation, shutdown.

**Bench:** submit-to-start, submit-to-finish, queue latency, shard throughput.

### Phase 14: Full primitive implementation

**Deliver:** All primitives: `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, `ask`, `try_again`, `on_error`, `then`, `finish`.

**Bench:** for_each 10k noop, together 100 noop, reduce 10k, repeat 100 attempts.

### Phase 15: Binary trace and counters

**Deliver:** Atomic counters, binary trace event struct, bounded trace ring, trace modes (off/counters/binary), trace dump CLI.

### Phase 16: Binary IPC

**Deliver:** Frame protocol, Unix socket server/client, submit/cancel/inspect, bounded payload.

### Phase 17: CLI end-to-end

**Deliver:** All CLI commands: `validate`, `compile --emit ir`, `compile --emit rust`, `run`, `run-compiled`, `ipc-serve`, `inspect`, `events`, `replay`, `doctor`, `bench-run`.

### Phase 18: Generated Rust mode

**Deliver:** Rust emitter, generated workflow crate, equivalence tests vs. IR mode.

**Bench:** IR vs. generated, 1 step and 1000 steps.

### Phase 19: Recovery and replay

**Deliver:** Recovery from Fjall, journal replay, snapshot+tail replay, workflow snapshot binding, action side-effect replay policy, replay CLI command.

### Phase 20: Performance hardening

**Deliver:** `maxperf` profile, PGO script, `target-cpu=native` build, full benchmark suite, regression thresholds.

---

## 18. Mandatory Benchmarks

```
parse_yaml_small · parse_yaml_1mb
validate_minimal · validate_1000_steps
compile_ir_minimal · compile_ir_1000_steps
expr_eq_symbol · expr_number_compare · expr_boolean_chain · expr_arithmetic
slot_read · slot_write · slot_copy
transition_set · transition_choose_2 · transition_choose_100 · transition_finish
run_noop_1 · run_noop_10 · run_noop_1000 · run_set_chain_1000 · run_choose_heavy
for_each_noop_10000 · together_noop_100 · reduce_numeric_10000
postcard_encode_event · postcard_decode_event
fjall_append_event_no_persist · fjall_append_event_strict · fjall_read_1000_events
arrayqueue_push_pop · rtrb_push_pop
shard_submit_to_start · shard_submit_to_finish
ipc_frame_encode · ipc_frame_decode · ipc_submit_to_finish
ir_vs_generated_1000
trace_off_vs_binary
```

**Acceptance rule:** No speed claim without benchmark numbers. No optimization PR without before/after benchmark output.

---

## 19. Fuzz Targets

```
fuzz_targets/yaml_events.rs      arbitrary bytes → parser never panics
fuzz_targets/expression.rs        arbitrary bytes → tokenizer/parser/compiler never panics
fuzz_targets/ipc_frame.rs         arbitrary bytes → decoder never panics, length checks enforced
fuzz_targets/journal_event.rs     arbitrary bytes → postcard decode failure is typed
fuzz_targets/compiled_ir.rs       arbitrary bytes → decode/validate never panics
```

---

## 20. Property Tests

```
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

---

## 21. CI Gate

```bash
cargo +nightly fmt --all -- --check
cargo +nightly clippy --workspace --all-targets --all-features -- \
  -D warnings \
  -D clippy::unwrap_used -D clippy::expect_used \
  -D clippy::panic -D clippy::panic_in_result_fn \
  -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro \
  -D clippy::indexing_slicing -D clippy::string_slice \
  -D clippy::get_unwrap -D clippy::arithmetic_side_effects \
  -D clippy::as_conversions -D clippy::let_underscore_must_use
cargo +nightly test --workspace --all-features
cargo +nightly nextest run --workspace --all-features
cargo +nightly doc --workspace --all-features --no-deps
cargo audit · cargo deny check · cargo geiger · cargo vet · cargo machete
cargo +nightly miri test -p vb_core -p vb_expr -p vb_compile
cargo +nightly bench --no-run
```

---

## 22. PGO / Maxperf Build

```bash
cargo +nightly build --profile maxperf
RUSTFLAGS="-C target-cpu=native" cargo +nightly build --profile maxperf

# PGO
rm -rf /tmp/velvet-ballastics-pgo
RUSTFLAGS="-Cprofile-generate=/tmp/velvet-ballastics-pgo" \
  cargo +nightly build --profile maxperf
./target/maxperf/velvet-ballastics bench-run tests/fixtures/e2e/minimal_set.yaml
./target/maxperf/velvet-ballastics bench-run tests/fixtures/e2e/full_workflow.yaml
./target/maxperf/velvet-ballastics bench-run tests/fixtures/e2e/reduce.yaml
LLVM_PROFDATA="$(rustc +nightly --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata"
"$LLVM_PROFDATA" merge -o /tmp/velvet-ballastics-pgo/merged.profdata /tmp/velvet-ballastics-pgo
RUSTFLAGS="-Cprofile-use=/tmp/velvet-ballastics-pgo/merged.profdata -Cllvm-args=-pgo-warn-missing-function" \
  cargo +nightly build --profile maxperf
```

---

## 23. CLI Commands

```bash
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

No JSON contract in v1. Machine output is binary or compact text.

---

## 24. Final Definition of Done

`velvet-ballastics` is done when:

```
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
13. maxperf and PGO workflow are documented and tested.
14. Benchmarks report transition latency, submit-to-finish latency, Fjall write latency,
    queue latency, IPC latency, and generated-vs-IR speed.
15. The runtime core contains no HTTP and no JSON.
```

---

## 25. AI Agent Acceptance Contract

Every implementation PR must output:

```
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

**Automatic rejection** (any one = PR closed without merge):

```
uses unsafe · uses unwrap/expect/panic/todo/unimplemented/dbg
unchecked indexing/slicing · unchecked arithmetic/casts
ignored Result · unbounded queue/loop/retry/fanout
YAML interpreted at runtime · JSON inserted into runtime core
HTTP inserted into runtime core · HashMap<String, Value> runtime state
one task per step · no tests for new code · speed claim without benchmark
```
