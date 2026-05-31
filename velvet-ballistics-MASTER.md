# velvet-ballistics — Master Mechanical Build Contract

**Status:** current backend/IR-interpreter scope; single source of truth for this repository
**Audience:** AI coding agents, runtime implementers, performance engineers, QA agents
**Product name:** `velvet-ballistics`
**Binary name:** `velvet-ballistics`
**Package name:** `velvet-ballistics`
**Rust crate/module prefix:** `velvet_ballistics`
**Bead rig:** `velvet-ballistics`
**Bead database:** `velvet_ballistics`
**Language version:** `velvet-ballistics/v1`

This repo-root file, `velvet-ballistics-MASTER.md`, is the authoritative build plan, lifecycle tracker, architecture contract, and implementation acceptance contract for this repository. Other docs provide context only and cannot override this document.

Project spelling rule: any use of legacy capitalized product spelling is invalid except for explicitly labeled migration references to pre-existing external artifacts. The current repository root path is `/home/lewis/src/velvet-ballistics`; the current master filename is repo-root `velvet-ballistics-MASTER.md`. New code, docs, beads, crate names, package names, generated paths, CLI examples, diagnostics, and implementation artifacts must use the canonical names above.

---

## 0. Prime Directive

`velvet-ballistics` is a Rust-nightly, no-unsafe, no-panic, single-server, ultra-low-latency durable execution engine for workflow orchestration. YAML is an authoring format only. The runtime never interprets YAML, parses JSON, serves HTTP, or routes text commands. Workflows compile into numeric state machines over numeric slots, numeric actions, numeric steps, and bounded resource contracts.

The current implementation goal is **Backend / IR Interpreter Complete**: strict YAML authoring, validation, verification, compiled numeric IR, IR-interpreter execution, Fjall durability, direct Rust API, binary IPC, CLI observability, replay/recovery, and evidence gates. Rust workflow code generation, `maxperf` acceptance, and all native UI/Makepad work are removed from the current core feature set. Residual codegen/UI/maxperf material is cleanup debt unless it is explicitly quarantined as historical evidence.

The runtime uses numeric state machines, numeric slots, numeric actions, shard-owned state, and deterministic synchronous execution until suspension. Fjall is required for persistence. Postcard is required for compact binary records. Ingress is direct Rust API plus binary IPC. `CompiledWorkflow` IR is the only active execution artifact for the current milestone. Any section explicitly marked removed, historical, or quarantined is non-normative for the current milestone and cannot block Backend / IR Interpreter Complete acceptance.

### Product Positioning Contract

Publicly, `velvet-ballistics` must not be described as a generic DAG runner, low-code graph editor, YAML-as-programming framework, Airflow replacement, or Temporal clone. Those frames hide the actual wedge and invite false comparisons.

The product identity is: an AI-safe, local-first, single-server durable execution engine that verifies AI-authored workflows before admission, persists an inspectable journal, protects side effects with idempotency evidence, and enforces resource and taint bounds. Generated Rust execution is not a current product path.

The unit of trust is the accepted artifact, not the YAML source. YAML is a cold authoring surface. Verification certificates, compiled IR digests, resource budgets, action contracts, capability grants, journals, snapshots, and replay reports are the operational truth.

Competitive comparison is allowed only with scope discipline:

1. Compare durability and replay semantics to Temporal, DBOS, and AWS Step Functions.
2. State the v1 single-server boundary plainly: no replication, no quorum, no leader election, no distributed control plane.
3. Compare data orchestration ergonomics to Airflow and Dagster only when explaining non-goals.
4. Never claim production readiness, performance superiority, or crash safety without executable evidence and benchmark/recovery artifacts attached to the bead or release.
5. The public demo path is `verify -> simulate -> submit -> incident/replay`, not drawing a DAG on a canvas.

The final product must provide all of the following. None are optional:

1. Rust nightly toolchain with mechanical lint gates.
2. First-party code forbids `unsafe`, `unwrap`, `expect`, `panic`, unchecked indexing, unchecked slicing, unchecked casts, unchecked arithmetic, ignored `Result`, and unbounded resources.
3. YAML authoring only through a strict parser and validator.
4. No runtime YAML, JSON, or HTTP in `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc`.
5. Compiled numeric workflow IR with `WorkflowId`, `StepIdx`, `SlotIdx`, `ExprIdx`, `ActionId`, `AccessorIdx`, `ConstIdx`, and bounded tables.
6. Handle-based runtime values using interned symbol/list/object/blob handles and finite numeric values.
7. Deterministic state-machine execution until suspension on action, wait, ask, retry, fanout join, queue admission, or storage policy boundary.
8. Shard-owned run state with bounded queues, bounded frame pools, bounded trace rings, bounded retries, bounded fanout, bounded expression stacks, bounded IPC frames, and bounded persistence batches.
9. Fjall persistence for workflow source, compiled IR, run headers, journal events, snapshots, blobs, and indexes.
10. Postcard encoding for internal journal, snapshot, IPC payload, and compiled artifact records.
11. Direct Rust API ingress for fastest local embedding.
12. Binary IPC ingress for external local processes.
13. IR-interpreter execution is the required runtime mode for the current milestone.
14. Typed validation, compile, runtime, IPC, and storage failures.
15. Benchmarked optimizations only; no speed claim without measured before/after data.
16. AI changes are accepted only with actual evidence that the relevant formatting, linting, tests, fuzzing, recovery, benchmark, and CI reproducibility gates ran and passed; merely adding or naming a task is not acceptance evidence. Dependency/supply-chain/API reports are advisory under the 2026-05-23 owner waiver unless a separate bead explicitly makes a specific report blocking.

HTTP/JSON exclusion rule: HTTP and JSON are excluded from the v1 runtime core. Any future adapter must be a separate cold-path adapter crate and must not enter `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc`.

---

## 1. Naming Contract

| Concept | Canonical spelling |
|---------|--------------------|
| Product | `velvet-ballistics` |
| Binary | `velvet-ballistics` |
| Cargo package | `velvet-ballistics` |
| Rust crate/module | `velvet_ballistics` |
| Bead rig | `velvet-ballistics` |
| Bead database | `velvet_ballistics` |
| Language version | `velvet-ballistics/v1` |

Mechanical rule: if an implementation agent introduces legacy capitalized product spelling or any non-canonical product/crate/database/language spelling in a new file, test, path, diagnostic, bead, package, crate, command, or generated artifact, the change is rejected unless the text explicitly documents migration from a pre-existing external artifact.

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

Dependency rule: third-party crates may contain internal unsafe only if pinned and justified by the repository dependency policy. `cargo-geiger`, `cargo-vet`, `cargo-deny`, and related supply-chain tools remain advisory reports under the 2026-05-23 owner waiver; their warnings do not block the current Backend / IR Interpreter Complete milestone unless a specific bead opts back into blocking enforcement.

---

## 3. Holzmann Compliance Matrix

