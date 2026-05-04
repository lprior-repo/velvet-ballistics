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

**Source of truth:** `crates/vb_core/src/`. The authoritative type layout is the code. This section states behavioral contracts that the code must satisfy. If code and doc disagree on field layout, the code wins. If code and doc disagree on behavior, the doc wins.

### `ids.rs` — Numeric ID Types

Required ID types (all `#[repr(transparent)]`, `Copy`, `Serialize`, `Deserialize`):

| Type | Inner | Purpose |
|------|-------|---------|
| `WorkflowId` | `u32` | Workflow identity |
| `RunId` | `u64` | Run identity |
| `StepIdx` | `u16` | Step/node index into `CompiledWorkflow.nodes` |
| `SlotIdx` | `u16` | Slot index into `RunFrame.slots` |
| `ExprIdx` | `u16` | Expression program index |
| `ActionId` | `u16` | Action identity |
| `AccessorIdx` | `u16` | Accessor program index |
| `ConstIdx` | `u16` | Constant pool index |
| `SymbolId` | `u32` | Interned string handle |
| `ListId` | `u32` | List arena handle |
| `ObjectId` | `u32` | Object arena handle |
| `BlobId` | `u64` | Blob arena handle |
| `SeqNo` | `u64` | Monotonic event sequence |
| `WorkflowDigest` | `[u8; 32]` | BLAKE3 digest |

Behavioral contracts:
- Table-index types (`StepIdx`, `SlotIdx`, `ExprIdx`, `AccessorIdx`, `ConstIdx`) must provide checked access to slices (via `CheckedIndex` trait or equivalent).
- No ID type may be constructed from unchecked arithmetic or unchecked casts.
- `WorkflowDigest` must provide `from_bytes` and `as_bytes` for storage interop.

### `value.rs` — Slot Value Model

Required types:

| Type | Contract |
|------|----------|
| `Taint` | Three-level lattice: `Clean < DerivedFromSecret < Secret`. `#[repr(u8)]` with explicit discriminants. Propagation rules in Section 47. |
| `FiniteF64` | Newtype over `f64`. Rejects NaN, +inf, -inf in both debug and release builds. Manual `Serialize`/`Deserialize` (not derive) to enforce rejection on decode. |
| `SlotValue` | Handle-only `Copy` enum: `Null`, `Bool(bool)`, `I64(i64)`, `F64(FiniteF64)`, `Symbol(SymbolId)`, `List(ListId)`, `Object(ObjectId)`, `Blob(BlobId)`. Must provide `type_name()` and `is_true()`. |
| `ConstValue` | Compile-time constant: `Null`, `Bool(bool)`, `I64(i64)`, `F64(FiniteF64)`, `Symbol(SymbolId)`. Must convert to `SlotValue` via `to_slot_value()` with no silent `Null` fallback. |

Behavioral contracts:
- `SlotValue` is handle-based; text and large payloads are referenced by handles, never stored inline.
- `FiniteF64::new` returns `CoreError::NonFiniteNumber` for non-finite inputs. No panic path.
- `ConstValue::to_slot_value` must map every variant; no default/fallback.

### `error.rs` — Core Error Types

Required error variants (the authoritative list is in the code; this lists the minimum):

```text
InvalidCompiledWorkflow { reason }
InvalidProgramCounter { step }
MissingNextStep { step }
MissingOutputSlot { step }
SlotOutOfBounds { slot }
ConstOutOfBounds { index }
ExprOutOfBounds { expr }
StepStateOutOfBounds { step }
ExpressionStackOverflow { max }
ExpressionStackUnderflow
UnsupportedPrimitive { primitive }
TypeMismatch { expected, found }
DivisionByZero
NonFiniteNumber
QueueFull
ResourceLimitExceeded { resource }
AllocationFailed
InternalInvariantViolation { reason }
```

All errors must be typed (no stringly errors), must carry diagnostic codes (Section 16), and must never require heap allocation in the hot path.

### `workflow.rs` — Compiled IR Types

Required types (authoritative layout in code):

| Type | Contract |
|------|----------|
| `CompiledWorkflow` | Immutable compiled artifact. Holds `nodes`, `expressions`, `accessors`, `constants`, `slot_count`, `entry: StepIdx`, `digest: WorkflowDigest`, `name`, `resource_contract`. Fields are private with getter methods. Constructed via `try_from_parts()` which validates all bounds. |
| `CompiledNode` | Single IR node: `id: StepIdx`, `output: Option<SlotIdx>`, `next: Option<StepIdx>`, `kind: CompiledNodeKind`. |
| `CompiledNodeKind` | 34+ variants covering all primitives (Section 15 lists them). The authoritative variant list is in the code. |
| `ExprProgram` | Postfix bytecode: `ops: Box<[ExprOp]>`, `max_stack: u8`. Stack effects validated by `check_expr_stack_bound`. |
| `ExprOp` | 30 opcodes: `LoadSlot`, `LoadConst`, `LoadAccessor`, comparison, logical, arithmetic, and helper ops (Section 46). |
| `AccessorProgram` | Path traversal: `root: SlotIdx`, `path: Box<[PathSegment]>` where `PathSegment = Field(SymbolId) \| Index(u32)`. |
| `ConstValue` | See `value.rs` above. |
| `ResourceContract` | 16 fields controlling hard limits (Section 13). |

Compiler rule: high-level YAML primitives may lower to multiple IR nodes. Runtime executes IR only. Generated Rust may skip IR dispatch but must preserve identical semantics. Final choose IR has exactly two checked forms: `Choose` evaluates expression-branch conditions from `ExprIdx`, and `ChooseSlot` reads pre-materialized boolean conditions from `SlotIdx` values produced by earlier IR. Raw YAML condition strings and untyped choose nodes are forbidden in final IR.

### `frame.rs` — Run Frame

`RunFrame` holds mutable execution state for a single run:

| Field | Type | Contract |
|-------|------|----------|
| `run_id` | `RunId` | Immutable after construction |
| `pc` | `StepIdx` | Program counter; set by `set_pc()` |
| `executed` | `u64` | Transition counter; incremented by deterministic steps |
| `states` | `Box<[StepState]>` | Per-step state machine; transitions validated (Section 45) |
| `slots` | `Box<[Option<SlotValue>]>` | Slot values; checked access only |
| `taint` | `Box<[Taint]>` | Per-slot taint; parallel to `slots` |

Behavioral contracts:
- `RunFrame::new` is the only constructor. Allocates exactly three boxed arrays. Rejects `step_count == 0` and out-of-range `first_step`. No arena/blob/symbol/journal allocation.
- `read_slot`/`write_slot`/`read_taint`/`write_taint` return `SlotOutOfBounds` for invalid indices.
- `mark_*` methods return `StepStateOutOfBounds` for invalid steps.
- Step-state transitions follow the contract in Section 45. Invalid transitions return `InternalInvariantViolation`.

### `engine.rs` — Execution Engine

Required types and functions:

| Type/Function | Contract |
|---------------|----------|
| `EngineSignal` | `Continue`, `Finished(SlotValue)`, `StepBudgetExhausted`, `AwaitingAction`, `AwaitingWait`, `AwaitingAsk` |
| `StepBudget` | Bounded step counter. `try_take() -> CoreResult<bool>`. Budget 0 returns `StepBudgetExhausted` immediately. |
| `step_once` | Execute single node dispatch. Returns `EngineSignal`. |
| `drive_deterministic` | Loop calling `step_once` until blocked by budget, suspension, or finish. |

`StepBudget` uses `remaining: u64`; `try_take() -> CoreResult<bool>`. Budget `0` executes zero transitions and returns `StepBudgetExhausted`. Budget `1` executes exactly one transition.

Known design limitation: `EngineSignal::Finished(SlotValue)` carries no taint. Taint information at the Finish boundary is discarded. This is documented in Section 47 and must be addressed in a future phase if finish-result taint tracking is required.

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

## 24. Mandatory Function Surface: `vb_core`

**Source of truth:** `crates/vb_core/src/`. This section states required behavioral coverage. Exact function names and signatures are defined by the code.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| ID accessors | Every numeric ID type must provide checked raw access (e.g., `get()`, `as_usize()`). |
| Value operations | `FiniteF64::new`, `FiniteF64::get`, `SlotValue::type_name`, `SlotValue::is_true`, `ConstValue::to_slot_value`. |
| Frame operations | `RunFrame::new`, `run_id`, `pc`, `executed`, `set_pc`, `read_slot`, `write_slot`, `read_taint`, `write_taint`, `write_slot_with_taint`, `mark_*` for all 7 step states, `step_state`, `reinitialize`, `increment_executed`. |
| Budget | `StepBudget::new`, `try_take`, `remaining`. |
| Execution | `step_once`, `drive_deterministic` (core), expression evaluation with `ValueStore`, accessor evaluation. |
| IR validation | `CompiledWorkflow::try_from_parts` validates node bounds, resource contracts, transition targets, expression stack bounds. |
| Value store | `ValueStore::new`, `insert_symbol`, `insert_list`, `insert_object`, `insert_blob`, lookup methods for each handle type, `object_field`, `list_item`. |

---

## 25. Mandatory Function Surface: `vb_yaml`

**Source of truth:** `crates/vb_yaml/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Parsing | `parse_yaml_events`, `parse_workflow_source`. |
| Profile validation | `validate_yaml_profile` (rejects anchors, aliases, merge keys, duplicate keys, ambiguous scalars, custom tags, binary scalars, multiple documents). |
| Source maps | `build_source_map`, `span_for_node`. |
| Fixtures | `load_fixture_source`. |

---

## 26. Mandatory Function Surface: `vb_validate`

**Source of truth:** `crates/vb_validate/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Schema validation | `validate_workflow_schema` (required fields, ID rules, primitive count, trigger types including HTTP rejection). |
| References | `validate_references` (forward refs, runtime refs, undeclared secrets). |
| Control flow | `validate_control_flow`, `validate_forward_only_then`, `validate_reachability`. |
| Type/taint | `validate_types`, `validate_taint` (taint propagation, secret leak detection). |
| Resources | `validate_resource_limits`. |
| Diagnostics | `diagnostic_from_error`, `error_code`. |

---

## 27. Mandatory Function Surface: `vb_expr`

**Source of truth:** `crates/vb_expr/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Lexer | `lex_expr` (bounded token stream, 256 max tokens). |
| Parser | `parse_expr` (Pratt parser, AST output, 64 max depth). |
| Typechecker | `typecheck_expr` (type propagation, mismatch detection). |
| Bytecode compiler | `compile_expr_to_bytecode`, `compile_expr_with_pool`, `compile_expr_with_resolver`, `const_fold_expr`, `check_expr_stack_bound`. |
| Evaluator | `eval_expr_program`, `eval_binary_op`, `eval_unary_op`, `eval_helper`. |

---

## 28. Mandatory Function Surface: `vb_compile`

**Source of truth:** `crates/vb_compile/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Entry point | `compile_workflow`, `YamlCompiler::compile`, `parse_ast`. |
| Slot compilation | Slot layout, accessor table, constant pool construction. |
| Lowering | Per-primitive lowering (set, do, choose, for_each, together, collect, reduce, repeat, wait, ask, finish). |
| Validation | Schema, reference, control flow, type-taint validation integrated into compile pipeline. |
| Expression | Expression compilation with reference resolution to `SlotIdx`. |
| Output | Digest computation, compiled artifact emission. |

---

## 29. Mandatory Function Surface: `vb_storage`

