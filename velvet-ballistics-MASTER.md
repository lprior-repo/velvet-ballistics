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
16. AI changes are accepted only with actual evidence that the relevant formatting, linting, tests, fuzzing, recovery, benchmark, dependency audit, supply-chain review, unsafe scan, and CI reproducibility gates ran and passed; merely adding or naming a task is not acceptance evidence.

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
- CI must include a first-party source check equivalent to `moon run :nightly-feature-gate`; a strict Cargo `-Zallow-features=try_blocks,portable_simd,allocator_api,generic_const_exprs` probe is required where transitive dependency feature attributes permit it. A represented task or probe is not proof that all governance policies passed; reports must state the actual command outcome.
- Normal source-allowed features: `try_blocks`, `portable_simd`.
- Perf-only features: `allocator_api`, `generic_const_exprs`, restricted to `crates/*/src/perf/**`, `crates/*/src/generated/**`, `benches/**`, or a file carrying `velvet-allow-perf-nightly-feature` if the feature-gate script implements that marker exception.
- Detailed operational policy lives in `docs/rust-governance.md` and is subordinate to this master contract.

---

## 5. Library Choices

| Library | Purpose | Constraint |
|---------|---------|------------|
| `saphyr-parser` | Strict YAML event parsing | Cold path only. |
| `postcard` | Compact binary records | Required for journal, snapshots, IPC payloads, compiled artifacts. |
| `fjall` | Embedded LSM persistence | Required storage engine. |
| `thiserror` | Typed errors | Public errors must stay typed and stable. |
| `byteorder` | Little-endian binary boundary helpers | Allowed for IPC/header/record field encode/decode only. Fjall keys remain explicit big-endian byte layouts for lexicographic ordering. |
| `bytes` | Payload and blob sharing | Handles only in hot runtime state. |
| `arrayvec` | Fixed-capacity expression stacks and bounded scratch buffers | Hot path allowed when capacity is explicit. |
| `logos` | Expression lexer state machine | Compile-time/cold-path lexer only. Must preserve exact spans, diagnostics, token limits, and fuzz coverage. No runtime execution path dependency. |
| `indexmap` | Deterministic object field side indexes | Cold `ValueStore` object lookup side table only. `SlotValue` remains handle-only; insertion order and duplicate-key behavior must remain stable. |
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
| `blake3` | Digest computation for envelopes and artifacts | Required for compiled digests, journal digests, blob digests. |
| `crc32c` | CRC32C header checksum for binary envelopes | Required for envelope header integrity. |

`crossbeam-queue::ArrayQueue` is required for bounded MPMC queues because capacity is fixed at construction and admission can fail without allocating. `rtrb` is required for SPSC trace/action rings where single-producer/single-consumer ownership gives predictable bounded behavior.

`serde` is allowed only for deriving binary/data schema serialization used by Postcard or cold diagnostics. `serde_json` is excluded from v1 runtime core.

`ordered-float` is not approved as the v1 `FiniteF64` implementation. `ordered_float::NotNan<f64>` rejects NaN but permits positive and negative infinity, while this language requires finite-only scalar values. Any future replacement must prove release-mode rejection of NaN and infinities, unchanged serialized representation, no panic/unwrap path, and no larger transitive footprint than the custom newtype.

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
    // NOTE: current implementation uses u32; the contract prefers WorkflowId for type safety.
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
    Choose { branches: Box<[ExprBranch]>, otherwise: Option<StepIdx> },
    ChooseSlot { branches: Box<[SlotBranch]>, otherwise: Option<StepIdx> },
    ForEachStart { input: SlotIdx, item_slot: SlotIdx, limit: u32, body: StepIdx, done: StepIdx },
    ForEachNext { iterator_slot: SlotIdx, body: StepIdx, done: StepIdx },
    ForEachJoin { output: SlotIdx },
    TogetherStart { branches: Box<[StepIdx]>, join: StepIdx },
    TogetherBranch { branch: u16, entry: StepIdx, join: StepIdx },
    TogetherJoin { branch_count: u16, accumulator: SlotIdx },
    CollectStart { source: SlotIdx, limit: u32, page_size: u32, body: StepIdx, done: StepIdx },
    CollectPage { collector_slot: SlotIdx, body: StepIdx, done: StepIdx },
    CollectNext { collector_slot: SlotIdx, body: StepIdx, done: StepIdx },
    CollectFinish { collector_slot: SlotIdx },
    ReduceStart { input: SlotIdx, accumulator: SlotIdx, initial: ConstIdx, body: StepIdx, done: StepIdx },
    ReduceNext { iterator_slot: SlotIdx, accumulator: SlotIdx, body: StepIdx, done: StepIdx },
    ReduceFinish { accumulator: SlotIdx },
    RepeatStart { max_attempts: u16, body: StepIdx, done: StepIdx },
    RepeatAttempt { attempt_slot: SlotIdx, body: StepIdx, done: StepIdx },
    RepeatCheck { attempt_slot: SlotIdx, body: StepIdx, exhausted: StepIdx },
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

The final `CompiledNodeKind` includes all primitives: `Nop`, `SetConst`, `Copy`, `EvalExpr`, `BuildObject`, `BuildList`, `Do`, `Choose`, `ChooseSlot`, `ForEachStart`, `ForEachNext`, `ForEachJoin`, `TogetherStart`, `TogetherBranch`, `TogetherJoin`, `CollectStart`, `CollectPage`, `CollectNext`, `CollectFinish`, `ReduceStart`, `ReduceNext`, `ReduceFinish`, `RepeatStart`, `RepeatAttempt`, `RepeatCheck`, `RepeatFinish`, `WaitUntil`, `WaitEvent`, `Ask`, `AskResume`, `RetryCheck`, `ErrorHandler`, `Jump`, and `Finish`.

Compiler rule: high-level YAML primitives may lower to multiple IR nodes. Runtime executes IR only. Generated Rust may skip IR dispatch but must preserve identical semantics. Final choose IR has exactly two checked forms: `Choose` evaluates expression-branch conditions from `ExprIdx`, and `ChooseSlot` reads pre-materialized boolean conditions from `SlotIdx` values produced by earlier IR. Raw YAML condition strings and untyped choose nodes are forbidden in final IR.

Legacy names such as `CopySlot`, `DoAction`, `TryAgain`, and `OnError` are migration notes only and must not be the final public IR. Deprecated untyped choose examples may appear only in import adapters or migration tests that immediately normalize to `Choose` or `ChooseSlot` before validation succeeds.

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

`choose` lowering rule: conditions must be `ExprIdx` in `Choose`, or must be materialized by `EvalExpr -> SlotIdx` followed by `ChooseSlot`. Runtime must not evaluate raw YAML condition strings. `ChooseSlot` accepts only slots whose validated static type is boolean; non-boolean slot values return `CoreError::TypeMismatch { expected: "boolean", found }`.

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
        *self
            .slots
            .get_mut(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })? = Some(value);
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
Choose
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

Final choose contract: `Choose { branches, otherwise }` evaluates `ExprBranch { condition: ExprIdx, target }` in order and jumps to the first true expression; `ChooseSlot { branches, otherwise }` reads `SlotBranch { condition: SlotIdx, target }` in order after those slots have been materialized by prior IR. `ChooseSlot` condition slots must be validated as boolean slots. If no branch matches, `otherwise` is taken; missing `otherwise` with no match is `CoreError::MissingNextStep { step: current }`. Untyped or string-condition choose nodes are migration-only and are not part of final IR.

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

