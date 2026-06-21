# SJ-002: `inject_seq_gap` writes a `RunCancelled` event masquerading as a "gap marker"

- **Severity**: Critical
- **Category**: bug
- **Location**: `crates/vb_storage/src/journal/injection.rs:37`
- **Confidence**: confirmed

## Description

`inject_seq_gap` claims to inject a "sequence gap marker" that allows replay
to skip past a gap. Instead it serializes a real `JournalEvent::RunCancelled`
payload (with `RecordKind::RunCancelled` and empty `&()` body) into the events
keyspace. Any subsequent reader — `events_for_run`, recovery, lifecycle
derivation, incident analysis — will decode this as a legitimate `RunCancelled`
event and treat the run as cancelled at `gap_seq`.

## Evidence

```rust
pub fn inject_seq_gap(
    &self,
    run: vb_core::RunId,
    gap_seq: EventSeq,
) -> Result<(), JournalError> {
    let key = run_event_key(run, gap_seq)?;
    // Injected gaps use an empty record that specifically doesn't match normal
    // event serialization, but we can encode it as a placeholder.
    let value = encode_record(
        MAGIC_JOURNAL_EVENT,
        crate::records::RecordKind::RunCancelled, // Valid kind for journal events
        gap_seq.get(),
        &(), // Empty payload - decode will succeed but be meaningless
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    self.events.insert(key.to_vec(), value)?;
    Ok(())
}
```

The inline comment admits the decode will succeed but be "meaningless". This
is incorrect: every consumer of the journal (`incident::lifecycle`,
`recovery::replay::terminal::extract_terminal`,
`recovery::replay::summary::runtime_summary::apply_summary_event`) treats
`RunCancelled` as a hard terminal event. An injected gap at seq N will:

1. Set `LifecycleState::Cancelled` for the run (`lifecycle.rs:43`).
2. Set `RecoveryTerminalState::Cancelled` in the runtime summary
   (`runtime_summary.rs:53`).
3. Be returned by `extract_terminal` as the run's terminal event
   (`terminal.rs:13`).
4. Set `IncidentFailureKind::RunCancelled` in incident analysis
   (`analysis.rs:47`).

The "gap marker" is therefore a corruption event, not a marker.

## Adversarial Check

One might argue that disaster recovery operators know what they are doing and
this is documented expert-only behavior. But the function lives on the public
`FjallJournal` impl with no feature gate, and the only signal that it is
dangerous is the comment `// DANGER: This is an expert recovery tool.` There
is no on-disk tag distinguishing the injected record from a genuine
`RunCancelled` written by the runtime. Any later replay by `events_for_run`
or `recover_runtime_summary` will read it as a real cancellation. The
function also bypasses the write lock and the duplicate-detection index,
further breaking invariants. There is no code path that strips these markers
out before recovery.

## Suggested Fix

Introduce a dedicated `RecordKind::SequenceGap` (or a sentinel magic) that
decodes to a distinct `JournalEvent` variant, and have replay skip it via
`extract_terminal` / `apply_summary_event` matches. Without a distinct wire
format, this function must be removed or made `#[cfg(test)]` only.