**Source of truth:** `crates/vb_storage/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Database | `FjallJournal::open` (creates/opens Fjall with 9 keyspaces). |
| Write path | `append_journaled`, `append_strict`, `append_strict_batch`, `persist_strict`. Write lock for ordering. |
| Keyspaces | Per-keyspace put/get: `put_workflow_source`, `put_compiled_ir`, `put_run_header`, `put_snapshot`, `put_blob`, index puts. |
| Read path | `workflow_source`, `compiled_ir`, `run_header`, `run_headers`, `snapshot`, `blob`, `events_for_run`. |
| Record encoding | `encode_record`, `decode_record` (BLAKE3 digest + CRC32C envelope). |
| Key construction | `workflow_source_key`, `compiled_ir_key`, `run_header_key`, `run_event_key`, `run_snapshot_key`, `blob_key`, index key constructors. |
| Recovery | `recover_full_journal`, `recover_snapshot_plus_tail`, `recover_runtime_summary`, `recover_runtime_frame_seed`, `recover_all_incomplete_runs`, `replay_events`, `is_terminal_event`, `extract_terminal`. |
| Digest verification | `verify_digests`, `check_workflow_source_digest`, `check_compiled_ir_digest`. |
| Writer queue | `JournalWriterQueue` for bounded group commit. |

---

## 30. Mandatory Function Surface: `vb_runtime`

**Source of truth:** `crates/vb_runtime/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Runtime | `Runtime::new`, `new_with_journal`, `submit_direct`, `submit_compiled`, `submit_compiled_with_inputs`, `cancel_run`, `inspect_run`, `tick_all`, `tick_shard`, `complete_action_with_output`, `fail_action`, `timer_fired`, `shutdown_graceful`, `drain_trace`, `take_inspect_response`, `counters_snapshot`. |
| Shard | `Shard::new`, `new_with_journal`, `enqueue`, `tick`, internal drive/action/timer handlers, `drain_for_shutdown`, `counters`, `snapshot_run`. |
| Engine | `execute_node_full` (all node kinds), `drive_deterministic_full`, `drive_with_actions`, `resume_action_outcome`. |
| Primitives | Per-primitive handlers in `primitives/`: for_each, together, collect, reduce, repeat, wait_ask. |
| Frame pool | `FramePool::take`, `release`, `available`, `capacity`. |
| Action dispatch | `ActionRegistry::register`, `dispatch`. |
| Trace | `TraceRing` with SPSC ring, drain, and history. |
| Journal adapters | `NoopRuntimeJournal`, `VolatileRuntimeJournal`, `StorageRuntimeJournal`, `QueuedStorageRuntimeJournal`. |

---

## 31. Mandatory Function Surface: `vb_ipc`

**Source of truth:** `crates/vb_ipc/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Frame encode/decode | `encode_frame`, `decode_frame_header`, `decode_frame_payload`, `validate_frame_bounds`. |
| Server | `serve_ipc` (mio-based Unix socket loop, all 11 command handlers). |
| Client | `IpcClient::connect`, `send_command`, `recv_response`. |
| Command handlers | `handle_submit_run`, `handle_submit_run_inline`, `handle_cancel_run`, `handle_inspect_run`, `handle_list_events`, `handle_answer_ask`, `handle_complete_action`, `handle_fail_action`, `handle_drain_trace`, `handle_health`, `handle_shutdown`. |

---

## 32. Mandatory Function Surface: `vb_codegen`

**Source of truth:** `crates/vb_codegen/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Code generation | `emit_rust_workflow` (CompiledWorkflow to Rust source). |
| Components | emit for IDs, drive function, step function, expression function, action boundary, finish, action match dispatch, resource contract. |
| Validation | `compare_generated_to_ir`, `validate_generated_subset`, compile-check generated Rust, trybuild fixture emission. |

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
  "fuzz",
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
| 37 | Whole-workflow boundedness | Static dataflow analyzer: compute `WholeWorkflowBudget` from IR, propagate bounds through nested loops/branches, reject if any budget exceeds policy. New `BoundednessPolicy` config. Tests: nested fanout, sequential sum, conditional max, unbounded rejection. Resolves DRIFT-3 (aggregate budget gap) with Phase 45. |
| 38 | Idempotency verification gate | `SideEffect` + `RetrySafety` classification per action. Verification gate rejects retry on side-effecting actions without idempotency key. Key ingredient validation (reject secrets, random, time in keys). New `IdempotencyViolation` error type. Tests: every side-effect class, key restriction, retry reachability. |
| 39 | Accepted artifacts + admission | `AcceptedArtifact` record with `VerificationProof`. `RunAdmission` flow: artifact digest, input validation, capability check, secret availability, `RunAccepted` event. Runs bind to artifact by digest, not loose YAML. CLI `--strict` mode for AI-authored workflows. Tests: admission rejection paths, artifact binding, strict-mode warnings. |
| 40 | Evidence chain completion | Slot value/taint snapshots in journal. Action input/output payload persistence for completed actions. Durability proof per primitive (each primitive must document what journal events constitute proof of completion). `VerificationProof.durable` field gates acceptance. Tests: crash recovery with evidence chain, payload reconstruction. |
| 41 | Capability model | `Capability` struct. Actions declare required capabilities. Admission checks granted capabilities. `CapabilityDenied` rejection. Operator grants capabilities at run submission. Tests: missing capability rejection, granted capability acceptance. |
| 42 | Validation deduplication | Eliminate duplicate validation between `vb_validate` and `vb_compile`. Single validation pipeline operating on a shared intermediate representation. Both crate APIs preserved for backward compatibility but backed by one implementation. Resolves DRIFT-5. |
| 43 | Taint propagation fix | Fix runtime taint tracking: `EvalExpr` joins taint from loaded slots, `BuildObject`/`BuildList` join taint from field/item slots, `Finish` carries taint in signal. Expression evaluator returns `(SlotValue, Taint)` pairs. Compile-time checks remain as defense-in-depth. Resolves DRIFT-1. |
| 44 | Recovery evidence chain | Emit `SlotWritten` + `StepStarted`/`StepSucceeded` for every deterministic step. Gate hydration on `UnsupportedRecoveryState` — fail with typed error if slots/taint cannot be reconstructed. Replace `Ok(()) \| Err(_) => {}` pattern in shard with propagated errors. Resolves DRIFT-2. |
| 45 | Resource budget enforcement | Per-run `ValueStore` arena cap. Tightened `ResourceContract` defaults (no `u16::MAX`). Hard ceiling on `StepBudget` per tick. Replace Collect global Mutex with per-run state. Resolves DRIFT-3. |
| 46 | IR structural validation | `try_from_parts` validates reachable nodes, forward-only edges, loop pairing, SymbolId ranges, accessor path segments. Artifact loading treats input as untrusted. Resolves DRIFT-4. |

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

## 36. Mandatory Test Coverage

**Test naming:** Exact test names are not mandated. Tests must exist that verify the following behaviors. The authoritative test list is the codebase; this section states required coverage areas.

### Core value and ID tests

Required coverage:
- `FiniteF64` accepts finite values, rejects NaN, rejects positive infinity, rejects negative infinity.
- `SlotValue` type names are stable and correct for every variant.
- `SlotValue` text uses symbol or blob handles (no inline strings).
- `ConstValue::to_slot_value` maps every variant; no silent Null fallback.
- `StepBudget` exhaustion returns false without error; remaining reaches zero cleanly.
- `RunFrame` bounds-checked for slots and step states; out-of-bounds returns typed errors.
- Step-state mark methods return errors on invalid step indices.
- `CompiledWorkflow::try_from_parts` rejects invalid PC, invalid edges, invalid tables.

### Parser and validator tests

Required coverage:
- Minimal valid manual and IPC workflows parse successfully.
- HTTP trigger rejected as out-of-core.
- Duplicate keys, anchors, aliases, merge keys, YAML 1.1 ambiguous booleans all rejected.
- Unknown top-level fields and unknown step fields rejected.
- Multiple primitives per step rejected; missing primitive rejected.
- Forward references rejected.
- Control-flow cycles detected and rejected.
- Secret-tainted finish results rejected at compile time.
- All diagnostics have code, path, span, and message.

### Engine invariant tests

Required coverage:
- Terminal states never transition back to running.
- Failed steps do not become succeeded without error handler.
- Budget exhaustion does not advance PC.
- Missing output slot, const out of bounds, expression stack overflow/underflow, unsupported primitive — all return typed errors.
- `SetConst` never reads unrelated slot zero.
- `Choose` and `ChooseSlot` produce identical results when conditions are pre-materialized.

### Recovery tests

Required coverage:
- Full journal replay reconstructs run state.
- Snapshot plus tail replay reconstructs run state.
- Replay detects divergence with typed error.
- Non-idempotent actions blocked during replay.
- Strict profile persists before ack.
- Journaled profile group commit recovers.
- Corrupt journal record returns typed error.

### IPC tests

Required coverage:
- Bad magic rejected before payload allocation.
- Oversized payload rejected.
- Command roundtrips (submit, cancel, inspect, events).
- Backpressure respected.
- Malformed frames return typed errors.

### Scheduler tests

Required coverage:
- Queue-full returns typed error.
- Run stays on one shard.
- Cancel pending and waiting runs.
- Shutdown drains gracefully or reports remaining.
- Timer resume order deterministic.
- Action completion resumes correct run.
- No task-per-step behavior under load.

### Compile-fail tests

Required coverage:
- Generated code cannot use unsafe, unwrap, unchecked indexing, or YAML runtime references.
- Public codegen contract rejects missing step.

---

## 37. Fuzz Targets

Required fuzz harnesses (actual paths: `fuzz/src/bin/*.rs`):

| Target | Coverage requirement |
|--------|---------------------|
| `yaml_events` | Arbitrary UTF-8 bytes → parser never panics |
| `expression` | Arbitrary UTF-8 bytes → lexer/parser/compiler never panics |
| `ipc_frame` | Arbitrary bytes → decoder never panics, length checks hold |
| `journal_event` | Arbitrary bytes → Postcard decode failure is typed |
| `compiled_ir` | Arbitrary bytes → decode/validate never panics |
| `generated_compare` | Generated/IR equivalence over small workflows |

---

## 38. Property Tests

Required proptest coverage areas:

| Property | Description |
|----------|-------------|
| Constant folding | Constant expressions fold to identical result as runtime evaluation |
| Bytecode/AST parity | Compiled bytecode produces same result as AST interpretation |
| Digest stability | Same input produces same compiled digest |
| Layout stability | Slot layout and accessor layout stable for same workflow |
| Replay determinism | Journal replay produces identical run state |
| Snapshot equivalence | Snapshot + tail replay equals full journal replay |
| Ordering invariants | `for_each` output order matches input order; `together` output order matches YAML order |
| Bound enforcement | Retry attempts never exceed limit; collect never exceeds page/item/time limits |
| State machine | No terminal state transitions back to running |
| Taint safety | Secret taint never enters finish result (at compile time) |
| IR/generated parity | IR interpreter and generated Rust produce identical outputs and errors |

---

## 39. Mandatory Benchmarks

**Benchmark naming:** Exact benchmark names are not mandated. Benchmarks must exist covering the following areas. The authoritative benchmark list is `benches/velvet_ballastics.rs`.

Required coverage areas:

| Area | Required benchmarks |
|------|-------------------|
| YAML parsing | Small workflow, large (1 MiB) workflow |
| Validation | Minimal workflow, 1000-step workflow |
| Compilation | Minimal workflow, 1000-step workflow |
| Expression | Symbol equality, number comparison, boolean chain, arithmetic |
| Slot operations | Read, write, copy |
| Core transitions | SetConst, EvalExpr, Choose (2-branch, 100-branch), Finish |
| Run chains | 1-step, 10-step, 1000-step save chains |
| Iteration | for_each, together, collect, reduce, repeat |
| Storage | Fjall append (no-persist, journaled, strict), Fjall read 1000 events |
| IPC | Frame encode, frame decode |
| Queues | ArrayQueue push/pop, rtrb push/pop |
| Trace | Trace event push, ring full policy |
| Writer queue | Journal writer queue push, group commit (batch 1, 64, 1024) |
| Scheduler | Shard submit-to-start, submit-to-finish |
| Direct API | Submit-to-finish |
| Async primitives | Ask answer resume, action complete resume, wait timer resume |
| Generated mode | Expression, save chain, choose |
| IR vs generated | 1-step, 1000-step comparison, ratio benchmarks |

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
    pub idempotency_keyed: Vec<ActionId>,   // actions with well-formed idempotency keys
    pub idempotency_attested: Vec<ActionId>, // actions attested idempotent by contract (external claim)
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
| 11. Idempotency | Stub | `Idempotency` enum exists in `ActionContract` (Section 19) for taint/replay classification. Phase 38 adds `SideEffect` + `RetrySafety` enums and the verification gate (section 65). These extend, not replace, the existing `Idempotency` classification. |
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

### Relationship to `ResourceContract`

`ResourceContract` (Section 13) defines per-workflow static limits: `max_steps`, `max_slots`, `max_retry_attempts`, `max_fanout`, `max_output_bytes`. These are declared by the workflow author and validated at compile time.

`WholeWorkflowBudget` is a computed analysis result: the verifier derives it from `ResourceContract` plus the IR's actual loop bounds and nesting structure. The relationship:

