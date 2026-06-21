# RE-010: `EvidenceCollector` silently drops events at capacity without surfacing to the drive loop

- **Severity**: Low
- **Category**: bug
- **Location**: `crates/vb_runtime/src/engine/evidence.rs:93-110, 120-131`
- **Confidence**: confirmed

## Description

`EvidenceCollector` enforces a fixed capacity (default `3 * 1024 = 3072` events). When `push_step_started` or `push_step_succeeded` or `push_slot_written_with_taint` is called at capacity, the event is silently dropped and `self.dropped` is incremented. The `push_*` methods return `()`, not `Result`, so the drive loop cannot detect the drop and continues as if the evidence chain were intact. Only `push_slot_written_with_extra` returns a `Result` (for the collect-pagination case); the other three paths are fallible in practice but typed infallible.

## Evidence

`crates/vb_runtime/src/engine/evidence.rs:53-57`:

```rust
const DEFAULT_EVIDENCE_CAPACITY: usize = 3 * 1024;
```

`crates/vb_runtime/src/engine/evidence.rs:91-99`:

```rust
pub fn push_step_started(&mut self, step: StepIdx) {
    if self.events.len() < self.capacity {
        self.events.push(EvidenceEvent::StepStarted { step });
    } else {
        self.dropped = self.dropped.saturating_add(1);
    }
}
```

The caller in `drive.rs:105`:

```rust
evidence.push_step_started(pc);
```

The drive loop never inspects `dropped()`. It also never resizes the collector. So if a workflow runs more than `1024` steps (3 events/step ÷ 3072 capacity = 1024 step budget equivalents), events fall on the floor silently. The journal will see a partial event stream.

The drive step budget (typically `StepBudget::DEFAULT` of a few thousand steps) is not coordinated with `DEFAULT_EVIDENCE_CAPACITY`. A workflow that legitimately consumes its full step budget can overflow the evidence buffer.

## Adversarial Check

1. *"The collector is drained each drive-loop iteration."* — If true, the capacity is per-iteration, and 3 072 events per iteration is plenty. Let me check: `drive_deterministic_full` (drive.rs:46-79) does **not** call `evidence.drain()` inside its loop. The drain is the caller's responsibility. So the collector accumulates across the entire drive invocation, not per-step.
2. *"The default step budget is small enough."* — The default is set in `vb_core::engine::StepBudget`. Even at 1 024 steps, the collector is at the edge. Larger budgets (which long-running workflows need) overflow.
3. *"Silent drop is documented."* — Yes (evidence.rs:60-62: "the evidence chain becomes incomplete but the system remains memory-safe"). Documented does not mean acceptable. The drive loop should at minimum propagate the drop count into the run's diagnostic record.

Severity Low: the system remains memory-safe and the journal will still have step-level events from the core engine path; the collector's additional evidence chain is the part that degrades. But for runs that depend on the evidence chain (e.g., post-hoc audit), the silent drop is a real correctness gap.

## Suggested Fix

Either:

(a) Make `push_step_started` / `push_step_succeeded` / `push_slot_written_with_taint` return `Result<(), EvidenceCapacityExceeded>` and surface the error to the drive loop, which can decide whether to abort the run or inflate capacity.

(b) Tie `DEFAULT_EVIDENCE_CAPACITY` to the configured `StepBudget`: `capacity = 3 * step_budget_limit` so overflow is impossible by construction.

(c) At minimum, when `drain()` is called and `dropped > 0`, log a warning or emit a synthetic `EvidenceEvent::EventsDropped { count }` marker so downstream consumers know the chain is incomplete.

Option (b) is the smallest correct fix and matches the existing comment that "3 * step_budget provides a safe upper bound" (evidence.rs:52-53) — but the actual constant `3 * 1024` is not derived from any step budget.
