# Fjall Storage Journal

Fjall is the required embedded durability substrate for the current Backend / IR Interpreter Complete milestone. It stores workflow source, compiled IR, run headers, journal events, snapshots, blobs, and indexes; it is not the in-memory execution state.

## Key Layout

Journal keys are fixed-width big-endian bytes:

```text
events: [RunId_16B | EventSeq_8B] = 24 bytes
```

Big-endian encoding preserves numeric ordering during prefix/range scans.

## Event Encoding

Internal events use compact binary encoding through `postcard`. JSONL is a public observability projection and must not be the primary durable journal format.

The storage API exposes explicit durability names: `append_journaled` writes without a caller-visible fsync barrier, while `append_strict` appends and calls `PersistMode::SyncAll` before returning.

Duplicate `(RunId, EventSeq)` appends are rejected. Event history is immutable; insert overwrite behavior from the underlying key-value store is not exposed as a journal operation.

## Durability Modes

| Mode | Meaning | Crash Behavior |
| --- | --- | --- |
| Volatile | no Fjall append | run is lost on process crash |
| Journaled | bounded group commit via `JournalWriterQueue` | acknowledged data-loss window until persistence barrier |
| Strict | synchronous `PersistMode::SyncAll` after critical writes | strongest local durability, highest latency |

Default policy target:

- `RunAccepted` is durable before acknowledgement.
- `StepStarted` for side-effecting actions is durable before the external effect.
- `StepSucceeded` for side-effecting actions is durable before downstream side effects.
- Pure `save`/`choose` chains may group-commit when replay semantics remain valid.

## Recovery

Current-scope recovery loads accepted artifacts by digest and never reparses YAML for existing runs. Full recovery replays the journal when no snapshot exists. Snapshot recovery hydrates from the latest snapshot and replays the tail journal.

Recovery must reconstruct slot values, slot taint, step lifecycle, pending action state where supported, and terminal outcomes from durable records. Unsupported live recovery states must fail closed with typed errors instead of hydrating a broken `RunFrame`.

The master drift register still treats pending-action hydration and strict acknowledgement behavior as high-risk evidence areas. Do not claim crash safety without end-to-end recovery evidence.
