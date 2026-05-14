# Test Plan: cli/runtime — Converged Binary Mode Activation Boundaries

## Bead: vb-am5q

## Context

This test plan covers the enforcement of converged binary mode activation boundaries in the `velvet-ballastics` CLI/runtime. The core guarantee is that each command mode (Pure, Storage, Runtime, UI) activates only the subsystems defined in its contract — no hidden storage, runtime, or UI initialization outside the mode contract.

## Target

`crates/velvet_ballastics/src/main.rs` — command dispatch, handler functions, and argument parsing.

---

## Section 1 — Behavior Inventory

### Pure Mode Behaviors

| Subject | Action | Outcome when Condition |
|---------|--------|------------------------|
| `cmd_validate` | validates workflow | succeeds with code 0 when workflow valid, no storage accessed |
| `cmd_validate` | validates workflow | fails with code 1 when workflow invalid, no storage accessed |
| `cmd_verify` | verifies workflow | succeeds with code 0 when workflow passes profile checks |
| `cmd_verify` | verifies workflow | fails with code 2 when profile checks fail |
| `cmd_compile` | compiles workflow | succeeds with code 0 when compilation succeeds |
| `cmd_explain` | explains validation errors | returns detailed errors without accessing storage |
| `cmd_graph` | outputs DOT graph | generates control flow graph without storage |
| `cmd_simulate` | dry-runs workflow | simulates execution without durable side effects |
| `cmd_bench_run` | benchmarks workflow | runs in-memory runtime only, no FjallJournal::open called |
| `cmd_agent_context` | emits CLI schema | returns static JSON schema, no storage argument required |
| `cmd_status` | reports shard status | builds in-memory Shard snapshot, no FjallJournal::open called |
| `cmd_action_list` | lists action contracts | returns registry without storage |
| `cmd_action_inspect` | inspects action contract | returns contract detail without storage |

### Storage Mode Behaviors

| Subject | Action | Outcome when Condition |
|---------|--------|------------------------|
| `cmd_run` durability=strict | opens FjallJournal | journal opened at specified path |
| `cmd_run` durability=journaled | opens FjallJournal | journal opened at specified path |
| `cmd_run` durability=none | skips storage | no FjallJournal::open called |
| `cmd_submit` | opens FjallJournal | journal opened regardless of durability |
| `cmd_inspect` | opens FjallJournal | fails fast with code 5 when path invalid |
| `cmd_events` | opens FjallJournal | fails fast with code 5 when path invalid |
| `cmd_replay` | opens FjallJournal | fails fast with code 5 when path invalid |
| `cmd_trace` | opens FjallJournal | fails fast with code 5 when path invalid |
| `cmd_retry` | opens FjallJournal | fails fast with code 5 when path invalid |
| `cmd_resume` | opens FjallJournal | fails fast with code 5 when path invalid |
| `cmd_doctor` | opens FjallJournal | fails fast with code 5 when path invalid |
| `cmd_answer` | opens FjallJournal | fails fast with code 5 when path invalid |
| `cmd_diff` | opens FjallJournal | fails fast with code 5 when path invalid |
| `cmd_incident` | opens FjallJournal | fails fast with code 5 when path invalid |
| `cmd_ai_context` | opens FjallJournal | fails fast with code 5 when path invalid |

### Runtime Mode Behaviors

| Subject | Action | Outcome when Condition |
|---------|--------|------------------------|
| `cmd_ipc_serve` | creates Runtime | Runtime::new_with_journal() called with correct config |
| `cmd_ipc_serve` | opens FjallJournal | journal opened before runtime creation |

### Argument Parsing Behaviors

| Subject | Action | Outcome when Condition |
|---------|--------|------------------------|
| `parse_args` | parses valid command | returns correct Command variant |
| `parse_args` | parses unknown command | returns ParseError::UnknownCommand → exit code 1 |
| `parse_args` | missing required arg | returns specific ParseError variant |
| `parse_args` | invalid durability | returns ParseError::UnknownDurability |

### Error Handling Behaviors

| Subject | Action | Outcome when Condition |
|---------|--------|------------------------|
| `parse_args` | unknown command string | ParseError::UnknownCommand rendered → exit code 1, message lists valid commands |
| `FjallJournal::open` failure | storage init fails | CliExitCode::StorageError (5), error output includes path and cause |
| `Runtime::new_with_journal` failure | runtime init fails | CliExitCode::RuntimeFailed (4), error message mentions runtime init cause |
| UI initialization failure | UI mode init fails | CliExitCode::ActionPolicyError (7), error message mentions UI init cause |
| Pure command accesses storage | DEFECT | CliExitCode::StorageError (5), error message identifies the defect |