| `ResourceContract` field | `WholeWorkflowBudget` field | Relationship |
|--------------------------|----------------------------|--------------|
| `max_steps: u16` | `max_steps_executable: u32` | Computed budget cannot exceed `max_steps` × nesting depth factor |
| `max_retry_attempts: u16` | `max_retries_per_action: u16` | Direct copy from contract |
| `max_fanout: u16` | `max_together_branches: u16` | Direct copy from contract |
| `max_output_bytes: u32` | `max_result_bytes: u32` | Computed budget cannot exceed `max_output_bytes` |

`BoundednessPolicy` (below) provides absolute upper limits that apply ACROSS all workflows. `ResourceContract` limits apply WITHIN a single workflow. Validation order: `ResourceContract` ≤ `BoundednessPolicy`. If a computed `WholeWorkflowBudget` exceeds either, the workflow is rejected.

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

### Relationship to Existing `Idempotency` Classification

The existing `Idempotency` enum (Section 19) classifies actions for taint propagation and replay behavior:

```rust
// Section 19 — existing, in production
pub enum Idempotency {
    DeterministicPure,      // pure computation, no side effects
    IdempotentExternal,     // external call, safe to repeat
    AtLeastOnceExternal,    // external call, may execute more than once
}
```

Phase 38 extends `ActionContract` with two additional fields. These do NOT replace `Idempotency` — they refine retry decisions that `Idempotency` alone cannot express:

```rust
// Phase 38 — extends ActionContract
pub enum SideEffect {
    Pure,           // no observable side effects (maps to DeterministicPure)
    LocalRead,      // reads local state only
    LocalWrite,     // writes local state
    ExternalRead,   // reads external state
    ExternalWrite,  // writes external state (maps to AtLeastOnceExternal)
    Process,        // spawns or manages a process
    UnsafeShell,    // arbitrary shell execution
}

pub enum RetrySafety {
    Idempotent,                // safe to retry unconditionally
    RequiresIdempotencyKey,    // safe with a valid idempotency key
    NotRetrySafe,              // retry rejected by default
    Unknown,                   // retry rejected
}
```

Mapping rules between the two classification systems:

| `Idempotency` | Implies `SideEffect` | Implies `RetrySafety` |
|----------------|---------------------|----------------------|
| `DeterministicPure` | `Pure` | `Idempotent` |
| `IdempotentExternal` | `ExternalRead` or `ExternalWrite` | `Idempotent` or `RequiresIdempotencyKey` (action-specific) |
| `AtLeastOnceExternal` | `ExternalWrite` | `NotRetrySafe` unless key provided |

Actions declare `Idempotency` at compile time (existing). Phase 38 adds `SideEffect` and `RetrySafety` as additional action contract fields. The verifier uses all three to make retry decisions.

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

### Secret Availability Check

Admission checks that every secret declared in the workflow is available in the runtime's secret store. Missing secrets cause `SecretUnavailable` rejection. Secret values are never part of the artifact or admission record — only presence is checked.

### Persistence of Admission

`RunAccepted` journal event is recorded durably before the run begins execution. The existing storage layer already defines `JournalEvent::RunAccepted { run, seq, workflow }` (Section 49). Phase 39 extends this event with artifact digest and admission metadata. Under `Strict` durability, this means `SyncAll` before returning `run_id`. Under `Journaled` durability, the event is queued and the run may begin before the write hits disk (acknowledged data-loss window).

### Migration from Existing Submit Flow

The existing `Runtime::submit_direct(run, workflow: CompiledWorkflow)` and `ShardCommand::Submit { run, workflow }` bypass artifact verification — they accept a raw `CompiledWorkflow` with no digest binding, capability check, or secret availability check. These functions remain available for testing and internal use but are gated behind a `RuntimePolicy` flag:

```rust
pub struct RuntimePolicy {
    pub require_accepted_artifact: bool,  // default: false (backward compatible)
    pub strict_admission: bool,           // default: false
}
```

When `require_accepted_artifact` is `true`, `submit_direct` is rejected with `AdmissionRequired`. New admission-aware functions replace it:

```rust
pub fn submit_artifact(&self, run: RunId, artifact_digest: WorkflowDigest, input: &[u8], capabilities: &[Capability]) -> RuntimeResult<()>
```

This migration path allows existing tests and benchmarks to continue using `submit_direct` while production deployments enforce the admission gate. The IPC protocol already defines `SubmitRun` which carries a workflow reference; Phase 39 extends it to carry an artifact digest.

### Capability Model

v1 capabilities are named permissions that actions declare and operators grant:

```rust
pub struct Capability {
    pub name: Box<str>,  // e.g. "network.github", "secrets.read.github_token"
    pub action: ActionId,
}
```

`Capability` appears in two contexts with distinct semantics:
1. **Declared requirement** (in `AcceptedArtifact`): the set of capabilities the artifact's actions require.
2. **Granted permission** (in `RunAdmission`): the set of capabilities the operator has granted for this run.

Admission checks that every declared requirement is satisfied by a granted permission. Ungranted capabilities cause `CapabilityDenied` rejection.

Capability checking occurs at admission time (cold path) only. The runtime does not re-check capabilities during execution. `Box<str>` is acceptable because admission is cold-path.

---

## 67. Architectural Drift Register

This section tracks known architectural defects discovered through adversarial review. Each entry states the defect, the root cause, the resolution contract, and the phase that resolves it. Entries are removed when the resolution phase is complete and evidenced.

### DRIFT-1: Runtime Taint Tracking Is Incomplete

**Defect:** `EvalExpr`, `BuildObject`, and `BuildList` nodes write `Taint::Clean` unconditionally via `write_slot`. The `Finish` node emits `EngineSignal::Finished(SlotValue)` which carries no taint metadata. Compile-time taint checking is the only effective defense. A hand-crafted `CompiledWorkflow` that bypasses the compiler has no runtime taint protection.

**Root cause:** `write_slot` hardcodes `Taint::Clean`. Only `copy_slot` and `write_slot_with_taint` propagate taint. The expression evaluator, object builder, and list builder read tainted values but discard taint on output.

**Resolution contract:**
1. `EvalExpr` must read taint from every `LoadSlot` operand and join them into the output taint.
2. `BuildObject` must join taint from every field slot into the output taint.
3. `BuildList` must join taint from every item slot into the output taint.
4. `EngineSignal::Finished` must carry taint alongside the value, or the finish handler must check taint before emitting.
5. The expression evaluator's `eval_load_slot` must return both value and taint.
6. Compile-time taint checks remain as defense-in-depth.

**Coding style:** No functional combinators. No iterator chains. Use explicit `for` loops with checked indexing. Taint join is `max(left, right)` using the `repr(u8)` discriminant — a simple `u8` comparison, not a trait.

**Resolves in:** Phase 43 (Taint Propagation Fix)

### DRIFT-2: Crash Recovery Cannot Reconstruct Live State

**Defect:** The journal records no slot values, no slot taint, no step lifecycle events (`StepStarted`/`StepSucceeded`) for deterministic steps. After a crash, `UnsupportedRecoveryState` reports `slot_values: true`, `slot_taint: true`, but `hydrate_run_frame` proceeds with empty frames anyway. The system is not crash-recoverable for any workflow that performs deterministic computation between suspension points.

**Root cause:** Journal events are only emitted at suspension points (action dispatch, wait, ask). Deterministic steps between suspensions are treated as atomic but the journal cannot reconstruct them.

**Resolution contract:**
1. Every deterministic step must emit `SlotWritten` events (value + taint) to the journal before advancing PC.
2. `StepStarted`/`StepSucceeded` events must be emitted for every step, not just suspension points.
3. Recovery must reconstruct slot values and taint from journal events.
4. `UnsupportedRecoveryState` must gate hydration: if `slot_values == true`, hydration must fail with a typed error, not produce a broken frame.
5. Journal error handling in shard.rs must not use `Ok(()) | Err(_) => {}`. Journal write failures must propagate as runtime errors or at minimum log a diagnostic.

**Performance note:** Emitting `SlotWritten` per deterministic step increases journal write volume. Under `Journaled` durability, these batch via the writer queue. Under `Strict`, each step gets an fsync — this is the correct safety tradeoff. `Volatile` mode remains zero-journal for testing.

**Coding style:** No async. No channels. Synchronous journal append within the shard's single-threaded drive loop. Bounded writer queue absorbs burst. If queue is full, the step blocks (backpressure), not silently drops.

**Resolves in:** Phase 44 (Recovery Evidence Chain)

### DRIFT-3: No Aggregate Resource Budget Across Primitive Composition

**Defect:** Individual primitive bounds exist (`ForEach limit`, `Together branches`, `Repeat max_attempts`) but their composition is unbounded. `ForEach(limit=1000)` wrapping `Together(branches=256)` can create 256,000 sequential step executions and 256,000 ValueStore arena entries in a single run. The `ValueStore` has no cap on total arena entries (symbols, lists, objects, blobs are all append-only with no GC).

**Root cause:** Bounds are per-primitive, not per-run. No dataflow analysis propagates bounds through nested compositions. `ResourceContract` defaults (`max_fanout: u16::MAX`, `max_collect_items: u32::MAX`, `max_step_budget_per_tick: u64::MAX`) are effectively unbounded.

**Resolution contract:**
1. Phase 37 (Whole-Workflow Boundedness) computes `WholeWorkflowBudget` from IR — this resolves the static analysis gap.
2. `ValueStore` must have a per-run arena cap (e.g., `max_arena_entries: u32`). Insert methods must check the cap and return a typed error on overflow.
3. `ResourceContract` defaults must be tightened from `u16::MAX`/`u32::MAX`/`u64::MAX` to policy-specified defaults.
4. `StepBudget` per tick must have a hard ceiling (e.g., 100,000) regardless of configuration.
5. Collect global `Mutex<Vec>` must be replaced with per-run pagination state to eliminate cross-run interference.

**Resolves in:** Phase 37 (boundedness) + Phase 45 (Resource Budget Enforcement)

### DRIFT-4: IR Validation Is Bounds-Only, Not Structural

**Defect:** `try_from_parts` validates that all numeric indices are within array bounds. It does NOT validate structural correctness: reachable nodes, forward-only edges, well-formed loop structures (ForEachStart pairs with ForEachNext), valid SymbolId references, or accessor path segment validity. A postcard-deserialized artifact from untrusted input bypasses all compiler-level structural validation.

**Root cause:** The compiler's structural validations (control flow, reference, type/taint) operate on the AST, not on the compiled IR. They are never re-checked at the IR level.

**Resolution contract:**
1. `try_from_parts` must validate that every node is reachable from `entry`.
2. `try_from_parts` must reject backward edges (Jump targets, Choose targets, loop body/done targets must be forward).
3. `try_from_parts` must validate that loop primitives are paired correctly (ForEachStart has a matching ForEachNext and ForEachJoin).
4. `try_from_parts` must validate that BuildObject SymbolIds are within the symbol table range.
5. `try_from_parts` must validate AccessorProgram path segments (Field SymbolId range, Index bounds).
6. The artifact loading path (`run-compiled` CLI command) must treat the artifact as untrusted input.

**Coding style:** Straightforward `for` loops over nodes. Checked indexing. No recursion (bounded by node count). Each check returns a typed `IRValidationError` identifying the specific node and check that failed.

**Resolves in:** Phase 46 (IR Structural Validation)

### DRIFT-5: Validation Logic Duplicated Between vb_validate and vb_compile

**Defect:** Both `vb_validate` and `vb_compile` contain parallel modules (schema, references, control_flow, type_taint) that must be kept in sync manually. The two crates operate on different input types (document model vs AST) but enforce the same rules.

**Root cause:** Historical. `vb_validate` was built first on the document model. `vb_compile` was built later with its own validation on the AST. Both must accept the same workflow language.

**Resolution contract:**
1. Single validation pipeline on a shared intermediate representation.
2. Both crate public APIs preserved for backward compatibility.
3. Internal delegation to one implementation.
4. Remove the sync requirement.

**Coding style:** No traits, no generics, no higher-order functions. A plain `pub fn validate(parts: &WorkflowParts) -> Result<ValidationOutput, ValidationError>` that each crate calls.

**Resolves in:** Phase 42 (Validation Deduplication)

---

## 47. Durable Execution Architecture Contract

`velvet-ballastics` is a log-first durable execution engine. The architecture follows the same core model as production-grade orchestrators (Restate, AWS Step Functions): journal events are the ground truth, state is deterministically derived from the journal, and side effects are never re-executed without explicit idempotency proof.

