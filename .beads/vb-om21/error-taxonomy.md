# Error Taxonomy — vb-om21

## Railway Error Classes

| Class | Error | Behavior |
|---|---|---|
| Input/metadata | `TailMismatch { run, declared, reconstructed }` | Fail closed when suspect metadata is stale/below committed key tail. |
| Missing durable data | `MissingJournal { run }` | Recovery-required mode cannot recover a run with no `run_event` prefix. |
| Key construction | `JournalError::KeyCapacity` or existing key error | Propagate through recovery as typed storage/journal error. |
| Key parse | `InvalidJournalKeyLength` equivalent if encountered | Fail closed; do not decode sequence from short key. |
| Arithmetic | `TailOverflow { run, max_seq }` or equivalent | Fail closed if `max_seq + 1` exceeds `u64::MAX`. |
| Storage engine | `fjall::Error` mapped through existing journal/recovery error | Fail closed; no partial recovery success. |
| Payload replay | Existing `WrongRun`, `SequenceGap`, decode errors | Continue to apply when recovery decodes events after tail decision. |

## Required Typed Recovery Errors

The bead text explicitly names `TailMismatch` and `MissingJournal`. They must be structured typed variants or semantically equivalent typed wrappers, not only message strings.

### `TailMismatch`

Trigger:

```text
declared_tail < reconstructed_tail
```

Required fields:

- `run: RunId`
- `declared: EventSeq` or equivalent numeric tail type
- `reconstructed: EventSeq` or equivalent numeric tail type

Forbidden behavior:

- Continuing recovery using the lower declared tail.
- Mapping this condition to success with a warning.
- Collapsing into unstructured `String` diagnostics only.

### `MissingJournal`

Trigger:

```text
mode == RecoveryRequiresJournal && no key starts with run_prefix_key(run)
```

Required field:

- `run: RunId`

Allowed nuance:

- A pure tail-query helper may return zero tail for empty keyspace; recovery that needs journal data must still fail `MissingJournal`.

## Existing Error Interop

- Existing `RecoveryError::NoRecoveryData { run }` covers broad recovery absence today, but bead acceptance requires `MissingJournal` specificity for absent `run_event` prefix.
- Existing `JournalError::SequenceGap` and `JournalError::WrongRun` remain payload replay validation failures and should not be overloaded for stale tail metadata.
