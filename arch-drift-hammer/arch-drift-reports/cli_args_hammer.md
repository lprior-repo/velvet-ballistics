# Architectural Drift Report: vb_cli/src/args.rs

## Line Count Violation

| Metric | Value |
|--------|-------|
| **Current** | 2969 lines |
| **Limit** | 300 lines |
| **Ratio** | 9.9x OVER |
| **Violation** | CATASTROPHIC - single file is 10x the size limit |

---

## Parse Functions Found (47 total)

| # | Function | Line | Signature | Primitive Args |
|---|----------|------|-----------|----------------|
| 1 | `parse_args` | 336 | `fn parse_args(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 2 | `parse_system` | 384 | `fn parse_system(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 3 | `parse_ai_context` | 394 | `fn parse_ai_context(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 4 | `parse_agent_context` | 404 | `fn parse_agent_context(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 5 | `parse_status` | 448 | `fn parse_status(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 6 | `parse_status_active_runs` | 456 | `fn parse_status_active_runs(args: &[OsString], options: StatusOptions) -> Result<(StatusOptions, &[OsString]), ParseError>` | None |
| 7 | `parse_status_queue_depth` | 471 | `fn parse_status_queue_depth(...)` | None |
| 8 | `parse_status_trace_dropped` | 486 | `fn parse_status_trace_dropped(...)` | None |
| 9 | `parse_status_options` | 500 | `fn parse_status_options(mut args: &[OsString], mut options: StatusOptions) -> Result<StatusOptions, ParseError>` | None |
| 10 | `parse_system_status_tokens` | 578 | `fn parse_system_status_tokens(tokens: &[OsString]) -> Result<Command, ParseError>` | None |
| 11 | `parse_system_status_options` | 584 | `fn parse_system_status_options(args: &[OsString], options: SystemStatusOptions) -> Result<SystemStatusOptions, ParseError>` | None |
| 12 | `parse_system_status_emit` | 608 | `fn parse_system_status_emit(args: &[OsString], options: SystemStatusOptions) -> Result<SystemStatusOptions, ParseError>` | **YES** - `raw.to_str()` → `&str` compared directly |
| 13 | `parse_system_status_profile` | 634 | `fn parse_system_status_profile(args: &[OsString], options: SystemStatusOptions) -> Result<SystemStatusOptions, ParseError>` | **YES** - `raw.to_str()` → `&str` |
| 14 | `parse_system_status_server` | 665 | `fn parse_system_status_server(args: &[OsString], options: SystemStatusOptions) -> Result<SystemStatusOptions, ParseError>` | **YES** - `raw.to_str()` → `&str` |
| 15 | `parse_action` | 683 | `fn parse_action(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 16 | `parse_action_inspect` | 711 | `fn parse_action_inspect(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 17 | `parse_action_inspect_args` | 734 | `fn parse_action_inspect_args(args: &[OsString], state: ActionInspectParseState) -> Result<ActionInspectParseState, ParseError>` | **YES** - `raw.to_str()` → `&str` |
| 18 | `parse_action_inspect_emit` | 754 | `fn parse_action_inspect_emit(args: &[OsString], state: ActionInspectParseState) -> Result<ActionInspectParseState, ParseError>` | **YES** - `raw.to_str()` → `&str` |
| 19 | `parse_action_list_args` | 784 | `fn parse_action_list_args(args: &[OsString], state: ActionListParseState) -> Result<ActionListParseState, ParseError>` | **YES** - `raw.to_str()` → `&str` |
| 20 | `parse_action_list_emit` | 802 | `fn parse_action_list_emit(args: &[OsString], state: ActionListParseState) -> Result<ActionListParseState, ParseError>` | **YES** - `raw.to_str()` → `&str` |
| 21 | `parse_status_usize_value` | 837 | `fn parse_status_usize_value<'a>(args: &'a [OsString], flag: &'static str) -> Result<ParsedStatusValue<'a, usize>, ParseError>` | **YES** - `flag: &'static str` |
| 22 | `parse_status_u64_value` | 853 | `fn parse_status_u64_value<'a>(args: &'a [OsString], flag: &'static str) -> Result<ParsedStatusValue<'a, u64>, ParseError>` | **YES** - `flag: &'static str` |
| 23 | `parse_status_value` | 869 | `fn parse_status_value<'a>(args: &'a [OsString], flag: &'static str) -> Result<ParsedStatusValue<'a, &'a str>, ParseError>` | **YES** - `flag: &'static str`, returns `&str` |
| 24 | `validate_status_options` | 885 | `fn validate_status_options(options: StatusOptions) -> Result<StatusOptions, ParseError>` | None |
| 25 | `validate_status_usize_limit` | 896 | `fn validate_status_usize_limit(value: Option<usize>, max: usize, flag: &'static str) -> Result<(), ParseError>` | **YES** - `flag: &'static str` |
| 26 | `parse_action_registry_arg` | 909 | `fn parse_action_registry_arg(args: &[OsString], state: ActionListParseState) -> Result<ActionListParseState, ParseError>` | **YES** - `raw.to_str()` → `&str` |
| 27 | `parse_action_inspect_registry_arg` | 925 | `fn parse_action_inspect_registry_arg(args: &[OsString], state: ActionInspectParseState) -> Result<ActionInspectParseState, ParseError>` | **YES** - `raw.to_str()` → `&str` |
| 28 | `parse_action_registry_mode` | 941 | `fn parse_action_registry_mode(value: &str) -> Result<ActionRegistryMode, ParseError>` | **YES** - `value: &str` matched against literals |
| 29 | `parse_verify` | 950 | `fn parse_verify(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 30 | `parse_validate` | 976 | `fn parse_validate(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 31 | `parse_explain` | 984 | `fn parse_explain(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 32 | `parse_compile` | 992 | `fn parse_compile(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 33 | `parse_run` | 1013 | `fn parse_run(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 34 | `parse_optional_step` | 1035 | `fn parse_optional_step(args: &[OsString]) -> Result<Option<StepTarget>, ParseError>` | None |
| 35 | `parse_run_compiled` | 1051 | `fn parse_run_compiled(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 36 | `parse_optional_run_db` | 1071 | `fn parse_optional_run_db(args: &[OsString], durability: DurabilityMode) -> Result<Option<PathBuf>, ParseError>` | None |
| 37 | `parse_ipc_serve` | 1085 | `fn parse_ipc_serve(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 38 | `parse_inspect` | 1095 | `fn parse_inspect(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 39 | `parse_run_db_args` | 1112 | `fn parse_run_db_args(args: &[OsString], command: &'static str) -> Result<RunDbArgs, ParseError>` | **YES** - `command: &'static str` |
| 40 | `parse_events` | 1125 | `fn parse_events(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 41 | `parse_event_status` | 1145 | `fn parse_event_status(raw: &str) -> Result<EventStatus, ParseError>` | **YES** - `raw: &str` |
| 42 | `parse_event_limit` | 1157 | `fn parse_event_limit(raw: &str) -> Result<i64, ParseError>` | **YES** - `raw: &str` |
| 43 | `parse_replay` | 1162 | `fn parse_replay(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 44 | `parse_trace` | 1172 | `fn parse_trace(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 45 | `validate_trace_args` | 1184 | `fn validate_trace_args(args: &[OsString]) -> Result<(), ParseError>` | None |
| 46 | `parse_trace_filters` | 1247 | `fn parse_trace_filters(args: &[OsString]) -> Result<TraceFilters, ParseError>` | None |
| 47 | `parse_trace_u16` | 1283 | `fn parse_trace_u16(flag: &'static str, raw: &str) -> Result<u16, ParseError>` | **YES** - `flag: &'static str`, `raw: &str` |
| 48 | `parse_trace_limit` | 1288 | `fn parse_trace_limit(raw: &str) -> Result<usize, ParseError>` | **YES** - `raw: &str` |
| 49 | `parse_trace_u64` | 1293 | `fn parse_trace_u64(flag: &'static str, raw: &str) -> Result<u64, ParseError>` | **YES** - `flag: &'static str`, `raw: &str` |
| 50 | `parse_trace_status` | 1298 | `fn parse_trace_status(raw: &str) -> Result<TraceStatus, ParseError>` | **YES** - `raw: &str` |
| 51 | `parse_retry` | 1310 | `fn parse_retry(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 52 | `parse_resume` | 1320 | `fn parse_resume(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 53 | `parse_cancel` | 1330 | `fn parse_cancel(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 54 | `parse_bench_run` | 1349 | `fn parse_bench_run(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 55 | `parse_doctor` | 1357 | `fn parse_doctor(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 56 | `parse_answer` | 1364 | `fn parse_answer(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 57 | `parse_graph` | 1386 | `fn parse_graph(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 58 | `parse_diff` | 1394 | `fn parse_diff(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 59 | `parse_incident` | 1408 | `fn parse_incident(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 60 | `parse_simulate` | 1418 | `fn parse_simulate(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 61 | `parse_submit` | 1426 | `fn parse_submit(args: &[OsString]) -> Result<Command, ParseError>` | None |
| 62 | `parse_durability` | 1446 | `fn parse_durability(raw: &str) -> Result<DurabilityMode, ParseError>` | **YES** - `raw: &str` |
| 63 | `parse_server_mode` | 1455 | `fn parse_server_mode(raw: &str) -> Result<DurabilityMode, ParseError>` | **YES** - `raw: &str` |
| 64 | `parse_output_format` | 1463 | `fn parse_output_format(args: &[OsString]) -> OutputFormat` | None |
| 65 | `parse_compile_output_format` | 1471 | `fn parse_compile_output_format(_args: &[OsString]) -> OutputFormat` | None |
| 66 | `positional_str` | 1475 | `fn positional_str(args: &[OsString], index: usize, name: &'static str) -> Result<String, ParseError>` | **YES** - `name: &'static str` |
| 67 | `named_flag` | 1486 | `fn named_flag(args: &[OsString], flag: &str) -> Option<String>` | **YES** - `flag: &str` |
| 68 | `optional_named_flag` | 1498 | `fn optional_named_flag(args: &[OsString], flag: &'static str) -> Result<Option<String>, ParseError>` | **YES** - `flag: &'static str` |
| 69 | `find_positional` | 1519 | `fn find_positional(args: &[OsString], start_idx: usize, command: &'static str) -> Option<PathBuf>` | **YES** - `command: &'static str` |
| 70 | `has_subcommand_help` | 1536 | `fn has_subcommand_help(args: &[OsString]) -> bool` | None |
| 71 | `validate_known_flags` | 1543 | `fn validate_known_flags(args: &[OsString], command: &'static str) -> Result<(), ParseError>` | **YES** - `command: &'static str` |
| 72 | `validate_flag_value` | 1563 | `fn validate_flag_value(args: &[OsString], index: usize, command: &'static str, spec: FlagSpec) -> Result<usize, ParseError>` | **YES** - `command: &'static str` |
| 73 | `validate_flag_value_domain` | 1586 | `fn validate_flag_value_domain(command: &'static str, name: &'static str, value: &str) -> Result<(), ParseError>` | **YES** - `command: &'static str`, `name: &'static str`, `value: &str` |
| 74 | `advance_arg_index` | 1607 | `fn advance_arg_index(index: usize, amount: usize) -> Result<usize, ParseError>` | None |
| 75 | `argument_index_overflow` | 1613 | `fn argument_index_overflow() -> ParseError` | None |
| 76 | `known_flag_spec` | 1617 | `fn known_flag_spec(command: &'static str, token: &str) -> Option<FlagSpec>` | **YES** - `command: &'static str`, `token: &str` |
| 77 | `output_flag_spec` | 1688 | `fn output_flag_spec(token: &str) -> Option<FlagSpec>` | **YES** - `token: &str` |
| 78 | `value_flag_spec` | 1696 | `fn value_flag_spec(token: &str, flag: &'static str) -> Option<FlagSpec>` | **YES** - `token: &str`, `flag: &'static str` |

**Total parse functions: 47 command-level + 31 utility = 78 total functions**
**Primitive obsession violations: 36 functions accept raw `&str`**

---

## Primitive Obsession Violations (parse_* functions using raw `&str`)

### Category 1: Flag name parameters as `&'static str`

These should be typed domain **flag identifiers**:

| Function | Line | Parameter |
|----------|------|-----------|
| `parse_status_usize_value` | 837 | `flag: &'static str` |
| `parse_status_u64_value` | 853 | `flag: &'static str` |
| `parse_status_value` | 869 | `flag: &'static str` (also returns `&str`) |
| `validate_status_usize_limit` | 896 | `flag: &'static str` |
| `parse_run_db_args` | 1112 | `command: &'static str` |
| `parse_trace_u16` | 1283 | `flag: &'static str` |
| `parse_trace_u64` | 1293 | `flag: &'static str` |
| `positional_str` | 1475 | `name: &'static str` |
| `named_flag` | 1486 | `flag: &str` |
| `optional_named_flag` | 1498 | `flag: &'static str` |
| `find_positional` | 1519 | `command: &'static str` |
| `validate_known_flags` | 1543 | `command: &'static str` |
| `validate_flag_value` | 1563 | `command: &'static str` |
| `validate_flag_value_domain` | 1586 | `command: &'static str`, `name: &'static str` |
| `known_flag_spec` | 1617 | `command: &'static str` |
| `output_flag_spec` | 1688 | `token: &str` |
| `value_flag_spec` | 1696 | `token: &str`, `flag: &'static str` |

### Category 2: Raw value `&str` being pattern-matched

These should use domain **value types**:

| Function | Line | Problem |
|----------|------|---------|
| `parse_system_status_emit` | 608 | `raw.to_str()` → match against `"yaml"`, `"text"` literals |
| `parse_system_status_profile` | 634 | `raw.to_str()` → match against `"quick"`, `"standard"`, `"full"` |
| `parse_system_status_server` | 665 | `raw.to_str()` → `parse_server_mode(value)` |
| `parse_action_inspect_args` | 734 | `raw.to_str()` → match `"--emit"`, `"--registry"` |
| `parse_action_inspect_emit` | 754 | `raw.to_str()` → match `"yaml"`, `"postcard"`, `"text"` |
| `parse_action_list_args` | 784 | `raw.to_str()` → match `"--emit"`, `"--registry"` |
| `parse_action_list_emit` | 802 | `raw.to_str()` → match `"yaml"`, `"postcard"`, `"text"` |
| `parse_action_registry_arg` | 909 | `raw.to_str()` → passed to `parse_action_registry_mode` |
| `parse_action_inspect_registry_arg` | 925 | `raw.to_str()` → passed to `parse_action_registry_mode` |
| `parse_action_registry_mode` | 941 | `value: &str` → match `"registered"`, `"empty"`, `"uninitialized"` |
| `parse_event_status` | 1145 | `raw: &str` → match `"pending"`, `"active"`, etc. |
| `parse_event_limit` | 1157 | `raw: &str` → `.parse::<i64>()` |
| `parse_trace_u16` | 1283 | `raw: &str` → `.parse::<u16>()` |
| `parse_trace_limit` | 1288 | `raw: &str` → `.parse::<usize>()` |
| `parse_trace_u64` | 1293 | `raw: &str` → `.parse::<u64>()` |
| `parse_trace_status` | 1298 | `raw: &str` → match `"pending"`, `"active"`, etc. |
| `parse_durability` | 1446 | `raw: &str` → match `"strict"`, `"journaled"`, `"none"` |
| `parse_server_mode` | 1455 | `raw: &str` → match `"none"` |
| `validate_flag_value_domain` | 1586 | `value: &str` → match `"text"`, `"yaml"`, `"postcard"` |

---

## Missing Domain Types

### Identity Types (newtypes wrapping `String`)

| Current | Proposed Domain Type | File |
|---------|---------------------|------|
| `String` (run_id) | `RunId(String)` | `types/run.rs` |
| `String` (run_a, run_b in diff) | `DiffRunId(String)` | `types/run.rs` |
| `String` (reason in cancel) | `CancelReason(String)` | `types/run.rs` |
| `String` (deliver in agent-context) | `DeliverTarget(String)` | `types/run.rs` |

### Path Types (newtypes wrapping `PathBuf`)

| Current | Proposed Domain Type | File |
|---------|---------------------|------|
| `PathBuf` (workflow) | `WorkflowPath(PathBuf)` | `types/workflow.rs` |
| `PathBuf` (input_bin) | `InputBin(PathBuf)` | `types/workflow.rs` |
| `PathBuf` (socket) | `SocketPath(PathBuf)` | `types/workflow.rs` |
| `PathBuf` (value_file) | `ValueFilePath(PathBuf)` | `types/workflow.rs` |
| `PathBuf` (db) | `DatabasePath(PathBuf)` | `types/workflow.rs` |

### Numeric Types (newtypes wrapping primitives)

| Current | Proposed Domain Type | File |
|---------|---------------------|------|
| `u16` (action_id) | `ActionId(u16)` | `types/action.rs` |
| `u16` (step_id) | `StepId(u16)` | `types/step.rs` |
| `u64` (since_seq, until_seq) | `SeqNumber(u64)` | `types/step.rs` |
| `usize` (limit) | `Limit(usize)` | `types/step.rs` |
| `i64` (event limit) | `EventLimit(i64)` | `types/step.rs` |

### Enum-Like Domain Types (from string matching)

| Current | Proposed Domain Type | File |
|---------|---------------------|------|
| `&str` matched to `"quick"`, `"standard"`, `"full"` | `VerifyProfileKind` | `types/verify.rs` |
| `&str` matched to `"strict"`, `"journaled"`, `"none"` | `DurabilityKind` | `types/durability.rs` |
| `&str` matched to `"registered"`, `"empty"`, `"uninitialized"` | `RegistryKind` | `types/action.rs` |
| `&str` matched to `"pending"`, `"active"`, etc. | `EventStatusKind` | `types/event.rs` |
| `&str` matched to `"text"`, `"yaml"`, `"postcard"` | `EmitKind` | `types/output.rs` |
| `&str` matched to `"ir"`, `"yaml"`, `"postcard"` | `EmitTargetKind` | `types/output.rs` |
| `&str` matched to `"none"` (server mode) | `ServerModeKind` | `types/durability.rs` |

### Flag Identifier Types

| Current | Proposed Domain Type | File |
|---------|---------------------|------|
| `&'static str` flag names like `"--emit"`, `"--db"` | `EmitFlag`, `DbFlag`, `ProfileFlag`, etc. | `args/flags.rs` |

---

## Recommended File Split

### Target Structure

```
crates/vb_cli/src/args/
├── mod.rs                    # ~50 lines - re-exports
├── types/
│   ├── mod.rs               # ~20 lines
│   ├── run.rs               # ~80 lines - RunId, DiffRunId, CancelReason, DeliverTarget
│   ├── workflow.rs          # ~80 lines - WorkflowPath, InputBin, SocketPath, ValueFilePath, DatabasePath
│   ├── step.rs              # ~80 lines - StepId, SeqNumber, Limit, EventLimit, StepTarget
│   ├── action.rs            # ~80 lines - ActionId, RegistryKind, ActionRegistryMode
│   ├── verify.rs            # ~60 lines - VerifyProfileKind, VerifyProfile
│   ├── durability.rs        # ~60 lines - DurabilityKind, DurabilityMode, ServerModeKind
│   ├── event.rs             # ~60 lines - EventStatusKind, EventStatus
│   └── output.rs            # ~80 lines - EmitKind, EmitTargetKind, OutputFormat
├── flags/
│   ├── mod.rs               # ~30 lines
│   ├── spec.rs              # ~120 lines - FlagSpec, known_flag_spec, output_flag_spec, value_flag_spec
│   └── validate.rs          # ~80 lines - validate_known_flags, validate_flag_value, validate_flag_value_domain
├── shared/
│   ├── mod.rs               # ~30 lines
│   ├── named.rs             # ~60 lines - named_flag, optional_named_flag
│   ├── positional.rs        # ~60 lines - find_positional, positional_str
│   └── parse.rs             # ~50 lines - advance_arg_index, argument_index_overflow, ParsedValue
├── commands/
│   ├── mod.rs               # ~50 lines - Command enum + ParseError + VALID_COMMANDS
│   ├── help.rs              # ~20 lines - help/version parsing
│   ├── agent_context.rs     # ~60 lines - parse_agent_context
│   ├── ai_context.rs        # ~40 lines - parse_ai_context
│   ├── status.rs            # ~200 lines - StatusOptions, SystemStatusOptions + parsers
│   ├── action.rs            # ~180 lines - ActionListParseState, ActionInspectParseState + parsers
│   ├── workflow.rs         # ~200 lines - verify, validate, explain, compile, graph, simulate, bench_run
│   ├── run.rs              # ~200 lines - run, run_compiled, submit + step parsing
│   ├── ipc.rs              # ~40 lines - ipc_serve
│   ├── inspect.rs          # ~60 lines - inspect, replay, retry, resume, incident
│   ├── events.rs           # ~80 lines - events + event_status, event_limit parsing
│   ├── trace.rs            # ~180 lines - trace + all filter parsers
│   ├── cancel.rs           # ~60 lines - cancel + reason length validation
│   ├── doctor.rs           # ~40 lines - doctor
│   ├── answer.rs           # ~60 lines - answer
│   └── diff.rs             # ~50 lines - diff
├── parser.rs               # ~50 lines - parse_args dispatcher
└── display.rs              # ~100 lines - ParseError Display impl
```

### Estimated Final Line Counts

| File | Est. Lines |
|------|------------|
| `mod.rs` (root re-exports) | 50 |
| `types/mod.rs` | 20 |
| `types/*.rs` (8 files) | 560 |
| `flags/mod.rs` | 30 |
| `flags/spec.rs` | 120 |
| `flags/validate.rs` | 80 |
| `shared/mod.rs` | 30 |
| `shared/named.rs` | 60 |
| `shared/positional.rs` | 60 |
| `shared/parse.rs` | 50 |
| `commands/mod.rs` | 150 |
| `commands/help.rs` | 20 |
| `commands/agent_context.rs` | 60 |
| `commands/ai_context.rs` | 40 |
| `commands/status.rs` | 200 |
| `commands/action.rs` | 180 |
| `commands/workflow.rs` | 200 |
| `commands/run.rs` | 200 |
| `commands/ipc.rs` | 40 |
| `commands/inspect.rs` | 60 |
| `commands/events.rs` | 80 |
| `commands/trace.rs` | 180 |
| `commands/cancel.rs` | 60 |
| `commands/doctor.rs` | 40 |
| `commands/answer.rs` | 60 |
| `commands/diff.rs` | 50 |
| `parser.rs` | 50 |
| `display.rs` | 100 |
| **TOTAL** | **2350** |

**Remaining overage**: ~2350 lines still exceeds 300 line limit per-file principle.

### Second-Order Split Required

The `commands/` directory itself violates cohesion. Group by DDD bounded context:

```
commands/
├── workflow_ctx/           # verify, validate, explain, compile, graph, simulate, bench_run, submit
│   ├── mod.rs              # ~40 lines
│   └── *.rs                # 7 command files
├── run_ctx/               # run, run_compiled, answer, cancel, doctor
│   ├── mod.rs              # ~40 lines
│   └── *.rs                # 5 command files
├── journal_ctx/           # events, trace, inspect, replay, retry, resume, incident
│   ├── mod.rs              # ~40 lines
│   └── *.rs                # 7 command files
├── agent_ctx/             # agent-context, ai-context, status, system
│   ├── mod.rs              # ~40 lines
│   └── *.rs                # 4 command files
├── action_ctx/            # action list, action inspect
│   ├── mod.rs              # ~30 lines
│   └── *.rs                # 2 command files
└── sys_ctx/               # ipc-serve, diff
    ├── mod.rs              # ~30 lines
    └── *.rs                # 2 command files
```

---

## Violation Summary

| Category | Count | Severity |
|----------|-------|----------|
| Lines over limit | 2669 (2969 - 300) | CRITICAL |
| Parse functions using raw `&str` | 36 | HIGH |
| Missing domain types (identities) | 4 | HIGH |
| Missing domain types (paths) | 5 | HIGH |
| Missing domain types (numerics) | 5 | MEDIUM |
| Missing domain types (enum-like) | 7 | HIGH |
| Missing flag identifier types | 17 | MEDIUM |

---

## Scott Wlaschin DDD Violations

1. **Primitive Obsession**: 36 parse functions accept raw `&str` instead of domain types. Stringly-typed `RunId`, `WorkflowPath`, `ActionId`, `StepId`, `SeqNumber`, `Limit`, `Reason`, `DeliverTarget`.

2. **Invalid Embedded Data**: String literals like `"quick"`, `"standard"`, `"full"` embedded in match statements throughout. Should be `VerifyProfileKind::Quick`, `VerifyProfileKind::Standard`, `VerifyProfileKind::Full`.

3. **Data Clumps**: `args: &[OsString]` repeated 47 times. Should be `ParseContext<'a>`.

4. **Shotgun Surgery**: Flag validation logic scattered across `known_flag_spec`, `validate_known_flags`, `validate_flag_value`, `validate_flag_value_domain`, `output_flag_spec`, `value_flag_spec`. Every new flag touches 6 functions.

5. **Parallel Hierarchies**: `ActionListParseState` and `ActionInspectParseState` are nearly identical structs with duplicated parsing patterns.

---

## Remediation Priority

1. **IMMEDIATE** (< 300 lines per file): Split into `args/` module directory
2. **HIGH** (Domain types): Introduce `RunId`, `WorkflowPath`, `ActionId`, `StepId`, `SeqNumber`, `Limit` newtypes
3. **HIGH** (Enum domains): Replace string literals with `VerifyProfileKind`, `DurabilityKind`, `EmitKind`, etc.
4. **MEDIUM** (Flag types): Create `FlagName` and `FlagValue` marker types
5. **LOW** (Refactor): Extract `ParseContext<'a>` to replace `args: &[OsString]` clumping
