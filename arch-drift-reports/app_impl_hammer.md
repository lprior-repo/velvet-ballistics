# Architectural Drift Report: vb_cli/src/app_impl.rs

## Line Count Violation

**Current: 6296 lines | Limit: 300 lines | Ratio: 20.9x OVER**

This file is a **CATASTROPHIC god module**. It violates every architectural principle in the book.

---

## Functions That Need Extraction

### Command Handlers (cmd_* functions)

| Function | Lines | Assessment |
|----------|-------|------------|
| `cmd_doctor` | 298 | **CRITICAL** - Must split into `diagnostic/` module |
| `cmd_answer` | 174 | **CRITICAL** - Must extract to `ipc/` module |
| `cmd_compile` | 162 | **CRITICAL** - Must extract to `compile/` module |
| `cmd_submit` | 161 | **CRITICAL** - Must extract to `run/` module |
| `cmd_verify` | 144 | **CRITICAL** - Must extract to `verify/` module |
| `cmd_incident` | 122 | **HIGH** - Extract to `inspect/` module |
| `cmd_diff` | 88 | **HIGH** - Extract to `inspect/` module |
| `cmd_events` | 88 | **HIGH** - Extract to `inspect/` module |
| `cmd_ipc_serve` | 79 | **HIGH** - Extract to `ipc/` module |
| `cmd_run_compiled` | 72 | **HIGH** - Extract to `run/` module |
| `cmd_inspect` | 66 | **MEDIUM** - Extract to `inspect/` module |
| `cmd_replay` | 74 | **MEDIUM** - Extract to `inspect/` module |
| `cmd_run_step` | 59 | **MEDIUM** - Extract to `run/` module |
| `cmd_retry` | 56 | **MEDIUM** - Extract to `inspect/` module |
| `cmd_resume` | 49 | **MEDIUM** - Extract to `inspect/` module |
| `cmd_validate` | 48 | **MEDIUM** - Extract to `validate/` module |
| `cmd_run` | 58 | **MEDIUM** - Extract to `run/` module |
| `cmd_explain` | 91 | **HIGH** - Extract to `validate/` module |
| `cmd_graph` | 29 | **LOW** - Extract to `compile/` module |
| `cmd_simulate` | 50 | **MEDIUM** - Extract to `compile/` module |
| `cmd_bench_run` | 117 | **HIGH** - Extract to `benchmark/` module |
| `cmd_cancel` | 69 | **MEDIUM** - Extract to `inspect/` module |
| `cmd_action_list` | 16 | **LOW** - Extract to `action/` module |
| `cmd_action_inspect` | 20 | **LOW** - Extract to `action/` module |
| `cmd_agent_context` | 13 | **LOW** - Extract to `agent/` module |
| `cmd_status` | 12 | **LOW** - Extract to `status/` module |
| `cmd_system_status` | 11 | **LOW** - Extract to `status/` module |

### Helper Functions That Need Extraction

| Function | Lines | Assessment |
|----------|-------|------------|
| `explain_error` | 285 | **CRITICAL** - Massive match statement - extract to `validate/explain.rs` |
| `explain_validation_error` | 726 | **CATASTROPHIC** - 726-line match block - MUST be in `validate/` submodule |
| `event_to_json` | 167 | **HIGH** - Extract to `inspect/` module |
| `print_event` | 90 | **HIGH** - Extract to `inspect/` module |
| `print_diff_entry` | 57 | **MEDIUM** - Extract to `inspect/` module |
| `trace_entry_to_json` | 18 | **LOW** - Extract to `inspect/` module |
| `explain_repair_hint` | 7 | **LOW** - Extract with explain group |
| `explain_gate_pass` | 3 | **LOW** - Extract with explain group |
| `explain_verification_failure` | 84 | **MEDIUM** - Extract to `verify/` module |
| `explain_failure_report` | 16 | **LOW** - Extract to shared output module |
| `explain_success_report` | 16 | **LOW** - Extract to shared output module |
| `explain_compile_failure_report` | 12 | **LOW** - Extract to shared output module |
| `explain_verification_failure_report` | 12 | **LOW** - Extract to shared output module |

---

## Primitive Obsession Violations Found

### 1. **raw `&str` run_id parsed manually everywhere** (Lines 235-257)