Round 2 state: the workspace has been rebaselined to the underscore crate contract above (`vb_core`, `vb_yaml`, `vb_validate`, `vb_expr`, `vb_compile`, `vb_storage`, `vb_runtime`, `vb_ipc`, `vb_codegen`, and `velvet_ballastics`). Any future hyphenated internal crate name is a regression unless it is explicitly labeled as a migration artifact.

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
byteorder = "1.5"
bytes = "1"
arrayvec = "0.7"
indexmap = "2"
logos = "0.15"
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
| 13 | Minimal deterministic engine | `SetConst`, `Copy`, `Choose`, `ChooseSlot`, `Finish`, `StepBudget`, invariant tests. |
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
| 37 | Whole-workflow boundedness | Static dataflow analyzer: compute `WholeWorkflowBudget` from IR, propagate bounds through nested loops/branches, reject if any budget exceeds policy. New `BoundednessPolicy` config. Tests: nested fanout, sequential sum, conditional max, unbounded rejection. |
| 38 | Idempotency verification gate | `SideEffect` + `RetrySafety` classification per action. Verification gate rejects retry on side-effecting actions without idempotency key. Key ingredient validation (reject secrets, random, time in keys). New `IdempotencyViolation` error type. Tests: every side-effect class, key restriction, retry reachability. |
| 39 | Accepted artifacts + admission | `AcceptedArtifact` record with `VerificationProof`. `RunAdmission` flow: artifact digest, input validation, capability check, secret availability, `RunAccepted` event. Runs bind to artifact by digest, not loose YAML. CLI `--strict` mode for AI-authored workflows. Tests: admission rejection paths, artifact binding, strict-mode warnings. |
| 40 | Evidence chain completion | Slot value/taint snapshots in journal. Action input/output payload persistence for completed actions. Durability proof per primitive (each primitive must document what journal events constitute proof of completion). `VerificationProof.durable` field gates acceptance. Tests: crash recovery with evidence chain, payload reconstruction. |
| 41 | Capability model | `Capability` struct. Actions declare required capabilities. Admission checks granted capabilities. `CapabilityDenied` rejection. Operator grants capabilities at run submission. Tests: missing capability rejection, granted capability acceptance. |
| 42 | Validation deduplication | Eliminate duplicate validation between `vb_validate` and `vb_compile`. Single validation pipeline operating on a shared intermediate representation. Both crate APIs preserved for backward compatibility but backed by one implementation. |

Round 2 current implementation state, observed in this tree and not a final release claim:

| Area | Round 2 state | Remaining gap before final DoD |
|------|---------------|--------------------------------|
| Naming/workspace | Canonical crate layout and package spelling are represented in the workspace. | Mechanical spelling gates and bead evidence still decide acceptance for future changes. |
| Core/value/IR | `vb_core` exposes numeric IDs, handle-based `SlotValue`, `ValueStore`, taint/state APIs, bounded expression/accessor evaluation, resource contracts, and deterministic transition surfaces. | Full final primitive semantics still require end-to-end compiler/runtime/generated parity evidence. |
| YAML/validation/compile | Strict YAML parsing, AST validation, reference/control/type-taint checks, slot/accessor/constant APIs, digesting, artifact emission, and mandatory lowering function surfaces exist. | Source-to-IR lowering must be proven for the full v1 primitive set, not only constructor/API coverage. |
| Expression engine | Lexer/parser/typecheck/bytecode surfaces exist with bounded execution contracts. | Helper coverage, mutation resistance, and generated-mode equivalence require gate evidence. |
| Storage/recovery | `vb_storage` exposes required keyspace names, key encoders, record envelope encode/decode, journal writer queue, snapshots, replay helpers, and recovery summary APIs. | Runtime admission/header persistence and full live-frame hydration must be proven end-to-end; recovery summaries alone are not final recovery acceptance. |
| Runtime/direct API | `vb_runtime` exposes direct API, shard/frame-pool/action/wait/ask/trace/counter surfaces and typed runtime errors. | Collect pagination state, strict persistence-before-ack behavior, shutdown/cancellation edge cases, and recovery hydration need executable evidence. |
| IPC | `vb_ipc` exposes bounded frame/header/payload validation, typed payloads, memory ingress, client/server surfaces, and required command handlers. | Socket-loop fuzz/backpressure evidence and runtime integration gates remain required. |
| Generated Rust | `vb_codegen` emits and checks a supported subset covering scalar constants, copies, expression math/comparisons, action dispatch, waits, asks, jumps, choices, handlers, and finish nodes. | Generated mode is not yet accepted for the full final IR; unsupported primitives/accessor traversal are intentionally rejected until equivalence tests prove parity. |
| Tests/audits | Error-variant completeness and diagnostic-code range tests exist; companion docs record benchmark and dependency policy constraints. | Full matrix gates, fuzz, Miri, coverage, mutants, sanitizer, supply-chain, benchmark metadata, and bead closure evidence are still required. |

Round 2 status rule: a public function existing in a crate is only API surface evidence. It is not proof that the phase is complete unless the required tests, fuzz/property coverage, benchmark evidence where applicable, and bead closure evidence have actually passed.

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

Acceptance rule: no speed claim without benchmark numbers. No optimization PR without before/after benchmark output and correctness evidence. Compileable Criterion scaffold benchmarks are placeholders only; no-op scaffolds such as `black_box(())` prove the harness builds, not that the implementation is faster, lower allocation, lower latency, or production ready.

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

Dependency-scoping beads are mandatory when a library is added to reduce code footprint. They must record the removed handwritten code, semantic parity tests, hot-path allocation impact, and rollback decision. Current required dependency-scope beads:

```text
byteorder-ipc-little-endian-helpers
logos-expression-lexer-parity
indexmap-valuestore-object-field-index
ordered-float-finitef64-rejection-record
```

Round 2 black-hat findings remediated in this master document:

```text
hot-slotvalue-handle-only-model
finish-copy-out-compatible-with-copy-slotvalue
runframe-constructor-and-taint-api-contract
narrow-canonical-spelling-allowlist
hot-function-length-hard-gate
choose-ir-final-variant-disambiguation
action-abi-type-and-binary-semantics
persistence-record-envelope-byte-contract
mvp-wording-removed-from-final-ir-contract
```

Current black-hat/test-review gaps that are not optional phase polish require dedicated beads before final acceptance:

```text
generated-interpreter-suspension-error-parity
generated-full-final-ir-equivalence
compiler-full-v1-primitive-source-lowering
runtime-collect-next-pagination-state
runtime-admission-run-header-persistence
runtime-journal-sequence-hydration
runtime-full-live-frame-recovery-hydration
unsafe-fuzz-cabi-isolation
workspace-exact-assertion-sharpness
rust-test-loop-removal
silent-discard-elimination
test-plan-current-api-mutation-refresh
full-gate-evidence-refresh
```

The previous `error-variant-completeness-audit` gap has Round 2 implementation evidence in `tests/error_variant_completeness_test.rs` and `docs/error-variant-completeness.md`; it remains subject to the full gate matrix like every other test surface.

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
speed claim without real benchmark baseline/result evidence
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
22. Every speed claim is backed by real benchmark evidence with p50/p95/p99, instruction counts, allocation counts, bytes allocated, latency, durability mode, and fixture metadata; compileable scaffold placeholders do not count.
23. Full gates pass: fmt, clippy hard denies, tests, nextest, trybuild, Miri, coverage, fuzz smoke, mutants smoke, supply chain, geiger, feature powerset, docs, and benchmark build.
24. Maxperf, PGO, and `target-cpu=native` workflows are documented, executable, and measured.
25. Sanitizer nightly jobs pass for binary decoders, IPC, storage, runtime, and generated workflows.
26. Every phase parent bead, function-cluster child bead, fuzz target bead, benchmark bead, and P0 blocker bead is closed with evidence.
27. Mechanical gates can accept AI changes without human guesswork only when the relevant executable checks, tests, benchmarks, and bead evidence have actually run and passed; represented tasks/probes alone are not acceptance evidence.

