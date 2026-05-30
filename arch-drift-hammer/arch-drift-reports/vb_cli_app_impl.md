# Architectural Drift Report: `vb_cli/src/app_impl.rs`

**File**: `crates/vb_cli/src/app_impl.rs`
**Total Lines**: 6296
**Line Limit**: 300
**Over Limit By**: 5996 lines (2099% of allowed)

---

## Executive Summary

| Metric | Value |
|--------|-------|
| **Lines Count** | 6296 |
| **Limit** | 300 |
| **Status** | CRITICAL DRIFT |
| **DDD Cohesion** | ANTI-PATTERN: God Module |
| **Priority** | P0 — Immediate refactor required |

---

## Violations

### V1: LINE COUNT EXCEEDED (CRITICAL)
- **Actual**: 6296 lines
- **Allowed**: 300 lines
- **Severity**: P0

### V2: GOD MODULE ANTI-PATTERN (CRITICAL)
This file implements 30+ CLI commands in a single monolithic module:

| Command Group | Functions |
|--------------|-----------|
| Agent Context | `cmd_agent_context` |
| Status | `cmd_status`, `cmd_system_status` |
| Action Registry | `cmd_action_list`, `cmd_action_inspect` |
| Verify | `cmd_verify` |
| Validate | `cmd_validate` |
| Compile | `cmd_compile` |
| Run | `cmd_run`, `cmd_run_step`, `cmd_run_compiled` |
| Submit | `cmd_submit` |
| IPC | `cmd_ipc_serve` |
| Inspect | `cmd_inspect` |
| Events | `cmd_events` |
| Replay | `cmd_replay` |
| Trace | `cmd_trace` |
| Retry | `cmd_retry` |
| Resume | `cmd_resume` |
| Answer | `cmd_answer` |
| Cancel | `cmd_cancel` |
| Incident | `cmd_incident` |
| Diff | `cmd_diff` |
| Explain | `cmd_explain` |
| Graph | `cmd_graph` |
| Simulate | `cmd_simulate` |
| Bench | `cmd_bench_run` |
| Doctor | `cmd_doctor`, `cmd_doctor_without_db` |

### V3: DOMAIN KNOWLEDGE BLEED (CRITICAL)
Error explanation logic (~1500 lines) belongs in domain crates, not CLI:

| Function | Lines | Issue |
|----------|-------|-------|
| `explain_error` | ~300 | `CompileError` match arm with 80+ cases |
| `explain_validation_error` | ~750 | `ValidationError` match arm with 100+ cases |
| `explain_verification_failure` | ~100 | `VerifyError` match arm |
| `explain_compile_repair_hint` | ~300 | Repair hint tables |

**These types already have `Display` implementations in `vb_compile` and `vb_validate`.**

### V4: INFRASTRUCTURE IN CLI LAYER (HIGH)
- `StorageWorkflowResolver` (lines 2456-2478): IPC trait impl belongs in `vb_ipc`
- `cmd_ipc_serve` event loop (lines 2422-2451): Runtime management in CLI

### V5: TYPE DEFINITIONS IN WRONG MODULE (HIGH)
| Struct | Lines | Should Be |
|--------|-------|-----------|
| `ActionContractDetail` | 542-582 | `vb_core::action` |
| `ActionTableRow` | 584-593 | `vb_core::action` |
| `CliActionSpec` | 677-686 | `vb_core::action` |
| `StepStateSnapshots` | 1809-1839 | `vb_runtime::frame` |
| `InputMappingError` | 2085-2100 | `vb_core` |

### V6: PRIMITIVE OBSESSION (MEDIUM)
| Location | Issue | Fix |
|----------|-------|-----|
| `cmd_answer` (line 3198) | `run_id.parse::<u64>()` | Use `vb_core::RunId::parse()` |
| `parse_run_id` | Manual `u64` parsing | `vb_core::RunId::parse()` |

### V7: MASSIVE CODE DUPLICATION (MEDIUM)
- JSON error formatting duplicated across 30+ functions
- `write_failure_message` variants repeated
- `emit_json_or_return!` macro used 50+ times

### V8: OUTPUT FORMATTING COUPLED (MEDIUM)
- `json_out`, `write_json_pretty_stdout`, `encode_postcard_json_frame` should be in `cli_postcard` crate
- `OutputError` enum should be in separate output module

---

## DDD Cohesion Analysis

### Current Structure (VIOLATES DDD)
```
vb_cli/src/app_impl.rs (6296 lines)
├── Command dispatch (run_from_env)
├── 30+ cmd_* functions
├── File I/O helpers
├── Output formatting
├── Error handling + explanations
├── Action registry display logic
├── JSON serialization helpers
├── IPC server + resolver
└── Doctor diagnostics
```

### Scott Wlaschin DDD Violations:
1. **Bounded Contexts**: All commands in one context
2. **Cohesion**: Functions grouped by CLI surface, not domain
3. **Single Responsibility**: Each `cmd_*` does too much
4. **Tell, Don't Ask**: Procedural code, not OOP

---

## Recommended Split

```
crates/vb_cli/src/
├── app_impl.rs              # 300 lines: run_from_env dispatch only
├── commands/
│   ├── mod.rs               # Command router
│   ├── run.rs               # cmd_run, cmd_run_step, cmd_run_compiled (~400 lines)
│   ├── verify.rs            # cmd_verify, validate, explain (~400 lines)
│   ├── inspect.rs           # cmd_inspect, events, replay, trace (~300 lines)
│   ├── workflow.rs          # cmd_submit, graph, simulate, bench (~300 lines)
│   ├── status.rs            # cmd_status, system_status (~150 lines)
│   ├── actions.rs           # cmd_action_list, cmd_action_inspect (~200 lines)
│   ├── ipc.rs               # cmd_ipc_serve, StorageWorkflowResolver (~150 lines)
│   ├── doctor.rs            # cmd_doctor (~300 lines)
│   ├── incident.rs          # cmd_incident, cmd_diff (~200 lines)
│   └── lifecycle.rs         # cmd_retry, cmd_resume, cmd_cancel (~200 lines)
├── output/
│   ├── mod.rs               # OutputError, json_out
│   └── format.rs            # JSON/YAML/Postcard formatting
└── errors/
    └── explain.rs           # REMOVED — use domain crate Display impl
```

---

## Priority

**P0 — CRITICAL**

This file is 20x over the line limit and implements a God Module anti-pattern. It must be split before any new work can proceed.

---

## Status

**STATUS: MUST_REFACTOR**