```rust
fn parse_run_id(raw: &str, output: OutputFormat) -> Result<vb_core::RunId, ExitCode> {
    match raw.parse::<u64>() {
        Ok(id) => {
            if id == 0 {
                // manual validation
            }
            Ok(vb_core::RunId::new(id))
        }
        Err(e) => {
            // manual error formatting
        }
    }
}
```

**Problems:**
- `&str` passed around instead of `RunId`
- Manual validation scattered: `if id == 0`
- Manual error messages constructed in each caller
- **Parse Don't Validate violation**: Should use `impl TryFrom<&str> for RunId` in a types module

**FIX REQUIRED**: Create `vb_cli/src/types/run_id.rs` with:
```rust
pub struct RunId(vb_core::RunId);

impl TryFrom<&str> for RunId {
    type Error = RunIdParseError;
    fn try_from(value: &str) -> Result<Self, Self::Error> { ... }
}
```

### 2. **Raw `&str` for output format string parsing** (Lines 6135-6141)

```rust
fn parse_emit_output_format(raw: Option<&str>) -> OutputFormat {
    match raw {
        Some("yaml") => OutputFormat::Yaml,
        Some("postcard") => OutputFormat::Postcard,
        Some("text") | Some(_) | None => OutputFormat::Text,
    }
}
```

**FIX REQUIRED**: `impl TryFrom<&str> for OutputFormat` in types module.

### 3. **Raw `u64` timestamps generated manually** (Lines 1517-1525, 5830-5838)

```rust
fn generate_submit_run_id() -> u64 {
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return u64::MAX;
    };
    match u64::try_from(now.as_nanos()) {
        Ok(value) => value,
        Err(_) => now.as_secs(),
    }
}
```

**FIX REQUIRED**: `impl RunId { pub fn generate() -> Self }` on the RunId wrapper type.

### 4. **`&str` command names passed to trace helpers** (Lines 2976-2987)

```rust
fn write_locked_read_surface(
    command: &'static str,
    run_id: &str,
    output: OutputFormat,
) -> ExitCode {
```

**FIX REQUIRED**: Enum-based command dispatch instead of stringly-typed.

---

## Parse Don't Validate Violations

### 1. **Manual run_id validation in `cmd_answer`** (Lines 3198-3214)

```rust
let rid = match run_id.parse::<u64>() {
    Ok(id) => vb_core::RunId::new(id),
    Err(e) => {
        // manual error construction
        return CliExitCode::ValidationFailed.into();
    }
};
```

**Should be**: `let rid = run_id.try_into()?;`

### 2. **Manual journal existence check before opening** (Lines 278-286)

```rust
if !db.exists() {
    let msg = format!("journal directory does not exist: {}", db.display());
    // manual error
    return Err(CliExitCode::StorageError.into());
}
```

**Should be**: Journal open should return this error directly - don't pre-validate.

### 3. **Manual step_id bounds checking** (Lines 1557-1576)

```rust
let step_idx = vb_core::StepIdx::new(target.step_id);
let node = match compiled.node(step_idx) {
    Some(n) => n,
    None => {
        let msg = format!("step {} not found in workflow", target.step_id);
        // manual error
    }
};
```

**Should be**: `impl TryFrom<u16> for StepIdx` that returns appropriate error.

### 4. **Duplicate code in `cmd_answer` IPC socket path derivation** (Lines 3235-3237)

```rust
let socket_path = db.with_extension("sock");
```

This is manual path manipulation that should be in a helper type.

---

## DDD Boundary Violations

### Current God Module Structure

```
app_impl.rs (6296 lines)
├── validate/commands (validate, verify, explain)
├── compile/commands (compile, graph, simulate)
├── run/commands (run, run_step, run_compiled, submit, bench_run)
├── ipc/commands (ipc_serve, answer)
├── inspect/commands (inspect, events, replay, trace, retry, resume, cancel, diff, incident)
├── action/commands (action_list, action_inspect)
├── status/ (status, system_status)
├── output/ (ALL json/yaml/text formatting helpers)
└── ERROR: 726-line explain_validation_error dumped in root
```

### Missing DDD Bounded Contexts