---

## 45. Normative Runtime Semantics

Every `CompiledNodeKind` variant has exact behavior defined here. Two implementations (IR interpreter, generated Rust) must match on: terminal result, typed error variant and fields, final pc, slot values, slot taints, step states, journal event sequence, action tickets, retry counts, wait/ask scheduling, and replay behavior.

### StepState Transition Contract

Valid transitions:

```text
Pending    → Running, Succeeded, Failed, Cancelled, Skipped
Running    → Succeeded, Failed, Waiting, Asking, Cancelled, Skipped
Waiting    → Running
Asking     → Running
Succeeded  → (terminal)
Failed     → (terminal)
Cancelled  → (terminal)
Skipped    → (terminal)
```

Idempotent re-mark (`state == next`) is valid. All other transitions return `InternalInvariantViolation { reason: "invalid_state_transition" }`.

### EngineSignal / RuntimeSignal

Core engine signals: `Continue`, `Finished(SlotValue)`, `StepBudgetExhausted`, `AwaitingAction`, `AwaitingWait`, `AwaitingAsk`.

Runtime engine extends `AwaitingAction` to carry `ActionTicket { run, step, seq, action, attempt, idempotency_key }`.

### Node Semantics

#### Nop

| Aspect | Behavior |
|--------|----------|
| Inputs read | None |
| Slots written | None |
| Taint | None |
| StepState | Pending → Running → Succeeded |
| Journal | None |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed |
| Errors | `MissingNextStep` if `node.next` is None |

#### SetConst { value: ConstIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Constant pool at `value` index |
| Slots written | `node.output` with `Taint::Clean` |
| Taint | Always Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed, const pool bounds |
| Errors | `ConstOutOfBounds`, `MissingOutputSlot`, `MissingNextStep` |

#### Copy { source: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `source` slot value and taint |
| Slots written | `node.output` with source taint |
| Taint | Propagated from source |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed |
| Errors | `SlotOutOfBounds` (covers both out-of-bounds and uninitialized), `MissingOutputSlot`, `MissingNextStep` |

#### EvalExpr { expr: ExprIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Expression program at `expr` index; expression may read arbitrary slots, constants, and accessors |
| Slots written | `node.output` with `Taint::Clean` |
| Taint | Always Clean (no taint join of expression operands) |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed, expression stack depth ≤ 64, expression ops ≤ 256 |
| Errors | `ExprOutOfBounds`, `MissingOutputSlot`, `MissingNextStep`, any `ExprError` (stack overflow, type mismatch, division by zero, integer overflow) |

#### BuildObject { fields: Box<[(SymbolId, SlotIdx)]> }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Each field's slot value |
| Slots written | `node.output` with `SlotValue::Object(ObjectId)`, taint `Clean` |
| Taint | Always Clean (no join of field taints) |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed, object field count bounds |
| Errors | `SlotOutOfBounds` (any field slot), `MissingOutputSlot`, `MissingNextStep` |
| Side effects | Allocates `ObjectId` in `ValueStore` |

#### BuildList { items: Box<[SlotIdx]> }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Each item's slot value |
| Slots written | `node.output` with `SlotValue::List(ListId)`, taint `Clean` |
| Taint | Always Clean (no join of item taints) |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed, list item count bounds |
| Errors | `SlotOutOfBounds` (any item slot), `MissingOutputSlot`, `MissingNextStep` |
| Side effects | Allocates `ListId` in `ValueStore` |