---

## Section 2 — Trophy Allocation

```
[ E2E / Acceptance ]                    ← 5%  — full binary invocation, exit code verification
[ Integration Tests /tests/ ]            ← 60% — mode activation, storage init, error paths
  [ Unit Tests #[cfg(test)] ]           ← 30% — pure function invariants, parse_args, proptest
[ Static: clippy, cargo-deny, geiger ] ← 5%  — no unsafe, no panic, dep audit
```

### Rationale

- **Integration tests** dominate because mode activation is a property of how subsystems are wired together — only testable via real invocation with real dependencies (vb_storage, vb_runtime).
- **Unit tests** cover argument parsing (pure, no I/O), exit code discriminants, and proptest invariants on pure functions.
- **E2E** covers the full binary entrypoint with subprocess invocation to verify exit codes.
- **Static analysis** is free and catches unsafe/panic/unwrap violations at compile time.

---

## Section 3 — BDD Scenarios

### Pure Mode: validate

```
### Behavior: validate succeeds on valid workflow without storage
Given: a valid workflow.yaml file exists at /tmp/valid.yaml
When: the user runs `velvet-ballastics validate /tmp/valid.yaml`
Then: exit code is 0
And: no FjallJournal::open is called during execution

### Behavior: validate fails on invalid workflow without storage
Given: an invalid workflow.yaml file exists at /tmp/invalid.yaml
When: the user runs `velvet-ballastics validate /tmp/invalid.yaml`
Then: exit code is 1
And: no FjallJournal::open is called during execution

### Behavior: validate succeeds when no storage path exists
Given: no --db argument is provided
And: no journal directory exists
When: the user runs `velvet-ballastics validate /tmp/valid.yaml`
Then: exit code is 0
```

### Pure Mode: verify

```
### Behavior: verify succeeds on passing workflow
Given: a workflow.yaml that passes quick profile
When: the user runs `velvet-ballastics verify /tmp/valid.yaml --profile quick`
Then: exit code is 0
And: stdout ends with the word "verified" on its own line

### Behavior: verify succeeds on passing workflow (JSON output)
Given: a workflow.yaml that passes quick profile
When: the user runs `velvet-ballastics verify /tmp/valid.yaml --profile quick --json`
Then: exit code is 0
And: stdout JSON contains `"success": true` and `"profile": "quick"`

### Behavior: verify fails with non-zero exit on failing workflow
Given: a workflow.yaml that fails standard profile
When: the user runs `velvet-ballastics verify /tmp/invalid.yaml`
Then: exit code is 2
And: no --db argument is required or accessed
```

### Pure Mode: compile

```
### Behavior: compile produces IR without storage
Given: a valid workflow.yaml
When: the user runs `velvet-ballastics compile /tmp/valid.yaml --emit ir --out /tmp/out.vbir`
Then: exit code is 0
And: /tmp/out.vbir contains valid IR bytes
And: no FjallJournal::open is called
```

### Pure Mode: bench-run

```
### Behavior: bench-run executes workflow in-memory without storage
Given: a valid workflow.yaml
When: the user runs `velvet-ballastics bench-run /tmp/valid.yaml`
Then: exit code is 0
And: output includes "compile:" and "execute:" timing lines
And: no --db argument is accepted or used
And: no FjallJournal::open is called during execution
```

### Pure Mode: agent-context

```
### Behavior: agent-context emits CLI schema without storage
Given: no --db argument is provided
When: the user runs `velvet-ballastics agent-context`
Then: exit code is 0
And: stdout is valid JSON containing "schema_version" and "AgentContext"
And: no FjallJournal::open is called during execution
```

### Pure Mode: status

```
### Behavior: status reports in-memory shard counters without storage
Given: no --db argument is provided
When: the user runs `velvet-ballastics status`
Then: exit code is 0
And: stdout contains "running:" or "shutting_down:"
And: no FjallJournal::open is called during execution
```

### Storage Mode: run

