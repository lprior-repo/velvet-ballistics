# RS-004: Hardcoded `attempt: 1` in journal StepSucceeded ignores retry/attempt context

- **Severity**: High
- **Category**: correctness / replay divergence
- **Location**: `crates/vb_runtime/src/shard/impl_parts/evidence_flush.rs:37-52`; `crates/vb_runtime/src/shard/lifecycle/chunk_001_action.rs:61-66`; `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:72-77`
- **Confidence**: confirmed

## Description

Three distinct call sites write `RuntimeJournalEvent::StepSucceeded` with the literal `attempt: 1`, regardless of the actual retry attempt counter for the step. The journal becomes a lie: a step that completed on its 3rd retry is recorded as if it completed on attempt 1. Replay and idempotency analysis then produce wrong histories.

## Evidence

`EvidenceEvent::StepSucceeded` (defined at `engine/evidence.rs:24-36`) carries only `{ step, output }` — no attempt field. The flush step hardcodes 1:

```rust
// impl_parts/evidence_flush.rs:37-52
fn flush_step_succeeded(
    &mut self, run: RunId, step: StepIdx, output: Option<SlotIdx>,
) -> RuntimeResult<()> {
    self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
        run, step,
        output: match output { Some(slot) => slot, None => SlotIdx::ZERO },
        attempt: 1,                              // ← hardcoded
    })
}
```

The legacy action-completion path:

```rust
// lifecycle/chunk_001_action.rs:61-66
self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
    run, step,
    output: SlotIdx::ZERO,
    attempt: 1,                                  // ← hardcoded
})?;
```

The ask-answer path:

```rust
// lifecycle/chunk_002.rs:72-77
self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
    run, step: answer.ticket.ask_step,
    output: answer.answer_slot,
    attempt: 1,                                  // ← hardcoded
})?;
```

Meanwhile the *failure* path correctly records `attempt: ticket.attempt` (`lifecycle/chunk_001_action.rs:85-90`), so the journal will contain a sequence like `ActionFailed(attempt=3)` followed by `StepSucceeded(attempt=1)` — internally inconsistent.

## Adversarial Check

A defender might argue that the `attempt` field is "purely informational" and not consulted on replay. That is wrong on its face: `RuntimeJournalEvent::ActionFailed` is recorded with the actual `ticket.attempt`, and the contract on `ActionTicket::attempt` is "the live per-step attempt counter" (consulted in `helpers/action.rs:78-89` for stale-attempt detection). If `attempt` were cosmetic, `ActionFailed` would also hardcode it. The inconsistency between `ActionFailed` and `StepSucceeded` alone proves the field carries semantic weight. Even if no current replay consumer reads `StepSucceeded.attempt`, the journal is a durable contract: downstream tools (audit, time-travel debugging, analytics) read the field and assume it reflects reality.

## Suggested Fix

Extend `EvidenceEvent::StepSucceeded` with an `attempt: u16` field populated from `state.action_attempts[step]` at emission time in the drive loop, and pass it through `flush_step_succeeded`. For the legacy and ask paths, thread the actual attempt from the ticket or `state.action_attempts`.