### Log-First Invariants

1. **Journal entry persisted = step happened.** Once a journal event is durably written (according to the active durability profile), that step is committed. Recovery never re-executes it without idempotency proof.
2. **State is derived from journal, never the reverse.** Slot values, taint arrays, step states, and run status are all reconstructed by replaying journal events. No mutable state is the source of truth.
3. **Side effects are never re-executed during replay unless declared idempotent.** Non-idempotent actions are blocked during replay by `ActionReplayTracker`. Idempotent actions require matching `ActionTicket.idempotency_key` on re-execution.
4. **Recovery is deterministic.** Replaying the same journal events on the same compiled workflow digest must produce identical slot values, taint, step states, and terminal result. Any divergence is a `ReplayDiverged` error.
5. **Journal sequence numbers are monotonic per run.** No gaps, no reordering. `SeqNo` is `u64` and wraps are forbidden (typed error before wrap).

### Recovery Model

Recovery follows the snapshot-plus-tail pattern:

1. Load latest snapshot for the run (slot values, taint, step states at sequence N).
2. Replay journal events from sequence N+1 onward.
3. Each event is applied deterministically: `SlotWritten` updates slot+taint, `StepStarted`/`StepSucceeded` advances state machine, `ActionScheduled`/`ActionCompleted`/`ActionFailed` track action lifecycle.
4. Terminal events (`RunFinished`, `RunFailed`, `RunCancelled`) end replay.
5. If any event cannot be applied (missing prerequisite state, digest mismatch, corrupt record), recovery fails with a typed error — never silently continues.

Epoch-based recovery (future): Crash recovery should support a "seal and start new segment" model where the current journal segment is sealed on crash detection and a new segment begins, preventing partial-write ambiguity. This is not required for v1 single-server but the journal format must not preclude it.

### Single-Server Contract

`velvet-ballastics` is a single-server engine. There is no distributed replication, no leader election, no quorum consensus, and no control plane. These are explicit v1 exclusions:

- No Raft/Paxos consensus.
- No multi-node replication.
- No partition rebalancing.
- No distributed log (Bifrost-equivalent).
- No disaggregated storage tiering to object stores.

The single-server constraint means:
- Fjall is the sole durability mechanism. If the node loses power, recovery depends on Fjall's write-ahead log surviving the crash.
- Strict durability mode (`persist_strict` + `fsync`) is the only profile that guarantees no data loss on power failure.
- Journaled mode provides bounded data loss window (group commit batch interval).
- Volatile mode is testing-only and accepts full loss on crash.

### Tiered Durability Model

| Profile | Write Path | Crash Safety | Use Case |
|---------|-----------|--------------|----------|
| `volatile` | No Fjall writes | None — all data lost | Benchmarks, unit tests |
| `journaled` | Bounded Fjall writer queue, group commit | Bounded loss window (last batch) | Production default |
| `strict` | Synchronous Fjall persist + fsync before ack | Zero data loss | Financial, compliance |

### Compilation vs Interpretation

Unlike orchestrators that interpret journal entries against SDK code (opaque foreign processes), `velvet-ballastics` compiles workflows to numeric IR and optionally to generated Rust:

| Mode | Execution | When to Use |
|------|-----------|-------------|
| IR interpreter | Dispatch through `CompiledNodeKind` enum | Debugging, portability, semantic equivalence tests |
| Generated Rust | Direct `match` arms on step indices, no dispatch | `maxperf` builds, production throughput |

Generated Rust must preserve identical observable semantics to IR execution. Equivalence tests are mandatory before any generated mode is accepted for a primitive.

### Bounded Execution Contract

Every execution dimension is bounded by `ResourceContract`. The engine must reject or suspend before exceeding any bound. Silent truncation is forbidden.

Key bounds enforced at runtime:
- Steps per tick (`StepBudget`)
- Total slots, expressions, constants, accessors (compile-time)
- Expression stack depth (evaluator)
- Queue depth (shard command queue)
- Journal batch bytes (writer queue)
- Fanout branches, collect items, retry attempts (per-primitive)
- ValueStore arena entries (per-run cap, Phase 45)

This is the Holzmann influence: bounded loops, bounded allocation, no hidden growth vectors.

### Taint Propagation

`Taint` is a three-level lattice: `Clean < DerivedFromSecret < Secret`. Propagation rules:

1. `EvalExpr` joins taint from all loaded input slots.
2. `BuildObject`/`BuildList` join taint from all field/item slots.
3. `Finish` carries taint in the result signal `(SlotValue, Taint)`.
4. Compile-time validation rejects workflows where a `Finish` result slot is `Secret`-tainted (defense-in-depth).
5. Action output taint must be at least as restrictive as input taint for `DeterministicPure` and `IdempotentExternal` actions.
6. `AtLeastOnceExternal` actions propagate conservatively as `DerivedFromSecret` when any input is tainted.
7. Secret-tainted failure details must not enter public diagnostics without redaction.

---

## 48. Operator CLI Contract

The CLI is the primary interface for operators and AI agents. It must provide the same operational affordances as mature orchestrators without cargo-culting their branding.

### Canonical Command Surface

```text
velvet-ballastics validate <workflow.yaml>
velvet-ballastics compile  <workflow.yaml> --emit <ir|rust> --out <file>
velvet-ballastics explain  <workflow.yaml> [--json]
velvet-ballastics diff     <workflow.yaml> [--against <old.yaml>] [--json]
velvet-ballastics run      <workflow.yaml> --input-bin <file> --durability <mode> [--db <path>]
velvet-ballastics run      <workflow.yaml> --step <step-id> --step-input <file> [--durability <mode>]
velvet-ballastics run-compiled <workflow.vbir> --input-bin <file> --durability <mode> [--db <path>]
velvet-ballastics inspect <run-id> --db <path> [--json]
velvet-ballastics events  <run-id> --db <path> [--jsonl] [--step <id>] [--tail <n>] [--limit <n>]
velvet-ballastics trace   <run-id> --db <path> [--jsonl]
velvet-ballastics replay  <run-id> --db <path> [--json]
velvet-ballastics cancel  <run-id> --db <path>
velvet-ballastics resume  <run-id> --db <path>
velvet-ballastics retry   <run-id> --step <step-id> --db <path>
velvet-ballastics answer  <run-id> --slot <slot-id> --value <file> --db <path>
velvet-ballastics ipc-serve --socket <path> --db <path>
velvet-ballastics bench-run <workflow.yaml>
velvet-ballastics doctor  --db <path> [--json]
```

The `vb` binary name is a mandatory alias. Both `velvet-ballastics` and `vb` invoke the same binary.

### Single-Step Testing

`run --step <step-id>` executes exactly one step in isolation with explicit input. This is a first-class feature for debugging and validation.

Contract:
- Compile the workflow as normal.
- Resolve `step-id` to `StepIdx` in the compiled IR.
- Construct a minimal `RunFrame` with slots needed for the target step.
- Execute `step_once()` once.
- Report: step ID, step kind, input slots, output slot, engine signal, taint.
- No journal, no persistence, no action dispatch — pure in-memory.
- Exit 0 on success, 1 on step error, 2 on setup error.

### Durable Execution Controls

Strict operational distinction between lifecycle commands:

| Command | What it does | Journal impact |
|---------|-------------|----------------|
| `cancel` | Halt a running/suspended run immediately | Appends `RunCancelled` event |
| `resume` | Resume a suspended run from its current state | Continues journal from last event |
| `retry` | Re-execute a single failed step within an existing run | Preserves journal prefix, appends retry events |
| `replay` | Re-read full journal and verify state (read-only) | No journal mutation |
| `answer` | Answer a pending `Ask` with a slot value | Appends `AskAnswered` event |

`resubmit` (create a brand new run from the same workflow) is `run` with the same workflow — it gets a new `RunId` and fresh journal. It is not a lifecycle command.

### Explain / Dry-Run

`explain <workflow.yaml>` compiles without executing and reports the execution plan:

- Step graph: every step ID, kind, output slot, next step
- Control flow: branches (`Choose`), loops (`ForEach`/`Together`/`Collect`/`Reduce`/`Repeat`), linear chains
- Resource contract: all 16 bounded fields
- Action contracts: which steps are `Do` (side effects)
- Suspension points: which steps can suspend (`Wait`/`Ask`/`Do`)
- Slot layout: total slots, expressions, accessors, constants
- Estimated max step count (budget computation)
- Secrets usage: which steps reference `$secrets`
- Trigger type

`--json` produces machine-readable output. No `serde_json` in the binary — write JSON manually with format strings.

### Semantic Diff

`diff <workflow.yaml>` compares a workflow against its previously compiled version:

- Textual diff: YAML source changes (line-level)
- Semantic diff: changes in step count, control flow graph, resource contracts, secret usage, action contracts, retry policies
- Digest comparison: if a compiled artifact exists in the DB, compare BLAKE3 digests
- Exit codes: 0 = no semantic changes, 1 = semantic changes detected, 2 = error
- `--json` for machine-readable output

### Structured Observability

Output format flags:
- `--json` for snapshot commands (`inspect`, `explain`, `diff`, `doctor`)
- `--jsonl` for streaming commands (`events`, `trace`, `replay`)

Filter flags for `events`:
- `--step <id>` — filter events by step index
- `--tail <n>` — last N events
- `--limit <n>` — maximum events to show
- `--since <date>` — events after timestamp

Logs, events, and trace serve different purposes and must not be merged. Trace includes: resolved inputs, evaluated conditions, expanded loops, chosen branches, retry attempts, emitted outputs.

### CLI Design Rules

- No giant overloaded commands. Each command does one operator job.
- No hidden server-side magic. Local-first, local-only in v1.
- No naming that depends on users knowing another platform.
- Copy the operator affordances, not the branding.
- Machine-readable output (`--json`/`--jsonl`) is mandatory for every reporting command. AI agents must be able to parse output without screen-scraping.

---

## 49. Phase Extension: Operator Features

The following phases extend Section 35 for operator-facing features:

| Phase | Name | Required delivery |
|-------|------|-------------------|
| 50 | Single-step testing | `run --step <id>` with input payload, isolated execution, step result reporting. Tests: step resolution, minimal frame construction, step_once execution, output reporting. |
| 51 | Explain / dry-run | `explain` command with step graph, resource contract, suspension points, secrets usage, `--json` output. Tests: explain output matches compiled IR, JSON format validation. |
| 52 | Durable lifecycle controls | `cancel`, `resume`, `retry`, `answer` CLI commands. Strict distinction between retry-step, replay-run, and resubmit-workflow. Tests: each lifecycle command against journaled runs, cancelled runs, suspended runs. |
| 53 | Semantic diff | `diff` command with textual + semantic diff, digest comparison, exit codes. Tests: diff detects step changes, resource contract changes, secret changes. |
| 54 | Structured observability | `--json`/`--jsonl` flags, filter flags (`--step`, `--tail`, `--limit`, `--since`). Tests: JSON output parses correctly, filter flags narrow results. |
| 55 | Timer wheel | Replace `IndexMap<RunId, PendingTimer>` with `TimerWheel` backed by `BTreeMap<Instant, Vec<TimerEntry>>`. Automatic timer-driven resume in shard tick. Tests: timer firing, cancellation, next-deadline accuracy. |
| 56 | Collect hardening | Per-run pagination state (replace global Mutex), time-based pagination limit, `RunId`-keyed state. Tests: concurrent collect runs, time limit enforcement, crash-recovery of pagination state. |
| 57 | Recovery evidence chain | `SlotWritten` + `StepSucceeded` per deterministic step, `UnsupportedRecoveryState` hydration gate, fix stubbed `verify_digests` at `Full` level. Tests: crash recovery with full evidence chain, hydration failure on missing state. |
| 58 | Codegen expansion | `BuildObject`, `BuildList`, helper expression ops (`Contains`, `Length`, `Empty`, `Sum`, `Count`, `Unique`), `RetryCheck`. IR/generated equivalence tests per newly supported primitive. |
| 59 | Behavioral property tests | 11 required properties from Section 38: constant folding parity, bytecode/AST parity, digest stability, layout stability, replay determinism, snapshot equivalence, ordering invariants, bound enforcement, state machine, taint safety, IR/generated parity. |
| 60 | `vb` binary alias | Cargo.toml `[[bin]]` entry for `vb` pointing to same `main.rs`. Both `velvet-ballastics` and `vb` produce identical behavior. |