```
### Behavior: run with durability journaled opens FjallJournal
Given: a valid workflow.yaml and input.bin
And: a journal directory at /tmp/journal
When: the user runs `velvet-ballastics run /tmp/valid.yaml --input-bin /tmp/input.bin --durability journaled --db /tmp/journal`
Then: exit code is 0 or 4
And: FjallJournal::open(/tmp/journal, None) is called

### Behavior: run with durability none skips storage
Given: a valid workflow.yaml and input.bin
And: no journal directory exists
When: the user runs `velvet-ballastics run /tmp/valid.yaml --input-bin /tmp/input.bin --durability none`
Then: exit code is 0 or 4
And: FjallJournal::open is NOT called
```

### Storage Mode: submit

```
### Behavior: submit always opens FjallJournal regardless of durability
Given: a valid workflow.yaml and input.bin
And: a journal directory at /tmp/journal
When: the user runs `velvet-ballastics submit /tmp/valid.yaml --input-bin /tmp/input.bin --db /tmp/journal --durability journaled`
Then: exit code is 0
And: FjallJournal::open is called
```

### Storage Mode: inspect (fail-fast)

```
### Behavior: inspect fails fast with StorageError on invalid path
Given: a run_id "1" and a non-existent journal path /tmp/nonexistent
When: the user runs `velvet-ballastics inspect 1 --db /tmp/nonexistent`
Then: exit code is 5
And: error message mentions the invalid path
And: no partial state is created
```

### Storage Mode: ai-context

```
### Behavior: ai-context opens FjallJournal
Given: a run_id "1" and a valid journal at /tmp/journal
When: the user runs `velvet-ballastics ai-context 1 --db /tmp/journal`
Then: exit code is 0
And: FjallJournal::open is called
```

### Runtime Mode: ipc-serve

```
### Behavior: ipc-serve creates Runtime with FjallJournal
Given: a valid socket path /tmp/socket and journal at /tmp/journal
When: the user runs `velvet-ballastics ipc-serve --socket /tmp/socket --db /tmp/journal`
Then: FjallJournal::open is called first
And: Runtime::new_with_journal is called with the journal
And: server binds to socket
```

### Argument Parsing

```
### Behavior: parse_args rejects unknown command
When: the user runs `velvet-ballastics foobar`
Then: exit code is 1
And: error message enumerates valid commands

### Behavior: parse_args requires --db for storage commands
When: the user runs `velvet-ballastics inspect 1` without --db
Then: exit code is 1
And: error message mentions missing --db

### Behavior: parse_args accepts --durability none for run
When: the user runs `velvet-ballastics run w.yaml --input-bin i.bin --durability none`
Then: parse returns Command::Run { durability: DurabilityMode::None, db: None }
```

### Error Taxonomy

```
### Behavior: ParseError::UnknownCommand produces exit 1 and lists valid commands
Given: the user runs `velvet-ballastics foobar`
When: parse_args fails with UnknownCommand
Then: exit code is 1
And: error output contains the unknown command name
And: error output enumerates valid commands from VALID_COMMANDS

### Behavior: ModeError::InvalidMode (defensive) produces exit 1
Given: args parsing produces a command variant not handled by main match (defensive path)
When: the unmatched command is dispatched
Then: exit code is 1
And: error message mentions the unrecognized command variant

### Behavior: StorageInitFailed produces structured diagnostic and exit 5
Given: a storage command with invalid --db path
When: FjallJournal::open fails
Then: exit code is 5
And: error output includes path and cause string

### Behavior: RuntimeInitFailed produces CliExitCode::RuntimeFailed (exit 4)
Given: ipc-serve command where Runtime::new_with_journal returns Err
When: runtime creation fails
Then: exit code is 4
And: error message includes the runtime init cause

### Behavior: UiInitFailed produces CliExitCode::ActionPolicyError (exit 7)
Given: UI mode is invoked and Makepad initialization fails
When: UI init fails
Then: exit code is 7
And: error message mentions UI init cause
Note: UI mode not yet implemented — this scenario is deferred to the UI bead

### Behavior: PureCommandStorageAccessAttempted produces exit 5
Given: a pure command handler attempts to call FjallJournal::open (defect)
When: the defect is detected
Then: exit code is 5
And: error message identifies ModeError::PureCommandStorageAccessAttempted
```

---

## Section 4 — Proptest Invariants

### Pure Functions with Non-Trivial Input Space

#### `parse_args` Invariants