1. **Validation Context** - validate, verify, explain are THREE commands but live with compilation
2. **Run Context** - run, run_step, run_compiled, submit, bench_run should be a cohesive `run/` module
3. **Inspect Context** - 8 commands all hitting journal - should be `inspect/` bounded context
4. **IPC Context** - ipc_serve and answer are clearly coupled
5. **Action Registry Context** - action_list and action_inspect are a mini-context
6. **Output Context** - ALL formatting code should be in `output/` module

---

## Recommended Module Split

```
vb_cli/src/
├── app_impl.rs          # <300 lines - dispatcher only
├── args.rs              # Keep argument parsing
├── exit_code.rs         # Keep exit codes
├── main_tests.rs        # Keep tests
├── mode_activation_tests.rs
│
├── cmd/                 # Command handlers - each file <300 lines
│   ├── mod.rs
│   ├── validate.rs      # cmd_validate (48 lines)
│   ├── verify.rs        # cmd_verify (144 lines)
│   ├── explain.rs       # cmd_explain + ALL explain_* helpers (1200+ lines across files)
│   │                    # BREAK THIS UP: explain_error.rs, explain_validation.rs
│   ├── compile.rs       # cmd_compile (162 lines)
│   ├── graph.rs         # cmd_graph (29 lines)
│   ├── simulate.rs       # cmd_simulate (50 lines)
│   ├── run.rs           # cmd_run (58 lines)
│   ├── run_step.rs      # cmd_run_step (59 lines)
│   ├── run_compiled.rs  # cmd_run_compiled (72 lines)
│   ├── submit.rs        # cmd_submit (161 lines)
│   ├── bench_run.rs     # cmd_bench_run (117 lines)
│   ├── ipc_serve.rs     # cmd_ipc_serve (79 lines)
│   ├── answer.rs        # cmd_answer (174 lines)
│   ├── inspect.rs       # cmd_inspect (66 lines)
│   ├── events.rs        # cmd_events (88 lines)
│   ├── replay.rs        # cmd_replay (74 lines)
│   ├── trace.rs         # cmd_trace (61 lines)
│   ├── retry.rs         # cmd_retry (56 lines)
│   ├── resume.rs        # cmd_resume (49 lines)
│   ├── cancel.rs        # cmd_cancel (69 lines)
│   ├── incident.rs      # cmd_incident (122 lines)
│   ├── diff.rs          # cmd_diff (88 lines)
│   ├── action.rs        # cmd_action_list + cmd_action_inspect (36 lines)
│   ├── status.rs        # cmd_status + cmd_system_status (23 lines)
│   └── agent.rs         # cmd_agent_context (13 lines)
│
├── types/               # DDD value objects
│   ├── mod.rs
│   ├── run_id.rs        # ParseDon'tValidate for RunId
│   ├── output_format.rs # ParseDon'tValidate for OutputFormat
│   └── diagnostic.rs    # Diagnostic output types
│
├── output/               # ALL formatting concerns
│   ├── mod.rs
│   ├── json.rs          # json_out, json_error, etc.
│   ├── text.rs          # outln!, errln!, text formatting
│   ├── yaml.rs          # YAML-specific output
│   └── postcard.rs      # Postcard-specific output
│
├── storage/             # Journal/storage helpers
│   ├── mod.rs
│   ├── journal.rs       # read_journal_events, open helpers
│   └── errors.rs        # Storage error helpers
│
└── explain/             # Validation explanation (massive)
    ├── mod.rs
    ├── error.rs         # explain_error (285 lines)
    ├── validation.rs    # explain_validation_error (726 lines) 
    │                    # SPLIT by error category
    └── gates.rs         # explain_verification_failure, explain_gate_pass
```

---

## Refactoring Order (Highest Impact First)

### Phase 1: CRITICAL - Extract Doctor and Answer (Largest Functions)

**1.1** Extract `cmd_doctor` (298 lines) → `cmd/doctor.rs`
- Also extract `cmd_doctor_without_db`, `open_doctor_journal`, `unique_doctor_run_id`
- Create `storage/doctor.rs` for journal diagnostics

**1.2** Extract `cmd_answer` (174 lines) → `cmd/answer.rs`
- Move `IpcClient` connection logic to `ipc/` module

**1.3** Extract `cmd_submit` (161 lines) → `cmd/submit.rs`
- Move `generate_submit_run_id` to `types/run_id.rs`

### Phase 2: CRITICAL - Extract Compile/Verify/Explain Triad

