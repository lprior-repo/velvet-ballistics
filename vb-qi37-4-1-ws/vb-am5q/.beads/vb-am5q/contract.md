# Contract Specification: cli/runtime — Enforce Converged Binary Mode Activation Boundaries

## Context

- **Feature**: Converged binary mode activation boundaries in `velvet-ballastics` CLI/runtime
- **Domain terms**:
  - *Converged binary*: single `velvet-ballastics` binary serving multiple command modes
  - *Mode*: one of {validate, verify, explain, compile, run, run-compiled, ipc-serve, submit, inspect, events, replay, trace, retry, resume, bench-run, doctor, answer, graph, diff, incident, simulate, ai-context, agent-context, status, action}
  - *Activation boundary*: the point at which a command mode decides which subsystems to initialize
  - *Subsystem*: vb_storage (Fjall), vb_runtime, vb_ui (Makepad), vb_ipc
  - *Pure mode*: command requiring no durable storage, runtime workers, or UI
  - *Storage mode*: command requiring Fjall durable storage
  - *Runtime mode*: command requiring runtime + storage
  - *UI mode*: command initializing Makepad desktop UI
- **Assumptions**:
  - CLI entrypoint routes to command handlers without global side-effects at startup
  - `vb_yaml`, `vb_compile`, `vb_validate`, `vb_core` have no transitive dependency on `vb_storage`
  - `vb_storage` is the only subsystem that opens Fjall
  - `vb_runtime` requires `vb_storage` when created with a journal
  - `vb_ui` / `vb_ui_makepad` are UI-only and should not load in non-UI modes
- **Open questions**:
  - Does `cmd_verify` transitively initialize any storage or runtime components? (Verified: it uses vb_yaml + vb_compile + vb_validate only — no storage)
  - Does `cmd_compile` transitively initialize any storage components? (Verified: it uses vb_yaml + vb_compile only — no storage)
  - Does `bench-run` require storage access? (Currently appears to use vb_compile + vb_core only — needs verification)
  - Does `agent-context` require storage? (Uses `agent_context::build` which appears static — needs verification)

## Preconditions

- PRE-001: CLI entrypoint and command routing are known
  - The `main()` function dispatches to typed command handlers without global initializer side-effects
- PRE-002: Subsystem initialization paths are identified
  - Each subsystem (vb_storage, vb_runtime, vb_ipc, vb_ui) has a distinct initialization function
  - Pure commands have zero subsystem dependencies beyond vb_yaml/vb_compile/vb_validate/vb_core
- PRE-003: Command mode classification is explicit
  - Every Command variant is classified as Pure | Storage | Runtime | UI mode
  - No command falls through to initialization without explicit classification

## Postconditions

- POST-001: Each command mode documents and tests its activated subsystems
  - Pure mode commands activate only: vb_yaml, vb_compile, vb_validate, vb_core
  - Storage mode commands additionally activate: vb_storage (FjallJournal)
  - Runtime mode commands additionally activate: vb_runtime
  - UI mode commands additionally activate: vb_ui_makepad / Makepad framework
- POST-002: Pure commands run without storage runtime or UI side effects
  - `validate`, `verify`, `explain`, `compile`, `graph`, `simulate`, `bench-run` do NOT open Fjall
  - These commands do NOT create Runtime instances
  - These commands do NOT initialize Makepad or any UI subsystem
- POST-003: Runtime commands still initialize required durable components
  - `run`, `run-compiled`, `submit`, `ipc-serve` open FjallJournal when durability != None
  - Storage-dependent inspection commands (`inspect`, `events`, `replay`, `trace`, `retry`, `resume`, `doctor`, `answer`, `diff`, `incident`, `ai-context`) open FjallJournal
- POST-004: Mode activation is fail-fast before any subsystem init
  - Invalid mode selection returns error exit code before attempting storage/runtime/UI init
- POST-005: Exit code is stable regardless of inactive subsystems
  - Running `validate` with no storage path present succeeds with code 0
  - Running `validate` with a valid workflow succeeds with code 0
  - Running `validate` with an invalid workflow fails with code != 0

## Invariants