| Holzmann rule | `velvet-ballistics` build contract |
|---------------|-------------------------------------|
| Simple control flow | Runtime transitions are explicit `StepIdx -> StepIdx`; no hidden graph mutation after compile. |
| Bounded loops | `for_each`, `collect`, `reduce`, `repeat`, retries, scheduler ticks, trace rings, storage batches, IPC frames, and expression stacks require explicit limits. |
| No dynamic allocation after init where avoidable | Current turbo-style backend paths preallocate or reserve frames, slots, step states, stacks, queues, trace rings, journal buffers, and IPC buffers before run admission. |
| Short functions | Hot functions must be <= 25 logical lines. Complex cold validation phase functions must be decomposed or carry a bead-linked justification and must stay out of hot paths. CI and Moon tasks must include a source-length gate that fails hot functions over 25 logical lines. |
| Assertions/contracts | User errors return typed errors. Debug assertions may check compiler invariants that are unreachable for validated IR. |
| Small scopes | Each run belongs to exactly one shard. Shards own mutable runtime state. No global mutable run map. |
| Checked parameters/returns | Parse, validate, compile, eval, storage, IPC, action dispatch, and scheduler return typed `Result`. |
| Restricted macros | No macro-hidden business logic in current backend crates. Codegen work is removed from current scope and cannot be used as release evidence. |
| Restricted pointer complexity | No first-party pointer manipulation. Tables are addressed by checked numeric IDs. |
| Zero warnings | CI denies first-party warnings, clippy violations, forbidden constructs, and missing benchmark metadata. Advisory dependency/supply-chain/API report warnings do not block release under the owner waiver unless a specific bead opts in. |

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
| `trybuild` | Compile-fail tests for public macro/schema contracts when such contracts are active. Generated Rust compile-fail testing is removed with codegen. |
| `cargo-audit` | Advisory vulnerability report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-deny` | Advisory license, duplicate, source, and advisory report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-vet` | Advisory supply-chain review report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-geiger` | Advisory unsafe dependency report; first-party unsafe remains forbidden by lint. |
| `cargo-machete` | Advisory unused dependency report. |
| `cargo-hack` | Feature powerset gate. |
| `cargo-semver-checks` | Advisory public compatibility report for released crates unless an API-stability bead opts in. |
| `cargo-public-api` | Advisory public API diff report unless an API-stability bead opts in. |
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
- Property/fuzz/compile diagnostics: `proptest`, `cargo-fuzz`, `arbitrary`, `trybuild` where active compile-fail contracts exist, and `insta` only when approved for golden diagnostics.
- Feature matrix: `cargo hack`.
- Advisory dependency/API reports: `cargo audit`, `cargo deny`, `cargo vet`, `cargo geiger`, `cargo machete`, `cargo semver-checks`, `cargo public-api`, and `cargo bloat`; these are non-blocking under the 2026-05-23 owner waiver unless a bead explicitly opts in.
- Performance: `criterion`, `iai-callgrind`, `flamegraph`, `samply`/`perf`, `hyperfine`, `callgrind`, `cachegrind`, and `DHAT` for current-scope IR-interpreter evidence. PGO, `target-cpu=native`, and maxperf release workflows are removed.
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
- Perf-only features: `allocator_api`, `generic_const_exprs`, restricted to `crates/*/src/perf/**`, `crates/*/src/generated/**`, `crates/workspace_tests/benches/**`, or a file carrying `velvet-allow-perf-nightly-feature` if the feature-gate script implements that marker exception.
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
| `trybuild` | Compile-fail tests | Required only for active public macro/schema contracts in the current milestone; generated Rust contracts are removed. |
| `cargo-nextest` | Test execution | Required CI test runner. |
| `cargo-audit` | Vulnerability scan | Advisory report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-deny` | Policy scan | Advisory report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-vet` | Supply-chain review | Advisory report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-geiger` | Unsafe dependency scan | Advisory report; first-party unsafe remains forbidden by lint. |
| `blake3` | Digest computation for envelopes and artifacts | Required for compiled digests, journal digests, blob digests. |
| `crc32c` | CRC32C header checksum for binary envelopes | Required for envelope header integrity. |

`crossbeam-queue::ArrayQueue` is required for bounded MPMC queues because capacity is fixed at construction and admission can fail without allocating. `rtrb` is required for SPSC trace/action rings where single-producer/single-consumer ownership gives predictable bounded behavior.

`serde` is allowed only for deriving binary/data schema serialization used by Postcard or cold diagnostics. `serde_json` is excluded from v1 runtime core.

`ordered-float` is not approved as the v1 `FiniteF64` implementation. `ordered_float::NotNan<f64>` rejects NaN but permits positive and negative infinity, while this language requires finite-only scalar values. Any future replacement must prove release-mode rejection of NaN and infinities, unchanged serialized representation, no panic/unwrap path, and no larger transitive footprint than the custom newtype.

---

## 6. Current Performance Rules — IR Interpreter Scope

The current performance goal is a fast, bounded IR-interpreter backend. Rust workflow code generation, generated-vs-IR ratio targets, `maxperf` acceptance, PGO release workflows, and public maximum-throughput claims are removed from the current contract.

Current rules:

1. `CompiledWorkflow` IR is the required runtime execution artifact.
2. Runtime state is numeric and handle-based.
3. Hot loops must use checked table access, bounded stacks, bounded queues, and preallocated or reservation-checked frame state.
4. Deterministic steps run synchronously inside the shard loop until suspension.
5. No async task is spawned per step.
6. No text formatting, YAML parsing, JSON parsing, HTTP handling, or string reference resolution on hot execution paths.
7. Any optimization must include before/after benchmark output, benchmark metadata, and no correctness regression.
8. `target-cpu=native`, PGO, and generated workflow execution are not current semantic or release-engineering requirements.
9. Runtime architecture is shard-owned, single-server, synchronous deterministic execution until suspension.
10. Data layout is hot/cold split: hot state has numeric IDs and handles; cold side tables carry spans, names, YAML paths, messages, and diagnostics.
11. Queues and scheduling use bounded `ArrayQueue`/`rtrb`, explicit backpressure, and no task-per-step spawning.
12. Persistence uses Postcard binary records and Fjall keyspaces with bounded writer queues and explicit durability modes.
13. Compilation resolves strings, references, actions, accessors, constants, branches, and resource contracts before run admission.
14. Turbo-style admission admits a run only after required slots, step states, expression stacks, frame space, trace space, journal buffers, IPC buffers, and queue commands are preallocated or reserved; deterministic transitions must not allocate after acceptance unless a documented resource contract permits it.

---

## 7. Nightly Governance

Nightly is required to target peak performance and strict lint behavior. It is not permission to use unstable APIs casually.

Nightly update contract:

1. Nightly version changes require a dedicated bead.
2. The bead must record current nightly, target nightly, motivation, changed compiler behavior, and rollback plan.
3. Full CI, Miri, fuzz smoke, benchmarks, and recovery tests must pass. Generated Rust compile tests are not current-scope gates.
4. Benchmark deltas must be recorded before and after the update.
5. Any new lint allowance requires explicit documented justification.

---

## 8. Language Specification

**Title:** velvet-ballistics Workflow Language v1
**Canonical version string:** `velvet-ballistics/v1`

Required top-level fields:

```yaml
version: velvet-ballistics/v1
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

v1 supports exactly these triggers in YAML authoring:

```yaml
when:
  manual: {}
```

```yaml
when:
  schedule:
    cron: "0 * * * *"
```

```yaml
when:
  event:
    type: github.pull_request
```

```yaml
when:
  webhook: {}
```

`manual` means direct Rust API submission (via `Runtime::submit`). `schedule`, `event`, and `webhook` are cold-path triggers handled by external adapters before submitting compiled artifacts to the runtime. HTTP/webhook adapters live outside `vb_core`, `vb_runtime`, `vb_storage`, and `vb_ipc`.

The binary IPC protocol (`vb_ipc`) is a separate runtime ingress mechanism, not a YAML trigger. `ipc` in the IR refers to the `ShardCommand::Submit` protocol, not a YAML-authored trigger.

---

## 10. Step Primitive Contract

Every YAML step has exactly one primitive:

```text
set · do · choose · for_each · together · collect · reduce · repeat · wait · ask · finish
```

**Canonical names.** The normative primitive names are `set · do · choose · for_each · together · collect · reduce · repeat · wait · ask · finish`. The implementation accepts these aliases:

| Alias | Canonical | Notes |
|-------|-----------|-------|
| `save` | `set` | Legacy alias in parser and compiler |
| `run` | `do` | Alternative step invocation |
| `foreach` | `for_each` | Single-word spelling in YAML parser |

These aliases are compiler-accepted; canonical names are preferred in authored YAML.

Control and metadata fields are not primitives:

```text
id · name · if · with · try_again · on_error · then
```

High-level YAML primitives may lower into multiple IR nodes. Runtime executes IR only in the current milestone.

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

Hot runtime state must not use `HashMap<String, Value>`, runtime state maps, dynamic object maps, or string-keyed lookup. Hot state uses numeric indices, handle tables, boxed slices, fixed-capacity stacks, bounded queues, and typed handles.

---

## 12. Forbidden Hot-Path APIs

The following are forbidden in hot runtime paths:

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

Nuance: these APIs are allowed in cold parser, validator, compiler, diagnostics, CLI, benchmark harness setup, and tests when covered by tests and kept out of hot runtime execution.

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

Compiler rule: high-level YAML primitives may lower to multiple IR nodes. Runtime executes IR only in the current milestone. Final choose IR has exactly two checked forms: `Choose` evaluates expression-branch conditions from `ExprIdx`, and `ChooseSlot` reads pre-materialized boolean conditions from `SlotIdx` values produced by earlier IR. Raw YAML condition strings and untyped choose nodes are forbidden in final IR.

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
| `EngineSignal` | `Continue`, `Finished(SlotValue, Taint)`, `StepBudgetExhausted`, `AwaitingAction`, `AwaitingWait`, `AwaitingAsk` |
| `StepBudget` | Bounded step counter. `try_take() -> CoreResult<bool>`. Budget 0 returns `StepBudgetExhausted` immediately. |
| `step_once` | Execute single node dispatch. Returns `EngineSignal`. |
| `drive_deterministic` | Loop calling `step_once` until blocked by budget, suspension, or finish. |

`StepBudget` uses `remaining: u64`; `try_take() -> CoreResult<bool>`. Budget `0` executes zero transitions and returns `StepBudgetExhausted`. Budget `1` executes exactly one transition.

`EngineSignal::Finished(SlotValue, Taint)` carries taint from the result slot. The Finish node reads slot taint and propagates it to the signal. Validation does not reject `Secret` or `DerivedFromSecret` finish results; runtime preserves the result-slot taint in the signal.

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

Current execution is through the IR interpreter only. Generated Rust execution is removed from active master scope.

**`Finish` taint contract:** The `Finish` IR node reads the taint from the result slot and emits `EngineSignal::Finished(SlotValue, Taint)`. Taint is joined from all slots contributing to the result. Runtime preserves `Clean`, `DerivedFromSecret`, and `Secret` result taints; validation does not reject tainted finish results.

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

Action names are resolved to `ActionId` during compile. The runtime dispatches by `ActionId` only. There is no `async_trait`, no dynamic string lookup, and no JSON input/output model.

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

Static dispatch shape:

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

Architectural decision: IPC v1 has exactly the 11 supported command identifiers
listed above. Their wire IDs are `1..=11` in that order. Every other `u16`
command value is reserved for future protocol versions and must decode as a
typed `UnknownCommand(value)`/equivalent and be rejected by dispatch. This
reserved range explicitly includes the former semantic query/verification IDs
`12..=16` (`ListRuns`, `GetMetrics`, `GetWorkflowGraph`, `GetTaintReport`, and
`VerifyWorkflow`); they are not supported IPC v1 commands unless a future master
contract revision assigns them explicit wire IDs and acceptance evidence.

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

## 22. Removed Rust Codegen and Maxperf

Rust workflow code generation is **out of the current core feature set**. The active product goal is backend execution through compiled IR and the IR interpreter.

Current command surface excludes `compile --emit rust`. Current acceptance excludes generated Rust semantic parity, generated compile-fail fixtures, generated-vs-IR ratio benchmarks, PGO release workflows, and `maxperf` release claims.

Historical notes live in:

```text
docs/generated-workflows.md
docs/deferred-codegen-maxperf.md
```

Codegen is not in current scope. Any future reintroduction requires a dedicated master amendment and cannot inherit acceptance credit from historical notes.

---

## 23. Workspace Structure

Target structure:

```text
velvet-ballistics/
  Cargo.toml
  rust-toolchain.toml
  clippy.toml
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
    vb_cli/
  fuzz/
  crates/workspace_tests/
```

Current state: the active backend workspace target is the underscore crate contract above (`vb_core`, `vb_yaml`, `vb_validate`, `vb_expr`, `vb_compile`, `vb_storage`, `vb_runtime`, `vb_ipc`, and `vb_cli`). Any future hyphenated internal crate name is a regression unless it is explicitly labeled as a migration artifact.

Removed crates: `vb_codegen`, `vb_ui_model`, and `vb_ui_makepad` are not active current-scope workspace requirements. They must not appear as active workspace members or current release gates.

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

## 32. Removed Function Surface: `vb_codegen`

`vb_codegen` is removed from current scope. No current-scope implementation bead may treat generated Rust workflow mode as required for acceptance.

---

## 33. CLI Commands

```bash
velvet-ballistics validate <workflow.yaml>
velvet-ballistics compile <workflow.yaml> --emit ir --out <file.vbir>
velvet-ballistics run <workflow.yaml> --input-bin <input.vbin> --durability <mode>
velvet-ballistics run-compiled <workflow.vbir> --input-bin <input.vbin> --durability <mode>
velvet-ballistics ipc-serve --socket <path> --db <path>
velvet-ballistics agent-context
velvet-ballistics inspect <run_id> --db <path>
velvet-ballistics events <run_id> --db <path>
velvet-ballistics replay <run_id> --db <path>
velvet-ballistics graph <workflow.yaml> --emit yaml
velvet-ballistics system status --emit yaml
velvet-ballistics action list --emit yaml
velvet-ballistics action inspect <action-name> --emit yaml
velvet-ballistics incident <run_id> --db <path> --emit yaml
velvet-ballistics ai context <run_id> --db <path> --emit yaml
velvet-ballistics bench-run <workflow.yaml>
velvet-ballistics doctor --db <path>
```

CLI structured output is a cold-path operator/agent contract and never enters `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc`. `--emit yaml` is the canonical structured text flag for v1; `--emit postcard` is the canonical binary machine-output flag where supported. JSON may be added later as a separate cold adapter. Runtime machine artifacts remain binary/Postcard.

The `ui` command and Makepad desktop application are removed from the current command surface.

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
  "crates/vb_cli",
  "crates/workspace_tests",
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

[profile.bench]
inherits = "release"
debug = true
lto = "thin"
codegen-units = 1
```

Removed workspace members and dependencies (`vb_codegen`, `vb_ui_model`, `vb_ui_makepad`, Makepad, generated workflow dependencies, and maxperf-only profile policy) must not be treated as current workspace acceptance requirements.

---

## 35. Implementation Phases

Phase build order is mandatory. The old giant primitive phase is rejected; every primitive family has its own implementation, test, fuzz, and benchmark beads.

| Phase | Name | Required delivery |
|-------|------|-------------------|
| -1 | Name/repo rebaseline | Canonical spelling, folder/package/crate/bead rebaseline, migration notes. |
| 0 | Toolchain/lints/CI/Moon | Nightly pin, hard lints, Moon tasks, and optional advisory supply-chain reporting skeleton. |
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
| 18 | Action ABI | Compile-time `ActionId`, ticket/outcome model, static numeric dispatch. |
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
| 31 | CLI | Validate, verify, compile IR, explain, diff, simulate, run, run-compiled, submit, replay, inspect, events, incident, IPC serve, action/system/doctor/AI context, bench-run. |
| 32 | Full recovery/replay | Digest mismatch detection, full primitive replay, non-idempotent policy. |
| 33 | Full benchmark suite | Criterion/iai suites, metadata, IR interpreter latency/throughput, storage, IPC, direct API, scheduler. |
| 34 | Hardening | Full gates, sanitizer jobs, fuzz expansion, docs, bead evidence, Backend / IR Interpreter Complete readiness. |
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

| Area | Round 2 state | Remaining gap before backend DoD |
|------|---------------|--------------------------------|
| Naming/workspace | Canonical crate layout and package spelling are represented in the workspace. | Mechanical spelling gates and bead evidence still decide acceptance for future changes. |
| Core/value/IR | `vb_core` exposes numeric IDs, handle-based `SlotValue`, `ValueStore`, taint/state APIs, bounded expression/accessor evaluation, resource contracts, and deterministic transition surfaces. | Full final primitive semantics still require end-to-end compiler/runtime/replay evidence. |
| YAML/validation/compile | Strict YAML parsing, AST validation, reference/control/type-taint checks, slot/accessor/constant APIs, digesting, artifact emission, and mandatory lowering function surfaces exist. | Source-to-IR lowering must be proven for the full v1 primitive set, not only constructor/API coverage. |
| Expression engine | Lexer/parser/typecheck/bytecode surfaces exist with bounded execution contracts. Store-aware helper implementations exist for the current interpreter surfaces. | Helper type/evaluator parity, F64 mixed/coercion behavior, and mutation resistance still require gate evidence. |
| Storage/recovery | `vb_storage` exposes required keyspace names, key encoders, record envelope encode/decode, journal writer queue, snapshots, replay helpers, recovery summaries, and frame-seed hydration for slot values/taint/step states. | Pending-action hydration, strict persistence-before-ack behavior, digest mismatch coverage, and end-to-end crash recovery evidence remain release gates. |
| Runtime/direct API | `vb_runtime` exposes direct API, shard/frame-pool/action/wait/ask/trace/counter surfaces, admission/capability surfaces, and typed runtime errors. | Strict persistence-before-ack behavior, shutdown/cancellation edge cases, pending-action recovery, and full lifecycle evidence remain gates. |
| IPC | `vb_ipc` exposes bounded frame/header/payload validation, typed payloads, memory ingress, client/server surfaces, and required command handlers. | Socket-loop fuzz/backpressure evidence and runtime integration gates remain required. |
| Removed codegen/UI | `vb_codegen`, `vb_ui_model`, and `vb_ui_makepad` are removed from active workspace scope. | They are not current acceptance gates and must not block Backend / IR Interpreter Complete. |
| Tests/audits | Error-variant completeness and diagnostic-code range tests exist; companion docs record benchmark and dependency policy constraints. | Full matrix gates, fuzz, Miri, coverage, mutants, sanitizer, benchmark metadata, and bead closure evidence are still required. Supply-chain/dependency reports are advisory under the owner waiver. |

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
- Secret-tainted finish results preserved in `Finished(SlotValue, Taint)`.
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
- Active public macro/schema contracts reject invalid usage at compile time when such contracts exist.
- Generated Rust compile-fail tests are removed with `vb_codegen`.

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

---

## 39. Mandatory Benchmarks

**Benchmark naming:** Exact benchmark names are not mandated. Benchmarks must exist covering the following areas. The authoritative benchmark directory is `crates/workspace_tests/benches/`; the migrated aggregate benchmark entry is `crates/workspace_tests/benches/velvet_ballistics.rs`.

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
execution mode (`ir-interpreter` for the current milestone)
p50/p95/p99 latency
instruction counts
allocation count
bytes allocated
Fjall write latency
direct API latency
IPC latency
```

Acceptance rule: no speed claim without benchmark numbers. No optimization PR without before/after benchmark output and correctness evidence. Compileable Criterion scaffold benchmarks are placeholders only; no-op scaffolds such as `black_box(())` prove the harness builds, not that the implementation is faster, lower allocation, lower latency, or production ready.

---

## 40. CI Gate

Required Moon tasks:

```text
check
test
feature-powerset
miri
coverage
mutants-smoke
bench-build
source-length
fuzz-smoke
```

CI must gate on `moon ci`, whose pipeline must include `check`, `test`, `fuzz-smoke`, `miri`, `coverage`, `mutants-smoke`, `bench-build`, `source-length`, and `feature-powerset`. Nightly sanitizer jobs are required for runtime, IPC, storage, and binary decoding crates. The `source-length` task must fail any hot runtime function over 25 logical lines. Advisory supply-chain reporting may exist as a Moon task, but supply-chain/advisory report warnings are non-blocking under the 2026-05-23 owner waiver unless a future bead explicitly opts in.

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
cargo hack check --feature-powerset --workspace
cargo llvm-cov --workspace --all-features
cargo mutants --in-place --timeout 60 --package vb_core
cargo fuzz build
```

Advisory report commands, non-blocking under the owner waiver unless a bead opts in:

```bash
cargo audit
cargo deny check
cargo vet
cargo geiger
cargo machete
cargo semver-checks check-release
cargo public-api diff
cargo bloat --release --crates
```

Moon expectation: each mandatory command above must have a Moon task before release, and the release gate must run through Moon rather than a hand-maintained shell script. Advisory report tasks may run in Moon, but warnings from those reports cannot block current release closure without a bead-specific opt-in.

---

## 41. Removed PGO and Maxperf Build

PGO, `target-cpu=native`, `maxperf`, and generated Rust benchmark workflows are removed. They do not block the current Backend / IR Interpreter Complete milestone and must not be current release gates.

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
cli
observability
tests-fuzz
benchmarks
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

Codegen and UI gaps are cleanup debt unless a bead explicitly deletes or quarantines residue. They are not reactivation tracks in the current master scope.

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

UI beads are limited to deletion or quarantine of residue unless the master scope is explicitly amended.

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
11. Tests added.
12. Benchmarks added.
13. Commands run.
14. Remaining follow-up work filed as beads.
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
one task per step
no tests for new code
speed claim without real benchmark baseline/result evidence
new velvet-ballistics spelling outside the exact allowlist
```

---

## 44. Backend / IR Interpreter Definition of Done

The current `velvet-ballistics` backend milestone is done when all 24 points are satisfied:

1. Canonical spelling is enforced for product, binary, package, crate/module, bead rig, bead database, and language version.
2. Any legacy capitalized product spelling outside explicitly labeled pre-existing external migration artifacts is rejected. The active repository root path is `/home/lewis/src/velvet-ballistics`; the active master filename is repo-root `velvet-ballistics-MASTER.md`.
3. Every primitive validates, compiles, runs, persists, recovers, and replays.
4. v1 supports both `manual` direct API submission and `ipc` binary IPC submission.
5. Runtime never interprets YAML and recovery never reparses YAML for existing runs.
6. JSON and HTTP are absent from `vb_core`, `vb_runtime`, `vb_storage`, and `vb_ipc`.
7. Runtime state uses numeric workflow, run, action, step, slot, expression, accessor, constant, and sequence IDs.
8. Action dispatch uses numeric `ActionId`; no runtime string action lookup exists.
9. Hot values use handle-based `SlotValue` with `SymbolId`, `ListId`, `ObjectId`, `BlobId`, and finite numbers.
10. Each run is owned by exactly one shard; no global mutable run map exists.
11. Queues, stacks, buffers, retries, fanout, timers, traces, batches, IPC frames, and resource contracts are bounded.
12. Turbo-style admission preallocates or reserves hot resources; deterministic transitions allocate nothing after acceptance unless a documented resource contract permits it.
13. Fjall stores workflow source, compiled IR, run headers, journals, snapshots, blobs, and indexes with magic/schema/version/kind/length envelopes.
14. Recovery and replay detect workflow, action, and policy digest mismatch and fail typed without default substitution.
15. Direct API implements submit, inspect, cancel, list events, answer ask, complete action, fail action, drain trace, health, and shutdown equivalents.
16. Binary IPC implements `SubmitRun`, `SubmitRunInline`, `CancelRun`, `InspectRun`, `ListEvents`, `AnswerAsk`, `CompleteAction`, `FailAction`, `DrainTrace`, `Health`, and `Shutdown`.
17. IR-interpreter execution covers every active final IR node and is the accepted execution mode.
18. Diagnostics include stable code, path, source span, message, and cold side-table context.
19. Validation, compile, runtime, storage, IPC, action, and replay failures are typed and graceful.
20. Forbidden constructs are absent: `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, ignored `Result`, runtime maps, hot formatting, runtime YAML/JSON/HTTP, and string reference/action lookup.
21. Unchecked indexing, slicing, casts, and arithmetic are absent from first-party code.
22. Every speed claim is backed by real benchmark evidence with p50/p95/p99, instruction counts, allocation counts, bytes allocated, latency, durability mode, and fixture metadata; compileable scaffold placeholders do not count.
23. Full current-scope gates pass: fmt, clippy hard denies, tests, nextest, Miri, coverage, fuzz smoke, mutants smoke, feature powerset, docs, benchmark build, storage/recovery evidence, IPC evidence, and direct API evidence. Supply-chain/dependency unsafe reports are advisory under the owner waiver unless a bead opts in.
24. Every phase parent bead, function-cluster child bead, fuzz target bead, benchmark bead, and P0 blocker bead in the current backend scope is closed with evidence, and mechanical gates can accept AI changes without human guesswork only when the relevant executable checks, tests, benchmarks, and bead evidence have actually run and passed.

---

## 45. Normative Runtime Semantics

Every `CompiledNodeKind` variant has exact behavior defined here. The current IR interpreter must produce the specified terminal result, typed error variant and fields, final pc, slot values, slot taints, step states, journal event sequence, action tickets, retry counts, wait/ask scheduling, and replay behavior.

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

Core engine signals: `Continue`, `Finished(SlotValue, Taint)`, `StepBudgetExhausted`, `AwaitingAction`, `AwaitingWait`, `AwaitingAsk`.

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
| Slots written | `node.output` with the joined taint of expression operand slot reads |
| Taint | `join_taint` over loaded expression operand slot taints |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed, expression stack depth ≤ 64, expression ops ≤ 256 |
| Errors | `ExprOutOfBounds`, `MissingOutputSlot`, `MissingNextStep`, any `ExprError` (stack overflow, type mismatch, division by zero, integer overflow) |

#### BuildObject { fields: Box<[(SymbolId, SlotIdx)]> }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Each field's slot value and taint |
| Slots written | `node.output` with `SlotValue::Object(ObjectId)` and joined field taint |
| Taint | `join_taint` over field slot taints |
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
| Inputs read | Each item's slot value and taint |
| Slots written | `node.output` with `SlotValue::List(ListId)` and joined item taint |
| Taint | `join_taint` over item slot taints |
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
| Inputs read | `answer` slot value and taint |
| Slots written | `node.output` with answer value and answer taint (if output is Some) |
| Taint | Propagated from answer slot |
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

`ExprType::F64`, `SlotValue::F64(FiniteF64)`, `ConstValue::F64`, `ExprLiteral::F64`, expression float lexing/parsing, bytecode constant lowering, and F64/F64 evaluator arithmetic/comparison arms exist. Strict YAML scalar floats remain forbidden by the YAML profile; float values enter authored workflows through expression strings, runtime slot initialization, or action outputs. Remaining gap: the typechecker still accepts broader numeric coercion than the evaluator. Mixed I64/F64 arithmetic and evaluator/typechecker parity remain current-scope expression evidence gaps. Generated F64 arithmetic semantics and codegen lint parity are removed with `vb_codegen`.

### Helper Signatures

| Helper | Arity | Input types | Return | Implementation status |
|--------|-------|-------------|--------|-----------------------|
| `exists` | 1 | Any | Bool | Implemented: `!matches!(value, Null)` |
| `length` | 1 | List or Null | I64 | Implemented store-aware for symbols/lists/objects/null; no-store helper reports context-required for handles. |
| `count` | 1 | List or Null | I64 | Implemented as count/length over store-aware list values. |
| `empty` | 1 | List or Null | Bool | Implemented store-aware for symbol/list/object/null emptiness. |
| `unique` | 1 | List | List | Implemented store-aware list deduplication preserving first occurrence order. |
| `contains` | 2 | List, T | Bool | Implemented in current evaluators as store-aware Symbol substring search; list-membership/spec parity evidence remains open. |
| `starts_with` | 2 | Symbol, Symbol | Bool | Implemented store-aware text helper; generated-mode behavior is removed with `vb_codegen`. |
| `ends_with` | 2 | Symbol, Symbol | Bool | Implemented store-aware text helper; generated-mode behavior is removed with `vb_codegen`. |
| `has` | 2 | Object, Symbol | Bool | Partially converged: `vb_expr` implements object-field lookup, while the core hot evaluator currently uses list membership semantics; helper parity evidence remains open. |
| `append` | 2 | List, T | List | Implemented store-aware list append. |
| `append_if` | 3 | List, T, Bool | List | Implemented store-aware conditional append. |
| `merge` | 2 | Object, Object | Object | Implemented store-aware object merge; typechecker returns `Object`; interpreter/runtime parity evidence remains open. |
| `sum` | 1 | List | I64 | Implemented store-aware I64 list sum with overflow rejection; arity remains 1. |

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
| `EvalExpr` | Output taint is the join of expression operand slot taints. |
| `BuildObject` | Output taint is the join of field slot taints. |
| `BuildList` | Output taint is the join of item slot taints. |
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

Generated workflow code is removed from current scope; any residue must be deleted or quarantined.

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

`maxperf` is removed and is not a current runtime profile requirement.

---

## 57. Feature Flag Policy

- Default features: none (all code always compiled).
- `bench` feature: enables benchmark-only harness code.
- `volatile` feature: enables volatile storage mode (test-only).
- Forbidden features: `json`, `http` in v1 runtime crates.
- `generated` and `maxperf` are removed and must not be current default or release features.

---

## 58. Platform Support

v1 supported target: `x86_64-unknown-linux-gnu`. Unix domain sockets required. Other targets are non-release experimental unless a dedicated portability bead adds evidence.

---

## 59. Security and Threat Model

### Trusted Components

Compiled IR, Fjall database, runtime engine.

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

v1 runtime core must not depend on `tokio`, `async-std`, `smol`, `futures` executors, `async_trait`, or async task scheduling. `mio` is the only approved low-level eventing mechanism for IPC. Actions may block only in bounded action worker contexts or return `Suspended`. No async function may appear in `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc`.

---

## 63. Plan Verifier and Accepted Artifacts

### Core Principle

AI may propose workflows. `velvet-ballistics` verifies them. Only accepted artifacts run.

The compiler does not merely check syntax. It acts as a safety gate: if `velvet-ballistics` cannot prove the plan is bounded, inspectable, retry-safe, and durable, the plan is rejected before execution. No accepted workflow has unknown bounds.

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
    pub workflow_version: &'static str,  // "velvet-ballistics/v1"
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
velvet-ballistics verify flow.yaml --profile strict --emit yaml
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
| 7. Boundedness | Implemented, evidence-gated | `WholeWorkflowBudget`/`BoundednessPolicy` exist and `vb_compile` calls shared validation; full release evidence still required. |
| 8. Resource budget | Implemented, evidence-gated | `ResourceContract`, whole-workflow computation, arena caps, `BudgetExceeded`, and hard step-budget ceilings exist; full gate evidence still required. |
| 9. Action contract | Partial | `ActionContract`, `SideEffect`, `RetrySafety`, idempotency checks, and action contract validation surfaces exist; external attestation/schema parity evidence remains required. |
| 10. Secret/taint | Implemented | Compile-time + runtime taint, leak rejection, 3-level lattice |
| 11. Idempotency | Implemented, evidence-gated | `Idempotency`, `SideEffect`, `RetrySafety`, `IdempotencyViolation`, and verifier/runtime admission plumbing exist; generated/replay parity evidence remains a release gate. |
| 12. Durability | Partial | Journal events, per-primitive durability matrix, and `SlotWritten` value/taint evidence exist; pending-action recovery and strict ack ordering remain gates. |
| 13. Capability | Implemented, evidence-gated | `Capability`/`CapabilitySet` types and runtime admission enforcement exist; schema/CLI/e2e parity evidence remains required. |
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

### DRIFT-1: Runtime Taint Tracking — RESOLVED

**Resolution evidence:**
- `EvalExpr` reads taint from all `LoadSlot` operands and joins into output taint (`crates/vb_expr/src/eval.rs`).
- `BuildObject` joins taint from all field slots into output taint.
- `BuildList` joins taint from all item slots into output taint.
- `Finish` node reads slot taint and emits `EngineSignal::Finished(SlotValue, Taint)` (`crates/vb_core/src/nodes.rs`, `node_helpers.rs`).
- `EngineSignal::Finished` carries taint alongside value in the result signal.
- Compile-time taint validation remains as defense-in-depth.

**DRIFT-1 is closed.**

### DRIFT-2: Crash Recovery Cannot Reconstruct Live State — PARTIALLY RESOLVED

**Original defect:** Earlier builds recorded no slot values, no slot taint, and no step lifecycle events (`StepStarted`/`StepSucceeded`) for deterministic steps. After a crash, `UnsupportedRecoveryState` could report `slot_values: true`, `slot_taint: true`, while hydration still proceeded with empty frames. The system was not crash-recoverable for workflows that performed deterministic computation between suspension points.

**Current evidence:** `SlotWrittenEvent` can carry encoded `SlotValue` and taint evidence; `RecoveryFrameSeed` reconstruction applies recovered slots and step states; `DurableFrameRecoveryBoundary::hydrate_run_frame` rejects unsupported live-frame state instead of silently producing a broken frame. Closed beads `vb-x0mt`, `vb-9fy4`, and `vb-vs7k` record Phase 44/recovery hydration work.

**Remaining gap:** Pending action hydration remains gated as unsupported when unresolved actions are present, summary-only hydration still returns `UnsupportedFullRecoveryHydration`, and `UnsupportedAsyncStrictAck` still marks strict async acknowledgement limitations. Release acceptance still requires end-to-end crash recovery evidence for all live recovery paths.

**Root cause:** Journal events are only emitted at suspension points (action dispatch, wait, ask). Deterministic steps between suspensions are treated as atomic but the journal cannot reconstruct them.

**Resolution contract:**
1. Every deterministic step must emit `SlotWritten` events (value + taint) to the journal before advancing PC.
2. `StepStarted`/`StepSucceeded` events must be emitted for every step, not just suspension points.
3. Recovery must reconstruct slot values and taint from journal events.
4. `UnsupportedRecoveryState` must gate hydration: if `slot_values == true`, hydration must fail with a typed error, not produce a broken frame.
5. Journal error handling in shard.rs must not use `Ok(()) | Err(_) => {}`. Journal write failures must propagate as runtime errors or at minimum log a diagnostic.

**Performance note:** Emitting `SlotWritten` per deterministic step increases journal write volume. Under `Journaled` durability, these batch via the writer queue. Under `Strict`, each step gets an fsync — this is the correct safety tradeoff. `Volatile` mode remains zero-journal for testing.

**Coding style:** No async. No channels. Synchronous journal append within the shard's single-threaded drive loop. Bounded writer queue absorbs burst. If queue is full, the step blocks (backpressure), not silently drops.

**Resolves in:** Phase 44 (Recovery Evidence Chain); remaining live pending-action recovery evidence is still a release gate.

### DRIFT-3: No Aggregate Resource Budget Across Primitive Composition — RESOLVED

**Original defect:** Individual primitive bounds existed (`ForEach limit`, `Together branches`, `Repeat max_attempts`) but their composition was unbounded. `ForEach(limit=1000)` wrapping `Together(branches=256)` could create 256,000 sequential step executions and 256,000 ValueStore arena entries in a single run. The `ValueStore` had no cap on total arena entries (symbols, lists, objects, blobs are all append-only with no GC).

**Root cause:** Bounds are per-primitive, not per-run. No dataflow analysis propagates bounds through nested compositions. `ResourceContract` defaults (`max_fanout: u16::MAX`, `max_collect_items: u32::MAX`, `max_step_budget_per_tick: u64::MAX`) are effectively unbounded.

**Resolution contract:**
1. Phase 37 (Whole-Workflow Boundedness) computes `WholeWorkflowBudget` from IR — this resolves the static analysis gap.
2. `ValueStore` must have a per-run arena cap (e.g., `max_arena_entries: u32`). Insert methods must check the cap and return a typed error on overflow.
3. `ResourceContract` defaults must be tightened from `u16::MAX`/`u32::MAX`/`u64::MAX` to policy-specified defaults.
4. `StepBudget` per tick must have a hard ceiling (e.g., 100,000) regardless of configuration.
5. Collect global `Mutex<Vec>` must be replaced with per-run pagination state to eliminate cross-run interference.

**Resolution evidence:** Phase 37 whole-workflow budget computation is represented in `vb_core::budget` and called from `vb_compile`; Phase 45 resource enforcement added `ValueStore` arena caps, `BudgetExceeded`, tightened defaults, and hard `StepBudget` ceilings. Closed beads `vb-u7vj`, `vb-i9sn`, and `vb-qwdn` record this work.

**DRIFT-3 is closed, subject to normal full-gate evidence refresh.**

### DRIFT-4: IR Validation Is Bounds-Only, Not Structural — RESOLVED

**Original defect:** `try_from_parts` validated that numeric indices were within array bounds but did not validate structural correctness: reachable nodes, forward-only edges, well-formed loop structures, valid `SymbolId` references, or accessor path segment validity. A postcard-deserialized artifact from untrusted input could bypass compiler-level structural validation.

**Root cause:** The compiler's structural validations (control flow, reference, type/taint) operate on the AST, not on the compiled IR. They are never re-checked at the IR level.

**Resolution contract:**
1. `try_from_parts` must validate that every node is reachable from `entry`.
2. `try_from_parts` must reject backward edges (Jump targets, Choose targets, loop body/done targets must be forward).
3. `try_from_parts` must validate that loop primitives are paired correctly (ForEachStart has a matching ForEachNext and ForEachJoin).
4. `try_from_parts` must validate that BuildObject SymbolIds are within the symbol table range.
5. `try_from_parts` must validate AccessorProgram path segments (Field SymbolId range, Index bounds).
6. The artifact loading path (`run-compiled` CLI command) must treat the artifact as untrusted input.

**Coding style:** Straightforward `for` loops over nodes. Checked indexing. No recursion (bounded by node count). Each check returns a typed `IRValidationError` identifying the specific node and check that failed.

**Resolution evidence:** `CompiledWorkflow::try_from_parts` now calls structural validators for accessor path symbols, reachability, and forward edges; workflow tests exercise unreachable-node, invalid edge, and accessor validation. Closed beads `vb-honk` and `vb-w1ww` record Phase 46 completion.

**DRIFT-4 is closed, subject to normal full-gate evidence refresh.**

### DRIFT-5: Validation Logic Duplicated Between vb_validate and vb_compile — PARTIALLY RESOLVED

**Defect:** Both `vb_validate` and `vb_compile` contain parallel modules (schema, references, control_flow, type_taint) that must be kept in sync manually. The two crates operate on different input types (document model vs AST) but enforce the same rules.

**Root cause:** Historical. `vb_validate` was built first on the document model. `vb_compile` was built later with its own validation on the AST. Both must accept the same workflow language.

**Resolution contract:**
1. Single validation pipeline on a shared intermediate representation.
2. Both crate public APIs preserved for backward compatibility.
3. Internal delegation to one implementation.
4. Remove the sync requirement.

**Coding style:** No traits, no generics, no higher-order functions. A plain `pub fn validate(parts: &WorkflowParts) -> Result<ValidationOutput, ValidationError>` that each crate calls.

**Current evidence:** `vb_compile` delegates compiled `WorkflowParts` validation through `vb_validate::shared::validate` / `validate_with_contracts`, re-exports validation errors, and shares reference validation via `vb_validate::references::RefTables` and `validate_single_reference`. Closed bead `vb-2pp9` records the Phase 42 reference/shared-parts deduplication work.

**Remaining gap:** Source-level schema/control-flow/type-taint modules still exist in both crates because they operate on different input representations. DRIFT-5 is not fully closed until the remaining duplicated source validation paths are either removed, proven equivalent by contract-as-data tests, or explicitly documented as representation-specific wrappers over one shared implementation.

**Resolves in:** Phase 42 (Validation Deduplication) plus remaining validation parity evidence.

---

## 68. Durable Execution Architecture Contract

> **Target contract.** The invariants in this section describe the intended architecture. Current implementation has frame-seed hydration for recovered slot values, taint, and step states, but live pending-action hydration and strict async acknowledgement paths remain gated. Summary-only recovery still returns `UnsupportedFullRecoveryHydration`, and `UnsupportedAsyncStrictAck` remains in the code until strict durability acknowledgement evidence is complete.

`velvet-ballistics` is a log-first durable execution engine. The architecture follows the same core model as production-grade orchestrators (AWS Step Functions): journal events are the ground truth, state is deterministically derived from the journal, and side effects are never re-executed without explicit idempotency proof.

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

`velvet-ballistics` is a single-server engine. There is no distributed replication, no leader election, no quorum consensus, and no control plane. These are explicit v1 exclusions:

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

Unlike orchestrators that interpret journal entries against SDK code (opaque foreign processes), the current `velvet-ballistics` milestone compiles workflows to numeric IR and executes that IR through the interpreter:

| Mode | Execution | When to Use |
|------|-----------|-------------|
| IR interpreter | Dispatch through `CompiledNodeKind` enum | Current backend execution, debugging, portability, replay validation |

Generated Rust is removed from the current execution model. IR interpreter is the only accepted execution mode.

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
4. Validation does not reject tainted finish results; `Clean`, `DerivedFromSecret`, and `Secret` finish taints are preserved in the result signal.
5. Action output taint must be at least as restrictive as input taint for `DeterministicPure` and `IdempotentExternal` actions.
6. `AtLeastOnceExternal` actions propagate conservatively as `DerivedFromSecret` when any input is tainted.
7. Secret-tainted failure details must not enter public diagnostics without redaction.

---

## 69. Operator CLI Contract

The CLI is the primary interface for operators and AI agents. It must provide the same operational affordances as mature orchestrators without cargo-culting their branding.

### Canonical Command Surface

```text
velvet-ballistics validate <workflow.yaml>
velvet-ballistics compile  <workflow.yaml> --emit ir --out <file>
velvet-ballistics explain  <workflow.yaml> [--emit yaml|postcard]
velvet-ballistics diff     <workflow.yaml> [--against <old.yaml>] [--emit yaml|postcard]
velvet-ballistics run      <workflow.yaml> --input-bin <file> --durability <mode> [--db <path>]
velvet-ballistics run      <workflow.yaml> --step <step-id> --step-input <file> [--durability <mode>]
velvet-ballistics run-compiled <workflow.vbir> --input-bin <file> --durability <mode> [--db <path>]
velvet-ballistics inspect <run-id> --db <path> [--emit yaml|postcard]
velvet-ballistics events  <run-id> --db <path> [--emit yaml|postcard] [--step <id>] [--tail <n>] [--limit <n>]
velvet-ballistics trace   <run-id> --db <path> [--emit yaml|postcard]
velvet-ballistics replay  <run-id> --db <path> [--emit yaml|postcard]
velvet-ballistics cancel  <run-id> --db <path>
velvet-ballistics resume  <run-id> --db <path>
velvet-ballistics retry   <run-id> --step <step-id> --db <path>
velvet-ballistics answer  <run-id> --slot <slot-id> --value <file> --db <path>
velvet-ballistics ipc-serve --socket <path> --db <path>
velvet-ballistics graph <workflow.yaml> --emit yaml
velvet-ballistics system status --emit yaml
velvet-ballistics action list --emit yaml
velvet-ballistics action inspect <action-name> --emit yaml
velvet-ballistics incident <run-id> --db <path> --emit yaml
velvet-ballistics ai context <run-id> --db <path> --emit yaml
velvet-ballistics bench-run <workflow.yaml>
velvet-ballistics doctor  --db <path> [--emit yaml|postcard]
```

The only supported CLI binary name is `velvet-ballistics`. Short aliases such as
`vb` are not part of the canonical interface and must not be added as Cargo bin
targets.

There is no `ui` command or native Makepad command center in the current contract.

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

`--emit yaml` produces machine-readable structured text. `--emit postcard` produces machine-readable binary output where supported. JSON is not canonical for v1 and must not be hand-formatted into the runtime binary.

### Semantic Diff

`diff <workflow.yaml>` compares a workflow against its previously compiled version:

- Textual diff: YAML source changes (line-level)
- Semantic diff: changes in step count, control flow graph, resource contracts, secret usage, action contracts, retry policies
- Digest comparison: if a compiled artifact exists in the DB, compare BLAKE3 digests
- Exit codes: 0 = no semantic changes, 1 = semantic changes detected, 2 = error
- `--emit yaml` for machine-readable output

### Structured Observability

Output format flags:
- `--emit text` for human-readable output (default)
- `--emit yaml` for structured text output (`inspect`, `explain`, `diff`, `doctor`, `events`, `trace`, `replay`)
- `--emit postcard` for binary machine output where the command returns a typed artifact

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
- Machine-readable output (`--emit yaml` and, where applicable, `--emit postcard`) is mandatory for every reporting command. AI agents must be able to parse output without screen-scraping.

### Agent-First CLI Principles

Underlying idea: agents are primary CLI users, not tolerated secondary users. The CLI must reduce token burn, retries, and hidden failure modes by making command shape introspectable, mutation boundaries explicit, and consistency mechanically enforced. Review-only consistency is rejected as Swiss-cheese control; schema, codegen, static checks, or generated context must carry the policy.

The CLI contract must preserve these ten principles:

1. Non-interactive by default. Commands must never wait on an unanswered prompt under non-TTY execution. Any destructive bypass flag is `--force`; `--skip-confirmations` and equivalents are banned.
2. Structured parseable output. Every data-returning/reporting command supports `--emit yaml`; typed artifact commands may additionally support `--emit postcard`. Data goes to stdout, diagnostics go to stderr, and ANSI is suppressed for non-TTY output.
3. Errors that teach and enumerate. Enum validation errors must include the valid set and, where useful, the corrective invocation shape. Parse failures occur before side effects.
4. Safe retries and explicit mutation boundaries. Mutations return stable identifiers, destructive operations require explicit flags, retryable submissions use durable idempotency keys or existing run/job discovery, and consequential commands grow `--dry-run` before release.
5. Bounded responses at every layer. List/event/report commands default to bounded output with `--limit`/cursor/filter narrowing, and MCP/tool/agent descriptions stay under an audited token budget.
6. Cross-CLI vocabulary consistency. CRUD resource verbs are `get`, `list`, `create`, `update`, `delete`; banned aliases include `info`, `ls`, `--format=json`, `--output=json`, and `--skip-confirmations`. Domain-specific verbs require documented justification and static checks.
7. Three-layer introspection. Human `--help`, versioned machine `agent-context`, and long-form skill/workflow guidance must describe the same implementation surface and be validated against it.
8. Async-aware execution. Any async submission gains `--wait` with bounded backoff/jitter and a durable local job ledger exposed through `jobs list`, `jobs get`, and `jobs prune` before async APIs are release-grade.
9. Persistent identity through profiles. Repeated agent invocations use named profiles with precedence `explicit flag > environment variable > profile > default`; available profiles are surfaced in `agent-context`.
10. Two-way I/O. Artifact-producing commands support `--deliver` sinks (`stdout`, `file:<path>`, `webhook:<url>`) with atomic file writes and structured refusal on unknown schemes. `feedback <text>` records local JSONL and optionally posts upstream when configured; availability is exposed in `agent-context`.

Mechanical enforcement required before release:

- `velvet-ballistics agent-context` emits a versioned JSON schema with command names, flags, enums, exit codes, output conventions, and planned agent primitives.
- CI runs `scripts/check-agent-cli-contract.sh` through Moon to reject banned parser vocabulary and require the introspection surface.
- Any generated CLI/schema pipeline must generate the CLI, agent context, skill manifest, and MCP/tool descriptions from one source; hand-written divergence is a release blocker.

---

## 70. Phase Extension: Operator Features

The following phases extend Section 35 for operator-facing features:

| Phase | Name | Required delivery |
|-------|------|-------------------|
| 50 | Single-step testing | `run --step <id>` with input payload, isolated execution, step result reporting. Tests: step resolution, minimal frame construction, step_once execution, output reporting. |
| 51 | Explain / dry-run | `explain` command with step graph, resource contract, suspension points, secrets usage, `--emit yaml` output. Tests: explain output matches compiled IR, YAML format validation. |
| 52 | Durable lifecycle controls | `cancel`, `resume`, `retry`, `answer` CLI commands. Strict distinction between retry-step, replay-run, and resubmit-workflow. Tests: each lifecycle command against journaled runs, cancelled runs, suspended runs. |
| 53 | Semantic diff | `diff` command with textual + semantic diff, digest comparison, exit codes. Tests: diff detects step changes, resource contract changes, secret changes. |
| 54 | Structured observability | `--emit yaml`/`--emit postcard` flags, filter flags (`--step`, `--tail`, `--limit`, `--since`). Tests: structured output parses correctly, filter flags narrow results. |
| 55 | Timer wheel | Replace `IndexMap<RunId, PendingTimer>` with `TimerWheel` backed by `BTreeMap<Instant, Vec<TimerEntry>>`. Automatic timer-driven resume in shard tick. Tests: timer firing, cancellation, next-deadline accuracy. |
| 56 | Collect hardening | Per-run pagination state (replace global Mutex), time-based pagination limit, `RunId`-keyed state. Tests: concurrent collect runs, time limit enforcement, crash-recovery of pagination state. |
| 57 | Recovery evidence chain | `SlotWritten` + `StepSucceeded` per deterministic step, `UnsupportedRecoveryState` hydration gate, fix stubbed `verify_digests` at `Full` level. Tests: crash recovery with full evidence chain, hydration failure on missing state. |
| 58 | Codegen residue removal | Delete or quarantine codegen stubs, tests, proof residue, and generated-mode references. |
| 59 | Behavioral property tests | Current-scope properties from Section 38: constant folding parity, bytecode/AST parity, digest stability, layout stability, replay determinism, snapshot equivalence, ordering invariants, bound enforcement, state machine, and taint safety. |
| 60 | Canonical CLI binary | Cargo.toml exposes only the canonical `velvet-ballistics` binary. Short aliases such as `vb` are rejected to preserve the naming contract. |
| 61-74 | UI residue removal | Delete or quarantine UI/Makepad/Figma/snapshot/perf-gate residue. |

---

## 71. Competitive Performance Targets

The following are internal engineering targets for `velvet-ballistics` as a single-server engine. They are not public performance claims, but no external claim is allowed until the measurement contract below is satisfied.

### Step-Level Latency Targets

| Metric | velvet-ballistics (single-server) | Notes |
|--------|-----------------------------------|-------|
| Single step p50 (no replication) | <= 1ms | No network roundtrip for quorum |
| Single step p50 (journaled) | <= 5ms | Fjall group commit |
| Single step p50 (strict) | <= 10ms | fsync on every step |
| Full workflow p50 (9 steps, low load) | <= 15ms | Compiled IR, no SDK roundtrip |
| Full workflow p50 (9 steps, high load) | <= 60ms | Single-server removes coordination overhead |
| Full workflow p99 (9 steps, high load) | <= 100ms | Tight bound from no-unsafe, checked arithmetic |

### Throughput Targets

| Metric | velvet-ballistics | Notes |
|--------|-------------------|-------|
| Full workflows per second (9 steps) | >= 10,000 | Single-server removes replication overhead |
| Concurrent active runs | >= 4,096 | Frame pool capacity |

### Why These Targets Are Achievable

`velvet-ballistics` eliminates replication overhead:
1. No replication — local Fjall write
2. No leader — single shard owns the run
3. No SDK — action dispatch is a function call within the same process
4. No async — synchronous deterministic loop
5. No competing flush — Fjall writes happen through bounded writer queue, not in the hot path

Generated Rust performance advantages are out of scope. Current speed claims must be scoped to the IR interpreter.

### Measurement Contract

Every performance claim must include:
- `criterion` or `iai-callgrind` output with p50/p95/p99
- Hardware: CPU model, cores, RAM, disk type (NVMe vs SSD)
- Build profile: debug, release, bench for current scope; maxperf/PGO removed
- Execution mode: IR interpreter only
- Durability profile: volatile, journaled, strict
- Number of concurrent runs
- Benchmark fixture digest (reproducible)

---

## 72. Execution Attempt Tracking

When a run fails and is retried, the engine must reject stale events from previous execution attempts. This prevents split-brain between overlapping retries.

### Contract

1. Every run attempt gets a monotonically increasing `attempt: u16` counter.
2. `ActionTicket` carries the `attempt` number.
3. On retry, the attempt counter increments. Any `ActionCompleted`/`ActionFailed` event carrying a stale attempt number is rejected with `StaleAttempt { expected, found }`.
4. Journal events are tagged with the attempt number.
5. Recovery replays events for the latest attempt only. Events from earlier attempts are ignored.
6. The attempt counter is journaled as part of `RunAccepted` and persists across crashes.

This provides invocation execution attempt tracking for single-server synchronous execution.

---

## 73. Journal Trimming

The journal cannot grow indefinitely. After a snapshot is taken, journal events older than the snapshot are eligible for trimming.

### Trimming Contract

1. A snapshot captures the full run state at `SeqNo` N.
2. Once a snapshot at N is confirmed durable (fsynced), all journal events with `SeqNo <= N` for that run are eligible for deletion.
3. Trimming must not delete events for runs that have no snapshot.
4. Terminal runs (finished/failed/cancelled) are eligible for trimming after their final snapshot, subject to a retention policy (default: keep last N terminal runs per workflow).
5. The `doctor` command must report journal size and suggest trimming if the journal exceeds a configured threshold.

This prevents unbounded disk growth in long-running production deployments.

---

## 74. Converged Binary Design

`velvet-ballistics` ships as a single binary that operates in different modes depending on the command invoked. This converged single-binary design is adapted for single-server operation.

### Modes

| Command | Binary Role | Components Active |
|---------|-------------|-------------------|
| `run` | Executor | Compiler + Engine + Storage |
| `run-compiled` | Executor | Engine + Storage |
| `validate` | Validator | YAML Parser + Validator |
| `compile` | Compiler | YAML Parser + Validator + Compiler + IR artifact writer |
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

If `velvet-ballistics` ever supports distributed operation (v2+), the binary gains additional roles (log-server, controller, ingress) but the converged model persists: a single binary, configured by role, no separate services to deploy.

---

## 75. AI-Native CLI Control Plane

The CLI is the AI-native control plane for humans and AI agents to operate, verify, repair, replay, and explain the system now.

North star:

1. Anything an adapter can show, the CLI must be able to emit as structured data first.
2. Anything an operator can inspect, an AI agent can inspect safely.
3. Anything that fails produces a machine-readable explanation.

### Dual-Personality Design

The CLI has two modes of output:

**Human mode** — Pretty, readable, fast:

```text
velvet-ballistics verify workflow.yaml
velvet-ballistics run issue_triage --input input.vbin
velvet-ballistics inspect run_123
velvet-ballistics replay run_123
```

Output is colored, summarized, and ergonomic.

**AI mode** — Stable, structured, boring:

```text
velvet-ballistics verify workflow.yaml --emit yaml
velvet-ballistics inspect run_123 --emit yaml
velvet-ballistics replay run_123 --explain --emit yaml
velvet-ballistics incident run_123 --emit yaml
```

No fragile pretty text. No hidden state. No "look at the dashboard." AI mode emits schemas that are documented and versioned.

### Lifecycle Command Surface

Command groups mirror the system lifecycle:

```text
velvet-ballistics validate workflow.yaml
velvet-ballistics verify   workflow.yaml
velvet-ballistics compile  workflow.yaml
velvet-ballistics graph    workflow.yaml
velvet-ballistics simulate workflow.yaml
velvet-ballistics run-compiled workflow.vbir
velvet-ballistics submit   issue_triage
velvet-ballistics inspect  run_123
velvet-ballistics events   run_123
velvet-ballistics replay   run_123
velvet-ballistics incident run_123
velvet-ballistics action list
velvet-ballistics action inspect github.issue.create
velvet-ballistics system status
velvet-ballistics doctor
velvet-ballistics ai context run_123
```

The CLI is not just "run workflow." It is a compiler/debugger/operator interface.

### verify Is the Hero Command

`verify` is the flagship. It answers: *is this workflow safe to run, and if not, what must change?*

```text
velvet-ballistics verify workflow.yaml --profile strict
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
schema_version: velvet-ballistics/cli-output/v1
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
schema_version: velvet-ballistics/cli-output/v1
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
velvet-ballistics explain workflow.yaml --emit yaml
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
velvet-ballistics graph workflow.yaml --emit yaml
```

Emits a graph artifact consumable by AI reasoning, CLI summaries, and documentation generators. One source, many consumers.

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
velvet-ballistics simulate workflow.yaml --input input.vbin --mocks mocks.yaml --emit yaml
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
velvet-ballistics submit issue_triage --input-bin input.vbin --emit yaml
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
velvet-ballistics inspect run_123 --emit yaml
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
velvet-ballistics events run_123 --tail 20 --emit yaml
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
velvet-ballistics replay run_123 --explain --emit yaml
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
velvet-ballistics incident run_123 --emit yaml
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
velvet-ballistics action list --emit yaml
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
velvet-ballistics action inspect github.issue.create --emit yaml
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
velvet-ballistics doctor --emit yaml
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
velvet-ballistics ai context run_123 --emit yaml
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
  - velvet-ballistics replay run_123 --explain --emit yaml
  - velvet-ballistics events run_123 --tail 50 --emit yaml
  - velvet-ballistics verify workflow.yaml --profile strict --emit yaml
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

### Future CLI-UI Parity Rule

No future UI-only truth. If a future UI shows taint graphs, replay timelines, action tickets, queue pressure, certificate status, or incident repair, the CLI must expose it first.

Backend emits typed artifacts:

- `VerificationReport`
- `WorkflowGraph`
- `RunInspection`
- `RunEvents`
- `ReplayReport`
- `IncidentReport`
- `SystemStatus`
- `ActionDescription`

CLI is the current view over those artifacts. Any future UI must consume the same data.

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
11. Future UI consumes the same data after reactivation

Build CLI before any future UI. The UI must not invent concepts — it visualizes proven backend artifacts.

### The Killer Demo

```text
velvet-ballistics verify issue-triage.yaml --profile strict --emit yaml
velvet-ballistics simulate issue-triage.yaml --input example.vbin --mocks mocks.yaml --emit yaml
velvet-ballistics submit issue_triage --input-bin prod.vbin --emit yaml
velvet-ballistics incident run_123 --emit yaml
```

Then hand the output to an AI and ask: *What failed, is it safe to retry, and what should I change?* If the AI can answer correctly from the CLI packet, the design works.

---

## 76. Workflow Command-Center Front-End

> **Removed.** The command-center front-end is not part of the current Backend / IR Interpreter Complete milestone. The remaining section content is historical residue only and not an implementation contract.

### Vision

The `velvet-ballistics` front-end is a premium native command center for workflow execution observability, verification, replay, and incident response. It is not a generic SaaS dashboard, not a low-code canvas, and not a decorative graph editor.

The UI product identity is:

> Step Functions observability, but cleaner, sharper, calmer, and more cinematic — a workflow black-box recorder inside an Apple-quality native desktop app.

The UI visualizes operational truth already produced by the backend: `VerificationReport`, `WorkflowGraph`, `RunInspection`, `RunEvents`, `ReplayReport`, `IncidentReport`, `SystemStatus`, `ActionDescription`, storage health, journal evidence, action tickets, resource budgets, and taint paths. The UI does not invent state and does not become a second source of truth.

### Design Direction

The v1 UI uses a crisp Apple Pro-style light shell:

- Ultra-clean off-white surfaces.
- Matte white cards.
- Faint translucent glass panels only where useful.
- Hairline dividers instead of heavy borders.
- Soft, realistic shadows.
- Rounded 14–20px cards.
- Precise 8px spacing rhythm.
- Minimal, high-signal color use.
- Crisp sans-serif typography for labels.
- Monospace only for run IDs, action IDs, digests, slot IDs, timestamps, sequence numbers, and binary/record metadata.
- No cyberpunk treatment, no overuse of neon, no overuse of glass, no thick borders, no 3D effects, and no generic web-dashboard chrome.

The UI may borrow broad observability structure from AWS Step Functions-style execution pages — execution summary, graph/table/event views, selected-step details, event history, recovery controls — but it must reinterpret these into the `velvet-ballistics` product model: accepted artifacts, verification certificates, typed journals, replay safety, idempotency evidence, and taint/resource contracts.

### Presentation Board and Figma Contract

The current canonical intake bundle is the 2026-05-08 23:51 zip at `/home/lewis/Downloads/velvet_ballistics_makepad_ui_master_plan_with_images.zip`. Its extracted repository copy is:

```text
velvet_ballistics_makepad_ui_master_plan_with_images/
  velvet-ballistics-MASTER-makepad-ui-update.md
  design_assets/canonical/
    figma_makepad_notes.md
    velvet_ballistics_figma_ready_tightened_board.png
  design_assets/velvet_ballistics_figma_ready_tightened/png/
  design_assets/velvet_ballistics_figma_ready_tightened/svg/
```

The design review artifact is an 8-screen desktop board:

```text
1. Execution Observatory Overview
2. Workflow Graph Authoring
3. Execution Details Graph View
4. Verification Certificate View
5. Replay Theater
6. Incident Failure Console
7. Action Registry / Contract Inspector
8. Storage / Journal Doctor + AI Context
```

Reference design assets live under:

```text
design/figma/
design/reference/
design/tokens/
```

Figma files, SVGs, and PNG boards are design reference only. The implementation source of truth is Makepad Splash (`script_mod!`) plus Rust widget code. Any design token divergence between Figma and Makepad is a release blocker.

### Shared App Chrome

Every screen uses one shared shell:

- Left sidebar with `velvet-ballistics` branding.
- Minimal icon navigation: Overview, Workflow Graph, Executions, Verification, Replay, Incidents, Actions, Storage, AI Context, Settings.
- Top action bar with compact capsule buttons: Verify, Simulate, Submit.
- Status chips: Strict durability, Running, Verified, Replay safe, Needs operator.
- Top right utility controls: profile/environment selector, local server status, notification indicator, optional command palette trigger.

Shared app chrome must be implemented once as `AppShell`, not copied per screen.

### Color System

Use color only for state and meaning:

| Meaning | Color role |
|--------|------------|
| Verified / succeeded / healthy | Green |
| Running / active / selected | Blue or cyan |
| Retry / warning / queue pressure | Amber |
| Failed / critical incident | Red |
| Taint / secret-sensitive path | Purple |
| Durable / replay-safe | Teal |
| Disabled / pending / muted | Gray |

The default UI is calm white, gray, and black. Accent color should appear as small chips, thin outlines, dots, timeline marks, node glows, graph packet markers, and status text. Large colored surfaces are reserved for rare success/failure banners and must stay visually restrained.

### Screen 1 — Execution Observatory Overview

Purpose: answer what is running, where pressure is building, and whether the local system is healthy.

Required elements:

- KPI row: Active runs, Healthy actions, Verification pass rate, Queue depth, Open incidents.
- Simplified executions table: run id, workflow, status, started, duration, shard, result.
- Shard flow map: shard lanes, tiny packet dots moving through active executions, queue pressure marks, action completion lane, timer lane.
- Event ticker: last N events, `RunAccepted`, `StepStarted`, `ActionScheduled`, `ActionCompleted`, `RunFinished`, `RunFailed`.
- System health cards: local server online, Fjall store healthy, writer queue health, IPC socket status.

Style: calm, spacious, operational, less dense than the Step Functions console or the previous dark reference board.

### Screen 2 — Workflow Graph Authoring

Purpose: show the compiled workflow graph as a structured projection of YAML/IR, not as a freeform whiteboard.

Required elements:

- State palette on the left: Start, Action, Branch, Parallel, Wait, Subflow, Finish.
- Center graph canvas: `Start`, `classify`, `route_issue`, `create_issue`, `notify_slack`, `build_result`, `Finish`.
- Node cards: matte white card, status dot, primitive/action label, small badges for strict-safe, idempotency, taint, retry, timeout.
- Edges: thin curved lines, tiny packet markers, branch labels, selected path emphasis.
- Right step inspector: step name, primitive, action id, resource impact, input slots, output slot, retry policy, idempotency key, taint state.
- Selected node: crisp blue outline, subtle glow, no large blue fill.

YAML source remains authoritative. The canvas may support structured editing only if edits round-trip through the parser/compiler/validator.

### Screen 3 — Execution Details Graph View

Purpose: inspect one active or past run in graph mode.

Required elements:

- Run summary: run id, workflow name, status, started timestamp, shard id, durability profile.
- Runtime graph: succeeded nodes in green, selected/running node in blue, pending nodes muted gray, failed node red outline, secret/taint overlay purple only when active.
- Event table below graph: seq, time, step, event, shard, evidence id.
- Right step details panel: step name, action id, action type, attempt, started time, elapsed, idempotency key hash, input tab, output tab, details tab.

This screen is the closest structural analog to Step Functions execution details, but it must show velvet-native concepts: journal evidence, action tickets, taint, slots, replay safety, and artifact digests.

### Screen 4 — Verification Certificate View

Purpose: pre-flight safety certificate for accepted artifacts.

Required elements:

- Green restrained banner: `Verification passed` or equivalent failure banner.
- Certificate cards: Structure, Boundedness, Resources, Taint / Secrets, Action policy, Durability, Idempotency, Capability.
- Horizontal verification gate pipeline: Parse, Graph check, Policy, Resources, Taint, Durability, Idempotency, Capability, Result.
- Accepted artifact side panel: artifact version, workflow version, workflow digest, IR digest, action ABI digest, policy digest, verified timestamp, warnings.
- Proof summary: bounded, taint safe, retry safe, durable, replayable.

This screen must feel like a safety certificate, not an analytics dashboard.

### Screen 5 — Replay Theater

Purpose: the hero screen. A premium black-box recorder for deterministic workflow replay.

Required elements:

- Runtime graph on the left or center.
- Journal timeline: event dots by sequence number, selected event highlight, scrubber position, jump to failure, jump to action, jump to divergence.
- Playback controls: back, play/pause, step forward, replay speed, live/frozen mode.
- Selected event panel: seq, timestamp, shard, step, event kind, evidence id, digest summary.
- Slot diff table: slot id, before, after, taint before, taint after.
- Recovery decision panel: strategy, max attempts, idempotency requirement, apply/replay action.

This screen should feel like a video editor or flight recorder: calm, precise, replayable, and cinematic. Motion is implied by packet dots, scrubber state, event pulses, and graph overlays.

### Screen 6 — Incident Failure Console

Purpose: incident diagnosis and safe recovery.

Required elements:

- Red restrained banner: `ACTION_TIMEOUT at create_issue`, run id, action id, attempt, timestamp.
- Compact chips: `Safe to retry: YES`, `Same idempotency key required`, `Strict durability`, `Replay safe`.
- Failure path graph: failure node red outline, failure path focus, muted non-failure nodes.
- Evidence chain: scheduled durable, completion durable, side-effect certainty, journal tail.
- Recovery controls: retry same key, schedule retry, cancel run, open replay.
- Action ticket panel: ticket id, action id, attempt, owner, rollback/retry metadata.
- Slot and taint diff panels.
- Repair hints: check API status, verify token scope, increase timeout, retry with backoff.

Do not flood the screen with red. Red is reserved for the failure node, banner accent, and critical text.

### Screen 7 — Action Registry / Contract Inspector

Purpose: inspect registered native actions, numeric `ActionId` mappings, contracts, capabilities, idempotency, retry safety, and schema/digest metadata.

Required elements:

- Action list: name, action id, side effect class, idempotency, retry safety, strict safe, required capability.
- Selected action inspector: `ActionContract`, input slot count, output slot count, max input bytes, max output bytes, timeout ms, idempotency classification, side effect classification, retry safety, action ABI digest.
- Capability panel: required permissions, granted permissions, missing permissions.
- Failure code panel: `RateLimited`, `Timeout`, `PermissionDenied`, `InvalidInput`, `ExternalUnavailable`.
- Example call view: no JSON, no HTTP core routing, typed binary/postcard schema summary.

### Screen 8 — Storage / Journal Doctor + AI Context

Purpose: storage health, journal evidence, replay readiness, and AI-safe operational context.

Required elements:

- Storage health: Fjall keyspaces, writer queue, journal batch health, snapshot status, blob store status, index health.
- Journal doctor: run event count, snapshot seq, tail seq, corrupt record status, trim recommendation, digest checks.
- AI context packet: safe for model, secrets redacted, blobs summarized, suggested next commands, failure summary, replay safety.
- Evidence card: last cert check, last replay check, last crash lab fixture, incomplete evidence warnings.

### AI Companion Panel

The AI panel is not a generic chat sidebar. It receives structured artifacts only:

- `WorkflowGraph`
- `VerificationReport`
- `RunInspection`
- `RunEvents`
- `ReplayReport`
- `IncidentReport`
- `SystemStatus`
- `ActionDescription`
- `AiContextPacket`

Prompts are action buttons, not open-ended chat by default:

- Explain this failure.
- Is this safe to retry?
- Show secret-sensitive paths.
- Explain strict-durability failure.
- Generate minimal repro.
- Suggest bounded retry policy.
- Summarize what changed since last good run.

AI output must cite graph nodes, journal events, slot diffs, action tickets, certificates, or diagnostics. AI output must never rely on hidden UI state.

### UI Build Order

| UI Phase | Deliverable | Why first |
|----------|-------------|-----------|
| UI-1 | `vb_ui_model` typed artifacts | Shared truth for CLI and UI. |
| UI-2 | Makepad app shell and design tokens | Common chrome, spacing, color, typography. |
| UI-3 | Replay Theater | Exercises hardest event/timeline/graph mapping first. |
| UI-4 | Verification Certificate View | Product differentiation and accepted-artifact proof surface. |
| UI-5 | Execution Details Graph View | Step Functions-style observability with velvet-native evidence. |
| UI-6 | Incident Failure Console | Operational recovery path. |
| UI-7 | Execution Observatory Overview | Macro health after per-run views work. |
| UI-8 | Workflow Graph Authoring | Structured graph projection and editing. |
| UI-9 | Action Registry / Storage Doctor / AI Context | Operator completeness. |
| UI-10 | Motion/perf/snapshot gates | Release readiness. |

The backend and CLI remain higher priority than decorative UI polish. UI concepts cannot introduce product states not emitted by backend artifacts.

---

## 77. AI-Safe Quality Infrastructure

AI changes must be small, checkable, replayable, benchmarked, and hard to merge when wrong. The closed loop is:

```
spec -> task -> patch -> mechanical checks -> evidence -> benchmark -> certificate -> merge
```

AI agents must not guess which checks to run. Every quality gate is exposed as a first-party `xtask` command that returns structured machine-readable output. No evidence bundle means no merge.

### 77.1 xtask Command Center

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

### 77.2 Three Check Levels

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
moon ci
```

Supply-chain/advisory reports are non-blocking under the 2026-05-23 owner waiver unless a future bead explicitly opts in.

The maxperf lane is removed and is not part of current release closure.

### 77.3 Evidence Bundles

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

### 77.4 Machine-Readable Invariants

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

### 77.5 Semantic Banned Scans

Token-level grep is necessary but insufficient. The quality infrastructure uses multiple scan layers:

| Layer | Tool | Checks |
|-------|------|--------|
| Token scan | ripgrep | `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, forbidden imports |
| Clippy denies | `clippy` | `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::arithmetic_side_effects`, etc. |
| Dependency unsafe advisory | `cargo geiger` | Transitive unsafe in dependencies; non-blocking under owner waiver |
| AST scanner | syn-based custom (`xtask forbidden-scan`) | Unchecked indexing, slicing, `as` casts, ignored `Result`, `HashMap<String, _>` in runtime, `serde_json` in runtime, HTTP crates in runtime |
| Public API diff advisory | `cargo public-api` | Accidental public contract changes; non-blocking unless an API-stability bead opts in |
| Allocation scanner | `xtask hotpath-scan` | `format!`, `println!`, `Vec::push` without pre-reserve, `String` construction in hot paths |

AI often satisfies the literal rule while violating the intent. Multi-layer scanning catches this.

### 77.6 AI Context Packets

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

### 77.7 Spec-to-Test Mapping

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

### 77.8 Property Tests, Fuzz Harnesses, and Proof Targets

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

### 77.9 Global Verifier Tooling Stabilization

Formal verification work must not be parallelized across many beads until the shared verifier substrate is stable. If multiple beads fail on the same Kani, Flux, Verus, TLA+, proptest, or fuzz tooling issue, the global tooling defect is fixed once before more bead agents are launched.

The approved execution pattern is five beads per wave, with one isolated workspace per bead. Do not run fifteen proof agents against one shared proof/tooling state.

Required verifier tooling baseline before proof-heavy bead waves:

| Tooling lane | Required baseline |
|--------------|-------------------|
| Kani | All bulky or stale `#[cfg(kani)]` harness groups are isolated behind package features. Package-specific Kani listing uses `bash scripts/kani-list.sh <package> [...]`, never root `cargo kani list --format json` as proof evidence. Unrelated Kani modules must not compile for a bead lane unless their feature is explicitly enabled. |
| Flux | Commands use `bash scripts/flux-check-package.sh <package>` or `cargo flux -p <package> --message-format human`. Unsupported target flags such as `--lib`, `--test`, `--tests`, `--benches`, and `--all-targets` are invalid. A package smoke pass is not proof unless the named refinement artifact is wired into the checked crate or checked by an explicit approved single-file Flux command. |
| Verus | Verus evidence uses `bash scripts/verify-verus.sh` for registry-driven obligations or `verus --crate-type=lib <file>` for one-off checks. Standalone algebra models are not production proof unless the proof artifact is explicitly bound to implementation behavior through source references, `requires`/`ensures`, bridge mapping, and raw verifier success logs. |
| TLA+ | TLA+ commands must use an available `tlc` wrapper or an absolute path to the installed `tla2tools.jar`. Commands that assume repository-local `tools/tla2tools.jar` or missing `verification/tla+` directories are invalid until those paths exist. Specs must model bounded hardware limits and error transitions, not unbounded `Nat` success paths. |
| proptest/fuzz | Proptest commands must execute real property tests and report nonzero applicable tests. Fuzz commands must target names present in `cargo fuzz list`, use the fuzz workspace conventions, and select a compatible target triple when sanitizer/libc constraints require it. Orphan fuzz files are not valid targets until registered in `fuzz/Cargo.toml`. |

Wave execution contract:

1. Create an isolated parent directory for proof waves, for example `/home/lewis/isolated/velvet-ballistics-proof-waves/`.
2. Create one subdirectory per wave and one isolated bead workspace per bead.
3. Run at most five bead agents per wave.
4. Keep one controller lane responsible for global tooling fixes and validator interpretation.
5. Do not start the next wave until repeated global blockers from the current wave are fixed or explicitly waived by a bead-linked decision.
6. Archive stale rejected review artifacts before rerunning earlier states.
7. Recompute invocation-ledger hashes only after real artifact repairs; never use ledger repair as proof evidence.
8. Promote a bead only when validator output, proof-review status, and raw command evidence agree.

Recommended proof waves for the current blocked verifier campaign:

| Wave | Beads | Purpose |
|------|-------|---------|
| 1 | `vb-4c1k`, `vb-kd9p`, `vb-v0bm`, `vb-eepg`, `vb-u8gi` | Exercise Kani, Flux, Verus, fuzz, and proptest tooling without starting from the heaviest missing-TLA IPC cluster. |
| 2 | `vb-8mdp.12`, `vb-8mdp.7`, `vb-8mdp.8`, `vb-klz0`, `vb-t6hx` | Address IPC/TLA/Kani-heavy proof closures after baseline tooling is normalized. |
| 3 | `vb-7m21`, `vb-om21`, `vb-aoah`, `vb-wfi4`, `vb-dybj` | Close remaining proof-review rejects and tooling-dependent beads. |

If all five agents in a wave report the same tooling failure, stop bead-local repair and fix the global verifier substrate first. More agents are not a substitute for a stable proof harness.

### 77.10 Mutation Testing as AI Correctness Check

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

### 77.11 Differential Testing

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

### 77.12 Crash/Recovery Lab

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

### 77.13 Performance Regression Gates

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

### 77.14 Allocation Tracing Gates

For hot paths, performance is not just time — it is allocations. Tests run hot transitions with an allocation counter.

Rules:
- `RunFrame` admission may allocate
- Deterministic transitions in turbo/maxperf must not allocate
- IPC decode must not allocate before payload length validation
- Expression eval must not allocate stack memory dynamically

Command: `cargo xtask alloc-check --suite hotpath`

### 77.15 Public API Diff Gate

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

### 77.16 Supply-Chain Policy

AI may not add a dependency without a dependency-scope bead that includes:

1. Why the dependency is needed
2. Which handwritten code it replaces
3. Hot-path impact assessment
4. Unsafe/geiger result
5. License status
6. Audit/vet status
7. Rollback plan

This stops "AI added 14 crates because convenient." Existing tools `cargo audit`, `cargo deny`, `cargo vet`, `cargo geiger`, and `cargo machete` enforce this.

### 77.17 Structured Patch Review

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

### 77.18 Rustdoc Examples as Executable Contracts

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

### 77.19 Trybuild Compile-Fail Suites

For active public macro/schema contracts, compile-fail tests pin policy. Generated-code trybuild suites are removed with `vb_codegen` and are not current-scope tests.

### 77.20 Minimal Repro Generator

When fuzz, property test, or crash lab fails, generate a tiny repro:

```bash
cargo xtask repro shrink --failure logs/failure.yaml
```

Output: `repros/ipc_bad_header_0007.bin`, `repros/workflow_replay_divergence_001.yaml`

Then: `cargo xtask repro run repros/workflow_replay_divergence_001.yaml`

Effective for AI repair loops — the agent gets the smallest possible failing case.

### 77.21 Contracts as Data

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

Current-scope generators may produce Rust enums, docs, CLI schemas, AI context, and tests from these sources. UI schemas and generated workflow code are removed from current scope. Contracts-as-data reduce drift because AI reasons from the same source that generates active code and documentation.

### 77.22 Failure Explanation

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

### 77.23 AI Patch Protocol

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

### 77.24 AI-Safe Code Zones

Code is marked by zone. Scanning rules vary by zone.

| Zone | Marker | Rules |
|------|--------|-------|
| `hot-runtime` | `// velvet-zone: hot-runtime` | No allocation, no formatting, no `HashMap<String, _>`, no dynamic dispatch |
| `cold-compiler` | `// velvet-zone: cold-compiler` | `HashMap` allowed, `format!` allowed in diagnostics |
| `generated` | `// velvet-zone: generated` | Compile-fail policy enforced, no `unsafe`, no `unwrap` |
| `storage-decode` | `// velvet-zone: storage-decode` | No allocation before length validation, fuzz coverage required |
| `test` | `// velvet-zone: test` | Relaxed rules, but must use typed assertions |

This prevents blanket rules from blocking useful code in cold paths.

### 77.25 Golden Internal Models

Executable reference models live in `reference/`:

| File | Purpose |
|------|---------|
| `reference/engine_model.rs` | Slow but clearly correct engine semantics |
| `reference/taint_model.rs` | Taint propagation reference |
| `reference/replay_model.rs` | Replay/recovery reference |
| `reference/resource_model.rs` | Resource bound reference |

Differential tests assert: optimized runtime == reference model.

AI modifies optimized code while the reference model keeps semantics pinned.

### 77.26 Perf Annotations for Hot Functions

Hot functions carry local rules that `xtask hotpath-scan` enforces:

```rust
// velvet-hot-path: no-alloc, no-format, max-lines=25
fn step_once(...) -> CoreResult<EngineSignal> {
    ...
}
```

Scanner checks: line count, allocation absence, formatting absence, bounded resource use. AI knows the local rules before editing.

### 77.27 AI Context for Spec-to-Implementation

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

---

## 78. Makepad UI Implementation Contract

> **Removed.** Makepad UI implementation is not part of the current core feature set. Remaining details in Sections 78-83 are historical residue only; no current backend bead may be blocked by Makepad, UI model artifacts, screenshot gates, or UI perf gates.

### Makepad Scope

Makepad is used only for the native UI crate `vb_ui_makepad`. It is forbidden in:

```text
vb_core
vb_runtime
vb_storage
vb_ipc
```

Makepad dependencies must not change runtime semantics, binary IPC semantics, or persistence semantics.

### Makepad Rationale

Makepad is selected for the UI because the design requires a native, GPU-driven desktop application with highly interactive graph, timeline, animation, and custom-rendered visual states. The UI uses Makepad 2.0 Splash (`script_mod!`) for layout/style iteration and Rust widgets for deterministic state handling.

### Crate Roles

| Crate | Role | Runtime-core dependency? |
|------|------|--------------------------|
| `vb_ui_model` | Typed UI artifacts shared by CLI/UI. No Makepad. | Cold path only |
| `vb_ui_makepad` | Native Makepad desktop app. | UI only |
| `velvet_ballistics` | CLI command dispatch, including `ui`. | Cold path command |

### `vb_ui_model` Required Types

```rust
pub enum UiScreenKind {
    ExecutionOverview,
    WorkflowGraphAuthoring,
    ExecutionDetailsGraph,
    VerificationCertificate,
    ReplayTheater,
    IncidentFailureConsole,
    ActionRegistry,
    StorageDoctorAiContext,
}

pub struct UiAppSnapshot {
    pub status: SystemStatusView,
    pub active_runs: Box<[RunSummaryView]>,
    pub selected_run: Option<RunInspectionView>,
    pub selected_workflow: Option<WorkflowGraphView>,
    pub verification: Option<VerificationReportView>,
    pub replay: Option<ReplayReportView>,
    pub incident: Option<IncidentReportView>,
    pub actions: Box<[ActionDescriptionView]>,
    pub storage: Option<StorageDoctorView>,
    pub ai_context: Option<AiContextView>,
}
```

All UI model structs must use bounded collections. Any list returned to the UI must carry a limit/cursor or a fixed bound. Unbounded UI lists are forbidden.

### Data Flow

```text
Compiler / verifier
  -> WorkflowGraph, VerificationReport, AcceptedArtifact
  -> vb_ui_model
  -> Makepad UI

Runtime / storage / replay
  -> RunInspection, RunEvents, ReplayReport, IncidentReport, SystemStatus
  -> vb_ui_model
  -> Makepad UI
```

The UI consumes typed artifacts. It does not parse YAML, does not execute workflows, does not resolve references, and does not dispatch actions by string.

### UI Connection Modes

| Mode | Command | Data source | Purpose |
|------|---------|-------------|---------|
| Embedded | `velvet-ballistics ui --db <path>` | Direct storage/runtime readers | Local desktop app with DB access |
| Attached | `velvet-ballistics ui --socket <path>` | Binary IPC | Operator app connected to running server |
| Demo | `velvet-ballistics ui --demo-fixture <fixture>` | Deterministic fixtures | Design review, screenshot tests, demos |

HTTP and JSON are not required for the UI. If a future streaming adapter is needed, it must be a separate cold-path adapter crate.

### Makepad Structure

Required module structure:

```text
crates/vb_ui_makepad/src/
  app.rs
  shell.rs
  theme.rs
  tokens.rs
  data.rs
  screens/
    execution_overview.rs
    workflow_graph_authoring.rs
    execution_details.rs
    verification_certificate.rs
    replay_theater.rs
    incident_failure.rs
    action_registry.rs
    storage_doctor_ai_context.rs
  widgets/
    app_shell.rs
    status_chip.rs
    metric_card.rs
    graph_canvas.rs
    graph_node.rs
    graph_edge.rs
    packet_dot.rs
    timeline_scrubber.rs
    event_table.rs
    slot_diff_table.rs
    certificate_card.rs
    evidence_card.rs
    action_ticket_card.rs
    taint_overlay.rs
    shard_flow_map.rs
    ai_context_panel.rs
  motion/
    timeline.rs
    easing.rs
    bounded_animation.rs
```

### Makepad 2.0 Splash Rules

Makepad Splash (`script_mod!`) must be used for layout, static style, theme tokens, and component composition. Rust code handles typed state, event routing, selection, filtering, and artifact binding.

Required pattern:

```rust
use makepad_widgets::*;

app_main!(App);

script_mod! {
    use mod.prelude.widgets.*

    startup() do #(App::script_component(vm)) {
        ui: Root {
            main_window := Window {
                window.inner_size: vec2(1920, 1080)
                body +: {
                    app_shell := AppShell {
                        // Sidebar, top action bar, and routed screen content.
                    }
                }
            }
        }
    }
}

impl App {
    fn run(vm: &mut ScriptVm) -> Self {
        crate::makepad_widgets::script_mod(vm);
        App::from_script_mod(vm, self::script_mod)
    }
}

#[derive(Script, ScriptHook)]
pub struct App {
    #[source]
    source: ScriptObjectRef,
    #[live]
    ui: WidgetRef,
    #[rust]
    state: UiRuntimeState,
}
```

Business state is Rust-owned and typed. Splash values may not become implicit workflow state. Old Makepad 1.x macro-based examples are not the implementation contract for this repository.

### Custom Widgets

The following custom widgets are required:

| Widget | Purpose |
|--------|---------|
| `GraphCanvas` | Pan/zoom workflow and runtime graphs. |
| `GraphNode` | Draw node cards with status, badges, selection, taint state. |
| `GraphEdge` | Draw curved edges, branch labels, packet markers. |
| `PacketDot` | Animated progress packets along edges. |
| `TimelineScrubber` | Replay timeline with event dots and selected seq. |
| `CertificateCard` | Verification proof card. |
| `StatusChip` | Compact semantic status display. |
| `EvidenceCard` | Digest, journal, artifact, policy evidence. |
| `SlotDiffTable` | Before/after slot and taint changes. |
| `ShardFlowMap` | Overview shard lanes and queue pressure. |
| `ActionTicketCard` | Action id, attempt, idempotency key, replay safety. |
| `AiContextPanel` | AI-safe context packet and suggested commands. |

### Rendering Rules

- Graph edges, packet dots, timeline dots, glows, and selection halos should be shader-rendered or custom draw widgets, not composed from hundreds of nested generic boxes.
- Text is drawn only where meaningful; animation must not relayout text every frame.
- The graph canvas stores precomputed node positions and edge paths. Per-frame layout recomputation is forbidden.
- Animation loops must be bounded and stop when the view is hidden or the app is idle.
- The UI may allocate during screen load, fixture load, and model update. Continuous per-frame animation should avoid heap allocation.

### Figma-to-Makepad Workflow

1. Figma board defines visual target, spacing, screen taxonomy, and interaction notes.
2. `design/tokens/velvet_ui_tokens.toml` defines implementation tokens.
3. `xtask ui-tokens` generates Makepad Splash token snippets, Figma token import metadata if supported, and Rust constants for layout metrics.
4. Makepad Splash implements app shell and reusable components.
5. `xtask ui-snapshot` captures deterministic screenshots from demo fixtures.
6. Screenshot diff gates catch overlap, alignment, density, and regression issues.

Figma is not the source of runtime data. Makepad is not allowed to scrape Figma assets at runtime.

### Layout and Alignment Rules

Every screen uses a 1920x1080 baseline layout with scalable constraints.

Required frame metrics from the 11:51 design bundle:

```text
Window baseline:       1920 x 1080
Outer margin:          32
Sidebar width:         246
Top bar height:        78
Content gutter:        16
Card radius:           14-22
Small radius:          10
Inspector width:       360-420
Bottom timeline min:   220
Graph canvas min:      720 x 520
```

All component positions use an 8px spacing rhythm. One-off pixel nudges are rejected unless documented in a design-token bead.

### UI Snapshot Gate

Every screen must have a deterministic demo fixture and snapshot:

```text
tests/ui_snapshots/execution_overview.png
tests/ui_snapshots/workflow_graph_authoring.png
tests/ui_snapshots/execution_details.png
tests/ui_snapshots/verification_certificate.png
tests/ui_snapshots/replay_theater.png
tests/ui_snapshots/incident_failure.png
tests/ui_snapshots/action_registry.png
tests/ui_snapshots/storage_doctor_ai_context.png
```

Snapshot diff acceptance:

- No overlapping panels.
- No clipped primary labels.
- No unreadable chips.
- No controls outside safe bounds.
- No hidden selected state.
- No accidental color-system drift.
- No canonical spelling violations.

---

## 79. UI Design System Tokens

### Design Token Source

The design token source is:

```text
design/tokens/velvet_ui_tokens.toml
```

Generated outputs:

```text
crates/vb_ui_makepad/src/generated/tokens.rs
crates/vb_ui_makepad/src/generated/tokens.splash
contracts/ui_tokens.yaml
```

Manual edits to generated token files are rejected.

### Color Tokens

```toml
[color]
background_board = "#F4F6F8"
shell = "#F8FAFC"
surface = "#FFFFFF"
surface_glass = "#FFFFFFCC"
surface_muted = "#F2F5F8"
line_hair = "#DDE3EA"
line_soft = "#E8EDF2"
text_primary = "#101828"
text_secondary = "#475467"
text_tertiary = "#7A8796"

success = "#16A66A"
running = "#1F7AF5"
active_cyan = "#19A7CE"
warning = "#F59E0B"
failure = "#E5484D"
taint = "#8B5CF6"
durable = "#14B8A6"
pending = "#98A2B3"
```

### Typography Tokens

```toml
[type]
family_sans = "Inter, SF Pro, system-ui"
family_mono = "JetBrains Mono, SF Mono, ui-monospace"
size_11 = 11
size_12 = 12
size_13 = 13
size_14 = 14
size_16 = 16
size_20 = 20
size_24 = 24
weight_regular = 400
weight_medium = 500
weight_semibold = 600
```

Monospace may be used only for:

```text
RunId
ActionId
WorkflowDigest
SeqNo
SlotIdx
StepIdx
timestamps
record kind IDs
IPC frame fields
artifact digests
```

### Spacing Tokens

```toml
[space]
px_4 = 4
px_8 = 8
px_12 = 12
px_16 = 16
px_20 = 20
px_24 = 24
px_32 = 32
px_40 = 40
```

### Radius Tokens

```toml
[radius]
chip = 10
control = 12
card_min = 14
card = 16
card_max = 22
panel = 20
window = 24
```

### Shadow Tokens

```toml
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
window = "0 20 60 rgba(16,24,40,0.14)"
focus = "0 0 0 4 rgba(31,122,245,0.14)"
failure = "0 0 0 4 rgba(229,72,77,0.12)"
taint = "0 0 0 4 rgba(139,92,246,0.12)"
```

### Density Rule

The UI must be spacious. Data tables are allowed, but screen density must not exceed these baseline limits:

| Screen | Max primary panels | Max table rows visible by default |
|--------|--------------------|-----------------------------------|
| Execution overview | 6 | 7 |
| Workflow authoring | 4 | 0 |
| Execution details | 5 | 7 |
| Verification | 6 | 0 |
| Replay theater | 6 | 6 |
| Incident console | 7 | 5 |
| Action registry | 6 | 8 |
| Storage doctor / AI context | 7 | 8 |

If more data is available, use scroll, filters, disclosure, pagination, or drill-in.

---

## 80. UI Motion and Interaction Contract

### Principle

Animation must communicate state, causality, and replay timing. Decorative animation is rejected. Motion must be calm, bounded, and GPU-friendly.

### Required Motion Primitives

| Motion | Purpose | Screens |
|--------|---------|---------|
| Packet dots on edges | Show work moving through workflow graph. | Overview, graph, execution, replay |
| Active node glow | Show selected/running step. | Graph, execution, replay |
| Timeline scrubber | Show replay position. | Replay theater, execution details |
| Selected event pulse | Show current journal event. | Replay theater |
| Failure path focus | Guide attention to failed node and evidence chain. | Incident console |
| Taint overlay | Show secret-sensitive path. | Verification, replay, incident |
| Queue pressure shimmer | Indicate rising queue pressure without noise. | Overview |
| Certificate check cascade | Show verification gate pass sequence. | Verification |

### Motion Budget

```text
Target frame rate:          60fps minimum, 120fps when available
Max animated graph nodes:   256 visible
Max animated packet dots:   512 visible
Max timeline event dots:    2,000 visible before clustering
Max per-frame allocations:  0 in animation loops after warm-up
Max animation tick when hidden: 0
```

### Animation State Rules

- Animation state is UI state only; it never mutates runtime state.
- Animation tickers pause when the screen is not visible.
- Demo/snapshot mode must support deterministic time control.
- Replay scrubber state must bind to `SeqNo`, not wall-clock time.
- Packet animation may interpolate over precomputed edge paths but must not change graph topology.
- Failure pulse and taint overlay must be accessible through static visual indicators as well.

### Interaction Rules

Required interactions:

- Pan and zoom graph canvas.
- Click node to open step inspector.
- Hover node to show compact digest/resource/taint tooltip.
- Click event row to sync graph and timeline.
- Drag replay scrubber to any journal event.
- Filter events by step, event kind, taint, and action id.
- Toggle taint overlay.
- Toggle evidence overlay.
- Open action ticket from event or failed node.
- Copy digest/run/action IDs from monospace fields.
- Open AI context packet from run/incident.

Forbidden interactions:

- Hidden destructive actions without explicit confirmation or `--force` equivalent.
- UI-only retry behavior not represented by CLI lifecycle command.
- Freeform graph edits that bypass validation.
- Unbounded event list rendering.

---

## 81. UI Artifact and Schema Contract

### Shared Artifact Rule

The UI and CLI render the same typed artifacts. A screen cannot display data unless the corresponding CLI command can emit it in structured form.

| UI screen | Required artifact | CLI parity command |
|-----------|-------------------|--------------------|
| Execution Overview | `SystemStatus`, `RunSummaries`, `RunEvents` | `system status --emit yaml`, `events` |
| Workflow Graph Authoring | `WorkflowGraph` | `graph --emit yaml` |
| Execution Details | `RunInspection`, `RunEvents` | `inspect --emit yaml`, `events --emit yaml` |
| Verification Certificate | `VerificationReport`, `AcceptedArtifact` | `verify --emit yaml` |
| Replay Theater | `ReplayReport`, `RunEvents`, `SlotDiffs` | `replay --explain --emit yaml` |
| Incident Console | `IncidentReport` | `incident --emit yaml` |
| Action Registry | `ActionDescription`, `ActionList` | `action list`, `action inspect` |
| Storage Doctor / AI Context | `DoctorReport`, `AiContextPacket` | `doctor --emit yaml`, `ai context --emit yaml` |

### Required UI Model Fields

Every UI artifact must include:

```text
schema_version
kind
generated_at
source
redaction_status
```

Every graph node must include:

```text
step_idx
step_id
kind
status
output_slot
taint
badges
position
```

Every graph edge must include:

```text
from_step_idx
to_step_idx
edge_kind
condition_summary
is_failure_path
is_taint_path
packet_state
```

Every event row must include:

```text
seq
timestamp
run_id
step_idx
event_kind
status
evidence_digest
attempt
```

Every action ticket view must include:

```text
ticket_digest
run_id
step_idx
action_id
attempt
idempotency_key_hash
scheduled_durable
completion_durable
replay_safe
side_effect_certainty
```

### Redaction Rule

The UI must never render raw secret values. Secret-sensitive values are represented by:

```text
redacted: true
taint: Secret | DerivedFromSecret
digest: blake3:<prefix>
summary: <bounded static summary>
```

Any UI path that displays full blobs or raw action details must require an explicit unsafe operator action and must be disabled in AI context mode.

---

## 82. UI Implementation Phases

The UI phase rows in Section 70 define the required delivery sequence after Phase 60:

| Phase | Name | Required delivery |
|-------|------|-------------------|
| 61 | UI model artifacts | `vb_ui_model` crate with typed `WorkflowGraph`, `VerificationReport`, `RunInspection`, `RunEvents`, `ReplayReport`, `IncidentReport`, `SystemStatus`, `ActionDescription`, `DoctorReport`, and `AiContextPacket` views. CLI/UI schema parity tests. |
| 62 | Makepad shell | `vb_ui_makepad` crate, shared app chrome, sidebar, topbar, command buttons, status chips, profile selector, demo fixture loading. |
| 63 | Design tokens and Figma bridge | Token source in `design/tokens`; generated Makepad token files; Figma-ready SVG/PNG references; token drift checker. |
| 64 | Graph canvas | Pan/zoom canvas, nodes, curved edges, packet dots, selection, status color rules, taint overlay, layout fixtures. |
| 65 | Execution observatory | Overview KPIs, shard flow map, active runs table, event ticker, queue pressure indicators, storage/IPC health summary. |
| 66 | Execution details view | Single-run graph view, event table, step details panel, input/output/details tabs, runtime state coloring. |
| 67 | Verification certificate view | Verification banner, certificate cards, gate pipeline, accepted artifact panel, warnings, proof summary. |
| 68 | Replay theater | Journal timeline, playback controls, scrubber, selected event details, slot diffs, recovery decision panel, deterministic replay fixture. |
| 69 | Incident failure console | Failure banner, failure path graph, evidence chain, action ticket, recovery controls, slot/taint diffs, repair hints. |
| 70 | Action registry / contract inspector | Action list, selected `ActionContract`, idempotency/side-effect/retry safety, capability requirements, failure codes. |
| 71 | Storage doctor / AI context | Fjall keyspace health, journal doctor, snapshot/tail status, AI-safe context packet, suggested commands. |
| 72 | UI motion/performance | Shader-based packet dots, active-node glow, timeline pulse, bounded animation loops, no per-frame allocations after warm-up, UI perf benchmark. |
| 73 | UI snapshot and overlap gates | Deterministic screenshots for all eight screens, image diff gate, overlap/clipping scanner, canonical spelling scan. |
| 74 | UI release hardening | Keyboard navigation, accessibility labels, redaction tests, CLI/UI parity tests, demo fixtures, documentation, Makepad dependency audit. |

---

## 83. UI Testing, Benchmarking, and Acceptance Gates

### UI Tests

Required tests:

- `ui_model_schema_versions_are_stable`
- `ui_artifacts_match_cli_output_kinds`
- `workflow_graph_view_has_no_missing_nodes`
- `workflow_graph_edges_reference_valid_nodes`
- `event_rows_are_bounded`
- `ai_context_redacts_secrets`
- `incident_report_has_replay_safety`
- `verification_certificate_maps_all_gates`
- `action_ticket_hides_raw_idempotency_key`
- `ui_tokens_generate_makepad_and_contract_outputs`
- `all_screens_have_demo_fixtures`

### UI Snapshot Tests

Required deterministic snapshot fixtures:

```text
fixtures/ui/execution_overview.fixture
fixtures/ui/workflow_graph_authoring.fixture
fixtures/ui/execution_details.fixture
fixtures/ui/verification_certificate.fixture
fixtures/ui/replay_theater.fixture
fixtures/ui/incident_failure.fixture
fixtures/ui/action_registry.fixture
fixtures/ui/storage_doctor_ai_context.fixture
```

Snapshot command:

```bash
cargo xtask ui-snapshot --all --emit yaml
```

Snapshot report:

```yaml
kind: UiSnapshotReport
status: pass
screens:
  - screen: execution_overview
    png: tests/ui_snapshots/execution_overview.png
    overlap_check: pass
    clipping_check: pass
    spelling_check: pass
    token_check: pass
```

### UI Performance Benchmarks

Required UI benchmarks:

| Benchmark | Requirement |
|----------|-------------|
| `ui_graph_pan_zoom_256_nodes` | Smooth interaction, no unbounded allocation. |
| `ui_graph_packet_animation_512_packets` | Animation remains within frame budget. |
| `ui_timeline_2000_events_clustered` | Timeline remains responsive. |
| `ui_event_table_scroll_10000_bounded` | Virtualized/bounded rendering only. |
| `ui_replay_scrub_1000_events` | Scrub updates selected graph/event without full relayout. |
| `ui_fixture_load_all_screens` | Demo fixtures load under bounded memory. |

### UI Acceptance Commands

```bash
cargo +nightly fmt --all -- --check
cargo +nightly clippy -p vb_ui_model -p vb_ui_makepad --all-targets --all-features -- -D warnings
cargo +nightly nextest run -p vb_ui_model -p vb_ui_makepad
cargo xtask ui-tokens --check
cargo xtask ui-snapshot --all
cargo xtask ui-overlap-check --all
cargo xtask ui-perf-smoke
cargo xtask forbidden-scan --changed
cargo xtask hotpath-scan --changed
```

### UI Definition of Done

The Makepad UI is accepted only when:

1. All eight required screens exist and are reachable from shared app chrome.
2. Every screen consumes typed `vb_ui_model` artifacts.
3. CLI/UI parity exists for all displayed artifact kinds.
4. Figma token source and Makepad token output are synchronized.
5. No UI panel overlap, clipping, or unreadable primary label exists in 1920x1080 baseline screenshots.
6. All secret-sensitive values are redacted or summarized.
7. Graph, replay, incident, and verification views expose journal/digest/evidence concepts accurately.
8. Motion is bounded, meaningful, and can be disabled or frozen for deterministic snapshots.
9. UI code does not introduce Makepad, HTTP, JSON, async runtimes, or web dependencies into runtime core crates.
10. UI snapshot, token, model, parity, redaction, performance-smoke, lint, and test gates pass with evidence.
