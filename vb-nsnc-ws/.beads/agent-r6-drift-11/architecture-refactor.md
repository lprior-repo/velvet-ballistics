# Architecture Refactor Report: vb_runtime/src/engine.rs

## Summary
Split oversized `engine.rs` (2351 lines) into modular submodules.

## Problem
- Original `engine.rs`: 2351 lines (exceeded 300-line limit)
- Contained types, node dispatch, drive loop, helpers, and extensive tests all in one file

## Solution
Split into 7 files under `engine/` directory:

| File | Lines | Purpose |
|------|-------|---------|
| `engine.rs` | 45 | Module declarations, re-exports, backward-compat |
| `signals.rs` | 113 | RuntimeEngineError, RetryPolicy, RuntimeSignal types |
| `action_engine.rs` | 148 | Do node execution, retry/error handling |
| `iteration_engine.rs` | 232 | ForEach/Together/Collect/Reduce handlers |
| `step_engine.rs` | 147 | execute_node_full dispatch |
| `run_engine.rs` | 71 | drive_deterministic_full, drive_with_actions |
| `transition.rs` | 71 | execute_retry_check, execute_error_handler, helpers |
| `tests.rs` | 788 | Unit and proptest tests (exempt from limits) |

## All Files Now Under 300 Lines ✓

## New Module Structure
```
engine/
├── mod.rs (45 lines) - re-exports and module declarations
├── signals.rs (113 lines) - error/signal types
├── action_engine.rs (148 lines) - action execution
├── iteration_engine.rs (232 lines) - iteration primitives
├── step_engine.rs (147 lines) - node dispatch
├── run_engine.rs (71 lines) - main loop
├── transition.rs (71 lines) - state transitions
└── tests.rs (788 lines) - tests
```

## Backward Compatibility
All public functions re-exported from `engine.rs`:
- `drive_deterministic_full`, `drive_with_actions`
- `execute_node_full`
- `execute_do`, `execute_do_without_contract`, `resume_action_outcome`
- `execute_retry_check`, `execute_error_handler`
- `RuntimeEngineError`, `RuntimeSignal`, `RetryPolicy`

## Pre-existing Issue
vb_core/workflow.rs has broken module declarations (missing submodule files). This is a pre-existing issue in the worktree, not caused by this refactor. vb_runtime/engine module structure is correct.