**Property 1**: Every valid command string produces a Some(Command) variant
```
for all args in valid_command_args:
  parse_args(args).is_ok()
```

**Property 2**: Unknown commands produce UnknownCommand error with valid command list
```
for all args in unknown_command_args:
  match parse_args(args):
    Err(UnknownCommand(cmd)) => VALID_COMMANDS.contains(cmd)
```

**Property 3**: ParseError variants are exhaustive for all invalid inputs
```
for all args in invalid_args:
  parse_args(args) returns one of the 12 ParseError variants
```

#### `DurabilityMode` Parsing Invariants

**Property**: Only "strict", "journaled", "none" parse successfully; anything else returns UnknownDurability
```
for all input in ["strict", "journaled", "none"]:
  parse_durability(input) == Ok(corresponding_variant)

for all input not in ["strict", "journaled", "none"]:
  parse_durability(input) == Err(UnknownDurability(input))
```

#### `cmd_bench_run` No-Storage Invariant

**Property**: bench-run does not open FjallJournal
```
for all valid workflow paths:
  invoking cmd_bench_run(path) results in no FjallJournal::open call
  (proven by code inspection: bench.rs calls vb_runtime::Runtime::new, not new_with_journal)
```

#### `cmd_agent_context` No-Storage Invariant

**Property**: agent-context does not access storage
```
for all version strings:
  agent_context::build(version) returns a Value derived entirely from serde_json::json!
  no vb_storage, vb_runtime, or file-system calls in the call chain
```

#### `cmd_status` No-Storage Invariant

**Property**: status does not open FjallJournal
```
for all StatusOptions:
  build_status(options) creates Shard::new (transient in-memory)
  no vb_storage::FjallJournal::open in the call chain
```

#### Exit Code Invariants

**Property 1**: CliExitCode discriminants are unique
```
distinct([c.discriminant for c in CliExitCode variants]) == len(CliExitCode variants)
```

**Property 2**: All 9 exit codes map to distinct u8 values
```
len(set([code as u8 for code in CliExitCode])) == 9
```

---

## Section 5 — Fuzz Targets

### Target: storage_path_malformed

**Risk class**: High (command-line provided path to storage)
**Input type**: arbitrary bytes → PathBuf string
**Corpus seeds**: `/tmp/exists`, `/nonexistent`, `../../etc`, `//invalid//path`, `with spaces`, `\0-null`

**What it proves**: Invalid storage paths produce `CliExitCode::StorageError` with structured diagnostic, not UB or panic.

```rust
fn fuzz_storage_path(input: &[u8]) {
    let path_str = String::from_utf8_lossy(input);
    let path = PathBuf::from(path_str.as_ref());
    // Invoke FjallJournal::open and verify:
    // - Returns Err, not panic
    // - Error message contains path (for diagnostics)
}
```

### Target: args_parsing_chaos

**Risk class**: Medium (CLI argument parsing)
**Input type**: random argv vectors
**Corpus seeds**: all 24 valid commands with their required args

**What it proves**: `parse_args` handles all malformed inputs without panic; returns specific ParseError variants.

```rust
fn fuzz_args_parsing(args: &[&str]) {
    let os_args: Vec<OsString> = args.iter().map(|s| OsString::from(s)).collect();
    let result = parse_args(&os_args);
    // Verify: no panic, returns Result<Command, ParseError>
}
```

### Target: workflow_file_invalid

**Risk class**: Medium (workflow file input)
**Input type**: arbitrary bytes → workflow YAML
**Corpus seeds**: valid YAML workflows, empty file, non-UTF8 bytes, YAML parse errors

**What it proves**: Pure commands (validate, verify, compile, explain, graph, simulate, bench-run, agent-context, status) handle malformed workflow files without accessing storage.

```rust
fn fuzz_workflow_input(input: &[u8]) {
    let temp_path = write_to_temp_file(input);
    // Run cmd_validate on the temp path
    // Verify: no storage access, exit code reflects validation failure
}
```

---

## Section 6 — Kani Harnesses

### Harness 1: Mode Activation Completeness

**Property**: For every Command variant, the correct mode classification is returned.

```rust
fn command_mode_is_correct(cmd: &Command) -> bool {
    match command_mode(cmd) {
        CommandMode::Pure => !requires_storage(cmd) && !requires_runtime(cmd) && !requires_ui(cmd),
        CommandMode::Storage => requires_storage(cmd) && !requires_runtime(cmd) && !requires_ui(cmd),
        CommandMode::Runtime => requires_storage(cmd) && requires_runtime(cmd),
        CommandMode::UI => requires_ui(cmd),
    }
}
```

