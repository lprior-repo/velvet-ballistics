# velvet-ballastics — Master Mechanical Build Contract

**Status:** implementation handoff; single source of truth for this repository
**Audience:** AI coding agents, runtime implementers, performance engineers, QA agents
**Product name:** `velvet-ballastics`
**Binary name:** `velvet-ballastics`
**Package name:** `velvet-ballastics`
**Rust crate/module prefix:** `velvet_ballastics`
**Bead rig:** `velvet-ballastics`
**Bead database:** `velvet_ballastics`
**Language version:** `velvet-ballastics/v1`

This file, `/velvet-ballistics-MASTER.md`, is the authoritative build plan, lifecycle tracker, architecture contract, and implementation acceptance contract for this repository. Other docs provide context only and cannot override this document.

Project spelling rule: any use of `velvet-ballistics` is invalid except for exactly these allowlisted legacy references: the current repository root path `/home/lewis/src/Velvet-ballistics`, the current master filename `/velvet-ballistics-MASTER.md`, and explicitly labeled migration references to pre-existing external artifacts. New code, docs, beads, crate names, package names, generated paths, CLI examples, diagnostics, and implementation artifacts must use the canonical names above.

---

## 0. Prime Directive

`velvet-ballastics` is a Rust-nightly, no-unsafe, no-panic, single-server, ultra-low-latency workflow orchestrator. YAML is an authoring format only. The runtime never interprets YAML, parses JSON, serves HTTP, or routes text commands. Workflows compile into numeric state machines over numeric slots, numeric actions, numeric steps, and bounded resource contracts.

The runtime uses numeric state machines, numeric slots, numeric actions, shard-owned state, and deterministic synchronous execution until suspension. Fjall is required for persistence. Postcard is required for compact binary records. Ingress is direct Rust API plus binary IPC. `CompiledWorkflow` IR lowers to mandatory generated Rust workflow mode for `maxperf` builds, and generated Rust must preserve the exact semantics of IR execution.

The final product must provide all of the following. None are optional:

1. Rust nightly toolchain with mechanical lint gates.
2. First-party code forbids `unsafe`, `unwrap`, `expect`, `panic`, unchecked indexing, unchecked slicing, unchecked casts, unchecked arithmetic, ignored `Result`, and unbounded resources.
3. YAML authoring only through a strict parser and validator.
4. No runtime YAML, JSON, or HTTP in `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, or generated workflow code.
5. Compiled numeric workflow IR with `WorkflowId`, `StepIdx`, `SlotIdx`, `ExprIdx`, `ActionId`, `AccessorIdx`, `ConstIdx`, and bounded tables.
6. Handle-based runtime values using interned symbol/list/object/blob handles and finite numeric values.
7. Deterministic state-machine execution until suspension on action, wait, ask, retry, fanout join, queue admission, or storage policy boundary.
8. Shard-owned run state with bounded queues, bounded frame pools, bounded trace rings, bounded retries, bounded fanout, bounded expression stacks, bounded IPC frames, and bounded persistence batches.
9. Fjall persistence for workflow source, compiled IR, run headers, journal events, snapshots, blobs, and indexes.
10. Postcard encoding for internal journal, snapshot, IPC payload, and compiled artifact records.
11. Direct Rust API ingress for fastest local embedding.
12. Binary IPC ingress for external local processes.
13. Generated Rust execution mode required for `maxperf` builds.
14. Typed validation, compile, runtime, IPC, and storage failures.
15. Benchmarked optimizations only; no speed claim without measured before/after data.
16. Mechanical gates accept AI changes only after formatting, linting, tests, fuzzing, recovery, benchmarks, dependency audit, supply-chain review, unsafe scan, and CI reproducibility pass.

HTTP/JSON exclusion rule: HTTP and JSON are excluded from the v1 runtime core. Any future adapter must be a separate cold-path adapter crate and must not enter `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, or generated workflow code.

---

## 1. Naming Contract

| Concept | Canonical spelling |
|---------|--------------------|
| Product | `velvet-ballastics` |
| Binary | `velvet-ballastics` |
| Cargo package | `velvet-ballastics` |
| Rust crate/module | `velvet_ballastics` |
| Bead rig | `velvet-ballastics` |
| Bead database | `velvet_ballastics` |
| Language version | `velvet-ballastics/v1` |

Mechanical rule: if an implementation agent introduces `velvet-ballistics` in a new file, test, path, diagnostic, bead, package, crate, command, or generated artifact, the change is rejected unless the text explicitly documents migration from a pre-existing external artifact.

---

## 2. Non-Negotiable Rust Rules

First-party Rust code under this workspace must satisfy these rules on every change:

- `#![forbid(unsafe_code)]` in every first-party crate.
- No `unsafe` blocks, traits, functions, impls, or FFI in first-party code.
- No `.unwrap()`.
- No `.expect()`.
- No `panic!`.
- No `todo!`, `unimplemented!`, or `dbg!`.
- No unchecked indexing with `[]`.
- No unchecked slicing.
- No unchecked `as` casts.
- No unchecked arithmetic, offset math, capacity math, or length math.
- No ignored `Result` or ignored fallible return value.
- No unbounded queues, loops, retries, fanout, buffers, task spawning, timers, pagination, or expression stacks.
- No YAML interpretation during run execution.
- No JSON in the runtime core.
- No HTTP in the runtime core.
- No dynamic string lookup for references during execution.
- No `HashMap<String, Value>` runtime state.
- No task-per-step scheduler.
- No formatted text output inside hot execution loops.

Dependency rule: third-party crates may contain internal unsafe only if pinned, audited, justified, and allowed by `cargo-geiger`, `cargo-vet`, `cargo-deny`, and the repository dependency policy.

---

## 3. Holzmann Compliance Matrix

| Holzmann rule | `velvet-ballastics` build contract |
|---------------|-------------------------------------|
| Simple control flow | Runtime transitions are explicit `StepIdx -> StepIdx`; no hidden graph mutation after compile. |
| Bounded loops | `for_each`, `collect`, `reduce`, `repeat`, retries, scheduler ticks, trace rings, storage batches, IPC frames, and expression stacks require explicit limits. |
| No dynamic allocation after init where avoidable | Turbo and maxperf modes preallocate frames, slots, step states, stacks, queues, trace rings, journal buffers, and IPC buffers. |
| Short functions | Hot functions must be <= 25 logical lines. Complex cold validation phase functions must be decomposed or carry a bead-linked justification and must stay out of hot paths. CI, justfile, and Moon tasks must include a source-length gate that fails hot functions over 25 logical lines. |
| Assertions/contracts | User errors return typed errors. Debug assertions may check compiler invariants that are unreachable for validated IR. |
| Small scopes | Each run belongs to exactly one shard. Shards own mutable runtime state. No global mutable run map. |
| Checked parameters/returns | Parse, validate, compile, eval, storage, IPC, action dispatch, scheduler, and generated execution return typed `Result`. |
| Restricted macros | No macro-hidden business logic. Codegen output is explicit Rust and checked by compile-fail and equivalence tests. |
| Restricted pointer complexity | No first-party pointer manipulation. Tables are addressed by checked numeric IDs. |
| Zero warnings | CI denies warnings, clippy violations, audit violations, unsafe scan findings, and missing benchmark metadata. |

---

## 4. Mandatory Rust Tooling

| Tool | Required use |
|------|--------------|
| `rustup` nightly | Only supported toolchain for first-party builds. |
| `rustfmt` | Formatting gate. |
| `clippy` | Hard deny lint gate. |
| `cargo-nextest` | Primary test runner. |
| `miri` | Pure crates: `vb_core`, `vb_expr`, `vb_compile`. |
| `criterion` | Local statistical benchmarks. |
| `iai-callgrind` | Instruction/cache benchmark gates. |
| `proptest` | Property and invariant tests. |
| `cargo-fuzz` | Parser, decoder, and IR fuzzing. |
| `trybuild` | Compile-fail tests for generated Rust and public macro/codegen contracts. |
| `cargo-audit` | Vulnerability gate. |
| `cargo-deny` | License, duplicate, source, and advisory gate. |
| `cargo-vet` | Supply-chain review gate. |
| `cargo-geiger` | Unsafe dependency scan. |
| `cargo-machete` | Unused dependency gate. |
| `cargo-hack` | Feature powerset gate. |
| `cargo-semver-checks` | Public compatibility gate for released crates. |
| `cargo-public-api` | Public API diff gate. |
| `cargo-bloat` | Size regression investigation. |
| `cargo-mutants` | Mutation testing, at least smoke scope in CI. |
| `cargo-llvm-cov` | Coverage report gate. |
| `cargo-insta` | Golden diagnostics only when approved by a bead. |
| `flamegraph` | Local profiling. |
| `samply` or `perf` | CPU profiling on Linux/native hosts. |
| `hyperfine` | CLI/end-to-end timing harness. |
| `valgrind` tools | `callgrind`, `cachegrind`, and `DHAT` investigation where available. |
| `moon` | CI orchestration gate; every mandatory command must be represented as a Moon task before release. |

Mandatory tooling categories:

- Formatting/linting: `cargo fmt`, hard-deny `clippy`, warnings as errors, banned-token scan.
- Test runners: `cargo test`, `cargo nextest`, `miri`, `cargo mutants`, `cargo llvm-cov`.
- Property/fuzz/compile diagnostics: `proptest`, `cargo-fuzz`, `arbitrary`, `trybuild`, and `insta` only when approved for golden diagnostics.
- Supply chain: `cargo audit`, `cargo deny`, `cargo vet`, `cargo geiger`, `cargo machete`, `cargo hack`, `cargo semver-checks`, `cargo public-api`, `cargo bloat`.
- Performance: `criterion`, `iai-callgrind`, `flamegraph`, `samply`/`perf`, `hyperfine`, `callgrind`, `cachegrind`, `DHAT`, PGO, and `target-cpu=native` builds.
- Nightly/dynamic verification: Miri, sanitizers, and coverage.