---

## 50. Competitive Performance Targets

The following targets are derived from published benchmarks of production-grade durable execution engines (Restate 1.2 on AWS c6id.8xlarge, 3-way replicated cluster, 1200 concurrent clients). As a single-server engine with no replication overhead, `velvet-ballastics` must meet or exceed these on equivalent hardware.

### Step-Level Latency Targets

| Metric | Restate (replicated) | Velvet Ballastics (single-server) | Notes |
|--------|---------------------|-----------------------------------|-------|
| Single step p50 (no replication) | 3ms | <= 1ms | No network roundtrip for quorum |
| Single step p50 (journaled) | 10ms | <= 5ms | Fjall group commit vs quorum replication |
| Single step p50 (strict) | N/A (same as journaled) | <= 10ms | fsync on every step; Restate has no equivalent |
| Full workflow p50 (9 steps, low load) | 31ms | <= 15ms | Compiled IR, no SDK roundtrip |
| Full workflow p50 (9 steps, high load) | 116ms | <= 60ms | Single-server removes coordination overhead |
| Full workflow p99 (9 steps, high load) | 163ms | <= 100ms | Tight bound from no-unsafe, checked arithmetic |

### Throughput Targets

| Metric | Restate | Velvet Ballastics | Notes |
|--------|---------|-------------------|-------|
| Actions (steps) per second | 94,286 | >= 100,000 | Generated Rust mode must hit this |
| Full workflows per second (9 steps) | 8,571 | >= 10,000 | Single-server removes replication overhead |
| Concurrent active runs | 1,200 (test clients) | >= 4,096 | Frame pool capacity |

### Why These Targets Are Achievable

Restate pays for every step:
1. Network roundtrip for quorum replication (fastest path is one RTT to 2 of 3 nodes)
2. Epoch checking and leader validation on every event
3. SDK roundtrip: server pushes to service process, service responds over network
4. Tokio async overhead (scheduler, waker, polling)
5. RocksDB async flush competing with event processing

`velvet-ballastics` eliminates all five:
1. No replication — local Fjall write
2. No leader — single shard owns the run
3. No SDK — action dispatch is a function call within the same process
4. No async — synchronous deterministic loop
5. No competing flush — Fjall writes happen through bounded writer queue, not in the hot path

The generated Rust mode adds another advantage: no IR dispatch table lookup. Steps compile to direct `match` arms on constant step indices. This should bring single-step latency under 100 microseconds for pure computation steps (no I/O).

### Measurement Contract

Every performance claim must include:
- `criterion` or `iai-callgrind` output with p50/p95/p99
- Hardware: CPU model, cores, RAM, disk type (NVMe vs SSD)
- Build profile: debug, release, maxperf, PGO
- Execution mode: IR interpreter vs generated Rust
- Durability profile: volatile, journaled, strict
- Number of concurrent runs
- Benchmark fixture digest (reproducible)

---

## 51. Execution Attempt Tracking

When a run fails and is retried, the engine must reject stale events from previous execution attempts. This prevents split-brain between overlapping retries.

### Contract

1. Every run attempt gets a monotonically increasing `attempt: u16` counter.
2. `ActionTicket` carries the `attempt` number.
3. On retry, the attempt counter increments. Any `ActionCompleted`/`ActionFailed` event carrying a stale attempt number is rejected with `StaleAttempt { expected, found }`.
4. Journal events are tagged with the attempt number.
5. Recovery replays events for the latest attempt only. Events from earlier attempts are ignored.
6. The attempt counter is journaled as part of `RunAccepted` and persists across crashes.

This mirrors Restate's invocation execution attempt tracking, adapted for single-server synchronous execution.

---

## 52. Journal Trimming

The journal cannot grow indefinitely. After a snapshot is taken, journal events older than the snapshot are eligible for trimming.

### Trimming Contract

1. A snapshot captures the full run state at `SeqNo` N.
2. Once a snapshot at N is confirmed durable (fsynced), all journal events with `SeqNo <= N` for that run are eligible for deletion.
3. Trimming must not delete events for runs that have no snapshot.
4. Terminal runs (finished/failed/cancelled) are eligible for trimming after their final snapshot, subject to a retention policy (default: keep last N terminal runs per workflow).
5. The `doctor` command must report journal size and suggest trimming if the journal exceeds a configured threshold.

This prevents unbounded disk growth in long-running production deployments.

---

## 53. Converged Binary Design

`velvet-ballastics` ships as a single binary that operates in different modes depending on the command invoked. This mirrors Restate's converged single-binary design, adapted for single-server operation.

### Modes

| Command | Binary Role | Components Active |
|---------|-------------|-------------------|
| `run` | Executor | Compiler + Engine + Storage |
| `run-compiled` | Executor | Engine + Storage |
| `validate` | Validator | YAML Parser + Validator |
| `compile` | Compiler | YAML Parser + Validator + Compiler + Codegen |
| `explain` | Analyzer | YAML Parser + Validator + Compiler |
| `diff` | Analyzer | Compiler + Digest comparison |
| `inspect` | Observer | Storage reader |
| `events` | Observer | Storage reader |
| `trace` | Observer | Storage reader |
| `replay` | Observer | Storage reader + Recovery |
| `ipc-serve` | Server | Engine + Storage + IPC server loop |
| `cancel`/`resume`/`retry`/`answer` | Controller | Storage reader + Engine + Storage writer |
| `bench-run` | Benchmarker | Compiler + Engine + Timer |
| `doctor` | Diagnostics | Storage reader + Health checks |

No mode starts components it doesn't need. The `validate` command never opens Fjall. The `inspect` command never compiles YAML. The `ipc-serve` command is the only mode that runs the full stack persistently.

### Future Extension

If `velvet-ballastics` ever supports distributed operation (v2+), the binary gains additional roles (log-server, controller, ingress) but the converged model persists: a single binary, configured by role, no separate services to deploy.

---

## 54. AI-Native CLI Control Plane

The CLI is the AI-native control plane. The UI is for humans to see the system. The CLI is for humans and AI agents to operate, verify, repair, replay, and explain the system.

North star:

1. Anything the UI can show, the CLI can emit as structured data.
2. Anything an operator can inspect, an AI agent can inspect safely.
3. Anything that fails produces a machine-readable explanation.

### Dual-Personality Design

The CLI has two modes of output:

**Human mode** — Pretty, readable, fast:

```text
velvet-ballastics verify workflow.yaml
velvet-ballastics run issue_triage --input input.vbin
velvet-ballastics inspect run_123
velvet-ballastics replay run_123
```

Output is colored, summarized, and ergonomic.

**AI mode** — Stable, structured, boring:

```text
velvet-ballastics verify workflow.yaml --emit yaml
velvet-ballastics inspect run_123 --emit yaml
velvet-ballastics replay run_123 --explain --emit yaml
velvet-ballastics incident run_123 --emit yaml
```

No fragile pretty text. No hidden state. No "look at the dashboard." AI mode emits schemas that are documented and versioned.

### Lifecycle Command Surface

Command groups mirror the system lifecycle:

```text
velvet-ballastics validate workflow.yaml
velvet-ballastics verify   workflow.yaml
velvet-ballastics compile  workflow.yaml
velvet-ballastics graph    workflow.yaml
velvet-ballastics simulate workflow.yaml
velvet-ballastics run-compiled workflow.vbir
velvet-ballastics submit   issue_triage
velvet-ballastics inspect  run_123
velvet-ballastics events   run_123
velvet-ballastics replay   run_123
velvet-ballastics incident run_123
velvet-ballastics action list
velvet-ballastics action inspect github.issue.create
velvet-ballastics system status
velvet-ballastics doctor
velvet-ballastics ai context run_123
```

The CLI is not just "run workflow." It is a compiler/debugger/operator interface.

### verify Is the Hero Command

`verify` is the flagship. It answers: *is this workflow safe to run, and if not, what must change?*

```text
velvet-ballastics verify workflow.yaml --profile strict
```

Human output:

```text
✓ structure valid
✓ bounded execution
✓ resource budget computed
✓ no secret-to-result flow
✓ all external actions strict-durable safe
✓ replay policy safe

compiled digest: 8c13...
max transitions: 842
max action calls: 4
max frame bytes: 19.2 KiB
```

AI output (`--emit yaml`):

```yaml
schema_version: velvet-ballastics/cli-output/v1
kind: VerificationReport
workflow:
  name: issue_triage
  source_digest: blake3:...
  compiled_digest: blake3:...
profile: strict
status: pass
certificates:
  structural:
    status: pass
    invalid_edges: []
    unreachable_steps: []
  boundedness:
    status: pass
    max_ir_transitions: 842
    max_action_calls: 4
    max_retries: 3
  resources:
    status: pass
    max_slots: 48
    max_expr_stack: 6
    max_frame_bytes: 19648
    max_result_bytes: 32768
  taint:
    status: pass
    public_result_secret_reachable: false
    forbidden_paths: []
  actions:
    status: pass
    external_actions:
      - action: github.issue.create
        action_id: 7
        idempotency: IdempotentExternal
        strict_safe: true
  durability:
    status: pass
    journal_before_dispatch: true
    completion_before_frame_mutation: true
```

### Structured Diagnostics with Repair Hints

When validation fails, the CLI emits structured repair hints — not just text:

```yaml
schema_version: velvet-ballastics/cli-output/v1
kind: DiagnosticReport
status: fail
diagnostics:
  - code: ACTION_REQUIRES_IDEMPOTENCY
    severity: error
    path: $.steps[2].do
    span:
      line_start: 18
      column_start: 5
      line_end: 29
      column_end: 12
    message: Strict durability requires idempotency for external action http.request.
    repair:
      kind: add_field
      path: $.steps[2].do.idempotency
      value: required
    explanation: The action may be retried after crash recovery, so it needs a durable idempotency key.
```

This lets an AI agent read error → patch YAML → verify again. No guessing.

### explain Command

```text
velvet-ballastics explain workflow.yaml --emit yaml
```

Output includes: what the workflow does, what actions it calls, what secrets it touches, what can fail, what is durable, what is safe to retry, what resource bounds exist.

```yaml
kind: WorkflowExplanation
summary: "Classifies a support ticket, creates a GitHub issue, and sends a Slack notification."
steps:
  - id: classify
    kind: do
    action: ai.classify_ticket
    reads:
      - $input.message
    writes:
      - $classify
    max_calls: 1
    taint:
      input: Clean
      output: Clean
  - id: create_issue
    kind: do
    action: github.issue.create
    idempotency: IdempotentExternal
    strict_durable: true
failure_modes:
  - step: create_issue
    errors:
      - RATE_LIMITED
      - PERMISSION_DENIED
      - TIMEOUT
durability:
  side_effects_journaled_before_dispatch: true
  replay_safe: true
```

### graph Command

```text
velvet-ballastics graph workflow.yaml --emit yaml
```

Emits a graph artifact consumable by Makepad UI, Figma/Miro prototype importers, AI reasoning, CLI summaries, and documentation generators. One source, many consumers.

```yaml
kind: WorkflowGraph
nodes:
  - step_idx: 0
    id: classify
    kind: do
    action: ai.classify_ticket
    output_slot: 8
    badges:
      strict_safe: true
      secret_sensitive: false
  - step_idx: 1
    id: create_issue
    kind: do
    action: github.issue.create
    output_slot: 15
edges:
  - from: classify
    to: create_issue
    kind: then
  - from: create_issue
    to: done
    kind: then
```

### simulate Command

```text
velvet-ballastics simulate workflow.yaml --input input.vbin --mocks mocks.yaml --emit yaml
```

Runs deterministically with mocked actions. Output:

```yaml
kind: SimulationReport
status: finished
events:
  - seq: 1
    kind: RunAccepted
  - seq: 2
    kind: StepStarted
    step: classify
  - seq: 3
    kind: ActionScheduled
    action: ai.classify_ticket
  - seq: 4
    kind: ActionCompleted
    action: ai.classify_ticket
    source: mock
  - seq: 5
    kind: SlotWritten
    slot: 8
    value_summary:
      type: object
      fields:
        priority: high
result:
  type: object
  fields:
    status: ok
taint:
  public_result_secret_reachable: false
```

This lets AI agents test before running.

### Runtime Commands

**Submit:**

```text
velvet-ballastics submit issue_triage --input-bin input.vbin --emit yaml
```