**Bound**: All 24 Command variants enumerated in `args.rs`.

### Harness 2: Durability Mode Storage Gate

**Property**: `cmd_run` opens storage iff `durability != DurabilityMode::None`.

```rust
fn durability_gate(durability: DurabilityMode, db: Option<PathBuf>) -> bool {
    match durability {
        DurabilityMode::None => db.is_none(), // storage not accessed
        _ => db.is_some(), // storage required
    }
}
```

### Harness 3: Exit Code Discriminant Safety

**Property**: All CliExitCode variants have distinct discriminant values (no collision).

```rust
fn exit_codes_distinct() -> bool {
    let codes = [
        CliExitCode::Success,
        CliExitCode::ValidationFailed,
        CliExitCode::VerificationFailed,
        CliExitCode::CompileFailed,
        CliExitCode::RuntimeFailed,
        CliExitCode::StorageError,
        CliExitCode::IpcError,
        CliExitCode::ActionPolicyError,
        CliExitCode::ReplayDivergence,
    ];
    codes.iter().map(|c| *c as u8).collect::<HashSet<_>>().len() == 9
}
```

---

## Section 7 — Mutation Testing Checkpoints

### Target: pure_mode_activation

**What it catches**: Any mutation that adds `FjallJournal::open` to a pure command handler.

| Mutation | Original | Mutated | Kill Condition |
|----------|----------|---------|----------------|
| Add storage call in cmd_validate | `vb_yaml::parse_workflow_source` | `vb_storage::FjallJournal::open` | `pure_no_storage` test fails |
| Add storage call in cmd_verify | `vb_yaml::parse_workflow_source` | `vb_storage::FjallJournal::open` | `verify_no_storage` test fails |
| Add storage call in cmd_compile | `vb_compile::compile_workflow` | `vb_storage::FjallJournal::open` | `compile_no_storage` test fails |
| Add storage call in cmd_explain | `vb_yaml::parse_workflow_source` | `vb_storage::FjallJournal::open` | `explain_no_storage` test fails |
| Add storage call in cmd_graph | `vb_compile::compile_workflow` | `vb_storage::FjallJournal::open` | `graph_no_storage` test fails |
| Add storage call in cmd_simulate | `vb_compile::compile_workflow` | `vb_storage::FjallJournal::open` | `simulate_no_storage` test fails |
| Add storage call in cmd_bench_run | `vb_runtime::Runtime::new` | `vb_storage::FjallJournal::open` | `bench_run_no_storage` test fails |
| Add storage call in cmd_agent_context | static JSON build | `vb_storage::FjallJournal::open` | `agent_context_no_storage` test fails |
| Add storage call in cmd_status | `Shard::new` | `vb_storage::FjallJournal::open` | `status_no_storage` test fails |

### Target: storage_mode_activation

**What it catches**: Any mutation that removes required `FjallJournal::open` from a storage command.

| Mutation | Original | Mutated | Kill Condition |
|----------|----------|---------|----------------|
| Remove storage call in cmd_inspect | `FjallJournal::open` | removed | `inspect_opens_storage` test fails |
| Remove storage call in cmd_events | `FjallJournal::open` | removed | `events_opens_storage` test fails |
| Remove storage call in cmd_replay | `FjallJournal::open` | removed | `replay_opens_storage` test fails |
| Remove storage call in cmd_submit | `FjallJournal::open` | removed | `submit_opens_storage` test fails |
| Remove storage call in cmd_doctor | `FjallJournal::open` | removed | `doctor_opens_storage` test fails |
| Remove storage call in cmd_ai_context | `FjallJournal::open` | removed | `ai_context_opens_storage` test fails |

### Target: runtime_mode_activation

**What it catches**: Any mutation that adds/removes Runtime::new_with_journal from non-runtime commands.

| Mutation | Original | Mutated | Kill Condition |
|----------|----------|---------|----------------|
| Remove Runtime::new_with_journal in ipc_serve | `Runtime::new_with_journal` | removed | `ipc_serve_creates_runtime` test fails |
| Add Runtime::new_with_journal in pure cmd | not present | added | `pure_no_runtime` test fails |

### Target: exit_code_stability