Bootstrap install block:

```bash
cargo install cargo-nextest cargo-audit cargo-deny cargo-vet cargo-geiger cargo-machete cargo-hack cargo-semver-checks cargo-public-api cargo-bloat cargo-mutants cargo-llvm-cov cargo-insta cargo-fuzz flamegraph hyperfine iai-callgrind-runner
```

`rust-toolchain.toml` contract:

```toml
[toolchain]
channel = "nightly-2026-04-28"
profile = "minimal"
components = ["rustfmt", "clippy", "rust-src", "miri", "llvm-tools-preview"]
targets = ["x86_64-unknown-linux-gnu"]
```

MSRV distinction: do not hardcode `rust-version = "1.91"` or any stable MSRV unless verified against actual stable support. The nightly pin controls builds today; a stable MSRV is a separate release promise and must be established by evidence.

Strict nightly governance:

- Nightly is mandatory.
- Unstable features are allowlisted; arbitrary `#![feature]` is rejected.
- `RUSTC_BOOTSTRAP` is rejected in developer shells, CI, scripts, and docs.
- CI must include a check equivalent to `RUSTFLAGS="-Zallow-features=portable_simd,likely_unlikely,allocator_api,generic_const_exprs"` for crates using unstable features.
- Source-allowed features: `portable_simd`, `likely_unlikely`.
- Performance-crate-only features: `allocator_api`, `generic_const_exprs`.

---

## 5. Library Choices

| Library | Purpose | Constraint |
|---------|---------|------------|
| `saphyr-parser` | Strict YAML event parsing | Cold path only. |
| `postcard` | Compact binary records | Required for journal, snapshots, IPC payloads, compiled artifacts. |
| `fjall` | Embedded LSM persistence | Required storage engine. |
| `thiserror` | Typed errors | Public errors must stay typed and stable. |
| `bytes` | Payload and blob sharing | Handles only in hot runtime state. |
| `arrayvec` | Fixed-capacity expression stacks and bounded scratch buffers | Hot path allowed when capacity is explicit. |
| `crossbeam-queue::ArrayQueue` | Bounded MPMC shard queues | No unbounded channel replacement. |
| `rtrb` | SPSC ring buffers and trace/action completion paths | Capacity required at construction. |
| `mio` | Low-level IPC event loop | No HTTP server, no JSON routing. |
| `criterion` | Statistical benchmarks | Required for local performance claims. |
| `iai-callgrind` | Instruction/cache benchmarks | Required for CI performance gates. |
| `proptest` | Property tests | Required for invariants. |
| `cargo-fuzz` | Fuzzing | Required for parsers/decoders. |
| `trybuild` | Compile-fail tests | Required for generated Rust contracts. |
| `cargo-nextest` | Test execution | Required CI test runner. |
| `cargo-audit` | Vulnerability scan | Required release gate. |
| `cargo-deny` | Policy scan | Required release gate. |
| `cargo-vet` | Supply-chain review | Required release gate. |
| `cargo-geiger` | Unsafe scan | Required release gate. |

`crossbeam-queue::ArrayQueue` is required for bounded MPMC queues because capacity is fixed at construction and admission can fail without allocating. `rtrb` is required for SPSC trace/action rings where single-producer/single-consumer ownership gives predictable bounded behavior.

`serde` is allowed only for deriving binary/data schema serialization used by Postcard or cold diagnostics. `serde_json` is excluded from v1 runtime core.

---

## 6. Maximum Performance Rules

1. `CompiledWorkflow` IR lowers to mandatory generated Rust workflow mode for `maxperf` builds.
2. IR interpreter mode remains mandatory for validation, portability, debugging, and semantic equivalence tests.
3. Generated Rust may skip dispatch tables, but it must preserve identical observable semantics to IR execution.
4. Runtime state is numeric and handle-based.
5. Hot loops must use checked table access, bounded stacks, bounded queues, and preallocated frame state.
6. Deterministic steps run synchronously inside the shard loop until suspension.
7. No async task is spawned per step.
8. No text formatting, YAML parsing, JSON parsing, HTTP handling, or string reference resolution on hot execution paths.
9. Any optimization must include before/after benchmark output, benchmark metadata, and no correctness regression.
10. PGO and `target-cpu=native` are release engineering tools, not semantic requirements.
11. Runtime architecture is shard-owned, single-server, synchronous deterministic execution until suspension.
12. Data layout is hot/cold split: hot state has numeric IDs and handles; cold side tables carry spans, names, YAML paths, messages, and diagnostics.
13. Queues and scheduling use bounded `ArrayQueue`/`rtrb`, explicit backpressure, and no task-per-step spawning.
14. Persistence uses Postcard binary records and Fjall keyspaces with bounded writer queues and explicit durability modes.
15. Compilation resolves strings, references, actions, accessors, constants, branches, and resource contracts before run admission.
16. Turbo mode admits a run only after required slots, step states, expression stacks, frame space, trace space, journal buffers, IPC buffers, and queue commands are preallocated or reserved; deterministic transitions must not allocate after acceptance.

---

## 7. Nightly Governance

Nightly is required to target peak performance and strict lint behavior. It is not permission to use unstable APIs casually.

Nightly update contract:

1. Nightly version changes require a dedicated bead.
2. The bead must record current nightly, target nightly, motivation, changed compiler behavior, and rollback plan.
3. Full CI, Miri, fuzz smoke, benchmarks, generated Rust compile tests, and recovery tests must pass.
4. Benchmark deltas must be recorded before and after the update.
5. Any new lint allowance requires explicit documented justification.

---

## 8. Language Specification

**Title:** Velvet Ballastics Workflow Language v1
**Canonical version string:** `velvet-ballastics/v1`

Required top-level fields:

```yaml
version: velvet-ballastics/v1
name: <workflow_name>
when: <trigger>
steps: <step_list>
```

Optional top-level fields:

```yaml
inputs:   # input schema declarations
vars:     # static non-secret constants
secrets:  # named secret requirements; literal secret values forbidden
result:   # final output mapping
examples: # executable test fixtures
```

Strict YAML profile:

- Allowed: strings, finite numbers, booleans, null, lists, objects, comments.
- Rejected: duplicate keys, anchors, aliases, merge keys, custom tags, binary scalars, multiple documents, unknown top-level fields, unknown step fields, YAML 1.1 ambiguous booleans (`yes`, `no`, `on`, `off`).

IDs:

- Pattern: `^[a-z][a-z0-9_]{0,63}$`
- Reserved roots: `input`, `inputs`, `vars`, `secrets`, `steps`, `result`, `when`, `item`, `error`, `attempt`, `total`.
- Reserved literals/primitives: `true`, `false`, `null`, `do`, `set`, `choose`, `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, `ask`, `try_again`, `on_error`, `then`, `finish`.

References:

- Allowed roots: `$input.x`, `$vars.x`, `$secrets.x`, `$step_id.x`, `$loop_name.x`, `$error.x`, `$attempt.x`, `$total.x`.
- Compiler rule: all references are parsed, validated, type-checked, and compiled to `SlotIdx` or `AccessorIdx` before execution.
- Runtime rule: the runtime never resolves reference strings.

Expressions:

- Operators: `==`, `!=`, `>`, `>=`, `<`, `<=`, `and`, `or`, `not`.
- Bounded arithmetic: `+`, `-`, `*`, `/`.
- Helpers: `contains`, `starts_with`, `ends_with`, `has`, `exists`, `length`, `empty`, `append`, `append_if`, `merge`, `sum`, `count`, `unique`.
- Forbidden: JavaScript, Python, jq, regex in v1, network calls, time/random functions, user-defined functions, loops inside expressions.

---

## 9. Trigger Contract

v1 supports exactly these core triggers:

```yaml
when:
  manual: {}
```

```yaml
when:
  ipc:
    name: issue_triage
```

`manual` means direct Rust API submission. `ipc` means local binary IPC submission. HTTP and webhook triggers are out of v1 runtime core. Any future HTTP or webhook adapter must be a separate cold-path adapter crate outside `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, and generated workflow code.

---

## 10. Step Primitive Contract

Every YAML step has exactly one primitive:

```text
set · do · choose · for_each · together · collect · reduce · repeat · wait · ask · finish
```

Control and metadata fields are not primitives:

```text
id · name · if · with · try_again · on_error · then
```

High-level YAML primitives may lower into multiple IR nodes. Runtime executes IR only. Generated Rust may skip dispatch, but it must preserve identical semantics.

---

## 11. Hot/Cold Data Layout

Hot runtime structs carry no diagnostic fields. They do not store source spans, YAML paths, human names, formatted messages, or string references. Cold side tables carry spans, names, YAML paths, source snippets, diagnostic messages, and trace rendering metadata.

No allocation after run admission in turbo mode: all hot slots, step states, taint arrays, expression stacks, trace events, queue commands, action tickets, and journal buffers are preallocated or reservation-checked before a run is accepted. If capacity cannot be reserved, admission fails with a typed error.

Cold path components may use maps when they improve clarity and diagnostics:

- `vb_yaml`
- `vb_validate`
- `vb_compile`
- diagnostics
- tests
- fixtures
- benchmark harness setup

`HashMap` and `BTreeMap` are allowed in parser, validator, compiler, diagnostics, and tests.

Hot runtime state and generated workflow state must not use `HashMap<String, Value>`, runtime/generated state maps, dynamic object maps, or string-keyed lookup. Hot state uses numeric indices, handle tables, boxed slices, fixed-capacity stacks, bounded queues, and typed handles.

---

## 12. Forbidden Hot-Path APIs

The following are forbidden in hot runtime paths and generated workflow execution:

