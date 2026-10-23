# vb-8eocj.3.2 Repeat Counter Slot Reset Probe — Local 5090 v3 Draft

## Config Restoration

Restored `.cargo/config.toml` to exact clean baseline from revision `913cf25b`:
- Layer 1 sccache wrapper with 12G cache
- jobs = 16
- No tool config changes beyond this restoration

## Draft Defects Identified

1. `.cargo/config.toml` had drifted from baseline; restored via `jj file show -r 913cf25b` content written with `tee` (jj restore failed on read-only shared .git/objects)

2. Test file `tests/vb_f1_repeat_counter_reset_probe.rs` did not exist; written from scratch

3. Previous v2 draft had:
   - Unauthorized config changes (not applied here)
   - Missing `CollectStates` import causing broad nextest compile failure (not executed here)
   - No pre-fix constructor/runtime outcome

## Tests Written (NOT RUN)

Two tests in `tests/vb_f1_repeat_counter_reset_probe.rs`:

**Test 1: `baseline_unmutated_source_control`**
- Compiles exact baseline YAML: `Set answer=42; Repeat max 3 body Set body_result="12884901888"; Finish answer`
- Drives bounded 64 steps via `advance_bounded`
- Collects actual RuntimeSignal from runtime (not fabricated)
- Finds Repeat body node via `iter_nodes().find()` matching RepeatStart body StepIdx
- Counts body step completions from actual journal events
- Expects: `Finished(I64(42))` and exactly 3 body completions

**Test 2: `malformed_ir_body_output_aliased_to_attempt_slot`**
- Compiles same baseline, converts to `WorkflowParts` via `to_parts()`
- Finds RepeatStart node, extracts body StepIdx and attempt slot from `node.output`
- Finds body node via `iter_mut().find()` matching body StepIdx
- Mutates ONLY `body_node.output = Some(attempt_slot)` — no other fields changed
- Calls `CompiledWorkflow::try_from_parts(parts.clone())` on cloned mutated parts
- If `Err`: prints exact typed WorkflowError, returns Ok (RED preserved via constructor rejection)
- If `Ok`: drives bounded 64 steps, collects actual RuntimeSignal, body count from StepSucceeded events, attempt slot writes from SlotWritten events, reads actual attempt slot value with typed inspection
- Returns `Err` with all actual observations (signal, body_count, attempt_writes, attempt_final) if mutant accepted
- No handbuilt IR; explicitly malformed-IR negative coverage via real compiled baseline

## Import Correctness

- `vb_compile::compile_workflow` — real compilation API
- `vb_runtime::primitives::collect::CollectStates` — real collect primitive (imported but not directly invoked in this test; runtime uses internally)
- `vb_runtime::runtime::Runtime` — real runtime API
- `vb_runtime::journal::StorageRuntimeJournal` — real journal wrapper
- `vb_storage::{FjallJournal, JournalEvent}` — real storage layer
- `vb_core::workflow::{CompiledNodeKind, CompiledWorkflow}` — real workflow types
- No expect/panic/unchecked indexing; Result propagation throughout

## Status

- Config restored: ✓
- Test written: ✓
- Tests compiled: ✗ (not executed per instructions)
- Tests run: ✗ (not executed per instructions)
- Production fix: ✗ (not in scope)
- Contract updated: ✓