```yaml
kind: SubmitRunResult
status: accepted
run_id: 123
workflow:
  name: issue_triage
  compiled_digest: blake3:...
durability:
  profile: strict
  run_accepted_durable: true
```

**Inspect:**

```text
velvet-ballastics inspect run_123 --emit yaml
```

```yaml
kind: RunInspection
run_id: 123
status: awaiting_action
current_step:
  idx: 2
  id: create_issue
action_ticket:
  action: github.issue.create
  action_id: 7
  attempt: 1
  idempotency_key_hash: blake3:...
  scheduled_durable: true
  dispatch_state: started
replay:
  safe_to_replay: true
  reason: idempotent_external_action
```

**Events:**

```text
velvet-ballastics events run_123 --tail 20 --emit yaml
```

```yaml
kind: RunEvents
run_id: 123
events:
  - seq: 11
    kind: StepStarted
    step_idx: 2
    timestamp: 1710000000
  - seq: 12
    kind: ActionScheduled
    action_id: 7
    ticket: ...
```

**Replay:**

```text
velvet-ballastics replay run_123 --explain --emit yaml
```

```yaml
kind: ReplayReport
run_id: 123
status: replayed
loaded:
  snapshot_seq: 80
  journal_tail_events: 17
result:
  divergence: false
  reconstructed_pc: 4
  reconstructed_status: awaiting_action
action_recovery:
  pending:
    - action: github.issue.create
      ticket: ...
      policy: retry_with_same_idempotency_key
```

### incident Command

```text
velvet-ballastics incident run_123 --emit yaml
```

Produces the AI-safe black box report:

```yaml
kind: IncidentReport
run_id: 123
status: failed
failure:
  code: ACTION_TIMEOUT
  step: create_issue
  action: github.issue.create
  retryable: true
side_effect_certainty:
  scheduled_durable: true
  completion_durable: false
  external_effect: uncertain
  safe_to_retry: true
  reason: same_idempotency_key
journal_tail:
  - seq: 14
    kind: ActionScheduled
  - seq: 15
    kind: ActionFailed
slot_diffs:
  - slot: 12
    before: null
    after:
      type: object
      redacted: false
taint:
  secret_leak_detected: false
repair_hints:
  - kind: increase_timeout
    path: $.steps[2].do.timeout_ms
    current: 5000
    suggested: 15000
  - kind: add_backoff
    path: $.steps[2].do.retry.backoff_ms
    suggested: 500
```

### Action Discovery

```text
velvet-ballastics action list --emit yaml
```

```yaml
kind: ActionList
actions:
  - name: github.issue.create
    action_id: 7
    idempotency: IdempotentExternal
    strict_safe: true
    input_schema_digest: blake3:...
  - name: ai.classify_ticket
    action_id: 12
    idempotency: DeterministicPure
    strict_safe: true
    input_schema_digest: blake3:...
```

```text
velvet-ballastics action inspect github.issue.create --emit yaml
```

```yaml
kind: ActionDescription
name: github.issue.create
idempotency: IdempotentExternal
strict_safe: true
requires:
  secrets:
    - github_token
input_schema:
  repo:
    type: symbol
    required: true
  title:
    type: symbol
    required: true
output_schema:
  issue_number:
    type: i64
  url:
    type: symbol
failure_codes:
  - RATE_LIMITED
  - PERMISSION_DENIED
  - INVALID_INPUT
examples:
  - name: minimal
    yaml: |
      do:
        action: github.issue.create
        with:
          repo: $input.repo
          title: $input.title
```

### doctor Command

```text
velvet-ballastics doctor --emit yaml
```

Checks: runtime daemon reachable, Fjall DB healthy, action packs loaded, action ABI digest, compiled workflows available, IPC socket permissions, strict durability available, journal writer healthy.

```yaml
kind: DoctorReport
status: pass
checks:
  - name: ipc_socket
    status: pass
  - name: fjall_store
    status: pass
  - name: action_registry
    status: pass
    action_count: 12
  - name: strict_durability
    status: pass
```

### ai context Command

Specifically for AI agents. Emits a compact, redacted packet:

```text
velvet-ballastics ai context run_123 --emit yaml
```

```yaml
kind: AiContextPacket
safe_for_model: true
run:
  id: 123
  status: failed
workflow:
  name: issue_triage
  compiled_digest: blake3:...
failure:
  code: ACTION_TIMEOUT
  step: create_issue
  replay_safe: true
redactions:
  secrets_redacted: 2
  blobs_summarized: 1
suggested_next_commands:
  - velvet-ballastics replay run_123 --explain --emit yaml
  - velvet-ballastics events run_123 --tail 50 --emit yaml
  - velvet-ballastics verify workflow.yaml --profile strict --emit yaml
```

This is a stable AI interface, not a gimmick.

### Output Format Contract

```text
--emit text      # Human-readable (default)
--emit yaml      # AI-structured
--emit postcard  # Machine binary
```

JSON may follow as a cold adapter, but YAML and binary are canonical for v1.

Rules:

1. Every structured output has `schema_version`.
2. Every output has `kind`.
3. Every diagnostic has `code`, `path`, `span`, `message`, `repair`.
4. Secret values are never emitted unless explicit `--unsafe` flag.
5. Large blobs are summarized by digest/type/size.
6. Exit codes are stable and documented.

Stable exit codes:

| Code | Meaning |
|------|---------|
| 0 | success |
| 1 | validation failed |
| 2 | verification failed |
| 3 | compile failed |
| 4 | runtime failed |
| 5 | storage error |
| 6 | IPC error |
| 7 | action policy error |
| 8 | replay divergence |

AI agents can branch on exit codes. No parsing error text.

### CLI-UI Parity Rule

No UI-only truth. If the UI shows taint graphs, replay timelines, action tickets, queue pressure, certificate status, or incident repair, the CLI must expose it too.

Backend emits typed artifacts:

- `VerificationReport`
- `WorkflowGraph`
- `RunInspection`
- `RunEvents`
- `ReplayReport`
- `IncidentReport`
- `SystemStatus`
- `ActionDescription`

CLI and UI are views over those same artifacts. Makepad UI consumes the same data.

### CLI Build Order

1. `validate --emit yaml`
2. `verify --emit yaml`
3. `compile --emit ir/cert/graph`
4. `simulate --emit yaml`
5. `run`/`submit`
6. `inspect --emit yaml`
7. `events --emit yaml`
8. `replay --explain --emit yaml`
9. `incident --emit yaml`
10. `system status --emit yaml`
11. Makepad UI consumes the same data

Build CLI before fancy UI. The UI should not invent concepts — it visualizes proven backend artifacts.

### The Killer Demo

```text
velvet-ballastics verify issue-triage.yaml --profile strict --emit yaml
velvet-ballastics simulate issue-triage.yaml --input example.vbin --mocks mocks.yaml --emit yaml
velvet-ballastics submit issue_triage --input-bin prod.vbin --emit yaml
velvet-ballastics incident run_123 --emit yaml
```

Then hand the output to an AI and ask: *What failed, is it safe to retry, and what should I change?* If the AI can answer correctly from the CLI packet, the design works.

---

## 55. Workflow Command-Center Front-End

### Vision

A mission control–style front-end for the durable workflow system. The UI should feel like a game: alive, causal, and inspectable, rather than a CRUD dashboard. When an operator opens the application they must immediately understand the state of the system — what is running, blocked or failed, where bottlenecks are forming, whether sensitive data is flowing to forbidden sinks, and whether replaying an execution is safe.

Two metaphors guide the design:

1. **Air traffic control** for durable workflows — track many independent runs, see queue depths and shard health, spot collisions before they happen.
2. **Mission control** for automation — monitor subsystem health and intervene during incidents.

Bad metaphors to avoid: kanban boards, generic forms dashboards, low-code builders. Focus on flow of state, movement of events, replay of decisions, and subsystem health.

### Visual Theme Reference

The visual identity is defined by the two reference images at:

- `docs/ui-reference/control-center-theme-a.png` — Grid-based multi-panel layout with process flow diagram, activity feed, metrics dashboard, and detailed log table. Deep navy/black background, bright green/red/yellow/cyan accents for status encoding. Flat design with monospaced data fonts.
- `docs/ui-reference/control-center-theme-b.png` — Status indicator sidebar with shield icons, central workflow canvas with color-coded nodes, real-time metrics panel, and bottom KPI strip. Dark theme with green/yellow/red/purple/cyan accent palette. Geometric nodes, rounded corners, high contrast.

Core aesthetic rules:

- Dark background (near-black/deep navy), high-contrast accent colors.
- Color encodes meaning: green = healthy/success, amber = warning/retry, red = failed/critical, cyan = informational/in-progress, purple = active/secret-tainted.
- Monospaced fonts for data. Sans-serif for labels.
- Flat design, no gratuitous gradients or 3D effects.
- Negative space between panels. Data-dense but not cluttered.
- Subtle animation pulses for live state (queue bars, moving packets, node glows), not decorative animation.

### Core Questions the UI Must Answer

At all times the UI must answer these instantly:

1. **What is running?** Which workflows are active, where are they executing, what state is each run in?
2. **What is blocked?** Which runs are waiting for external events, retries, or timers?
3. **What failed and why?** Error code, context, replay safety.
4. **Did side effects occur?** Durable action tickets and idempotency information.
5. **Where is pressure building?** Queue depths, shard utilization, storage health.
6. **Did secrets leak?** Taint propagation overlays showing secret-sensitive paths and sinks.
7. **What changed since the last good run?** Diff slot values, taint status, certificates between runs.

### Primary Screens

#### A. System Overview ("World Map")

The entire runtime as a living machine. Four panels:

- **Left — Topology/System map:** Shards with identifiers and health indicators. Each shard displays active runs, ready queue depth, action completion queue depth, timer counts, frame pool usage, trace ring fill, and per-shard throughput metrics. Motion cues (pulses on queue bars, glowing packets between lanes) indicate work moving through the system.
- **Centre — Activity lanes:** Horizontal lanes per shard, visualizing active runs flowing across steps. Blocked runs blink red, waiting runs glow blue, retries pulse amber. Lane height/intensity conveys queue pressure.
- **Right — Alerts, incidents, and pressure:** Stack of alert cards summarizing current incidents, replay divergences, and blocked reconciliation. Cards are clickable to open the Run Inspector.
- **Bottom — Event ticker:** Scrolling strip of recent system events (RunAccepted, StepStarted, ActionScheduled, etc.) with color-coded severity. Clicking an event jumps to the corresponding run.

Smooth panning, zooming, inertial scrolling, and heat-bar overlays. Simulation-like overview where operators watch workflows move through shards in real time.

#### B. Workflow Graph / Authoring View

Individual workflow definition as a structured graph (not freeform whiteboard). YAML is source of truth; the canvas is a projection supporting structured editing.

Each node card shows:

- Step ID, primitive type (Task, Choice, Wait, Pass, Parallel, Map, Succeed, Fail), and action name for Task nodes.
- Retry policy, timeout, resource impact.
- Taint sensitivity and strict-safety status.
- Inline badges: action id, retry count (R3), timeout (T5s), secret participation (S), strict-durable safety (D), recent failures (!).

Edges labelled with transition type (normal, branch condition, error route, retry route, join). Historical branch frequencies on hover. Minimap for navigation. Scrubber overlays run-time state onto the design graph for replaying past executions.

Inspector pane: YAML source, compiled IR details, input/output slots, resource contracts, retry/catch policies, taint information, last-run statistics. Diff two workflow versions directly on the graph.

#### C. Run Inspector / Replay Theater

The hero feature — a replayable mission log for a single execution. Unifies graph, timeline, event log, and inspectors:

- **Left:** Workflow graph with nodes coloured by runtime state (green = succeeded, blue = waiting, amber = retrying, red = failed, grey = not executed, purple = secret-tainted, white/teal = verification-safe). Selecting a node opens a details drawer.
- **Centre:** Timeline/playback control showing every journal event (RunAccepted, StepStarted, SlotWritten, ActionScheduled, ActionCompleted, ActionFailed, WaitScheduled, AskScheduled, RetryScheduled, RunFinished, RunFailed). Play, pause, step forward/backward, jump to failure, jump to action, jump to replay divergence, change playback speed. Event scrubber drives the graph overlay.
- **Right:** Detail inspectors for selected node/event — slot diffs, taint diffs, action ticket details (run, step, attempt, idempotency key hash, timestamps, outcome, replay safety, duplicate completions). Slot diff panel shows how individual slot values change at each event.
- **Bottom:** Event log with timestamps and severity, clickable to sync timeline. Tabs switch between event stream, slot changes, taint flows, action tickets, and system counters.