**2.1** Extract `cmd_compile` (162 lines) → `cmd/compile.rs`
- Move `EmitTarget` handling to its own match-based formatter

**2.2** Extract `cmd_verify` (144 lines) → `cmd/verify.rs`
- Extract `verify_success_report`, `verify_error_message`

**2.3** Extract `explain_*` family (1000+ lines) → `explain/` module
- **BREAK UP `explain_validation_error`** - split by error category files
- Create `explain/errors.rs`, `explain/validation.rs`, `explain/gates.rs`

### Phase 3: HIGH - Extract Run Context

**3.1** Extract `cmd_run`, `cmd_run_step`, `cmd_run_compiled` → `cmd/run.rs` (or separate files)
**3.2** Extract `cmd_bench_run` → `cmd/bench_run.rs`
**3.3** Move `map_runtime_inputs`, `runtime_journal_for_mode`, `run_compiled_workflow` to `run/engine.rs`

### Phase 4: HIGH - Extract Inspect Context

**4.1** Extract 8 inspect commands → `cmd/inspect/` directory
- `cmd_inspect`, `cmd_events`, `cmd_replay`, `cmd_trace`
- `cmd_retry`, `cmd_resume`, `cmd_cancel`, `cmd_diff`, `cmd_incident`

**4.2** Extract `print_event`, `event_to_json` → `inspect/event.rs`
**4.3** Extract `print_diff_entry` → `inspect/diff.rs`

### Phase 5: HIGH - Extract IPC Context

**5.1** Extract `cmd_ipc_serve` → `cmd/ipc_serve.rs`
**5.2** Extract `StorageWorkflowResolver` → `ipc/resolver.rs`

### Phase 6: Types Module (Parse Don't Validate)

**6.1** Create `types/run_id.rs`
```rust
pub struct CliRunId(vb_core::RunId);

impl TryFrom<&str> for CliRunId {
    type Error = RunIdError;
}

impl CliRunId {
    pub fn inner(&self) -> vb_core::RunId;
    pub fn generate() -> Self;
}
```

**6.2** Update all 15+ call sites to use `CliRunId::try_from(raw)?`

### Phase 7: Output Module

**7.1** Extract ALL `json_out`, `emit_json_or_return!`, `json_error` → `output/json.rs`
**7.2** Extract `outln!`, `errln!` macros + text helpers → `output/text.rs`
**7.3** Extract postcard encoding → `output/postcard.rs`

### Phase 8: Validation Suite (Verify the Refactor)

After each phase:
1. Run `cargo check` to ensure no breakage
2. Run tests to ensure behavior preserved
3. Run `cargo clippy` to catch new issues
4. Verify no new `unsafe`, `unwrap`, `expect`, `panic` introduced

---

## Summary of Required Changes

| Category | Count | Lines to Move |
|----------|-------|---------------|
| Command handlers | 27 | ~1800 lines |
| Explain helpers | 10+ | ~1100 lines |
| Output/formatters | 15+ | ~500 lines |
| Storage helpers | 5+ | ~200 lines |
| Types (ParseDon'tValidate) | 3 | ~100 lines |
| **TOTAL TO EXTRACT** | **60+** | **~3700 lines** |

**Remaining in app_impl.rs after refactor**: ~2500 lines
- This is still too much
- Second pass needed to extract remaining helpers

---

## Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|------------|
| Phase 1 | HIGH - large functions with complex logic | Write behavior tests BEFORE extraction |
| Phase 2 | CRITICAL - 1000+ lines of explain_* | Split validation errors into categories FIRST |
| Phase 3-5 | MEDIUM - multiple related commands | Extract as group, test together |
| Phase 6 | LOW - pure type additions | Straightforward, well-typed |
| Phase 7 | MEDIUM - macros involved | Keep macros in original location, call from output module |
| Phase 8 | LOW - verification only | No production changes |

---

## Conclusion

This file is a **code smell monument**. It has:
- 27 command handlers in one file
- 60+ helper functions
- 726 lines of error explanation alone
- Zero ParseDon'tValidate wrappers
- Zero DDD bounded context separation

**Recommended Action**: Do NOT attempt to refactor in one PR. Create 8 separate beads (one per phase) and execute incrementally. Each phase should include behavior tests that prove the extracted code behaves identically to the original.

The architectural drift is severe but fixable. The key is incremental extraction with test coverage at every step.