**What it catches**: Any mutation that changes exit code behavior for pure commands when storage is unavailable.

| Mutation | Original | Mutated | Kill Condition |
|----------|----------|---------|----------------|
| Change validate success code | `ExitCode::SUCCESS` | `ExitCode::from(5)` | `validate_exit_stable` test fails |
| Change verify error code | `CliExitCode::VerificationFailed` | `CliExitCode::StorageError` | `verify_exit_stable` test fails |

**Target kill rate**: ≥90%

---

## Section 8 — Combinatorial Coverage Matrix

### Mode × Subsystem Activation

| Command | vb_yaml | vb_compile | vb_validate | vb_core | vb_storage | vb_runtime | vb_ui |
|---------|---------|------------|-------------|---------|------------|------------|-------|
| validate | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ |
| verify | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ |
| explain | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ |
| compile | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ |
| graph | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ |
| simulate | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ |
| bench-run | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ |
| agent-context | ✗ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ |
| status | ✗ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ |
| action list | ✗ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ |
| action inspect | ✗ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ |
| run (dur=none) | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ |
| run (dur=*) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |
| run-compiled | ✗ | ✗ | ✗ | ✓ | ✓ | ✓ | ✗ |
| submit | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ |
| inspect | ✗ | ✗ | ✗ | ✓ | ✓ | ✗ | ✗ |
| events | ✗ | ✗ | ✗ | ✓ | ✓ | ✗ | ✗ |
| replay | ✗ | ✗ | ✗ | ✓ | ✓ | ✗ | ✗ |
| trace | ✗ | ✗ | ✗ | ✓ | ✓ | ✗ | ✗ |
| retry | ✗ | ✗ | ✗ | ✓ | ✓ | ✗ | ✗ |
| resume | ✗ | ✗ | ✗ | ✓ | ✓ | ✗ | ✗ |
| doctor | ✗ | ✗ | ✗ | ✓ | ✓ | ✗ | ✗ |
| answer | ✗ | ✗ | ✗ | ✓ | ✓ | ✗ | ✗ |
| diff | ✗ | ✗ | ✗ | ✓ | ✓ | ✗ | ✗ |
| incident | ✗ | ✗ | ✗ | ✓ | ✓ | ✗ | ✗ |
| ai-context | ✗ | ✗ | ✓ | ✓ | ✓ | ✗ | ✗ |
| ipc-serve | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |

### ParseError Variant Coverage

| ParseError Variant | Test Name | Layer |
|--------------------|-----------|-------|
| MissingArgument | `parse_run_requires_input_bin` | unit |
| UnknownEmitTarget | `parse_compile_rejects_unknown_emit_target` | unit |
| UnknownDurability | `parse_run_rejects_unknown_durability` | unit |
| UnknownProfile | `parse_verify_rejects_unknown_profile` | unit |
| UnknownCommand | `parse_unknown_command_returns_error` | unit |
| InvalidStatusArgument | `parse_status_rejects_invalid_numeric_argument` | unit |
| UnknownActionCommand | `parse_error_exact_variant_coverage` | unit |
| UnknownActionRegistry | `parse_action_list_rejects_invalid_registry` | unit |
| MissingActionRegistryValue | `parse_action_list_rejects_missing_registry_value` | unit |
| UnknownActionListFlag | `parse_action_list_rejects_unknown_flag` | unit |
| UnexpectedActionListArgument | `parse_action_list_rejects_trailing_argument` | unit |
| UnknownActionInspectFlag | (covered via error coverage) | unit |
| UnexpectedActionInspectArgument | (covered via error coverage) | unit |
| InvalidActionId | `parse_action_inspect_rejects_invalid_action_id` | unit |
| NoCommand | `parse_no_command_returns_error` | unit |
| InvalidStep | `parse_answer_rejects_invalid_step` | unit |

### CliExitCode Coverage

| CliExitCode | Value | Test Name | Layer |
|-------------|-------|-----------|-------|
| Success | 0 | `discriminant_values_match_spec` | unit |
| ValidationFailed | 1 | `discriminant_values_match_spec` | unit |
| VerificationFailed | 2 | `discriminant_values_match_spec` | unit |
| CompileFailed | 3 | `discriminant_values_match_spec` | unit |
| RuntimeFailed | 4 | `discriminant_values_match_spec` | unit |
| StorageError | 5 | `discriminant_values_match_spec` | unit |
| IpcError | 6 | `discriminant_values_match_spec` | unit |
| ActionPolicyError | 7 | `discriminant_values_match_spec` | unit |
| ReplayDivergence | 8 | `discriminant_values_match_spec` | unit |

