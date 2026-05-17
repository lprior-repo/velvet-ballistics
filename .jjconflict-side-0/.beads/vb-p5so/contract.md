bead_id: vb-p5so
bead_title: "runtime: Forcefully clear pending suspended timers on drain_for_shutdown"
phase: 3
updated_at: 2026-05-09T00:00:00Z

# Contract Specification: drain_for_shutdown Timer Clearance

## Context
- Feature: Forcefully clear all pending suspended timers when `drain_for_shutdown()` is called
- Domain terms:
  - `Shard`: single-threaded runtime unit owning run state
  - `pending_timers`: `IndexMap<RunId, PendingTimer>` tracking wait/ask timers per run
  - `drain_for_shutdown()`: processes command queue until shutdown or capacity limit
  - `tick()`: processes one command; returns false when shard should shut down
  - `PendingTimer`: `{ step: StepIdx, kind: PendingTimerKind }`
  - `PendingTimerKind`: `Wait | Ask`
- Assumptions:
  - `drain_for_shutdown` is called exactly once per shard shutdown sequence
  - The timer wheel lives outside the shard; shard's `pending_timers` is a fast lookup mirror
  - Clearing `pending_timers` does not need to interact with the external timer wheel (the wheel will be dropped with the runtime)
- Open questions: NONE

## Preconditions
- P1: The shard exists and is in a valid state (may have 0+ active runs and 0+ pending timers)
- P2: `drain_for_shutdown` may be called with an empty command queue
- P3: `drain_for_shutdown` may be called when `pending_timers` is already empty

## Postconditions
- PO1: After `drain_for_shutdown` returns `Ok(())`, `self.pending_timers.is_empty()` is true
- PO2: After `drain_for_shutdown` returns `Ok(())`, all runs that had pending timers no longer have timer entries in the shard
- PO3: After `drain_for_shutdown` returns `Ok(())`, `self.shutting_down` is true
- PO4: If `drain_for_shutdown` returns `Err(ShutdownInProgress)`, `pending_timers` state is unchanged (capacity limit hit before shutdown command)

## Invariants
- I1: `pending_timers` contains at most one entry per run ID
- I2: Every entry in `pending_timers` corresponds to a run that is in a suspended state (Wait or Ask)
- I3: After successful shutdown drain, `pending_timers.len() == 0`
- I4: `pending_timers.len() <= active_run_count` (a run can have at most one pending timer)

## Error Taxonomy
- `RuntimeError::ShutdownInProgress`: returned when the command queue capacity is reached before a `Shutdown` command is processed. In this case, timers are NOT cleared (shutdown was not confirmed).
- No other error variants are introduced by this change.

## Contract Signatures
```rust
impl Shard {
    /// Drains the command queue by processing commands until shutdown or capacity limit.
    /// 
    /// Postcondition: on Ok(()) — pending_timers is empty and shutting_down is true.
    /// Postcondition: on Err(ShutdownInProgress) — state unchanged.
    pub fn drain_for_shutdown(&mut self) -> RuntimeResult<()>
}
```

## Non-goals
- Do not modify the external TimerWheel (out of scope)
- Do not emit journal events for forcefully cleared timers (shutdown path is best-effort)
- Do not attempt to resume or drive runs whose timers are cleared
- Do not change the `tick()` method signature or behavior