Time-travel debugging for durable workflows: scrub any run like a video replay and see every state transition, durable event, ticket, secret flow, and resource change.

#### D. Verification / Certificate View

Pre-flight certificates: structural validity, boundedness, resource bounds, taint flow, and action policy. Answers "is this workflow safe?" and surfaces proofs.

Panels:

- **Structure:** Unreachable steps, invalid transitions, incorrect joins, cycle analysis.
- **Boundedness:** Max transitions, max retries, fan-out, timer waits, action count.
- **Resources:** Slot count, max frame size, max action payload, max result size, queue requirements.
- **Taint/secret flow:** Source-to-sink graph. Purple paths = tainted flow; shield icons = safe outputs.
- **Action policy:** Idempotency classification, timeout coverage, missing caps, strict-durability eligibility.
- **Replay/durability:** Journaled vs strict profile differences, potential divergence points, worst-case recovery.

Each panel: summary pass/fail, numeric bounds, interactive overlays on the graph.

#### E. Incident / Failure Console

Tactical incident board, not a log list. Top: failure code, offending step, run ID, workflow digest, replay safety, side-effect certainty. Tabs: cause, timeline, state diff, replay behaviour, repair suggestions.

Highlights the failure path, shows slot diffs just before failure, proposes recovery (increase timeout, reduce payload, add retry backoff, pin idempotency, fix secret leak). AI assistance can summarise or generate repro steps.

### AI Companion Panel

Not a generic chat sidebar. The AI copilot receives structured artifacts: graph model, certificates, event stream, slot diffs, error packets, taint graph, resource counters.

Useful actions:

- Explain a failure and summarise what changed in a run.
- Show all secret-sensitive paths and identify leaks.
- Explain why strict-durability eligibility failed.
- Suggest bounded retry policies, minimal reproductions, or resource optimisations.

Prompts presented as buttons, not free-form chat. Responses shown alongside referenced data. AI output always maps back to YAML, compiled IR, journal events, or certificates — never hidden UI state.

### Front-End Data Contract

Backend must expose machine-readable artifacts consumed by both CLI and UI:

| Artifact | Consumer |
|----------|----------|
| `WorkflowGraph` | CLI `graph`, UI authoring canvas, AI |
| `VerificationReport` | CLI `verify`, UI certificate view, AI |
| `RunInspection` | CLI `inspect`, UI run inspector |
| `RunEvents` | CLI `events`, UI timeline/event ticker |
| `ReplayReport` | CLI `replay`, UI replay theater |
| `IncidentReport` | CLI `incident`, UI incident console, AI |
| `SystemStatus` | CLI `system status`, UI system overview |
| `ActionDescription` | CLI `action inspect`, UI action panels, AI |

These artifacts are delivered via streaming APIs or websockets so the UI remains live and responsive. The CLI emits the same artifacts in YAML or postcard format.

### Front-End Build Order

| Phase | Deliverable | Why first |
|-------|-------------|-----------|
| 1 | Run Inspector / Replay Theater | Immediate debugging value; exercises the hardest backend APIs first. |
| 2 | Verification / Certificate View | Differentiates the product via static analysis and safety proofs. |
| 3 | System Overview (World Map) | Macro health visibility once individual runs are inspectable. |
| 4 | Authoring Canvas | Full editing lifecycle; requires incremental layout and careful UX. |

Phase 1 surfaces the hardest problems early: mapping journal events to graph changes, aligning time series with visual layout, implementing diff inspectors. It also provides immediate debugging value before any other screen exists.

### Technology

The front-end is built with Makepad, consuming the same artifact types emitted by the CLI. The Makepad UI does not invent concepts — it visualizes proven backend artifacts. Both CLI and UI are views over the same typed data.

---

## 68. AI-Safe Quality Infrastructure

AI changes must be small, checkable, replayable, benchmarked, and hard to merge when wrong. The closed loop is:

```
spec -> task -> patch -> mechanical checks -> evidence -> benchmark -> certificate -> merge
```

AI agents must not guess which checks to run. Every quality gate is exposed as a first-party `xtask` command that returns structured machine-readable output. No evidence bundle means no merge.

### 68.1 xtask Command Center

A first-party `xtask` crate provides the AI-safe command interface for development. AI agents invoke `cargo xtask <command>` and receive structured YAML/JSON output; they never guess which checks apply.

Required commands:

| Command | Purpose |
|---------|---------|
| `cargo xtask ai-context --crate <crate> --topic <topic>` | Emit relevant files, contracts, required tests, and fast commands for a focused working set |
| `cargo xtask ai-plan --bead <id>` | Validate that a plan covers the bead scope and references correct invariants |
| `cargo xtask ai-check --scope <crate>` | Run fmt, clippy, nextest, forbidden-scan, hotpath-scan for a single crate; stop at first failure |
| `cargo xtask ai-evidence --bead <id>` | Generate or validate the evidence bundle for a bead |
| `cargo xtask invariants` | Run all invariant checks from `contracts/invariants.yaml` |
| `cargo xtask hotpath-scan [--changed]` | Scan for allocation, formatting, or unbounded patterns in hot-path code |
| `cargo xtask forbidden-scan [--changed]` | Scan for forbidden tokens, macros, patterns, imports |
| `cargo xtask cert-check` | Validate verification certificates for compiled workflows |
| `cargo xtask perf-compare --against main` | Benchmark comparison against a baseline |
| `cargo xtask perf-report --emit yaml` | Emit structured performance report |
| `cargo xtask perf-baseline save` | Save current performance baseline |
| `cargo xtask replay-lab` | Run differential replay tests |
| `cargo xtask crash-lab --workflow <name> [--crash-at <point> \| --all-crash-points]` | Deterministic fault-injection harness |
| `cargo xtask diff-test --suite <name>` | Run differential test suite |
| `cargo xtask alloc-check --suite hotpath` | Verify allocation behavior for hot paths |
| `cargo xtask api-diff` | Public API diff against baseline |
| `cargo xtask review --changed --emit yaml` | Structured patch review report |
| `cargo xtask why-failed <log>` | Explain a failure in human/AI-readable form |
| `cargo xtask mutants --scope touched` | Mutation testing for changed code |
| `cargo xtask loom --model <name>` | Run Loom concurrency model test |
| `cargo xtask kani --harness <name>` | Run Kani proof harness |
| `cargo xtask fuzz-target new <name>` | Create a new fuzz target |
| `cargo xtask prop-test new <name>` | Create a new proptest harness |
| `cargo xtask repro shrink --failure <log>` | Shrink a failure to minimal repro |
| `cargo xtask repro run <repro-file>` | Replay a minimal repro |
| `cargo xtask test-plan --phase <n>` | List required tests for a phase |
| `cargo xtask test-plan --missing` | List required tests not yet implemented |

All output is structured (YAML by default). Example `ai-check` output:

```yaml
kind: AiCheckReport
scope: vb_core
status: fail
commands:
  - name: fmt
    status: pass
  - name: clippy
    status: fail
    diagnostics:
      - file: crates/vb_core/src/frame.rs
        code: clippy::arithmetic_side_effects
        line: 88
  - name: nextest
    status: not_run
    reason: clippy_failed
recommended_next_action:
  kind: fix_diagnostic
  file: crates/vb_core/src/frame.rs
```

### 68.2 Three Check Levels

AI needs fast feedback first, then deep proof later. Three levels provide a ladder instead of one impossible command.

#### ai-fast (run constantly while coding)

```bash
cargo +nightly fmt --all -- --check
cargo +nightly check --workspace --all-targets
cargo +nightly clippy -p <touched-crate> --all-targets --all-features -- -D warnings
cargo +nightly nextest run -p <touched-crate>
cargo xtask forbidden-scan --changed
cargo xtask hotpath-scan --changed
```

#### ai-deep (run before closing a bead)

```bash
cargo +nightly nextest run --workspace --all-features
cargo +nightly test --doc --workspace --all-features
cargo +nightly miri test -p vb_core -p vb_expr -p vb_compile
cargo mutants --package <touched-crate> --timeout 60
cargo llvm-cov --workspace --all-features
cargo fuzz build
```

#### ai-release (run before release)

```bash
just check
just test
just supply-chain
just miri
just fuzz-smoke
just coverage
just mutants-smoke
just bench-build
just feature-powerset
just source-length
just maxperf
```

### 68.3 Evidence Bundles

Every AI-authored change produces an evidence bundle at `.evidence/<bead-id>/evidence.yaml`. No evidence bundle means no merge. This extends section 60 (Evidence Artifact Format) with AI-specific fields.

```yaml
kind: AiImplementationEvidence
bead: runtime-engine-setconst
phase: 13
git_commit: abc123
model_notes:
  summary: "Implemented SetConst typed error behavior."
files_changed:
  - crates/vb_core/src/engine.rs
  - crates/vb_core/tests/engine.rs
public_api_changed: false
hot_path_changed: true
commands:
  - command: cargo +nightly fmt --all -- --check
    exit_code: 0
    log: logs/fmt.txt
  - command: cargo +nightly clippy -p vb_core --all-targets --all-features -- -D warnings
    exit_code: 0
    log: logs/clippy-vb-core.txt
  - command: cargo +nightly nextest run -p vb_core
    exit_code: 0
    log: logs/nextest-vb-core.txt
tests_added:
  - missing_output_slot_is_typed_error
  - const_out_of_bounds_is_typed_error
benchmarks:
  required: false
remaining_risk:
  - "Copy primitive not implemented in this bead."
```

### 68.4 Machine-Readable Invariants

Invariants live in `contracts/invariants.yaml` as executable rules. `cargo xtask invariants` outputs exactly which invariant failed.

```yaml
invariants:
  - id: no_runtime_yaml
    applies_to:
      - crates/vb_core/**
      - crates/vb_runtime/**
      - crates/vb_storage/**
      - crates/vb_ipc/**
      - generated/**
    forbidden:
      - saphyr_parser
      - parse_workflow_source
      - yaml

  - id: no_hot_path_formatting
    applies_to:
      - crates/vb_core/src/engine.rs
      - crates/vb_runtime/src/shard/**
      - generated/**
    forbidden_macros:
      - format
      - println
      - eprintln
      - dbg

  - id: no_unchecked_indexing
    applies_to:
      - crates/**
    forbidden_patterns:
      - indexing
      - slicing
```

### 68.5 Semantic Banned Scans

Token-level grep is necessary but insufficient. The quality infrastructure uses multiple scan layers:

| Layer | Tool | Checks |
|-------|------|--------|
| Token scan | ripgrep | `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, forbidden imports |
| Clippy denies | `clippy` | `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::arithmetic_side_effects`, etc. |
| Unsafe scan | `cargo geiger` | Transitive unsafe in dependencies |
| AST scanner | syn-based custom (`xtask forbidden-scan`) | Unchecked indexing, slicing, `as` casts, ignored `Result`, `HashMap<String, _>` in runtime, `serde_json` in runtime, HTTP crates in runtime |
| Public API diff | `cargo public-api` | Accidental public contract changes |
| Allocation scanner | `xtask hotpath-scan` | `format!`, `println!`, `Vec::push` without pre-reserve, `String` construction in hot paths |

AI often satisfies the literal rule while violating the intent. Multi-layer scanning catches this.

### 68.6 AI Context Packets

AI must not read the whole repo. `cargo xtask ai-context --crate vb_core --topic engine` emits a precise working set:

```yaml
kind: AiCodeContext
crate: vb_core
topic: engine
relevant_files:
  - crates/vb_core/src/engine.rs
  - crates/vb_core/src/frame.rs
  - crates/vb_core/src/compiled.rs
  - crates/vb_core/src/errors.rs
contracts:
  - "No unsupported primitive may silently continue."
  - "StepBudget(0) executes zero transitions."
  - "SetConst has no Null fallback."
required_tests:
  - missing_output_slot_is_typed_error
  - const_out_of_bounds_is_typed_error
commands:
  fast:
    - cargo +nightly nextest run -p vb_core engine::