- INV-001: No mode performs hidden network or storage initialization outside its contract
  - FjallJournal::open is called only from command handlers classified as Storage or Runtime
  - FjallJournal::open is NEVER called from Pure mode command handlers
- INV-002: UI dependencies remain scoped to UI mode
  - vb_ui_makepad or Makepad framework symbols are NOT linked into pure or storage command paths
  - UI mode is the ONLY activation mode for Makepad initialization
- INV-003: Exit codes remain stable regardless of inactive subsystems
  - Pure command exit code depends only on workflow validity, not on storage/runtime availability
  - Storage command exit code reflects storage errors appropriately
- INV-004: Runtime is created only for runtime-dependent commands
  - Runtime::new_with_journal is called only from Runtime mode commands
  - Pure commands never construct Runtime instances
- INV-005: Command handler functions are pure with respect to subsystem initialization
  - Each handler function explicitly calls only the subsystems it requires
  - No handler function silently initializes a subsystem beyond its mode classification

## Error Taxonomy

- `ModeError::InvalidMode` — when command routing produces an unrecognized command variant (should not occur given args parsing; defensive)
- `ModeError::StorageInitFailed { path: PathBuf, cause: String }` — when FjallJournal::open fails for storage-dependent commands
- `ModeError::RuntimeInitFailed { cause: String }` — when Runtime::new_with_journal fails
- `ModeError::UiInitFailed { cause: String }` — when Makepad UI initialization fails (UI mode only)
- `ModeError::PureCommandStorageAccessAttempted` — DEFECT: a pure command handler attempted to open storage (should never happen; indicates contract violation)

## Contract Signatures

```rust
// Pure mode commands — no storage, no runtime, no UI
fn cmd_validate(workflow: &Path) -> ExitCode;
fn cmd_verify(workflow: &Path, profile: VerifyProfile, output: OutputFormat) -> ExitCode;
fn cmd_explain(workflow: &Path) -> ExitCode;
fn cmd_compile(workflow: &Path, emit: EmitTarget, out: &Path, output: OutputFormat) -> ExitCode;
fn cmd_graph(workflow: &Path, output: OutputFormat) -> ExitCode;
fn cmd_simulate(workflow: &Path, output: OutputFormat) -> ExitCode;
fn cmd_bench_run(workflow: &Path, output: OutputFormat) -> ExitCode;

// Storage mode commands — Fjall storage required
fn cmd_run(workflow: &Path, input_bin: &Path, durability: DurabilityMode, db: Option<&Path>, output: OutputFormat) -> ExitCode;
fn cmd_submit(workflow: &Path, input_bin: &Path, db: &Path, durability: DurabilityMode, output: OutputFormat) -> ExitCode;
fn cmd_inspect(run_id: &str, db: &Path, output: OutputFormat) -> ExitCode;
fn cmd_events(run_id: &str, db: &Path, output: OutputFormat) -> ExitCode;
fn cmd_replay(run_id: &str, db: &Path, output: OutputFormat) -> ExitCode;
fn cmd_trace(run_id: &str, db: &Path, output: OutputFormat) -> ExitCode;
fn cmd_retry(run_id: &str, db: &Path, output: OutputFormat) -> ExitCode;
fn cmd_resume(run_id: &str, db: &Path, output: OutputFormat) -> ExitCode;
fn cmd_doctor(db: &Path, output: OutputFormat) -> ExitCode;
fn cmd_answer(run_id: &str, step: u16, value_file: &Path, db: &Path, output: OutputFormat) -> ExitCode;
fn cmd_diff(run_a: &str, run_b: &str, db: &Path, output: OutputFormat) -> ExitCode;
fn cmd_incident(run_id: &str, db: &Path, output: OutputFormat) -> ExitCode;
fn cmd_ai_context(run_id: &str, db: &Path, output: OutputFormat) -> ExitCode;

// Runtime mode commands — Runtime + Storage
fn cmd_ipc_serve(socket: &Path, db: &Path) -> ExitCode;

// UI mode commands — Makepad UI
// fn cmd_ui(...) -> ExitCode;  // future

// Mode classification query
fn command_mode(cmd: &Command) -> CommandMode;
enum CommandMode { Pure, Storage, Runtime, UI }
```

