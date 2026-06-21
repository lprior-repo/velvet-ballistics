# RS-205-help: Evidence flush drops unprocessed events after the first journal error

- **Severity**: High
- **Category**: correctness / durability
- **Location**: `crates/vb_runtime/src/shard/impl_parts/evidence_flush.rs:6`
- **Confidence**: confirmed

## Description

`flush_evidence` drains the collector into an iterator and uses `try_for_each`. If one event fails to flush, iteration stops and the remaining drained events are dropped without being journaled, traced, or restored to the collector.

## Evidence

```rust
// evidence_flush.rs:6-15
pub(crate) fn flush_evidence(
    &mut self,
    run: RunId,
    evidence: &mut EvidenceCollector,
) -> RuntimeResult<()> {
    evidence
        .drain()
        .into_iter()
        .try_for_each(|event| self.flush_evidence_event(run, event))
}
```

`try_for_each` returns immediately on the first `Err`. Because the events have already been removed from `evidence` and the local iterator is consumed only up to the failing event, every later event in the drained batch is discarded by normal `Vec` drop semantics.

## Adversarial Check

This is not mitigated by coalescing. In immediate mode, any `append_journal_event` failure from `flush_step_started`, `flush_slot_written`, or `flush_step_succeeded` stops the flush. In coalesced mode, serialization failure in `flush_slot_written` can fail before later evidence events are appended to the buffer. The collector has already been drained in both cases, so retrying cannot recover the skipped events.

## Suggested Fix

Do not destructively drain until the flush succeeds. Either keep the drained vector and restore the unprocessed suffix on error, or add a collector API that peeks events and commits removal only after all corresponding journal writes are accepted. If partial writes are allowed, return a structured partial-flush result so callers can retry the exact remainder.