```

### 68.7 Spec-to-Test Mapping

Required tests live in `contracts/tests.yaml` as executable metadata. This makes the master document's mandatory test coverage (section 36) queryable.

```yaml
tests:
  - name: const_out_of_bounds_is_typed_error
    crate: vb_core
    module: engine
    phase: 13
    invariant: const_lookup_checked
    required_error: CoreError::ConstOutOfBounds

  - name: set_const_never_reads_unrelated_slot_zero
    crate: vb_core
    module: engine
    phase: 13
    invariant: no_null_fallback
```

Commands:

- `cargo xtask test-plan --phase 13` — list required tests for a phase
- `cargo xtask test-plan --missing` — list required tests not yet implemented

### 68.8 Property Tests, Fuzz Harnesses, and Proof Targets

AI is good at writing examples but misses edge cases. Harnesses are generated from contracts.

**proptest** for invariants: `cargo xtask prop-test new compiled_ir_bounds`

**cargo-fuzz** for binary decoders/parsers: `cargo xtask fuzz-target new yaml_events`, `cargo xtask fuzz-target new ipc_frame`

Fuzz rules for every binary decoder:
- Fuzz arbitrary bytes
- Assert typed error or valid object
- Never panic
- Never allocate before length validation

**Kani** for small critical proofs (model checking, not whole-program verification):

`cargo xtask kani --harness <name>`

Target properties:

| Harness | Property |
|---------|----------|
| `step_budget` | `StepBudget(0)` never decrements; `StepBudget(n)` returns true exactly n times |
| `taint_join` | Commutative and associative |
| `ipc_header_bounds` | Payload length check rejects `len > max` before allocation |
| `resource_bound` | Arithmetic does not overflow |

Kani targets restricted to: `StepBudget` arithmetic, `FiniteF64` rejection, record header lengths, IPC frame bounds, small transition-target validators, resource bound arithmetic, taint lattice joins.

**Loom** for concurrency-critical runtime pieces only:

| Model | What it tests |
|-------|---------------|
| `action_completion_cancel` | Bounded queue wrapper + action completion handoff |
| `shutdown_drain` | Shutdown/cancel race model |
| `journal_writer_queue` | Journal writer queue model |
| `timer_fired_cancel` | Timer fired vs cancel race |

Loom is not used everywhere. Only where shared mutable state exists.

**Miri** for pure crates: `vb_core`, `vb_expr`, `vb_compile` (already in section 4).

**cargo-careful** as extra paranoid job for pure crates: runs with extra nightly-only debug assertions for UB detection.

**Prusti** is research/optional only in `verification/prusti/`. Not in the critical path until proven stable.

### 68.9 Mutation Testing as AI Correctness Check

AI writes tests that pass but often do not pin behavior. Mutation testing catches this.

`cargo xtask mutants --scope touched` — mutation testing for changed code only.

Failure output:

```yaml
kind: MutantsReport
status: fail
survived:
  - file: crates/vb_core/src/engine.rs
    mutation: "changed ok_or MissingOutputSlot to MissingNextStep"
    implication: "tests do not distinguish missing output from missing next"
```

This tells the agent exactly what its tests failed to prove.

### 68.10 Differential Testing

The system has many pairs that must produce identical results. Differential tests assert equivalence.

Required diff suites:

| Suite | Left | Right |
|-------|------|-------|
| `ir-generated` | AST interpreter | ExprProgram bytecode |
| `replay` | Snapshot + tail replay | Full journal replay |
| `api-ipc` | Direct API result | IPC result |
| `yaml-events` | YAML parser event stream | AST expectations |
| `strict-simulated` | Strict replay | Simulated replay |

Command: `cargo xtask diff-test --suite <name>`

This is the most important correctness pattern for AI-generated code.

### 68.11 Crash/Recovery Lab

Deterministic fault-injection harness. Every crash point asserts:

- Recovery succeeds, or recovery blocks with typed reconciliation state
- Never duplicates non-idempotent action
- Never loses durable completion
- Snapshot + tail matches full replay

```bash
cargo xtask crash-lab --workflow issue_triage --crash-at ActionScheduled
cargo xtask crash-lab --workflow issue_triage --crash-at ActionCompletedBeforeSlotWrite
cargo xtask crash-lab --workflow issue_triage --all-crash-points
```

AI must add crash points when it modifies journal, action, or replay behavior.

### 68.12 Performance Regression Gates

AI will make "clean" Rust slower. Performance gates are first-class.

Tracking metrics: instruction count, allocations, bytes allocated, p50/p95/p99, journal latency, IPC latency, transition latency, generated-vs-IR ratio.

Tools: `iai-callgrind` for stable instruction/cache comparisons, `criterion` for statistical local benchmarking.

Performance budget file at `contracts/perf-budget.yaml`:

```yaml
benchmarks:
  transition_set:
    max_regression_percent: 3
  ipc_frame_decode:
    max_regression_percent: 5
  run_noop_1000:
    max_regression_percent: 3
```

If AI changes code and `transition_set` regresses by 12%, the harness rejects it. Speed claims are impossible without stored benchmark evidence.

### 68.13 Allocation Tracing Gates

For hot paths, performance is not just time — it is allocations. Tests run hot transitions with an allocation counter.

Rules:
- `RunFrame` admission may allocate
- Deterministic transitions in turbo/maxperf must not allocate
- IPC decode must not allocate before payload length validation
- Expression eval must not allocate stack memory dynamically

Command: `cargo xtask alloc-check --suite hotpath`

### 68.14 Public API Diff Gate

`cargo xtask api-diff` uses `cargo-public-api` to detect accidental public contract changes.

```yaml
kind: PublicApiDiff
status: fail
removed:
  - vb_core::errors::CoreError::ConstOutOfBounds
added:
  - vb_core::errors::CoreError::Unknown
risk: "stable error model changed"
```

AI must not casually alter stable errors, action ABI structs, IPC commands, certificate schemas, or public function signatures.

### 68.15 Supply-Chain Policy

AI may not add a dependency without a dependency-scope bead that includes:

1. Why the dependency is needed
2. Which handwritten code it replaces
3. Hot-path impact assessment
4. Unsafe/geiger result
5. License status
6. Audit/vet status
7. Rollback plan

This stops "AI added 14 crates because convenient." Existing tools `cargo audit`, `cargo deny`, `cargo vet`, `cargo geiger`, and `cargo machete` enforce this.

### 68.16 Structured Patch Review

Every patch gets a structured review report:

```yaml
kind: PatchReviewReport
risk: high
areas:
  - hot_path
  - durability
  - public_api
files_changed:
  - crates/vb_runtime/src/shard.rs
required_checks:
  - ai-fast
  - loom:shutdown_drain
  - crash-lab:all
  - perf-compare:shard_submit_to_finish
blocking_questions:
  - "Does this change preserve journal-before-dispatch?"
  - "Does this add allocation after run admission?"
```

`cargo xtask review --changed --emit yaml` classifies the patch and determines which deep checks apply.

### 68.17 Rustdoc Examples as Executable Contracts

Every public API includes a `/// # Examples` doc block that compiles and runs:

```rust
/// # Examples
/// ```
/// # use vb_core::engine::StepBudget;
/// let mut budget = StepBudget::new(1);
/// assert!(budget.try_take().unwrap());
/// assert!(!budget.try_take().unwrap());
/// ```
```

Verified by `cargo +nightly test --doc --workspace --all-features`. Doc examples are runnable contracts.

### 68.18 Trybuild Compile-Fail Suites

For generated code and macros, compile-fail tests pin policy:

- `generated_code_cannot_use_unsafe`
- `generated_code_cannot_unwrap`
- `generated_code_cannot_index_unchecked`
- `generated_code_cannot_reference_yaml_runtime`
- `public_codegen_contract_rejects_missing_step`

AI generates code that compiles but may violate policy. Compile-fail tests catch this.

### 68.19 Minimal Repro Generator

When fuzz, property test, or crash lab fails, generate a tiny repro:

```bash
cargo xtask repro shrink --failure logs/failure.yaml
```

Output: `repros/ipc_bad_header_0007.bin`, `repros/workflow_replay_divergence_001.yaml`

Then: `cargo xtask repro run repros/workflow_replay_divergence_001.yaml`

Effective for AI repair loops — the agent gets the smallest possible failing case.

### 68.20 Contracts as Data

Every stable contract emitted as data in `contracts/`:

| File | Content |
|------|---------|
| `contracts/errors.yaml` | Error codes, variants, messages |
| `contracts/ipc_commands.yaml` | IPC command schema |
| `contracts/journal_events.yaml` | Journal event schema |
| `contracts/certificates.yaml` | Certificate schema |
| `contracts/action_abi.yaml` | Action ABI schema |
| `contracts/runtime_profiles.yaml` | Runtime profile defaults |
| `contracts/hot_paths.yaml` | Hot path annotations |
| `contracts/invariants.yaml` | Executable invariant rules |
| `contracts/tests.yaml` | Required test metadata |
| `contracts/perf-budget.yaml` | Performance regression thresholds |

Codegen produces Rust enums, docs, CLI schemas, UI schemas, AI context, and tests from these sources. Reduces drift. AI reasons from the same source that generates code.

### 68.21 Failure Explanation

`cargo xtask why-failed logs/ai-check.yaml` explains failures:

```yaml
kind: FailureExplanation
summary: "Patch added format! to a hot path."
why_it_matters: "Hot deterministic transitions must not allocate or format text."
fix:
  - "Return CoreError with static reason."
  - "Render diagnostics in cold path."
```

Better harness explanations produce better AI behavior.

### 68.22 AI Patch Protocol

Binding protocol for every code change, enforced by convention and `xtask`:

1. State bead ID.
2. State invariant touched.
3. Modify smallest possible surface.
4. Add or update tests first when behavior changes.
5. Run `ai-fast`.
6. If hot path, durability, storage, or IPC touched — run targeted deep checks.
7. Produce evidence bundle.
8. Never claim success without command output.

Required patch footer in every commit/bead:

```
Evidence:
- ai-fast: pass
- nextest -p vb_core: pass
- fuzz build: not required, parser untouched
- perf compare: not required, no hot path touched
```

### 68.23 AI-Safe Code Zones

Code is marked by zone. Scanning rules vary by zone.

| Zone | Marker | Rules |
|------|--------|-------|
| `hot-runtime` | `// velvet-zone: hot-runtime` | No allocation, no formatting, no `HashMap<String, _>`, no dynamic dispatch |
| `cold-compiler` | `// velvet-zone: cold-compiler` | `HashMap` allowed, `format!` allowed in diagnostics |
| `generated` | `// velvet-zone: generated` | Compile-fail policy enforced, no `unsafe`, no `unwrap` |
| `storage-decode` | `// velvet-zone: storage-decode` | No allocation before length validation, fuzz coverage required |
| `test` | `// velvet-zone: test` | Relaxed rules, but must use typed assertions |

This prevents blanket rules from blocking useful code in cold paths.

### 68.24 Golden Internal Models

Executable reference models live in `reference/`:

| File | Purpose |
|------|---------|
| `reference/engine_model.rs` | Slow but clearly correct engine semantics |
| `reference/taint_model.rs` | Taint propagation reference |
| `reference/replay_model.rs` | Replay/recovery reference |
| `reference/resource_model.rs` | Resource bound reference |

Differential tests assert: optimized runtime == reference model.

AI modifies optimized code while the reference model keeps semantics pinned.

### 68.25 Perf Annotations for Hot Functions

Hot functions carry local rules that `xtask hotpath-scan` enforces:

```rust
// velvet-hot-path: no-alloc, no-format, max-lines=25
fn step_once(...) -> CoreResult<EngineSignal> {
    ...
}
```

Scanner checks: line count, allocation absence, formatting absence, bounded resource use. AI knows the local rules before editing.

### 68.26 AI Context for Spec-to-Implementation

`cargo xtask ai-context` consumes contracts data to produce context packets. The AI agent flow for a bead is:

1. `cargo xtask ai-context --crate <crate> --topic <topic>` — get working set
2. `cargo xtask test-plan --phase <n>` — get required tests
3. Implement
4. `cargo xtask ai-check --scope <crate>` — fast verification
5. `cargo xtask ai-evidence --bead <id>` — generate evidence bundle
6. If hot path / durability / IPC / storage touched:
   - `cargo xtask perf-compare --against main`
   - `cargo xtask crash-lab --workflow <name> --all-crash-points`
   - `cargo xtask loom --model <name>`
7. `cargo xtask review --changed --emit yaml` — structured review
8. Close bead with evidence

This turns AI from "creative coder" into "mechanical implementer."