### ModeError → CliExitCode Mapping

| ModeError Variant | Maps to CliExitCode | Exit Code | BDD Scenario |
|-------------------|---------------------|-----------|--------------|
| `InvalidMode` (defensive) | `ValidationFailed` | 1 | `parse_args_rejects_unknown_command` |
| `StorageInitFailed { path, cause }` | `StorageError` | 5 | `storage_init_error_produces_exit_5` |
| `RuntimeInitFailed { cause }` | `RuntimeFailed` | 4 | `runtime_init_error_produces_exit_4` |
| `UiInitFailed { cause }` | `ActionPolicyError` | 7 | `ui_init_error_produces_exit_7` (deferred to UI bead) |
| `PureCommandStorageAccessAttempted` | `StorageError` | 5 | `pure_storage_violation` |

---

## Section 9 — Proof Obligations Mapping

All 48 proof obligations from `proof-obligations.jsonl` are addressed:

| ID | Layer | Test Method | Evidence |
|----|-------|-------------|----------|
| PRE-001 | miri | `MiriBufferAccess` harness | miri-test-log.txt |
| PRE-002 | static-scan | `cargo geiger` + `cargo-machete` | geiger-report.txt |
| PRE-003 | static-scan | `static_scan_mode_classification` | static-scan-report.txt |
| POST-001 | proptest | `mode_activation_matrix` | test-results.txt |
| POST-002-PURE-STORAGE | proptest | `cmd_validate_no_storage` | test-results.txt |
| POST-002-PURE-VERIFY | proptest | `cmd_verify_no_storage` | test-results.txt |
| POST-002-PURE-COMPILE | proptest | `cmd_compile_no_storage` | test-results.txt |
| POST-002-PURE-EXPLAIN | proptest | `cmd_explain_no_storage` | test-results.txt |
| POST-002-PURE-GRAPH | proptest | `cmd_graph_no_storage` | test-results.txt |
| POST-002-PURE-SIMULATE | proptest | `cmd_simulate_no_storage` | test-results.txt |
| POST-002-PURE-BENCH-RUN | proptest | `cmd_bench_run_no_storage` | test-results.txt |
| POST-002-PURE-AGENT-CONTEXT | proptest | `cmd_agent_context_no_storage` | test-results.txt |
| POST-002-PURE-STATUS | proptest | `cmd_status_no_storage` | test-results.txt |
| POST-002-PURE-RUNTIME | proptest | `pure_no_runtime` | test-results.txt |
| POST-002-PURE-UI | static-scan | `cargo geiger \| grep makepad` | geiger-report.txt |
| POST-003-STORAGE-RUN | proptest | `cmd_run_opens_storage` | test-results.txt |
| POST-003-STORAGE-RUN-NONE | proptest | `cmd_run_durability_none` | test-results.txt |
| POST-003-STORAGE-SUBMIT | proptest | `cmd_submit_opens_storage` | test-results.txt |
| POST-003-STORAGE-INSPECT | proptest | `cmd_inspect_opens_storage` | test-results.txt |
| POST-003-STORAGE-EVENTS | proptest | `cmd_events_opens_storage` | test-results.txt |
| POST-003-STORAGE-REPLAY | proptest | `cmd_replay_opens_storage` | test-results.txt |
| POST-003-STORAGE-TRACE | proptest | `cmd_trace_opens_storage` | test-results.txt |
| POST-003-STORAGE-RETRY | proptest | `cmd_retry_opens_storage` | test-results.txt |
| POST-003-STORAGE-RESUME | proptest | `cmd_resume_opens_storage` | test-results.txt |
| POST-003-STORAGE-DOCTOR | proptest | `cmd_doctor_opens_storage` | test-results.txt |
| POST-003-STORAGE-ANSWER | proptest | `cmd_answer_opens_storage` | test-results.txt |
| POST-003-STORAGE-DIFF | proptest | `cmd_diff_opens_storage` | test-results.txt |
| POST-003-STORAGE-INCIDENT | proptest | `cmd_incident_opens_storage` | test-results.txt |
| POST-003-STORAGE-AI-CONTEXT | proptest | `cmd_ai_context_opens_storage` | test-results.txt |
| POST-004 | manual-qa | `fail_fast_verification` | manual-qa-report.md |
| POST-005 | proptest | `cmd_validate_exit_stable` | test-results.txt |
| POST-005-VERIFY | proptest | `cmd_verify_exit_stable` | test-results.txt |
| INV-001-FJALL-OPEN | miri | `miri storage_init_in_pure_mode` | miri-test-log.txt |
| INV-001-PURE-NO-FJALL | proptest | `pure_handler_no_fjall` | test-results.txt |
| INV-002-UI-SCOPE | static-scan | `cargo geiger 2>&1 \| grep -c makepad == 0` | geiger-report.txt |
| INV-003 | proptest | `pure_exit_independent` | test-results.txt |
| INV-004 | proptest | `runtime_init_only_runtime_mode` | test-results.txt |
| INV-004-IPC | proptest | `cmd_ipc_serve_creates_runtime` | test-results.txt |
| INV-005 | static-scan | `static scan for implicit subsystem init` | static-scan-report.txt |
| ERR-STORAGE-INIT | proptest | `storage_init_error` | test-results.txt |
| ERR-STORAGE-INIT-INVALID | cargo-fuzz | `cargo fuzz run storage_path_malformed` | fuzz-results.txt |
| ERR-RUNTIME-INIT | proptest | `runtime_init_error` | test-results.txt |
| ERR-UI-INIT | proptest | `ui_init_error` (deferred to UI bead) | deferred |
| ERR-PURE-STORAGE-ACCESS | proptest | `pure_storage_violation` | test-results.txt |
| WAIVER-UI | waiver | N/A — UI bead not started | UI mode deferred |
| WAIVER-BENCH-RUN | resolved | Static analysis confirms bench.rs: cmd_bench_run calls vb_runtime::Runtime::new (not new_with_journal), no FjallJournal::open | bench.rs lines 9–65 |
| WAIVER-AGENT-CONTEXT | resolved | Static analysis confirms agent_context::build uses only serde_json, no vb_storage | agent_context.rs lines 1–38 |
| WAIVER-STATUS | resolved | Static analysis confirms commands_status::build_status creates Shard::new (transient), no FjallJournal::open | commands_status.rs lines 24–31 |
| WAIVER-LEAN | waiver | N/A — Lean proof deferred | formal-methods bead |
| GATE-001 | gauntlet-fast | `moon run :verify-fast` | moon-ci-report.txt |
| GATE-002 | gauntlet-standard | `moon run :verify-standard` | moon-ci-report.txt |
| GATE-003 | gauntlet-deep | `moon run :verify-deep` | moon-ci-report.txt |