## Non-goals

- Changing the binary CLI interface or command names
- Modifying the internal behavior of vb_yaml, vb_compile, vb_validate, vb_core
- Adding new command modes
- Implementing the UI mode (that is a separate future bead)
- Proving algebraic properties of the workflow language itself
- Verifying generated Rust code equivalence (separate codegen bead)

## Mode Activation Matrix

| Command       | Mode     | vb_yaml | vb_compile | vb_validate | vb_core | vb_storage | vb_runtime | vb_ipc | vb_ui |
|---------------|----------|---------|------------|-------------|---------|------------|------------|--------|-------|
| validate      | Pure     | ✓       | ✓          | ✓           | ✓       | ✗          | ✗          | ✗      | ✗     |
| verify        | Pure     | ✓       | ✓          | ✓           | ✓       | ✗          | ✗          | ✗      | ✗     |
| explain       | Pure     | ✓       | ✓          | ✓           | ✓       | ✗          | ✗          | ✗      | ✗     |
| compile       | Pure     | ✓       | ✓          | ✓           | ✓       | ✗          | ✗          | ✗      | ✗     |
| graph         | Pure     | ✓       | ✓          | ✓           | ✓       | ✗          | ✗          | ✗      | ✗     |
| simulate      | Pure     | ✓       | ✓          | ✓           | ✓       | ✗          | ✗          | ✗      | ✗     |
| bench-run     | Pure?    | ✓       | ✓          | ✓           | ✓       | ✗?         | ✗?         | ✗      | ✗     |
| agent-context | Pure?    | ✓       | ✗          | ✗           | ✓       | ✗?         | ✗          | ✗      | ✗     |
| status        | Pure?    | ✗       | ✗          | ✗           | ✓       | ✗?         | ✗          | ✗      | ✗     |
| action list   | Pure     | ✗       | ✗          | ✗           | ✓       | ✗          | ✗          | ✗      | ✗     |
| action inspect| Pure     | ✗       | ✗          | ✗           | ✓       | ✗          | ✗          | ✗      | ✗     |
| run           | Storage  | ✓       | ✓          | ✓           | ✓       | ✓          | ✓          | ✗      | ✗     |
| run-compiled  | Storage  | ✗       | ✗          | ✗           | ✓       | ✓          | ✓          | ✗      | ✗     |
| submit        | Storage  | ✓       | ✓          | ✓           | ✓       | ✓          | ✗          | ✗      | ✗     |
| inspect       | Storage  | ✗       | ✗          | ✗           | ✓       | ✓          | ✗          | ✗      | ✗     |
| events        | Storage  | ✗       | ✗          | ✗           | ✓       | ✓          | ✗          | ✗      | ✗     |
| replay        | Storage  | ✗       | ✗          | ✗           | ✓       | ✓          | ✗          | ✗      | ✗     |
| trace         | Storage  | ✗       | ✗          | ✗           | ✓       | ✓          | ✗          | ✗      | ✗     |
| retry         | Storage  | ✗       | ✗          | ✗           | ✓       | ✓          | ✗          | ✗      | ✗     |
| resume        | Storage  | ✗       | ✗          | ✗           | ✓       | ✓          | ✗          | ✗      | ✗     |
| doctor        | Storage  | ✗       | ✗          | ✗           | ✓       | ✓          | ✗          | ✗      | ✗     |
| answer        | Storage  | ✗       | ✗          | ✗           | ✓       | ✓          | ✗          | ✗      | ✗     |
| diff          | Storage  | ✗       | ✗          | ✗           | ✓       | ✓          | ✗          | ✗      | ✗     |
| incident      | Storage  | ✗       | ✗          | ✗           | ✓       | ✓          | ✗          | ✗      | ✗     |
| ai-context    | Storage  | ✗       | ✗          | ✓           | ✓       | ✓          | ✗          | ✗      | ✗     |
| ipc-serve     | Runtime  | ✓       | ✓          | ✓           | ✓       | ✓          | ✓          | ✓      | ✗     |
| ui            | UI       | ✓       | ✓          | ✓           | ✓       | ✓          | ✓          | ✗      | ✓     |
