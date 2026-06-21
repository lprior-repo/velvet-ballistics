# RE-004: `emit_slot_evidence` silently swallows `read_slot` errors

- **Severity**: Low
- **Category**: bug
- **Location**: `crates/vb_runtime/src/engine/drive.rs:129-150`
- **Confidence**: confirmed

## Description

`emit_slot_evidence` uses `if let Ok(value) = run.read_slot(slot)` to probe the slot. When `read_slot` returns an error (e.g., `SlotUninitialized` or `SlotOutOfBounds`), the `else` branch falls through silently and no evidence event is emitted, with no log, no counter, and no error returned to the caller. This can break the journal's "StepStarted → SlotWritten → StepSucceeded" invariant without any visible failure.

## Evidence

`crates/vb_runtime/src/engine/drive.rs:135-149`:

```rust
if let Some(slot) = collect_written_slot(node)
    && let Ok(value) = run.read_slot(slot)
{
    let extra = collect_states.capture_state(run.run_id(), slot);
    let taint = run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
    evidence
        .push_slot_written_with_extra(slot, *value, taint, extra)
        .map_err(RuntimeEngineError::Core)?;
} else if let Some(slot) = node.output
    && let Ok(value) = run.read_slot(slot)
{
    let taint = run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
    evidence.push_slot_written_with_taint(slot, *value, taint);
}
```

The `if let Ok(...) = ...` form treats any error as "no slot written here". The engine's documentation in `evidence.rs:18-22` claims:

> "This satisfies the Phase 40/44 evidence chain requirement that every deterministic step emits `StepStarted` before `SlotWritten`, followed by `StepSucceeded`."

But if `read_slot` fails on a step that *did* write (via a code path that bypassed the slot-write tracking — e.g., a primitive that writes via `value_store` directly without `run.write_slot`), the evidence chain silently loses the `SlotWritten` event and the journal sees `StepStarted → StepSucceeded` with no intermediate write. Replay will not be able to reconstruct slot state.

## Adversarial Check

1. *"The engine never writes to slots without going through `run.write_slot`."* — Mostly true, but `collect_finish` (collect/mod.rs:247-261) and `together_join` (together.rs:81-117) call `run.write_slot_with_taint(out, ...)`. If for any reason the slot read after the write fails (e.g., slot was reused, slot was invalidated by a cancellation), the evidence chain silently breaks.
2. *"The primitive would have errored on the write."* — Yes, but the slot might have been written successfully and then invalidated by a concurrent mechanism (e.g., snapshot rollback). The `else` branch hides that scenario.
3. *"Returning an error here would crash the drive loop."* — Surface it as a dedicated evidence-gap error variant, or at minimum increment an `evidence_gaps` counter. Silent dropping is the worst choice.

Severity Low: in normal operation the `if let Ok` arm always succeeds, so this is a defensive concern. But the silent fall-through is exactly the kind of error-swallowing pattern Holzman-Rust flags.

## Suggested Fix

Replace `if let Ok(value) = run.read_slot(slot)` with explicit error handling:

```rust
match run.read_slot(slot) {
    Ok(value) => { /* emit evidence */ }
    Err(e) => {
        evidence.record_evidence_gap(slot, e);
        // continue
    }
}
```

Add a `dropped_gaps: usize` counter to `EvidenceCollector` (mirroring the existing `dropped` counter for capacity overflow) and surface it via `dropped_gaps()`. Operators can then alert on a non-zero gap count.
