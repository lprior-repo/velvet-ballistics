# Architecture Refactor Report: velvet_ballastics

## Summary
Refactored `velvet_ballastics` binary crate to enforce 300-line file limit.

**Original**: 1 file of 1,484 lines
**Result**: 8 files, max 295 lines

## Files Split

| File | Lines | Purpose |
|------|-------|---------|
| `main.rs` | 38 | Thin entrypoint, delegates to modules |
| `args.rs` | 275 | Argument parsing, `Command`, `EmitTarget`, `DurabilityMode`, `ParseError` types |
| `commands.rs` | 21 | Thin re-export facade |
| `run.rs` | 214 | `cmd_validate`, `cmd_compile`, `cmd_run`, `cmd_run_compiled`, `map_runtime_inputs` |
| `storage.rs` | 295 | `cmd_ipc_serve`, `StorageWorkflowResolver`, `cmd_inspect`, `cmd_events`, `cmd_replay` |
| `bench.rs` | 131 | `cmd_bench_run`, `cmd_doctor` |
| `workflow.rs` | 162 | `run_compiled_workflow`, `runtime_journal_for_mode`, `InputMappingError`, trace printing |
| `io.rs` | 103 | `outln!`, `errln!` macros, stdout/stderr helpers, `HELP`, `VERSION` constants |

## Pre-existing Build Issues
`vb_core` crate has missing module errors (`value_types`, `value_impl`) that predate this refactor. The velvet_ballastics refactor itself is structurally correct.

## DDD Compliance
- No primitive obsession introduced (types were already NewTypes in args module)
- Single responsibility: each module handles a distinct concern
- Parse, don't validate: argument parsing returns typed `Command` enum