#### Jump { target: StepIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | None |
| Slots written | None |
| Taint | None |
| StepState | Pending → Running → Succeeded |
| Journal | None |
| Suspension | Never |
| Next pc | `target` |
| Resource checks | Budget consumed |
| Errors | None at this node (invalid target caught at next step's `InvalidProgramCounter`) |

#### Finish { result: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `result` slot value |
| Slots written | None (result is returned as signal, not written) |
| Taint | Result taint passed through (no rejection of Secret/DerivedFromSecret) |
| StepState | Pending → Running → Succeeded |
| Journal | RunFinished |
| Suspension | Terminal — run completes |
| Next pc | No change |
| Resource checks | Budget consumed |
| Errors | `SlotOutOfBounds` (result slot uninitialized) |

#### Do { action: ActionId, input: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `input` slot taint (runtime engine also reads slot value for ticket) |
| Slots written | None at suspension time; output written on action completion |
| Taint | Runtime engine checks: `DeterministicPure` rejects non-Clean input. Output taint via `propagate_action_taint()`. `AtLeastOnceExternal` upgrades Secret to DerivedFromSecret. |
| StepState | Pending → Running (stays Running while awaiting action) |
| Journal | ActionScheduled at suspension; ActionCompleted on resume |
| Suspension | Returns `AwaitingAction(ActionTicket)` |
| Next pc | No change at suspension; set to `node.next` on action completion |
| Resource checks | Budget consumed, contract resolution, action ID validation |
| Errors | `UnknownAction` (if contracts non-empty and action not found), `TaintViolation` (DeterministicPure with tainted input) |
| Resume | Action completion writes output slot with value + taint, marks step Succeeded, advances pc |

#### WaitUntil { deadline_slot: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `deadline_slot` (must be I64 or F64) |
| Slots written | None |
| Taint | None |
| StepState | Pending → Running → Waiting |
| Journal | WaitScheduled |
| Suspension | Returns `AwaitingWait` |
| Next pc | No change |
| Resource checks | Budget consumed |
| Errors | `TypeMismatch { expected: "deadline" }` if slot is not numeric |
| Resume | Timer fire marks step Succeeded, sets pc to `node.next` |

#### WaitEvent { event: SlotIdx, timeout_slot: Option<SlotIdx> }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `event` slot (numeric), optional `timeout_slot` (numeric) |
| Slots written | None |
| Taint | None |
| StepState | Pending → Running → Waiting |
| Journal | WaitScheduled |
| Suspension | Returns `AwaitingWait` |
| Next pc | No change |
| Resource checks | Budget consumed |
| Errors | `TypeMismatch` if event or timeout is not numeric |
| Resume | Timer fire marks step Succeeded, sets pc to `node.next` |

#### Ask { prompt: SlotIdx, timeout_slot: Option<SlotIdx> }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `prompt` slot (must be Symbol), optional `timeout_slot` (numeric) |
| Slots written | None at suspension; answer written on AskResume |
| Taint | None at suspension |
| StepState | Pending → Running → Asking |
| Journal | AskScheduled |
| Suspension | Returns `AwaitingAsk` |
| Next pc | No change |
| Resource checks | Budget consumed |
| Errors | `TypeMismatch { expected: "prompt" }` if prompt is not Symbol |
| Resume | AskResume writes answer slot, marks step Succeeded, sets pc to `AskResume.next` |

#### AskResume { answer: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `answer` slot value |
| Slots written | `node.output` with answer value (if output is Some) |
| Taint | Clean (write_slot, not write_slot_with_taint) |
| StepState | Running → Succeeded |
| Journal | SlotWritten, AskAnswered |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed |
| Errors | `SlotOutOfBounds`, `MissingNextStep` |

#### Choose { branches: Box<[ExprBranch]>, otherwise: Option<StepIdx> }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Each branch's `ExprIdx` expression evaluated in order |
| Slots written | None (branches redirect control flow) |
| Taint | No taint enforcement on branch conditions |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten for any internal expression evaluations |
| Suspension | Never |
| Next pc | First branch with `Bool(true)` → branch target. None match → `otherwise`. |
| Resource checks | Budget consumed |
| Errors | `TypeMismatch` if expression result is not Bool, `MissingNextStep` if no match and no `otherwise` |

#### ChooseSlot { branches: Box<[SlotBranch]>, otherwise: Option<StepIdx> }

Same as Choose but reads pre-materialized boolean slots instead of evaluating expressions.

#### ForEachStart { input, item_slot, limit, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `input` slot (must be List) |
| Slots written | `item_slot` with first item; `output` with tail list. If empty: `output` with empty list. |
| Taint | Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Empty → `done`. Non-empty → `body`. |
| Resource checks | `item_count <= limit`, else `IterationLimitExceeded` |
| Errors | `TypeMismatch` if input is not List, `MissingOutputSlot`, iteration limit |

#### ForEachNext { iterator_slot, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `iterator_slot` (must be List) |
| Slots written | `output` with first item; `iterator_slot` with tail list |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Empty → `done`. Non-empty → `body`. |
| Errors | `TypeMismatch` if iterator is not List |

#### ForEachJoin { output: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `materialized` slot (must be List) |
| Slots written | `output` with list value |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Errors | `MissingOutputSlot`, `MissingNextStep`, `TypeMismatch` |

#### TogetherStart { branches, join }

| Aspect | Behavior |
|--------|----------|
| Inputs read | None |
| Slots written | `output` with empty list |
| Taint | Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | First branch entry |
| Resource checks | Branch count ≤ u16, branches non-empty |
| Errors | `TogetherBranchLimitExceeded`, `InvalidCompiledWorkflow` if branches empty, `MissingOutputSlot` |

#### TogetherBranch { branch, entry, join, accumulator }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `accumulator` slot (must be List); `output` for previous result |
| Slots written | `accumulator` with appended result |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `entry` (branch 0) or `entry` for subsequent branches |
| Errors | `TypeMismatch` if accumulator is not List |

#### TogetherJoin { branch_count, accumulator }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `accumulator` slot (List), `output` for last result |
| Slots written | `output` with final merged list |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Errors | `MissingOutputSlot`, `MissingNextStep` |

#### CollectStart { source, limit, page_size, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `source` slot (must be List) |
| Slots written | `collector_slot` with first page; pagination state in global table |
| Taint | Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Empty → `done`. Non-empty → `body`. |
| Resource checks | `item_count <= limit`, `page_size > 0`, `page_size <= limit` |
| Errors | `TypeMismatch`, `CollectItemLimitExceeded`, `CollectPageLimitExceeded`, `InvalidCompiledWorkflow` |

#### CollectPage { collector_slot, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `collector_slot` validation (must be List) |
| Slots written | None |
| StepState | Running → Succeeded |
| Suspension | Never |
| Next pc | `body` |
| Errors | `TypeMismatch` if collector is not List |

#### CollectNext { collector_slot, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `collector_slot` (List), pagination state |
| Slots written | `collector_slot` with next page |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Cursor exhausted → `done`. Otherwise → `body`. |
| Errors | `TypeMismatch`, `InvalidCompiledWorkflow` if pagination state missing |

#### CollectFinish { collector_slot }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `collector_slot` (List) |
| Slots written | `output` with collector value; pagination state removed |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Errors | `MissingOutputSlot`, `MissingNextStep` |

#### ReduceStart { input, accumulator, initial, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Constant pool at `initial`; `input` slot (must be List) |
| Slots written | `accumulator` with initial value; `output` with first item; `input` with tail list |
| Taint | Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Empty → `done`. Non-empty → `body`. |
| Errors | `ConstOutOfBounds`, `TypeMismatch`, `MissingOutputSlot` |

#### ReduceNext { iterator_slot, accumulator, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `iterator_slot` (must be List) |
| Slots written | `output` with next item; `iterator_slot` with tail |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Empty → `done`. Non-empty → `body`. |
| Errors | `TypeMismatch` |

#### ReduceFinish { accumulator }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `accumulator` slot |
| Slots written | `output` with accumulator value |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Errors | `MissingOutputSlot`, `MissingNextStep` |

#### RepeatStart { max_attempts, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | None |
| Slots written | `output` with packed I64 `(max_attempts << 32) | 0` |
| Taint | Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `body` |
| Resource checks | `max_attempts > 0` |
| Errors | `MissingOutputSlot`, `InternalInvariantViolation` if max_attempts is 0 |

#### RepeatAttempt { attempt_slot, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `attempt_slot` (must be I64 packed state) |
| Slots written | None |
| StepState | Running → Succeeded |
| Suspension | Never |
| Next pc | `body` |
| Errors | `TypeMismatch`, `InternalInvariantViolation` if packed state invalid |

#### RepeatCheck { attempt_slot, body, exhausted }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `attempt_slot` (I64 packed state) |
| Slots written | `attempt_slot` with incremented attempt count |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Attempts exhausted → `exhausted`. Otherwise → `body`. |
| Errors | `MissingNextStep` if attempts remain but next is None |

#### RepeatFinish { result }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `result` slot |
| Slots written | `output` with result value |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Errors | `MissingOutputSlot`, `MissingNextStep` |

#### RetryCheck { policy_slot, body, exhausted }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Retry policy (attempts remaining) |
| Slots written | None |
| StepState | Running → Succeeded |
| Suspension | Never |
| Next pc | Attempts remain → `body`. Exhausted → `exhausted`. |
| Errors | None |

#### ErrorHandler { body, handler }

| Aspect | Behavior |
|--------|----------|
| Inputs read | None |
| Slots written | None |
| StepState | Failed step stays Failed; handler step becomes Running → Succeeded |
| Journal | StepFailed on the failed step |
| Suspension | Never |
| Next pc | `handler` |
| Errors | None at this node |

---

## 46. Expression Grammar, Type System, and Helper Signatures

### Precedence Table

Highest to lowest. All binary operators are left-associative.

| Binding power (left/right) | Operators |
|---------------------------|-----------|
| 11 / 12 | Unary `not`, unary `-` (prefix) |
| 11 / 12 | `*`, `/` |
| 9 / 10 | `+`, `-` |
| 7 / 8 | `>`, `>=`, `<`, `<=` |
| 5 / 6 | `==`, `!=` |
| 3 / 4 | `and` |
| 1 / 2 | `or` |

Parenthesized groups reset to minimum binding power. Max nesting depth: 64. Max helper args: 8. Max tokens: 256. Max source bytes: 4096. Stack depth: 64 (`ArrayVec<SlotValue, 64>`).

### Type Rules

| Operator | Accepted types | Return type | Error on mismatch |
|----------|---------------|-------------|-------------------|
| `+`, `-`, `*`, `/` | I64, I64 | I64 | `TypeMismatch { expected: "number" }`. Overflow → `IntegerOverflow`. Div-by-zero → `DivisionByZero`. |
| `>`, `>=`, `<`, `<=` | I64, I64 | Bool | `TypeMismatch { expected: "number" }` |
| `==`, `!=` | Any, Any | Bool | Never (accepts any `SlotValue` pair via `PartialEq`) |
| `and`, `or` | Bool, Bool | Bool | `TypeMismatch { expected: "boolean" }`. **No short-circuit** — both operands evaluated before operator applies. |
| `not` | Bool | Bool | `TypeMismatch { expected: "boolean" }` |
| `-` (unary) | I64 | I64 | `TypeMismatch { expected: "number" }`. `i64::MIN` → `IntegerOverflow`. |

### Null Comparison Rules

`Null == Null` → `true`. `Null == <anything_else>` → `false`. Equality uses `SlotValue::PartialEq` which is derived. `I64(0) != F64(FiniteF64(0.0))` — different types are never equal.

### F64 Status

`ExprType::F64`, `SlotValue::F64(FiniteF64)`, and `ConstValue::F64` exist in the type system and constant pool. The typechecker accepts F64 in arithmetic and coercion. However, the expression pipeline has no float literal syntax, no `F64` variant in `ExprLiteral`, no `F64` arm in `literal_to_const`, and no F64 arithmetic in the evaluator. F64 values can only enter through runtime slot initialization or action outputs. The typechecker is more permissive than the evaluator — expressions that typecheck as F64 may fail at eval time.

### Helper Signatures

| Helper | Arity | Input types | Return | Implementation status |
|--------|-------|-------------|--------|-----------------------|
| `exists` | 1 | Any | Bool | Implemented: `!matches!(value, Null)` |
| `length` | 1 | List or Null | I64 | Implemented: list item count, 0 for Null |
| `count` | 1 | List or Null | I64 | Implemented: alias for `length` |
| `empty` | 1 | List or Null | Bool | **Bug**: returns `true` for all lists regardless of length |
| `unique` | 1 | List | List | **Bug**: no-op, returns input unchanged |
| `contains` | 2 | List, T | Bool | Unimplemented: returns `UnknownHelper` at eval |
| `starts_with` | 2 | Symbol, Symbol | Bool | Unimplemented |
| `ends_with` | 2 | Symbol, Symbol | Bool | Unimplemented |
| `has` | 2 | Object, Symbol | Bool | Unimplemented |
| `append` | 2 | List, T | List | Unimplemented |
| `append_if` | 3 | List, T, Bool | List | Unimplemented |
| `merge` | 2 | Object, Object | Object | Unimplemented. Typechecker returns `List` (bug). |
| `sum` | 1 | List | I64 | Unimplemented. Spec says 2 args (list, field). Code defines arity 1. |

### Short-Circuit Policy

`and` and `or` do **not** short-circuit. Both operands are popped from the expression stack and evaluated before the boolean operator applies. A type error in the second operand fires even when the first operand determines the result. The bytecode compiler emits both sub-expression bytecodes before the operator bytecode, so no bytecode-level short-circuit is possible either.

---

## 47. Taint Lattice and Propagation Rules

### Lattice Ordering

```text
Clean < DerivedFromSecret < Secret
```

`join_taint` returns the input unchanged. The lattice ordering is enforced by the propagation rules below: Secret never downgrades to Clean, and Clean never upgrades without explicit action.

### Propagation by Operation

| Operation | Taint behavior |
|-----------|---------------|
| `SetConst` | Always `Clean` — constants are compile-time values with no secret origin |
| `Copy` | Preserves source taint — `write_slot_with_taint(output, value, source_taint)` |
| `EvalExpr` | Always `Clean` — `write_slot` (not `write_slot_with_taint`). No taint join of expression operands. |
| `BuildObject` | Always `Clean` — no join of field taints |
| `BuildList` | Always `Clean` — no join of item taints |
| `Do` (DeterministicPure) | Output ≥ input. `TaintViolation` if input is not Clean. Clean input → Clean output. |
| `Do` (IdempotentExternal) | Same propagation as DeterministicPure |
| `Do` (AtLeastOnceExternal) | Secret input → `DerivedFromSecret` output. `DerivedFromSecret` input → `DerivedFromSecret`. Clean input → Clean. |
| `Choose` / `ChooseSlot` | No taint tracking on branch conditions |
| `Finish` | Result taint passed through. No rejection of Secret or DerivedFromSecret results. |

### Control-Flow Taint

v1 does **not** track control-flow taint. A secret value can choose which public value is returned without triggering a taint violation. Example:

```yaml
choose:
  - if: "$secrets.token == 'x'"
    then: return_a
  - otherwise: return_b
```

Both `return_a` and `return_b` are Clean regardless of `$secrets.token` taint. This is an explicit v1 design decision. If control-flow taint is needed, it must be added as a v2 feature with a dedicated bead and evidence.

### Secret Storage

Secrets are referenced by `SymbolId` at runtime. The runtime never holds raw secret values — only taint markers. Secret values are resolved at compile time and stored as taint flags on the corresponding input slots.

---

## 48. Value Arena, Handle Lifetime, and Blob Contract

### Arena Types

| Arena | Storage type | Deduplication | Growth |
|-------|-------------|---------------|--------|
| Symbol | `Vec<Box<str>>` | None — same string yields different `SymbolId` on each insert | Append-only |
| List | `Vec<Box<[SlotValue]>>` | None | Append-only |
| Object | `Vec<(Box<[(SymbolId, SlotValue)]>, IndexMap<SymbolId, SlotValue>)>` | Duplicate keys: later value wins | Append-only |
| Blob | `Vec<Box<[u8]>>` | None | Append-only |

### Handle Validity

A handle (`SymbolId`, `ListId`, `ObjectId`, `BlobId`) is valid if `id.as_usize() < arena.len()`. No generational indices. Handles are `Copy`. Handle validity lasts for the lifetime of the `ValueStore` — handles are not valid across different `ValueStore` instances.

### Object Field Lookup

Objects use a dual representation: primary `Box<[(SymbolId, SlotValue)]>` for serialization order, secondary `IndexMap<SymbolId, SlotValue>` for O(1) field lookup. Field order is insertion order.

### Blob Size vs Envelope

`ResourceContract.max_blob_bytes` is `u64` (default 16 MiB). Envelope `payload_len_u32` is `u32` (max ~4 GiB). **v1 design decision**: logical blobs are capped at `u32::MAX` bytes. No blob chunking in v1. A blob exceeding `u32::MAX` is rejected at admission.

### No GC in v1

Blobs, symbols, lists, and objects are write-once, read-many. No deletion, TTL, or garbage collection. Long-running servers must manage storage externally or restart.

---

## 49. Journal Event Payload Schemas and Crash-Consistency Ordering

### TraceEvent Variants (hot ring)

```text
StepStarted   { run: RunId, step: StepIdx }
StepEnded     { run: RunId, step: StepIdx }
SlotWritten   { run: RunId, slot: SlotIdx }
ActionScheduled { run: RunId, step: StepIdx }
ActionCompleted { run: RunId, step: StepIdx }
ActionFailed  { run: RunId, step: StepIdx, code: ActionFailureCode }
AskAnswered   { run: RunId, step: StepIdx, slot: SlotIdx }
RunSubmitted  { run: RunId }
RunFinished   { run: RunId }
RunFailed     { run: RunId }
RunCancelled  { run: RunId }
```

### Runtime Journal Events (durable)

```text
RunSubmitted     { run: RunId, workflow: WorkflowDigest }
SlotWritten      { run: RunId, slot: SlotIdx }
ActionScheduled  { run: RunId, step: StepIdx, action: ActionId }
ActionCompleted  { run: RunId, step: StepIdx, action: ActionId }
WaitScheduled    { run: RunId, step: StepIdx }
AskScheduled     { run: RunId, step: StepIdx }
WaitResolved     { run: RunId, step: StepIdx }
AskAnswered      { run: RunId, step: StepIdx, slot: SlotIdx }
RunFinished      { run: RunId, result: SlotIdx }
RunFailed        { run: RunId }
RunCancelled     { run: RunId }
```

### Ordering Invariants

1. `RunSubmitted` before any `StepStarted` or `SlotWritten`.
2. `StepStarted` before `SlotWritten` for that step.
3. `ActionScheduled` before external action dispatch.
4. `ActionCompleted` before frame mutation on resume.
5. `RunFinished` after final result slot is persisted.
6. Timer resume: step marked `Running` then `Succeeded` before continuing drive loop.

### Crash-Consistency Rule

External side effects must not be dispatched until `ActionScheduled` is durably recorded under strict durability. For journaled mode, dispatch may occur after queue admission.

### Trace Ring

SPSC ring via `rtrb::RingBuffer`. On full, events are dropped and `dropped` counter incremented. `history: VecDeque` stores all successfully pushed events for snapshot queries. `drain_for_run` consumes non-matching events silently.

---

## 50. IPC Transport, Backpressure, and Error Codes

### Transport

- Socket type: Unix stream socket.
- Max concurrent clients: 256.
- Read chunk: 4096 bytes.
- Backpressure: Bounded command queue (`ArrayQueue`). Queue full → `IpcError::Full` (E3001).
- Per-connection: Non-blocking writes with writable-event polling via `mio`.
- Pipelining: Not supported in v1 — one command per connection, response before next command.
- Shutdown: `Shutdown` acknowledged. Pending runs are not forcibly cancelled.

### IpcResponse Variants

```text
AcceptedRun { run_id: u64 }
Healthy
ShuttingDown
TraceCount { count: u32 }
Events { events: Vec<IpcTraceEvent> }
Inspected { run_id: u64 }
BadRequest
PayloadError { diagnostic: u16, message: String }
CommandPayloadMismatch
WorkflowResolutionRequired
WorkflowResolutionUnsupported
WorkflowDigestMismatch
CountOutOfRange { actual: usize, limit: u32 }
FrameError { message: String }
RuntimeError { message: String }
```

### IpcError Variants with Diagnostic Codes

```text
E3001  Full
E3002  Disconnected
E3003  PayloadTooLarge { actual, limit }
E3004  InvalidMagic { actual }
E3005  UnsupportedVersion { actual }
E3006  UnknownCommand(u16)
E3007  ReservedNonZero { actual }
E3008  PayloadLengthMismatch { header, actual }
E3009  HeaderEncodeFailed
E300A  HeaderDecodeFailed
E300B  PayloadLengthOutOfRange { actual }
E300C  PayloadEncodeFailed
E300D  PayloadDecodeFailed
E300E  ResponseDecodeFailed
```

---

## 51. Digest Canonicalization and Schema Versioning

### Required Digests

```text
workflow_source_digest = BLAKE3(raw source bytes)
compiled_digest       = BLAKE3(canonical compiled artifact payload)
action_abi_digest     = BLAKE3(canonical sorted action contracts)
policy_digest         = BLAKE3(canonical resource/durability/runtime policy)
payload_digest        = BLAKE3(postcard payload bytes)
```

### Canonical Ordering for Stable Digests

- Symbol IDs in definition order (index-based).
- Constant pool in index order.
- Accessor table in index order.
- Compiled nodes in `StepIdx` order.
- Object fields in insertion order (no sorting).
- Action contracts sorted by `ActionId`.
- `ResourceContract` fields in struct field order, encoded via Postcard.

### Libraries

`blake3 = "1"` and `crc32c = "0.6"` are required workspace dependencies for envelope digests and header checksums.

---

## 52. Fallible Allocation and No-Panic Enforcement

### OOM Policy

- Admission-time allocations must use fallible reservation paths where available.
- Hot runtime code must not call allocation APIs that can grow implicitly.
- OOM during admission returns `CoreError::AllocationFailed`.
- OOM after run admission is a bug and must be prevented by reservation in turbo mode.
- `vec![StepState::Pending; states_len]` in `RunFrame::new` can panic on OOM — acceptable for cold-path construction only if the frame is preallocated in turbo mode.

### `FiniteF64` Deserialization

Derived `Deserialize` for `FiniteF64` must reject NaN, `+inf`, and `-inf` in release mode. If the derive permits non-finite values through Postcard decode, a custom `Deserialize` impl is required that calls `FiniteF64::new` and maps failure to a typed deserialization error.

---

## 53. Hot/Cold Module Classification

### Hot Path Modules

No allocation after admission, no formatting, no maps, no string operations:

- `vb_core::engine`
- `vb_core::frame`
- `vb_runtime::engine`
- `vb_runtime::shard` (tick loop only)
- `vb_runtime::frame_pool`
- `vb_runtime::primitives::*`
- `vb_ipc` decoder after header validation
- Generated workflow code

### Cold Path Modules

Maps, formatting, and allocation allowed:

- `vb_yaml`
- `vb_validate`
- `vb_compile` (except final IR validation helpers used by runtime)
- `vb_runtime::action` (ActionRegistry, validation)
- `vb_runtime::trace` (event rendering)
- `vb_storage::recovery`
- Diagnostics
- CLI
- Test and bench harnesses

### Scanner Policy

The banned-token scanner (Section 12) must be path-aware. `format!` is forbidden in hot modules but allowed in cold modules. `HashMap` is forbidden in hot modules but allowed in cold modules.

---

## 54. Single-Server Ownership and Database Locking

- One active runtime process may own a database path at a time.
- Startup must acquire an exclusive process lock (e.g., `flock`).
- If the lock is already held, startup fails with a typed error.
- No distributed coordination, leader election, replication, or multi-writer mode in v1.
- Many IPC clients may connect to one server. One server owns the runtime and Fjall database.

---

## 55. Action Worker Model and Shard Non-Blocking Contract

- `DeterministicPure` actions may execute inline only if bounded and non-blocking.
- External actions must not block the shard loop.
- External action dispatch uses explicit `Suspended` ticket path.
- No per-action thread spawning unless through a bounded worker pool.
- Worker pool size is configured and bounded.
- Queue full returns `ActionError::QueueFull`.
- Current implementation: `execute_do_without_contract` always creates an `ActionTicket` and returns `AwaitingAction`. The shard suspends the run. External completion arrives via `ShardCommand::ActionCompleted`.

---

## 56. Runtime Profile Defaults

### ShardConfig Defaults

```text
command_queue_capacity: 1024
trace_capacity: 4096
step_budget_per_tick: 1000
max_active_runs: 1024
```

### ResourceContract::DEFAULT

```text
max_steps: 1_000
max_slots: 65_535 (u16::MAX)
max_constants: 65_535
max_accessors: 8_192
max_expressions: 4_096
max_expr_stack: 64
max_step_budget_per_tick: u64::MAX
max_input_bytes: 1 MiB
max_output_bytes: 1 MiB
max_blob_bytes: 16 MiB
max_ipc_payload_bytes: 1 MiB
max_retry_attempts: 65_535
max_fanout: 65_535
max_collect_items: 4_294_967_295 (u32::MAX)
max_queue_depth: 1_024
max_journal_batch_bytes: 1 MiB
```

### Named Profiles

| Profile | Persistence | Allocation | Code path |
|---------|-------------|------------|-----------|
| `dev` | Volatile | On-demand | IR interpreter |
| `test` | Volatile + deterministic tracing | On-demand | IR interpreter |
| `turbo` | Journaled | Preallocated frames, bounded queues | IR interpreter |
| `maxperf` | Strict | All preallocated | Generated Rust |

---

## 57. Feature Flag Policy

- Default features: none (all code always compiled).
- `generated` feature: enables generated workflow compilation support (codegen crate).
- `bench` feature: enables benchmark-only harness code.
- `volatile` feature: enables volatile storage mode (test-only).
- Forbidden features: `json`, `http` in v1 runtime crates.
- `maxperf` is a profile (Section 34), not a feature.

---

## 58. Platform Support

v1 supported target: `x86_64-unknown-linux-gnu`. Unix domain sockets required. Other targets are non-release experimental unless a dedicated portability bead adds evidence.

---

## 59. Security and Threat Model

### Trusted Components

Compiled IR, Fjall database, runtime engine, generated Rust code.

### Untrusted Inputs

Workflow YAML source, IPC client payloads, action outputs, persisted bytes during recovery.

### Threat Model

| Threat | Mitigation |
|--------|-----------|
| Malformed YAML | Strict parser, typed validation errors |
| Malformed IPC frames | Magic validation, length bounds, typed IPC errors, fuzz coverage |
| Oversized payloads | Bounded frames, bounded queues, typed `PayloadTooLarge` |
| Non-idempotent replay | `ActionReplayTracker` blocks re-execution, `Idempotency` policy |
| Digest tampering | BLAKE3 digests on source, IR, blobs. Mismatch → typed error, no silent continue |
| Secret leak via diagnostics | Taint tracking on action outputs. No raw secret values in hot state |
| Local privilege escalation | Unix socket permissions. No authentication in v1 |
| DoS via resource exhaustion | Bounded queues, bounded retries, bounded expression stacks, bounded trace rings |
| Storage corruption | Fjall WAL replay, snapshot recovery, typed storage errors |

---

## 60. Evidence Artifact Format

A bead is not closable without an evidence artifact:

```toml
# .evidence/<bead-id>.toml
bead = "runtime-engine-setconst"
phase = 13
git_commit = "abc123..."
rustc = "nightly-2026-04-28"

[[commands]]
command = "cargo +nightly fmt --all -- --check"
exit = 0
log = "logs/fmt.txt"

[[commands]]
command = "cargo +nightly nextest run -p vb_core"
exit = 0
log = "logs/nextest-vb-core.txt"

[[benchmarks]]
name = "transition_set"
before = "1234ns"
after = "987ns"
file = "bench/transition_set.json"
```

---

## 61. Fjall Storage Contract

### Keyspace Profiles

| Profile | Keyspaces | Tuning |
|---------|-----------|--------|
| `Hot` | run_event, index_status, index_workflow, index_action, run_header | Bloom filter (10 bits/key), no KV separation |
| `Cold` | workflow_source, compiled_ir, run_snapshot | KV separation at 4096-byte threshold |
| `Blob` | blob | KV separation at 1024-byte threshold |

### Key Format

All keys use prefix byte + big-endian numeric IDs. Fjall keys remain big-endian for lexicographic ordering. Record body envelopes are little-endian. String keys are forbidden on hot paths.

### Write Path

Writes use `Mutex<()>` write lock for ordering. Durability profiles:
- `Volatile`: no Fjall writes during run (test/bench only).
- `Journaled`: bounded group commit via `JournalWriterQueue`.
- `Strict`: synchronous `persist(PersistMode::SyncAll)` after write.

### Recovery

- Full journal replay when no snapshot exists.
- Snapshot + tail journal replay when snapshot exists.
- `ActionReplayTracker` prevents non-idempotent re-execution during recovery.
- Recovery never reparses YAML — loads by digest.

### Atomic Cross-Keyspace Writes

`OwnedWriteBatch` provides single-WAL-fsync atomicity for multi-keyspace writes. Recommended for event + index co-writes. Current implementation uses individual inserts with write lock.

### Single-Writer Enforcement

Fjall v3 acquires an exclusive file lock per database. Only one process may open a database path at a time. Second process receives a typed error on open.

---

## 62. No-Async Rule

v1 runtime core must not depend on `tokio`, `async-std`, `smol`, `futures` executors, `async_trait`, or async task scheduling. `mio` is the only approved low-level eventing mechanism for IPC. Actions may block only in bounded action worker contexts or return `Suspended`. No async function may appear in `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, or generated workflow code.

---

## 63. Plan Verifier and Accepted Artifacts

### Core Principle

AI may propose workflows. Velvet verifies them. Only accepted artifacts run.

The compiler does not merely check syntax. It acts as a safety gate: if Velvet cannot prove the plan is bounded, inspectable, retry-safe, and durable, the plan is rejected before execution. No accepted workflow has unknown bounds.

### Verification Gate Pipeline

```text
YAML/Rust workflow definition
  → strict YAML parser (gate 1: profile)
  → schema validator (gate 2: shape)
  → name/scope validator (gate 3: names)
  → reference validator (gate 4: references)
  → expression compiler (gate 5: expressions)
  → control-flow validator (gate 6: CFG)
  → boundedness analyzer (gate 7: bounded — section 64)
  → resource budget checker (gate 8: budgets)
  → action contract verifier (gate 9: contracts)
  → taint/secret checker (gate 10: taint)
  → idempotency verifier (gate 11: idempotency — section 65)
  → durability checker (gate 12: durability)
  → capability checker (gate 13: capabilities)
  → result/output validator (gate 14: results)
  → observability checker (gate 15: evidence)
  → accepted artifact
  → runtime admission (section 66)
```

A workflow must pass every gate to produce an accepted artifact. The runtime must not execute anything that is not an accepted artifact.

### Accepted Artifact Record

When a workflow passes all verification gates, the compiler persists a verifiable artifact:

```rust
pub struct AcceptedArtifact {
    pub artifact_version: &'static str,  // "velvet.artifact/v1"
    pub workflow_name: Box<str>,
    pub workflow_version: &'static str,  // "velvet-ballastics/v1"
    pub workflow_digest: WorkflowDigest, // BLAKE3 of YAML source
    pub ir_digest: WorkflowDigest,       // BLAKE3 of compiled IR
    pub action_contract_digest: WorkflowDigest, // BLAKE3 of action contracts
    pub verified_at: u64,                // Unix timestamp
    pub resource_budget: WholeWorkflowBudget, // section 64
    pub capabilities: Box<[Capability]>, // section 66
    pub warnings: Box<[VerificationWarning]>,
    pub verification: VerificationProof,
}

pub struct VerificationProof {
    pub bounded: bool,
    pub taint_safe: bool,
    pub retry_safe: bool,
    pub durable: bool,
    pub replayable: bool,
    pub idempotency_proven: Vec<ActionId>,  // actions with proven idempotency
    pub idempotency_attested: Vec<ActionId>, // actions with attested (not proven) idempotency
}

pub struct VerificationWarning {
    pub code: u32,
    pub message: Box<str>,
    pub gate: u8, // which verification gate produced it
}
```

Runs bind to this artifact by digest, not to loose YAML or unverified `CompiledWorkflow`.

### Accepted Artifact Persistence

Accepted artifacts are stored in the `compiled_ir` keyspace keyed by `ir_digest`. The storage layer already stores compiled IR by digest; the artifact record wraps the IR with verification metadata.

### Strict Verification Mode

For AI-authored workflows, strict mode is available:

```text
velvet validate flow.yaml --strict --json
```

Strict mode rejects not only errors but selected warnings:

- unused secrets
- unsafe shell actions
- large fanout (branches > policy threshold)
- missing examples
- retry on side-effecting action without idempotency proof
- possibly skipped references
- opaque object where schema could be declared

This is the workflow equivalent of compile-with-warnings-as-errors. AI agents should use `--strict` as the default.

### Verification Gate Status

| Gate | Status | Notes |
|------|--------|-------|
| 1. YAML profile | Implemented | vb_yaml strict profile, 19 error variants |
| 2. Shape/schema | Implemented | vb_validate + vb_compile schema validation |
| 3. Name/scope | Implemented | ID grammar, reserved words enforcement |
| 4. Reference | Implemented | Forward refs rejected, runtime refs rejected |
| 5. Expression | Implemented | 30 opcodes, bytecode compiler, bounded stacks |
| 6. Control flow | Implemented | Forward-only CFG, cycle rejection, reachability |
| 7. Boundedness | Partial | Individual loop bounds exist; whole-workflow budget needed (section 64) |
| 8. Resource budget | Partial | ResourceContract with 16 fields; whole-workflow computation needed |
| 9. Action contract | Partial | Classification exists; compile-time schema validation needed |
| 10. Secret/taint | Implemented | Compile-time + runtime taint, leak rejection, 3-level lattice |
| 11. Idempotency | Stub | Enum exists; verification gate needed (section 65) |
| 12. Durability | Partial | Journal events exist; slot/payload persistence gaps |
| 13. Capability | Not started | No capability model exists |
| 14. Output/result | Implemented | Result validation, finish semantics |
| 15. Observability | Partial | Trace ring + counters; evidence chain gaps |

---

## 64. Whole-Workflow Boundedness Analysis

### Principle

No accepted workflow has unknown bounds. The compiler must compute a conservative whole-workflow budget before accepting any artifact.

### Required Analysis

The boundedness analyzer performs static dataflow analysis on the compiled IR to compute:

```rust
pub struct WholeWorkflowBudget {
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
}
```

### Boundedness Rules

Reject if any of these conditions is true:

1. `for_each` over a list with no declared `max` in schema or policy.
2. `collect` without `pages`, `items`, or `time` limit.
3. `repeat` without `times` or `time` limit.
4. `try_again` without `max_attempts`.
5. `wait` event without timeout.
6. `ask` without timeout.
7. `together` with branch count exceeding policy.
8. Nested fanout that exceeds policy (e.g., `for_each` containing `together`).
9. `finish` with result of unknown max size where policy requires proof.

### Dataflow Propagation

The analyzer propagates bounds through the IR:

1. **Leaf bounds**: Each primitive contributes its declared bound.
2. **Sequential composition**: `max_steps` and `max_tickets` are summed.
3. **Nested loops**: Bounds multiply (outer `for_each` limit × inner action count).
4. **Conditional branches**: Take the maximum across branches.
5. **Parallel branches**: `max_parallel_in_flight` is the `together` branch count.

The compiler must be able to state: "This workflow can create at most N action tickets under declared limits." Even if N is conservative, having a bound is the requirement.

### Budget Validation

The computed `WholeWorkflowBudget` is validated against policy limits:

```rust
pub struct BoundednessPolicy {
    pub absolute_max_action_tickets: u32,     // default: 100_000
    pub absolute_max_parallel: u16,           // default: 256
    pub absolute_max_run_time_seconds: u64,   // default: 30 days
    pub absolute_max_result_bytes: u32,       // default: 256 KiB
    pub absolute_max_steps_executable: u32,   // default: 1_000_000
}
```

If any computed budget exceeds policy, the workflow is rejected with a typed `UnboundedWorkflow` error identifying which limit was exceeded.

---

## 65. Idempotency Verification Gate

### Principle

Every retry requires idempotency proof. Unsafe retry is rejected by default.

### Action Side-Effect Classification

Every action carries a side-effect class and retry safety rating:

```rust
pub enum SideEffect {
    Pure,
    LocalRead,
    LocalWrite,
    ExternalRead,
    ExternalWrite,
    Process,
    UnsafeShell,
}

pub enum RetrySafety {
    Idempotent,                // safe to retry unconditionally
    RequiresIdempotencyKey,    // safe with a valid idempotency key
    NotRetrySafe,              // retry rejected by default
    Unknown,                   // retry rejected
}
```

### Idempotency Verification Rules

| Side effect | Default retry rule |
|-------------|-------------------|
| `Pure` | Retry allowed |
| `LocalRead` | Retry allowed if action declares `Idempotent` |
| `ExternalRead` | Retry allowed if action declares `Idempotent` |
| `ExternalWrite` | Requires idempotency proof |
| `LocalWrite` | Requires idempotency proof or explicit policy override |
| `Process` | Retry rejected by default |
| `UnsafeShell` | Retry rejected by default |
| `Unknown` | Retry rejected |

### Idempotency Proof Requirements

For side-effecting actions (`ExternalWrite`, `LocalWrite`), the verifier requires:

```yaml
idempotency:
  required: true
  field: idempotency_key
  default: "$run.id:$step.id"
```

### Idempotency Key Restrictions

Reject idempotency keys that contain:

- `$secrets.*` — secret-tainted values in keys leak information
- `$attempt.number` — unless explicitly allowed by policy
- Random or time functions — keys must be deterministic

Valid key ingredients:

- Run ID
- Workflow digest
- Step ID or step index
- Loop item index
- Gather page cursor hash
- Trigger unique key

### Verification Gate Behavior

The verifier checks each `Do` node in the IR:

1. Look up the action's `SideEffect` and `RetrySafety`.
2. If the action is reachable from a `RetryCheck` node, verify retry is allowed.
3. If retry requires an idempotency key, verify the key is present and well-formed.
4. If retry is not safe, verify no `RetryCheck` can reach this action.
5. Emit `IdempotencyViolation` error if retry safety is not proven.

### Terminology Note

The verifier performs **idempotency attestation**, not idempotency proof. The verifier can require that an idempotency key is present and well-formed, and that the action contract declares idempotent behavior. It cannot prove that calling an external service twice with the same key will not create two side effects — that depends on external behavior. The word "proof" is reserved for properties the verifier can establish from the workflow alone.

---

## 66. Runtime Admission Gate

### Principle

The runtime only accepts verified artifacts. A run is not durable until `RunAccepted` is recorded.

### Admission Flow

```text
load artifact by digest
  → verify artifact digest matches stored IR
  → validate input against declared input schema
  → bind workflow digest to run
  → check required capabilities are granted
  → check required secrets are available (presence only, not values)
  → allocate run frame from pool
  → record RunAccepted event
  → return run_id
```

If `RunAccepted` is recorded, the run is durable. If any step before it fails, the run was never admitted.

### Admission Record

```rust
pub struct RunAdmission {
    pub run: RunId,
    pub artifact_digest: WorkflowDigest,
    pub input_digest: WorkflowDigest,
    pub capabilities_granted: Box<[Capability]>,
    pub secrets_available: Box<[SymbolId]>,
    pub admitted_at: u64,
}
```

### Capability Model

v1 capabilities are named permissions that actions declare and operators grant:

```rust
pub struct Capability {
    pub name: Box<str>,  // e.g. "network.github", "secrets.read.github_token"
    pub action: ActionId,
}
```

Admission checks that every capability required by the artifact's actions has been granted. Ungranted capabilities cause `CapabilityDenied` rejection.

### Secret Availability Check

Admission checks that every secret declared in the workflow is available in the runtime's secret store. Missing secrets cause `SecretUnavailable` rejection. Secret values are never part of the artifact or admission record — only presence is checked.

### Persistence of Admission

`RunAccepted` journal event is recorded durably before the run begins execution. Under `Strict` durability, this means `SyncAll` before returning `run_id`. Under `Journaled` durability, the event is queued and the run may begin before the write hits disk (acknowledged data-loss window).