```text
serde_json::Value
HashMap<String, _>
BTreeMap<String, _>
format!
println!
eprintln!
dbg!
identifier String::from/to_string
runtime maps
serde_json
String reference lookup
YAML parser calls
JSON parser calls
HTTP server/client calls
filesystem path parsing
environment variable reads
string action lookup
unbounded channel creation
Vec push without prior capacity/resource check
allocations for expression stack
allocations for trace event
allocations for queue command
blocking filesystem calls per deterministic step
blocking Fjall persist per deterministic node unless strict durability requires it
thread spawn per run
async task spawn per step
per-step thread spawn
unchecked indexing or slicing
unchecked arithmetic or casts
```

Nuance: these APIs are allowed in cold parser, validator, compiler, diagnostics, CLI, benchmark harness setup, and tests when covered by tests and kept out of hot runtime/generated execution.

---

## 13. Resource Contracts

Every accepted workflow has a compiled `ResourceContract`:

```rust
pub struct ResourceContract {
    pub max_steps: u16,
    pub max_slots: u16,
    pub max_constants: u16,
    pub max_accessors: u16,
    pub max_expressions: u16,
    pub max_expr_stack: u8,
    pub max_step_budget_per_tick: u64,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_ipc_payload_bytes: u32,
    pub max_retry_attempts: u16,
    pub max_fanout: u16,
    pub max_collect_items: u32,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
}
```

Compiler, runtime, IPC, and storage must reject or suspend before exceeding bounds. Silent truncation is forbidden.

Compile-time hard limits:

| Resource | Limit |
|----------|-------|
| YAML source bytes | 1 MiB |
| YAML parser depth | 64 |
| Language nesting depth | 8 |
| Steps | 1000 |
| Expressions | 4096 |
| Bytecode ops per expression | 256 |
| Expression stack depth | 64 |
| Constants | 8192 |
| Slots | `u16::MAX`, with a lower runtime default required |
| Accessors | 8192 |
| Path depth | 16 |

Runtime limits must be explicit per profile for active runs, ready queue depth, IPC frame bytes, action input bytes, action output bytes, step output bytes, result bytes, trace ring capacity, journal writer queue capacity, `for_each` item count and `at_once`, `together` branch count, `collect` pages/items/time, `repeat` attempts/time, retry attempts, maximum wait duration, and ask timeout.

---

## 14. Core Rust Types

All snippets are contracts for `crates/vb_core/src/`. Implementations may split files, but public behavior must match.