---

## Section 10 — Waivers

| Waiver | Reason | Impact | Resolution |
|--------|--------|--------|------------|
| UI mode activation | Not implemented yet — deferred to UI bead | Tests for UI mode commands skipped until UI bead exists | Deferred; see ERR-UI-INIT |
| Lean formalization | Not yet written | Proptest + manual-qa provide equivalent evidence | Deferred to formal-methods bead |

All three previously-uncertain commands (`bench-run`, `agent-context`, `status`) are now confirmed Pure via static analysis:

- **`bench-run`** (`bench.rs:9`): calls `vb_runtime::Runtime::new(shard_count, config)` — no `FjallJournal::open`. In-memory only. Classification: **Pure**.
- **`agent-context`** (`agent_context.rs:6`): `build()` returns a static `serde_json::json!` object. No storage, no runtime, no external calls. Classification: **Pure**.
- **`status`** (`commands_status.rs:28`): `Shard::new(ShardConfig::default())` — transient in-memory Shard. No `FjallJournal::open`. Classification: **Pure**.

---

## Exit Criteria

- [ ] Every proof obligation has a test method assigned
- [ ] Every ParseError variant has a dedicated test
- [ ] Every CliExitCode discriminant has a distinct-value test
- [ ] Every Pure command has a no-storage proptest invariant
- [ ] Every Storage command has an opens-storage proptest invariant
- [ ] cargo-fuzz target exists for malformed storage paths
- [ ] Kani harness covers mode activation completeness
- [ ] Mutation kill rate target: ≥90%
- [ ] No test assertion is `is_ok()` or `is_err()` — all use exact value matching
- [ ] All 48 proof obligations are traceable to test methods