### `ids.rs`

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
pub struct SymbolId(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ListId(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObjectId(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlobId(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SeqNo(pub u64);

impl StepIdx { #[must_use] pub fn as_usize(self) -> usize { usize::from(self.0) } }
impl SlotIdx { #[must_use] pub fn as_usize(self) -> usize { usize::from(self.0) } }
impl ExprIdx { #[must_use] pub fn as_usize(self) -> usize { usize::from(self.0) } }
impl ActionId { #[must_use] pub fn as_usize(self) -> usize { usize::from(self.0) } }
impl AccessorIdx { #[must_use] pub fn as_usize(self) -> usize { usize::from(self.0) } }
impl ConstIdx { #[must_use] pub fn as_usize(self) -> usize { usize::from(self.0) } }
impl WorkflowId { #[must_use] pub fn as_u32(self) -> u32 { self.0 } }
impl RunId { #[must_use] pub fn as_u64(self) -> u64 { self.0 } }
```

### `value.rs`

```rust
#![forbid(unsafe_code)]

use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Taint {
    Clean,
    Secret,
    DerivedFromSecret,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct FiniteF64(f64);

impl FiniteF64 {
    pub fn new(value: f64) -> CoreResult<Self> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(CoreError::NonFiniteNumber)
        }
    }

    #[must_use]
    pub fn get(self) -> f64 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SlotValue {
    Null,
    Bool(bool),
    I64(i64),
    F64(FiniteF64),
    Symbol(SymbolId),
    List(ListId),
    Object(ObjectId),
    Blob(BlobId),
}

impl SlotValue {
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::I64(_) | Self::F64(_) => "number",
            Self::Symbol(_) => "symbol",
            Self::List(_) => "list",
            Self::Object(_) => "object",
            Self::Blob(_) => "blob",
        }
    }

    #[must_use]
    pub fn is_true(&self) -> bool {
        matches!(self, Self::Bool(true))
    }
}
```

`SlotValue` is a handle-based, `Copy`-compatible hot value model. It must remain small enough for hot slot arrays. Text and field names are interned as `SymbolId`; large UTF-8/text payloads live in blob arenas/stores and are referenced by `BlobId`. Lists and objects live in arenas and are referenced by `ListId` and `ObjectId`. Raw `bytes::Bytes` live in blob arenas or IPC buffers; hot slots hold handles only. `FiniteF64` rejects NaN and infinities.

### `errors.rs`

```rust
#![forbid(unsafe_code)]

use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};
use thiserror::Error;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("invalid compiled workflow: {reason}")]
    InvalidCompiledWorkflow { reason: &'static str },

    #[error("invalid program counter: {step:?}")]
    InvalidProgramCounter { step: StepIdx },

    #[error("missing next step for {step:?}")]
    MissingNextStep { step: StepIdx },

    #[error("missing output slot for {step:?}")]
    MissingOutputSlot { step: StepIdx },

    #[error("slot index out of bounds: {slot:?}")]
    SlotOutOfBounds { slot: SlotIdx },

    #[error("constant index out of bounds: {index:?}")]
    ConstOutOfBounds { index: ConstIdx },

    #[error("expression index out of bounds: {expr:?}")]
    ExprOutOfBounds { expr: ExprIdx },

    #[error("step state index out of bounds: {step:?}")]
    StepStateOutOfBounds { step: StepIdx },

    #[error("expression stack overflow: max {max}")]
    ExpressionStackOverflow { max: u8 },

    #[error("expression stack underflow")]
    ExpressionStackUnderflow,

    #[error("unsupported primitive: {primitive}")]
    UnsupportedPrimitive { primitive: &'static str },

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

    #[error("internal invariant violation: {reason}")]
    InternalInvariantViolation { reason: &'static str },
}
```

Required core errors include `ConstOutOfBounds { index: ConstIdx }`, `MissingOutputSlot { step: StepIdx }`, `MissingNextStep { step: StepIdx }`, `StepStateOutOfBounds { step: StepIdx }`, `ExpressionStackOverflow { max: u8 }`, `ExpressionStackUnderflow`, `InvalidCompiledWorkflow { reason: &'static str }`, `InternalInvariantViolation { reason: &'static str }`, `UnsupportedPrimitive { primitive: &'static str }`, and `NonFiniteNumber`.

### `compiled.rs`

```rust
#![forbid(unsafe_code)]

use crate::errors::{CoreError, CoreResult};
use crate::ids::{ActionId, AccessorIdx, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use crate::value::{FiniteF64, SlotValue};
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
    pub resource_contract: ResourceContract,
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
    Nop,
    SetConst { value: ConstIdx },
    Copy { source: SlotIdx },
    EvalExpr { expr: ExprIdx },
    BuildObject { fields: Box<[(SymbolId, SlotIdx)]> },
    BuildList { items: Box<[SlotIdx]> },
    Do { action: ActionId, input: SlotIdx },
    ChooseExpr { branches: Box<[ExprBranch]>, otherwise: Option<StepIdx> },
    ChooseSlot { branches: Box<[SlotBranch]>, otherwise: Option<StepIdx> },
    ForEachStart { input: SlotIdx, item_slot: SlotIdx, limit: u32, body: StepIdx, done: StepIdx },
    ForEachNext { iterator_slot: SlotIdx, body: StepIdx, done: StepIdx },
    ForEachJoin { output: SlotIdx },
    TogetherStart { branches: Box<[StepIdx]>, join: StepIdx },
    TogetherBranch { branch: u16, entry: StepIdx, join: StepIdx },
    TogetherJoin { branch_count: u16 },
    CollectStart { source: SlotIdx, limit: u32, page_size: u32, body: StepIdx, done: StepIdx },
    CollectPage { collector_slot: SlotIdx, body: StepIdx, done: StepIdx },
    CollectNext { collector_slot: SlotIdx, body: StepIdx, done: StepIdx },
    CollectFinish { collector_slot: SlotIdx },
    ReduceStart { input: SlotIdx, accumulator: SlotIdx, initial: ConstIdx, body: StepIdx, done: StepIdx },
    ReduceNext { iterator_slot: SlotIdx, accumulator: SlotIdx, body: StepIdx, done: StepIdx },
    ReduceFinish { accumulator: SlotIdx },
    RepeatStart { max_attempts: u16, body: StepIdx, done: StepIdx },
    RepeatAttempt { attempt_slot: SlotIdx, body: StepIdx, done: StepIdx },
    RepeatCheck { attempt_slot: SlotIdx, done: StepIdx },
    RepeatFinish { result: SlotIdx },
    WaitUntil { deadline_slot: SlotIdx },
    WaitEvent { event: SlotIdx, timeout_slot: Option<SlotIdx> },
    Ask { prompt: SlotIdx, timeout_slot: Option<SlotIdx> },
    AskResume { answer: SlotIdx },
    RetryCheck { policy_slot: SlotIdx, body: StepIdx, exhausted: StepIdx },
    ErrorHandler { body: StepIdx, handler: StepIdx },
    Jump { target: StepIdx },
    Finish { result: SlotIdx },
}
```

The final `CompiledNodeKind` includes all primitives: `Nop`, `SetConst`, `Copy`, `EvalExpr`, `BuildObject`, `BuildList`, `Do`, `ChooseExpr`, `ChooseSlot`, `ForEachStart`, `ForEachNext`, `ForEachJoin`, `TogetherStart`, `TogetherBranch`, `TogetherJoin`, `CollectStart`, `CollectPage`, `CollectNext`, `CollectFinish`, `ReduceStart`, `ReduceNext`, `ReduceFinish`, `RepeatStart`, `RepeatAttempt`, `RepeatCheck`, `RepeatFinish`, `WaitUntil`, `WaitEvent`, `Ask`, `AskResume`, `RetryCheck`, `ErrorHandler`, `Jump`, and `Finish`.

Compiler rule: high-level YAML primitives may lower to multiple IR nodes. Runtime executes IR only. Generated Rust may skip IR dispatch but must preserve identical semantics. Final choose IR has exactly two forms: `ChooseExpr` evaluates expression-branch conditions from `ExprIdx`, and `ChooseSlot` reads pre-materialized boolean conditions from `SlotIdx` values produced by earlier IR. Raw YAML condition strings and an ambiguous generic choose node are forbidden in final IR.

Legacy names such as `CopySlot`, `DoAction`, `Choose`, `TryAgain`, and `OnError` are migration notes only and must not be the final public IR. The deprecated migration-only `Choose` name may appear only in import adapters or migration tests that immediately normalize to `ChooseExpr` or `ChooseSlot` before validation succeeds.

```rust
pub enum DeprecatedCompiledNodeKindExampleOnly {
    TogetherFork { branches: Box<[StepIdx]>, join: StepIdx },
    Fail { error_slot: SlotIdx },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExprBranch {
    pub condition: ExprIdx,
    pub target: StepIdx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotBranch {
    pub condition: SlotIdx,
    pub target: StepIdx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExprProgram {
    pub ops: Box<[ExprOp]>,
    pub max_stack: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExprOp {
    LoadSlot(SlotIdx),
    LoadConst(ConstIdx),
    LoadAccessor(AccessorIdx),
    Eq, NotEq, Gt, Gte, Lt, Lte,
    And, Or, Not,
    Add, Sub, Mul, Div,
    Contains, StartsWith, EndsWith, Has, Exists, Length, Empty,
    Append, AppendIf, Merge, Sum, Count, Unique,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessorProgram {
    pub root: SlotIdx,
    pub path: Box<[PathSegment]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathSegment {
    Field(SymbolId),
    Index(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstValue {
    Null,
    Bool(bool),
    I64(i64),
    F64(FiniteF64),
    Symbol(SymbolId),
}

impl ConstValue {
    pub fn to_slot_value(&self) -> CoreResult<SlotValue> {
        match self {
            Self::Null => Ok(SlotValue::Null),
            Self::Bool(value) => Ok(SlotValue::Bool(*value)),
            Self::I64(value) => Ok(SlotValue::I64(*value)),
            Self::F64(value) => Ok(SlotValue::F64(*value)),
            Self::Symbol(value) => Ok(SlotValue::Symbol(*value)),
        }
    }
}
```

`choose` lowering rule: conditions must be `ExprIdx` in `ChooseExpr`, or must be materialized by `EvalExpr -> SlotIdx` followed by `ChooseSlot`. Runtime must not evaluate raw YAML condition strings. `ChooseSlot` accepts only slots whose validated static type is boolean; non-boolean slot values return `CoreError::TypeMismatch { expected: "boolean", found }`.

### `frame.rs`

```rust
#![forbid(unsafe_code)]

use crate::errors::{CoreError, CoreResult};
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Waiting,
    Asking,
    Cancelled,
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
    pub fn new(run_id: RunId, first_step: StepIdx, step_count: u16, slot_count: u16) -> CoreResult<Self> {
        let states_len = usize::from(step_count);
        let slots_len = usize::from(slot_count);
        if states_len == 0 {
            return Err(CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" });
        }
        if first_step.as_usize() >= states_len {
            return Err(CoreError::InvalidProgramCounter { step: first_step });
        }
        let states = vec![StepState::Pending; states_len].into_boxed_slice();
        let slots = vec![None; slots_len].into_boxed_slice();
        let taint = vec![Taint::Clean; slots_len].into_boxed_slice();
        Ok(Self { run_id, pc: first_step, executed: 0, states, slots, taint })
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

    pub fn read_taint(&self, slot: SlotIdx) -> CoreResult<Taint> {
        self.taint
            .get(slot.as_usize())
            .copied()
            .ok_or(CoreError::SlotOutOfBounds { slot })
    }

    pub fn write_taint(&mut self, slot: SlotIdx, taint: Taint) -> CoreResult<()> {
        *self
            .taint
            .get_mut(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })? = taint;
        Ok(())
    }

    pub fn mark_running(&mut self, step: StepIdx) -> CoreResult<()> {
        *self.states
            .get_mut(step.as_usize())
            .ok_or(CoreError::StepStateOutOfBounds { step })? = StepState::Running;
        Ok(())
    }

    pub fn mark_succeeded(&mut self, step: StepIdx) -> CoreResult<()> {
        *self.states
            .get_mut(step.as_usize())
            .ok_or(CoreError::StepStateOutOfBounds { step })? = StepState::Succeeded;
        Ok(())
    }

    pub fn mark_failed(&mut self, step: StepIdx) -> CoreResult<()> {
        *self.states
            .get_mut(step.as_usize())
            .ok_or(CoreError::StepStateOutOfBounds { step })? = StepState::Failed;
        Ok(())
    }

    pub fn mark_skipped(&mut self, step: StepIdx) -> CoreResult<()> {
        *self.states
            .get_mut(step.as_usize())
            .ok_or(CoreError::StepStateOutOfBounds { step })? = StepState::Skipped;
        Ok(())
    }

    pub fn mark_waiting(&mut self, step: StepIdx) -> CoreResult<()> {
        *self.states
            .get_mut(step.as_usize())
            .ok_or(CoreError::StepStateOutOfBounds { step })? = StepState::Waiting;
        Ok(())
    }

    pub fn mark_asking(&mut self, step: StepIdx) -> CoreResult<()> {
        *self.states
            .get_mut(step.as_usize())
            .ok_or(CoreError::StepStateOutOfBounds { step })? = StepState::Asking;
        Ok(())
    }

    pub fn mark_cancelled(&mut self, step: StepIdx) -> CoreResult<()> {
        *self.states
            .get_mut(step.as_usize())
            .ok_or(CoreError::StepStateOutOfBounds { step })? = StepState::Cancelled;
        Ok(())
    }

    pub fn step_state(&self, step: StepIdx) -> CoreResult<StepState> {
        self.states
            .get(step.as_usize())
            .copied()
            .ok_or(CoreError::StepStateOutOfBounds { step })
    }
}
```

`RunFrame::new` is the only frame constructor in `vb_core`. It allocates exactly three boxed arrays at admission: `states` length `step_count`, `slots` length `slot_count`, and `taint` length `slot_count`. It performs no arena, blob, symbol, object, list, journal, or queue allocation. It rejects `step_count == 0` with `CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" }` and rejects an out-of-range `first_step` with `CoreError::InvalidProgramCounter { step: first_step }`. Allocation failure must surface as `CoreError::AllocationFailed` when the implementation uses a fallible allocator path; it must not panic. `read_taint` and `write_taint` use the same slot bounds as `read_slot`/`write_slot` and return `CoreError::SlotOutOfBounds { slot }` for invalid slots. Step-state mutation methods `mark_running`, `mark_succeeded`, `mark_failed`, `mark_skipped`, `mark_waiting`, `mark_asking`, and `mark_cancelled` return `CoreResult<()>` and never silently ignore bad step IDs.

### `engine.rs`

```rust
#![forbid(unsafe_code)]

use crate::compiled::{CompiledNodeKind, CompiledWorkflow};
use crate::errors::{CoreError, CoreResult};
use crate::frame::RunFrame;
use crate::value::SlotValue;

#[derive(Debug, Clone, PartialEq)]
pub enum EngineSignal {
    Continue,
    AwaitingAction,
    AwaitingWait,
    AwaitingAsk,
    Finished(SlotValue),
    StepBudgetExhausted,
}

pub struct StepBudget {
    remaining: u64,
}

impl StepBudget {
    #[must_use]
    pub fn new(value: u64) -> Self { Self { remaining: value } }

    pub fn try_take(&mut self) -> CoreResult<bool> {
        if self.remaining == 0 {
            Ok(false)
        } else {
            self.remaining = self.remaining.saturating_sub(1);
            Ok(true)
        }
    }

    #[must_use]
    pub fn remaining(&self) -> u64 { self.remaining }
}

pub fn drive_deterministic(
    workflow: &CompiledWorkflow,
    frame: &mut RunFrame,
    budget: &mut StepBudget,
) -> CoreResult<EngineSignal> {
    loop {
        if !budget.try_take()? {
            return Ok(EngineSignal::StepBudgetExhausted);
        }
        let pc = frame.pc();
        let node = workflow
            .nodes
            .get(pc.as_usize())
            .ok_or(CoreError::InvalidProgramCounter { step: pc })?;

        frame.mark_running(pc)?;

        match &node.kind {
            CompiledNodeKind::SetConst { value } => {
                let out = node.output.ok_or(CoreError::MissingOutputSlot { step: pc })?;
                let constant = workflow
                    .constants
                    .get(value.as_usize())
                    .ok_or(CoreError::ConstOutOfBounds { index: *value })?;
                frame.write_slot(out, constant.to_slot_value()?)?;
                let next = node.next.ok_or(CoreError::MissingNextStep { step: pc })?;
                frame.set_pc(next);
                frame.mark_succeeded(pc)?;
            }
            CompiledNodeKind::Finish { result } => {
                let value = *frame.read_slot(*result)?;
                frame.mark_succeeded(pc)?;
                return Ok(EngineSignal::Finished(value));
            }
            CompiledNodeKind::Do { .. } => {
                return Ok(EngineSignal::AwaitingAction);
            }
            CompiledNodeKind::WaitUntil { .. } => {
                return Ok(EngineSignal::AwaitingWait);
            }
            CompiledNodeKind::Ask { .. } => {
                return Ok(EngineSignal::AwaitingAsk);
            }
            _ => return Err(CoreError::UnsupportedPrimitive { primitive: "not_yet_implemented" }),
        }
    }
}
```

The `Finish` arm may copy the result out of the frame because `SlotValue` is a handle-only `Copy` enum. If a future value model introduces non-`Copy` fields, this snippet must change to an explicit bounded clone/move contract before that value model can be accepted. `SetConst` has no silent `Null` fallback and no unrelated slot `0` guard. Missing constants, missing outputs, missing next steps, and unsupported primitives are typed errors.
`StepBudget` uses `remaining: u64`; `try_take() -> CoreResult<bool>`. Budget `0` executes zero transitions and returns `StepBudgetExhausted`. Budget `1` executes exactly one transition.

---

## 15. Final IR Contract

The runtime executes compiled IR only. YAML AST nodes never reach the runtime. High-level YAML primitives may lower into multiple primitive IR nodes.

Required IR coverage:

```text
Nop
SetConst
Copy
EvalExpr
BuildObject
BuildList
Do
ChooseExpr
ChooseSlot
ForEachStart
ForEachNext
ForEachJoin
TogetherStart
TogetherBranch
TogetherJoin
CollectStart
CollectPage
CollectNext
CollectFinish
ReduceStart
ReduceNext
ReduceFinish
RepeatStart
RepeatAttempt
RepeatCheck
RepeatFinish
WaitUntil
WaitEvent
Ask
AskResume
RetryCheck
ErrorHandler
Jump
Finish
```

Generated Rust execution may lower these into direct `match` arms or straight-line functions. It must keep the same step states, slot writes, taint behavior, suspension semantics, journal events, typed errors, and result values as IR mode.

Final choose contract: `ChooseExpr { branches, otherwise }` evaluates `ExprBranch { condition: ExprIdx, target }` in order and jumps to the first true expression; `ChooseSlot { branches, otherwise }` reads `SlotBranch { condition: SlotIdx, target }` in order after those slots have been materialized by prior IR. `ChooseSlot` condition slots must be validated as boolean slots. If no branch matches, `otherwise` is taken; missing `otherwise` with no match is `CoreError::MissingNextStep { step: current }`. The generic name `Choose` is migration-only and is not part of final IR.

---

## 16. Validation Error Codes

Required stable validation codes:

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
UNSUPPORTED_TRIGGER
HTTP_TRIGGER_OUT_OF_CORE
```

---

## 17. Runtime Error Codes

Required stable runtime codes:

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
CONST_OUT_OF_BOUNDS
MISSING_OUTPUT_SLOT
STEP_STATE_OUT_OF_BOUNDS
EXPRESSION_STACK_OVERFLOW
EXPRESSION_STACK_UNDERFLOW
INVALID_COMPILED_WORKFLOW
INTERNAL_INVARIANT_VIOLATION
UNSUPPORTED_PRIMITIVE
```

---

## 18. Fjall Persistence Behavior

Fjall is required. Recovery from Fjall is a product requirement, not an optional persistence layer.

Keyspaces:

```text
workflow_source   immutable YAML source by digest
compiled_ir       compiled workflow IR by digest
run_header        run metadata and status
run_event         compact binary event journal
run_snapshot      compact binary run snapshots
blob              large input/output/action payload blobs
index_status      status/time indexes
index_workflow    workflow/run indexes
index_action      pending action indexes
```

Binary key format uses prefix bytes plus big-endian numeric IDs. String keys are forbidden on hot paths.

```text
[0x01][workflow_digest_32]                         -> workflow_source
[0x02][compiled_digest_32]                         -> compiled_ir
[0x10][run_id_u64_be]                              -> run_header
[0x11][run_id_u64_be][seq_u64_be]                  -> run_event
[0x12][run_id_u64_be][seq_u64_be]                  -> run_snapshot
[0x20][blob_digest_32]                             -> blob
[0x30][state_u8][timestamp_u64_be][run_id_u64_be]  -> index_status
[0x31][workflow_id_u32_be][run_id_u64_be]          -> index_workflow
[0x32][action_id_u16_be][run_id_u64_be][step_u16]  -> index_action
```

Durability profiles:

| Profile | Behavior |
|---------|----------|
| `volatile` | No Fjall writes during run; only valid for explicit benchmark/test mode; restart loses accepted volatile runs. |
| `journaled` | Accepted runs append compact events to a bounded Fjall writer queue with bounded group commit; acknowledgement may occur after queue admission according to policy. |
| `strict` | Critical records are synchronously persisted and flushed before acknowledgement; blocking is allowed only at strict durability boundaries. |

Persistence invariants:

1. Accepted run binds immutably to one compiled workflow digest.
2. Journal sequence numbers are monotonic per run.
3. Recovery replays snapshots plus tail journal or full journal deterministically.
4. Replay never re-executes external side effects unless the action ABI declares the operation idempotent and replay-safe.
5. Corrupt records fail with typed storage/replay errors.
6. Storage writes obey durability profile and bounded batch contracts.
7. Recovery never reparses YAML for existing runs; it loads compiled artifacts, snapshots, and journal records by digest.
8. Replay checks workflow source digest, compiled workflow digest, action ABI digest, and policy digest. Mismatch returns typed replay failure and must not silently continue.

Every binary file, IPC frame, compiled artifact, snapshot, and journal record uses this envelope before payload decode. Multi-byte envelope fields are little-endian. Fjall keys remain big-endian as specified above for lexicographic ordering; record bodies are little-endian through this envelope and Postcard payloads.

```text
offset  bytes  field
0       4      magic_u32
4       2      schema_version_u16
6       2      record_kind_u16
8       4      header_len_u32 = 60
12      4      payload_len_u32
16      8      sequence_u64
24      32     payload_digest_blake3_256
56      4      header_crc32c
60      N      postcard payload, where N == payload_len_u32
```

Magic values:

| Family | Magic u32 | ASCII |
|--------|-----------|-------|
| Compiled artifact | `0x56424952` | `VBIR` |
| Journal event | `0x56424A45` | `VBJE` |
| Snapshot | `0x5642534E` | `VBSN` |
| Blob record | `0x5642424C` | `VBBL` |
| IPC frame | `0x56424C54` | `VBLT` |
| Workflow source record | `0x56425352` | `VBSR` |
| Index record | `0x56424958` | `VBIX` |

Required `record_kind_u16` IDs:

| ID | Kind |
|----|------|
| 1 | `WorkflowSource` |
| 2 | `CompiledIr` |
| 3 | `RunHeader` |
| 10 | `RunAccepted` |
| 11 | `StepStarted` |
| 12 | `SlotWritten` |
| 13 | `ActionScheduled` |
| 14 | `ActionCompleted` |
| 15 | `ActionFailed` |
| 16 | `WaitScheduled` |
| 17 | `AskScheduled` |
| 18 | `AskAnswered` |
| 19 | `RetryScheduled` |
| 20 | `StepFailed` |
| 21 | `RunCancelled` |
| 22 | `RunFinished` |
| 23 | `RunFailed` |
| 30 | `Snapshot` |
| 40 | `Blob` |
| 50 | `IndexUpdate` |

Decode order is mandatory: read 60-byte header, validate `magic_u32`, validate supported `schema_version_u16`, validate `record_kind_u16` is allowed for that family, validate `header_len_u32 == 60`, validate `payload_len_u32 <= ResourceContract.max_journal_batch_bytes` for journal batches or the configured family-specific maximum for compiled artifacts, snapshots, blobs, and IPC payloads, verify `header_crc32c` over bytes `0..56`, then read exactly `payload_len_u32` bytes, verify `payload_digest_blake3_256`, then Postcard-decode into the typed payload for the record kind. Payload allocation before length validation is forbidden.

Typed storage/decode errors must include `BadMagic { found: u32 }`, `UnsupportedSchemaVersion { version: u16 }`, `UnknownRecordKind { kind: u16 }`, `RecordKindFamilyMismatch { magic: u32, kind: u16 }`, `HeaderLengthMismatch { found: u32 }`, `PayloadTooLarge { len: u32, max: u32 }`, `HeaderChecksumMismatch`, `PayloadDigestMismatch`, `UnexpectedEof`, `PostcardDecodeFailed`, and `MigrationRequired { from: u16, to: u16 }`. Schema version migration is never implicit: an older supported version must pass through a named migration function that emits the current version and records migration evidence; unsupported versions fail with `MigrationRequired` or `UnsupportedSchemaVersion` and must not be replayed.

---

## 19. Action ABI

Actions are native Rust operations registered by numeric `ActionId` at compile time. Runtime dispatch never string-lookups action names.

Action names are resolved to `ActionId` during compile. The runtime and generated code dispatch by `ActionId` only. There is no `async_trait`, no dynamic string lookup, and no JSON input/output model.

Action contract:

```rust
pub struct ActionContract {
    pub id: ActionId,
    pub input_slot_count: u16,
    pub output_slot_count: u16,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub timeout_ms: u64,
    pub idempotency: Idempotency,
}

pub enum Idempotency {
    DeterministicPure,
    IdempotentExternal,
    AtLeastOnceExternal,
}

pub struct ActionInput {
    pub run: RunId,
    pub step: StepIdx,
    pub action: ActionId,
    pub input: SlotIdx,
    pub ticket: ActionTicket,
}

pub struct ActionOutput {
    pub output: SlotIdx,
    pub status: ActionOutcome,
}

pub type ActionResult<T> = Result<T, ActionError>;

pub struct ActionTicket {
    pub run: RunId,
    pub step: StepIdx,
    pub seq: SeqNo,
    pub action: ActionId,
    pub attempt: u16,
    pub idempotency_key: u128,
}

pub struct ActionOutputReady {
    pub output_slot: SlotIdx,
    pub value: SlotValue,
    pub taint: Taint,
    pub encoded_len: u32,
}

pub struct ActionFailure {
    pub code: ActionFailureCode,
    pub retryable: bool,
    pub taint: Taint,
    pub detail: Option<BlobId>,
    pub encoded_len: u32,
}

pub enum ActionFailureCode {
    Rejected,
    Timeout,
    RateLimited,
    ResourceExhausted,
    ExternalUnavailable,
    InvalidInput,
    PermissionDenied,
    Conflict,
    Unknown,
}

pub enum ActionError {
    UnknownAction { action: ActionId },
    InvalidTicket { ticket: ActionTicket },
    PayloadTooLarge { len: u32, max: u32 },
    OutputSlotOutOfBounds { slot: SlotIdx },
    NonIdempotentReplayBlocked { ticket: ActionTicket },
    CompletionAlreadyRecorded { ticket: ActionTicket },
    QueueFull,
    EncodingFailed,
    DispatchFailed,
}

pub enum ActionOutcome {
    Ready(ActionOutputReady),
    Suspended(ActionTicket),
    Failed(ActionFailure),
}
```

Action ABI referenced types are part of the stable binary contract. `ActionOutputReady.value` must be a handle-only `SlotValue`; large action output bytes are stored as a blob and returned as `SlotValue::Blob(BlobId)`. `encoded_len` is the Postcard payload byte length for the completion payload and must be `<= ActionContract.max_output_bytes` and `<= ResourceContract.max_blob_bytes` when a blob is produced. `ActionFailure.detail` is optional and must point to a bounded blob; error details never use heap strings in hot state.

Action completion payloads are encoded with the binary record envelope using `record_kind` `ActionCompleted` or `ActionFailed`. The payload contains `ActionTicket`, target output slot, outcome discriminant, `SlotValue` handle or `ActionFailure`, taint, and encoded length. Decode must validate ticket/run/step/action equality, output slot bounds, payload length bounds, idempotency policy, and duplicate completion before mutating a frame.

Taint propagation: action input taint is read from the input slot. `DeterministicPure` and `IdempotentExternal` actions must return output taint at least as restrictive as input taint; a clean result from tainted input is rejected unless the action contract declares a validator-proven declassification policy. `AtLeastOnceExternal` actions propagate taint conservatively as `DerivedFromSecret` when any input is `Secret` or `DerivedFromSecret`. Failure detail taint follows the same rule and secret-tainted failure details must not enter public diagnostics without redaction.

Retry and replay semantics: `DeterministicPure` may be re-executed during replay. `IdempotentExternal` may be retried or replay-completed only with the same `ActionTicket.idempotency_key`. `AtLeastOnceExternal` may be attempted more than once only according to a bounded retry policy and must not be re-executed during recovery after a scheduled journal record; recovery waits for explicit completion/failure or marks the run blocked by policy. Duplicate completion with the same ticket and same digest is idempotently ignored; duplicate completion with different digest returns `ActionError::CompletionAlreadyRecorded` and a replay divergence error.

Generated dispatch shape:

```rust
pub fn dispatch_action(action: ActionId, input: ActionInput) -> ActionResult<ActionOutcome> {
    match action {
        ActionId(0) => action_0(input),
        ActionId(1) => action_1(input),
        _ => Err(ActionError::UnknownAction { action }),
    }
}
```

Action rules:

1. Compile resolves action names to `ActionId`.
2. Runtime dispatches by `ActionId` only.
3. Inputs and outputs use `SlotValue` handles and blob references, not JSON values.
4. External side effects are at-least-once unless declared otherwise.
5. Action completion is explicit and journaled.
6. Action failures are typed and can enter `try_again` or `on_error` flows.
7. Replay policy must prevent accidental duplicate non-idempotent effects.
8. `Ready` resumes immediately with bounded output.
9. `Suspended` returns an `ActionTicket` and resumes only through direct API or IPC completion.
10. `Failed` returns typed failure data suitable for `RetryCheck` or `ErrorHandler`.

---

## 20. Runtime and Shard Design

Each shard owns:

- Bounded inbound command queue using `crossbeam_queue::ArrayQueue`.
- Run frame pool.
- Timer wheel for `wait`, `ask`, and retry delays.
- Action completion queue.
- Binary trace ring.
- Local counters.
- Fjall writer queue or handle according to durability profile.

No global `Arc<Mutex<RunState>>` is allowed. A run belongs to exactly one shard. Deterministic steps run synchronously inside the shard loop. Suspension boundaries are action, wait, ask, retry delay, fanout join, storage policy boundary, queue backpressure, cancellation, and shutdown.

Shard commands:

```rust
pub enum ShardCommand {
    Submit { run: RunId, workflow: WorkflowId },
    Resume { run: RunId },
    ActionCompleted { run: RunId, step: StepIdx },
    TimerFired { run: RunId },
    Cancel { run: RunId },
    Inspect { run: RunId, correlation: u64 },
    Shutdown,
}
```

---

## 21. Binary IPC Protocol

Fastest ingress is direct in-process Rust API. External local process ingress uses binary IPC.

Frame wire format:

```text
magic:       u32 = 0x56424C54  # VBLT
version:     u16
command:     u16
flags:       u16
reserved:    u16
correlation: u64
payload_len: u32
payload:     postcard-encoded bytes
```

Required IPC commands:

```text
SubmitRun
SubmitRunInline
CancelRun
InspectRun
ListEvents
AnswerAsk
CompleteAction
FailAction
DrainTrace
Health
Shutdown
```

Forbidden on IPC:

```text
HTTP ingress
JSON routing
unbounded channels
blocking producer admission
text command protocol
runtime YAML submission without prior compile/validation
```

IPC decoder requirements:

1. Validate magic before allocation.
2. Validate payload length against configured maximum before reading payload.
3. Decode Postcard into typed payloads only.
4. Return typed IPC errors for malformed frames.
5. Fuzz arbitrary bytes.

---

## 22. Generated Rust Workflow Mode

Generated Rust mode is mandatory for `maxperf` builds.

Command shape:

```bash
velvet-ballastics compile workflow.yaml --emit rust --out generated/issue_triage.rs
```

Generated code rules are identical to first-party code:

```text
no unsafe
no unwrap
no expect
no panic
no unchecked indexing
no unchecked slicing
no unchecked casts
no unchecked arithmetic
no JSON
no runtime YAML
no HTTP
no runtime string reference resolution
```

Generated Rust must:

1. Compile under the pinned nightly.
2. Pass `rustfmt`.
3. Pass `clippy` with repository deny settings.
4. Preserve IR semantics exactly.
5. Emit no hidden dynamic allocation in deterministic hot steps unless the resource contract explicitly allows it.
6. Produce equivalent journal events, slot values, taint states, errors, and terminal results to IR mode.
7. Be covered by equivalence tests and compile-fail tests.

---

## 23. Workspace Structure

Target structure:

```text
velvet-ballastics/
  Cargo.toml
  rust-toolchain.toml
  clippy.toml
  justfile
  deny.toml
  moon.yml
  supply-chain/
    config.toml
  crates/
    vb_core/
    vb_yaml/
    vb_validate/
    vb_expr/
    vb_compile/
    vb_storage/
    vb_runtime/
    vb_ipc/
    vb_codegen/
    velvet_ballastics/
  benches/
  fuzz/
  tests/
```

Existing scaffold mismatch must be rebaselined before feature implementation continues. `vb-compiler`, `vb-core`, `vb-ipc`, `vb-storage`, or hyphenated internal crates must be split/renamed to the underscore crate contract above, while preserving migration references only where needed.

---

## 24. Mandatory Function List: `vb_core`

Required public functions and methods:

```text
WorkflowId::as_u32
RunId::as_u64
StepIdx::as_usize
SlotIdx::as_usize
ExprIdx::as_usize
ActionId::as_usize
AccessorIdx::as_usize
ConstIdx::as_usize
FiniteF64::new
FiniteF64::get
SlotValue::type_name
SlotValue::is_true
ConstValue::to_slot_value
StepBudget::new
StepBudget::try_take
StepBudget::remaining
RunFrame::new
RunFrame::run_id
RunFrame::pc
RunFrame::executed
RunFrame::set_pc
RunFrame::read_slot
RunFrame::write_slot
RunFrame::read_taint
RunFrame::write_taint
RunFrame::mark_running
RunFrame::mark_succeeded
RunFrame::mark_failed
RunFrame::mark_skipped
RunFrame::mark_waiting
RunFrame::mark_asking
RunFrame::mark_cancelled
RunFrame::step_state
validate_compiled_workflow
drive_deterministic
step_once
eval_expr
eval_accessor
build_object
build_list
validate_resource_contract
validate_node_bounds
validate_transition_target
```

---

## 25. Mandatory Function List: `vb_yaml`

```text
parse_yaml_events
parse_workflow_source
validate_yaml_profile
reject_duplicate_keys
reject_forbidden_yaml_features
reject_anchors_aliases_merges
reject_multiple_documents
reject_yaml_1_1_ambiguous_scalars
build_source_map
span_for_node
load_fixture_source
```

---

## 26. Mandatory Function List: `vb_validate`

```text
validate_workflow_schema
validate_version
validate_trigger
validate_manual_trigger
validate_ipc_trigger
reject_http_trigger
validate_ids
validate_step_fields
validate_single_primitive
validate_references
validate_control_flow
validate_forward_only_then
validate_reachability
validate_types
validate_taint
validate_resource_limits
diagnostic_from_error
emit_golden_diagnostic
```

---

## 27. Mandatory Function List: `vb_expr`

```text
lex_expr
parse_expr
typecheck_expr
compile_expr
eval_expr_program
eval_binary_op
eval_unary_op
eval_helper
check_expr_stack_bound
compile_expr_to_bytecode
const_fold_expr
```

---

## 28. Mandatory Function List: `vb_compile`

```text
compile_workflow
build_slot_layout
build_accessor_table
build_constant_pool
lower_steps_to_ir
lower_set
lower_do
lower_choose
lower_for_each
lower_together
lower_collect
lower_reduce
lower_repeat
lower_wait
lower_ask
lower_finish
validate_ir
compute_compiled_digest
emit_compiled_artifact
compile_to_generated_rust
```

---

## 29. Mandatory Function List: `vb_storage`

```text
open_store
init_keyspaces
encode_key
put_workflow_source
put_compiled_ir
put_run_header
append_journal_event
write_snapshot
put_blob
read_blob
read_run_events
read_latest_snapshot
recover_run
recover_all_incomplete_runs
replay_journal
verify_replay_determinism
flush_profile
encode_record_header
decode_record_header
verify_digest_match
recover_snapshot_plus_tail
```

---

## 30. Mandatory Function List: `vb_runtime`

```text
Runtime::new
Runtime::submit_direct
Runtime::submit_compiled
Runtime::cancel_run
Runtime::inspect_run
Runtime::list_events
Runtime::answer_ask
Runtime::complete_action
Runtime::fail_action
Runtime::drain_trace
Runtime::shutdown_graceful
Shard::new
Shard::enqueue
Shard::tick
Shard::drive_run
Shard::handle_action_completion
Shard::handle_timer
ActionRegistry::register
ActionRegistry::resolve_compile_time
ActionRegistry::dispatch
FramePool::take
FramePool::release
drive_deterministic
step_once
```

---

## 31. Mandatory Function List: `vb_ipc`

```text
encode_frame
decode_frame_header
decode_frame_payload
validate_frame_bounds
serve_ipc
connect_ipc
send_command
recv_response
handle_ping
handle_submit_run
handle_submit_run_inline
handle_cancel_run
handle_inspect_run
handle_list_events
handle_answer_ask
handle_complete_action
handle_fail_action
handle_drain_trace
handle_health
handle_shutdown
```

---

## 32. Mandatory Function List: `vb_codegen`

```text
emit_rust_workflow
emit_ids
emit_drive_function
emit_step_function
emit_expr_function
emit_action_boundary
emit_finish
format_generated_rust
compile_check_generated_rust
compare_generated_to_ir
emit_action_match_dispatch
emit_resource_contract
emit_trybuild_fixture
```

---

## 33. CLI Commands

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

No JSON contract exists in v1. Machine output is binary or compact text diagnostics.

---

## 34. Workspace Cargo Contract

```toml
[workspace]
members = [
  "crates/vb_core",
  "crates/vb_yaml",
  "crates/vb_validate",
  "crates/vb_expr",
  "crates/vb_compile",
  "crates/vb_storage",
  "crates/vb_runtime",
  "crates/vb_ipc",
  "crates/vb_codegen",
  "crates/velvet_ballastics",
]
resolver = "2"

[workspace.package]
edition = "2024"
license = "MIT OR Apache-2.0"
version = "0.1.0"

[workspace.dependencies]
thiserror = "2"
serde = { version = "1", default-features = false, features = ["derive", "alloc"] }
postcard = { version = "1", default-features = false, features = ["alloc"] }
bytes = "1"
arrayvec = "0.7"
saphyr-parser = "0.0.6"
fjall = "3.1"
crossbeam-queue = "0.3"
rtrb = "0.3"
mio = "1"
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

## 35. Implementation Phases

Phase build order is mandatory. The old giant primitive phase is rejected; every primitive family has its own implementation, test, fuzz, and benchmark beads.

| Phase | Name | Required delivery |
|-------|------|-------------------|
| -1 | Name/repo rebaseline | Canonical spelling, folder/package/crate/bead rebaseline, migration notes. |
| 0 | Toolchain/lints/CI/justfile | Nightly pin, hard lints, Moon tasks, justfile, supply-chain skeleton. |
| 1 | Core scalar types | IDs, `WorkflowId::as_u32`, `RunId::as_u64`, `FiniteF64`, errors, limits. |
| 2 | Runtime value arenas | `SlotValue` handles, symbol/list/object/blob arenas, taint arrays. |
| 3 | Strict YAML event parser | `saphyr-parser` wrapper, YAML profile rejection, source maps, fuzz. |
| 4 | AST | Typed workflow, trigger, step, primitive, expression, result AST. |
| 5 | Schema validator | Required/unknown fields, ID rules, primitive count, diagnostics. |
| 6 | Reference validator | Reference tables, future/direct runtime reference rejection. |
| 7 | Control-flow validator | CFG, forward `then`, reachability, cycle rejection. |
| 8 | Type/taint validator | Input/action/result types, secret taint, leak rejection. |
| 9 | Expression lexer/parser | Bounded expression grammar, operators, helpers, parse diagnostics. |
| 10 | Expression bytecode | `ExprProgram`, fixed stack, overflow/underflow tests. |
| 11 | Slot compiler | Slot layout, accessors, constants, symbol interning, digests. |
| 12 | Core IR | Final `CompiledNodeKind`, IR validator, resource contracts. |
| 13 | Minimal deterministic engine | `SetConst`, `Copy`, `ChooseExpr`, `ChooseSlot`, `Finish`, `StepBudget`, invariant tests. |
| 14 | Direct API | Submit, cancel, inspect, list events, answer ask, complete/fail action. |
| 15 | Fjall base storage | Keyspaces, keys, workflow source, compiled IR, run headers, blobs. |
| 16 | Binary journal | Postcard record envelope, event records, schema versions, writer queue. |
| 17 | Snapshots/recovery base | Snapshot format, snapshot-plus-tail recovery, corruption handling. |
| 18 | Action ABI | Compile-time `ActionId`, ticket/outcome model, generated dispatch. |
| 19 | `do` | Action suspension, completion/failure resume, journal integration. |
| 20 | `retry`/`try_again` | Bounded retry policies, delay state, exhaustion semantics. |
| 21 | `on_error`/`then` | Handler routing, typed error slots, forward transitions. |
| 22 | `for_each` | Bounded iteration, `at_once`, item slots, ordered output. |
| 23 | `together` | Bounded branches, branch state, joins, failure policy. |
| 24 | `reduce` | Accumulator slots, bounded iteration, deterministic reducers. |
| 25 | `repeat` | Attempts, checks, finish semantics, time/attempt bounds. |
| 26 | `collect` | Page/item/time limits, pagination state, finish materialization. |
| 27 | `wait`/`ask` | Timer wheel, ask tickets, answer validation, timeout recovery. |
| 28 | Shard scheduler | Run ownership, bounded queues, frame pools, cancellation, shutdown. |
| 29 | Binary trace/counters | Trace ring, counters, binary drain, overhead benchmarks. |
| 30 | Binary IPC | `mio` Unix socket loop, required commands, frame fuzzing. |
| 31 | CLI | Validate, compile, run, replay, inspect, IPC serve, doctor, bench-run. |
| 32 | Generated Rust mode | Codegen, compile checks, equivalence tests, compile-fail tests. |
| 33 | Full recovery/replay | Digest mismatch detection, full primitive replay, non-idempotent policy. |
| 34 | Full benchmark suite | Criterion/iai suites, metadata, generated-vs-IR ratios. |
| 35 | Maxperf | PGO, target-cpu-native, mandatory generated Rust, regression thresholds. |
| 36 | Hardening | Full gates, sanitizer jobs, fuzz expansion, docs, bead evidence, release readiness. |

---

## 36. Mandatory Tests

Unit tests:

```text
finite_f64_accepts_finite
finite_f64_rejects_nan_and_infinity
slot_value_type_names_are_stable
slot_value_text_uses_symbol_or_blob_handles
const_value_to_slot_value_has_no_null_fallback
step_budget_try_take_exhausts_cleanly
run_frame_bounds_checked_for_slots
run_frame_bounds_checked_for_step_states
mark_methods_return_errors_on_invalid_step
compiled_workflow_rejects_invalid_pc
compiled_workflow_rejects_invalid_edges
compiled_workflow_rejects_invalid_tables
```

Parser and validator tests:

```text
minimal_manual_workflow_valid
minimal_ipc_workflow_valid
http_trigger_rejected_from_core
duplicate_keys_rejected
anchors_aliases_merge_tags_rejected
yaml_1_1_booleans_rejected
unknown_top_level_field_rejected
unknown_step_field_rejected
multiple_primitives_rejected
missing_primitive_rejected
future_reference_rejected
control_flow_cycle_rejected
secret_result_leak_rejected
all_diagnostics_have_code_path_span_message
```

Engine invariant tests:

```text
terminal_runs_do_not_return_to_running
failed_steps_do_not_become_succeeded_without_handler
budget_exhaustion_does_not_advance_pc
missing_output_slot_is_typed_error
const_out_of_bounds_is_typed_error
expression_stack_overflow_is_typed_error
expression_stack_underflow_is_typed_error
unsupported_primitive_is_typed_error
set_const_never_reads_unrelated_slot_zero
choose_expr_and_choose_slot_match_when_materialized
```

Recovery tests:

```text
recover_run_from_full_journal
recover_run_from_snapshot_plus_tail
replay_detects_divergence
replay_does_not_duplicate_non_idempotent_action
strict_profile_persists_before_ack
journaled_profile_group_commit_recovers
corrupt_journal_record_returns_typed_error
```

IPC tests:

```text
decode_rejects_bad_magic_before_payload_allocation
decode_rejects_oversized_payload
ping_roundtrip
submit_run_roundtrip
submit_run_compiled_roundtrip
cancel_run_roundtrip
inspect_run_roundtrip
get_events_roundtrip
stream_events_respects_backpressure
malformed_frame_returns_typed_error
```

Scheduler tests:

```text
queue_full_returns_typed_error
run_stays_on_one_shard
cancel_pending_run
cancel_waiting_run
shutdown_graceful_drains_or_reports_remaining
timer_resume_order_is_deterministic
action_completion_resumes_correct_run
no_task_per_step_behavior_under_load
```

Compile-fail tests:

```text
generated_code_cannot_use_unsafe
generated_code_cannot_unwrap
generated_code_cannot_index_unchecked
generated_code_cannot_reference_yaml_runtime
public_codegen_contract_rejects_missing_step
```

---

## 37. Fuzz Targets

```text
fuzz_targets/yaml_events.rs       arbitrary bytes -> parser never panics
fuzz_targets/expression.rs        arbitrary bytes -> tokenizer/parser/compiler never panics
fuzz_targets/ipc_frame.rs         arbitrary bytes -> decoder never panics and length checks hold
fuzz_targets/journal_event.rs     arbitrary bytes -> Postcard decode failure is typed
fuzz_targets/compiled_ir.rs       arbitrary bytes -> decode/validate never panics
fuzz_targets/generated_compare.rs generated/IR equivalence over small workflows
```

---

## 38. Property Tests

```text
expression_constant_folding_preserves_result
expression_bytecode_matches_ast_interpreter
compiled_digest_stable_for_same_input
slot_layout_stable_for_same_workflow
accessor_layout_stable_for_same_workflow
journal_replay_is_deterministic
snapshot_plus_tail_equals_full_journal_replay
for_each_output_order_matches_input_order
together_output_order_matches_yaml_order
retry_attempt_count_never_exceeds_limit
collect_never_exceeds_page_item_time_limits
no_terminal_state_transitions_back_to_running
secret_taint_never_enters_result
ir_and_generated_outputs_match
ir_and_generated_errors_match
```

---

## 39. Mandatory Benchmarks

Benchmark names:

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
const_lookup_checked
slot_taint_read_write
symbol_intern_compile
object_field_direct_slot
object_field_accessor
list_arena_append
list_arena_iter
blob_store_put_get
transition_set
transition_eval_expr
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
collect_page_10000
reduce_numeric_10000
repeat_100_attempts
postcard_encode_event
postcard_decode_event
fjall_append_event_no_persist
fjall_append_event_journaled
fjall_append_event_strict
fjall_read_1000_events
arrayqueue_push_pop
rtrb_push_pop
trace_event_push
trace_ring_full_policy
journal_writer_queue_push
group_commit_batch_1
group_commit_batch_64
group_commit_batch_1024
shard_submit_to_start
shard_submit_to_finish
direct_api_submit_to_finish
ask_answer_resume
action_complete_resume
wait_timer_resume
ipc_frame_encode
ipc_frame_decode
ipc_submit_to_finish
ir_vs_generated_1
ir_vs_generated_1000
generated_expr_eq
generated_set_chain_1000
generated_choose_100
trace_off_vs_binary
```

Every benchmark result must include metadata:

```text
git commit
rustc version
nightly date
CPU model
CPU governor
kernel version
build profile
RUSTFLAGS
benchmark tool and version
sample count or instruction count
input fixture digest
durability profile
generated vs IR mode
p50/p95/p99 latency
instruction counts
allocation count
bytes allocated
Fjall write latency
direct API latency
IPC latency
generated-vs-IR ratio
```

Acceptance rule: no speed claim without benchmark numbers. No optimization PR without before/after benchmark output and correctness evidence.

---

## 40. CI Gate

Required justfile targets:

```text
check
test
supply-chain
feature-powerset
miri
coverage
mutants-smoke
bench-build
source-length
maxperf
maxperf-native
fuzz-smoke
```

CI must gate on `just check`, `just test`, `just supply-chain`, `just fuzz-smoke`, `just miri`, `just coverage`, `just mutants-smoke`, `just bench-build`, `just source-length`, and `just feature-powerset`. Nightly sanitizer jobs are required for runtime, IPC, storage, and binary decoding crates. The `source-length` target must fail any hot runtime/generated function over 25 logical lines and must be represented by an equivalent Moon task.

Mandatory CI commands:

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
cargo +nightly nextest run --workspace --all-features
cargo +nightly test --doc --workspace --all-features
cargo +nightly doc --workspace --all-features --no-deps
cargo +nightly miri test -p vb_core -p vb_expr -p vb_compile
cargo +nightly bench --no-run
cargo audit
cargo deny check
cargo vet
cargo geiger
cargo machete
cargo hack check --feature-powerset --workspace
cargo semver-checks check-release
cargo public-api diff
cargo bloat --release --crates
cargo llvm-cov --workspace --all-features
cargo mutants --in-place --timeout 60 --package vb_core
cargo fuzz build
```

Moon expectation: each command above must have a Moon task before release, and the release gate must run through Moon rather than a hand-maintained shell script.

---

## 41. PGO and Maxperf Build

```bash
cargo +nightly build --profile maxperf
RUSTFLAGS="-C target-cpu=native" cargo +nightly build --profile maxperf

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

`maxperf` acceptance requires generated Rust mode for workflows being benchmarked.

---

## 42. Bead Work Breakdown

Beads are the only task tracking mechanism for this repository. Every implementation phase must be decomposed into beads. Each bead must include:

```text
phase
crate/module
public API touched
resource contract impact
hot-path impact
storage impact
IPC impact
tests required
benchmarks required
acceptance commands
rollback/migration notes
```

Required bead groups:

```text
naming-migration
toolchain-ci
core-types
yaml-parser
validator
expression-engine
compiler-ir
runtime-engine
fjall-storage
recovery-replay
action-abi
shard-scheduler
ipc-protocol
codegen
cli
observability
tests-fuzz
benchmarks
maxperf
release-gates
```

Every phase requires a parent bead. Every function cluster requires a child bead. The benchmark suite requires dedicated beads. Each fuzz target requires its own bead. Every P0 blocker requires a dedicated bead.

Required first beads:

```text
naming
crate-package-folder-rebaseline
optional-language-removal
manual-ipc-triggers
slotvalue-handle-model
stepbudget-setconst-corrections
primitive-subphases
toolchain-nightly-governance
holzmann-matrix
forbidden-hot-path-apis
```

Example bead commands:

```bash
bd create --title="P0: canonical naming rebaseline" --description="Align product, binary, package, crate, bead rig, bead database, and language version spelling." --type=task --priority=0
bd create --title="Phase 13: deterministic engine MVP" --description="Implement SetConst/Copy/Choose/Finish with StepBudget and invariant tests." --type=feature --priority=0
bd dep add <child-bead> <phase-parent-bead>
bd update <bead-id> --claim
bd close <bead-id> --reason="Completed with tests, benchmarks, and CI evidence"
```

No phase is complete until all beads for that phase are closed with test/benchmark evidence and `bd dolt push` has synced the bead database.

---

## 43. AI Agent Acceptance Contract

Every implementation PR or handoff must report:

```text
1. Phase implemented.
2. Beads touched.
3. Files changed.
4. New public functions/types.
5. Error model.
6. Resource bounds.
7. Allocation behavior.
8. Hot-path behavior.
9. Fjall persistence behavior if touched.
10. IPC behavior if touched.
11. Generated Rust behavior if touched.
12. Tests added.
13. Benchmarks added.
14. Commands run.
15. Remaining follow-up work filed as beads.
```

Automatic rejection triggers:

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
generated Rust omitted from maxperf
one task per step
no tests for new code
speed claim without benchmark
new velvet-ballistics spelling outside the exact allowlist
```

---

## 44. Final Definition of Done

`velvet-ballastics` is done when all 27 points are satisfied:

1. Canonical spelling is enforced for product, binary, package, crate/module, bead rig, bead database, and language version.
2. Any `velvet-ballistics` spelling outside the exact allowlist for `/home/lewis/src/Velvet-ballistics`, `/velvet-ballistics-MASTER.md`, or explicitly labeled pre-existing external migration artifacts is rejected.
3. Every primitive validates, compiles, runs, persists, recovers, and replays.
4. v1 supports both `manual` direct API submission and `ipc` binary IPC submission.
5. Runtime never interprets YAML and recovery never reparses YAML for existing runs.
6. JSON and HTTP are absent from `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, and generated workflow code.
7. Runtime state uses numeric workflow, run, action, step, slot, expression, accessor, constant, and sequence IDs.
8. Action dispatch uses numeric `ActionId`; no runtime string action lookup exists.
9. Hot values use handle-based `SlotValue` with `SymbolId`, `ListId`, `ObjectId`, `BlobId`, and finite numbers.
10. Each run is owned by exactly one shard; no global mutable run map exists.
11. Queues, stacks, buffers, retries, fanout, timers, traces, batches, IPC frames, and resource contracts are bounded.
12. Turbo/maxperf admission preallocates or reserves hot resources; deterministic transitions allocate nothing after acceptance.
13. Fjall stores workflow source, compiled IR, run headers, journals, snapshots, blobs, and indexes with magic/schema/version/kind/length envelopes.
14. Recovery and replay detect workflow, action, and policy digest mismatch and fail typed without default substitution.
15. Direct API implements submit, inspect, cancel, list events, answer ask, complete action, fail action, drain trace, health, and shutdown equivalents.
16. Binary IPC implements `SubmitRun`, `SubmitRunInline`, `CancelRun`, `InspectRun`, `ListEvents`, `AnswerAsk`, `CompleteAction`, `FailAction`, `DrainTrace`, `Health`, and `Shutdown`.
17. Generated Rust mode is implemented, mandatory for `maxperf`, and semantically equivalent to IR mode for success, failure, taint, journal, and replay behavior.
18. Diagnostics include stable code, path, source span, message, and cold side-table context.
19. Validation, compile, runtime, storage, IPC, action, and replay failures are typed and graceful.
20. Forbidden constructs are absent: `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, ignored `Result`, runtime maps, hot formatting, runtime YAML/JSON/HTTP, and string reference/action lookup.
21. Unchecked indexing, slicing, casts, and arithmetic are absent from first-party and generated code.
22. Every speed claim is backed by benchmark evidence with p50/p95/p99, instruction counts, allocation counts, bytes allocated, latency, durability mode, and fixture metadata.
23. Full gates pass: fmt, clippy hard denies, tests, nextest, trybuild, Miri, coverage, fuzz smoke, mutants smoke, supply chain, geiger, feature powerset, docs, and benchmark build.
24. Maxperf, PGO, and `target-cpu=native` workflows are documented, executable, and measured.
25. Sanitizer nightly jobs pass for binary decoders, IPC, storage, runtime, and generated workflows.
26. Every phase parent bead, function-cluster child bead, fuzz target bead, benchmark bead, and P0 blocker bead is closed with evidence.
27. Mechanical gates can accept AI changes without human guesswork because this document is implemented as executable checks, tests, benchmarks, and bead evidence.
