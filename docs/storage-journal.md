# Fjall Storage Journal

Fjall is the only embedded durability substrate in this scaffold. It is used as an append-only journal for workflow events, not as the in-memory execution state.

## Key Layout

Journal event keys use a one-byte type prefix followed by fixed-width
big-endian numeric fields:

```text
events: [0x11 | RunId_u64_be | EventSeq_u64_be] = 17 bytes
```

Big-endian encoding preserves numeric ordering during prefix/range scans.

## Event Encoding

Internal events use compact binary encoding through `postcard`. Each payload is
preceded by the 60-byte storage record envelope. All multi-byte integer fields
in that envelope are little-endian; it carries a 32-byte BLAKE3 payload digest
at bytes `24..56` and a little-endian CRC32C over bytes `0..56` at bytes
`56..60`. This record envelope is distinct from the 24-byte IPC header. Fjall
numeric key fields remain big-endian so range scans preserve numeric order.
JSONL is a public observability projection and must not be the primary durable
journal format.

The storage API exposes explicit durability names: `append_journaled` writes without a caller-visible fsync barrier, while `append_strict` appends and calls `PersistMode::SyncAll` before returning.

Duplicate `(RunId, EventSeq)` appends are rejected. Event history is immutable; insert overwrite behavior from the underlying key-value store is not exposed as a journal operation.

## Durability Modes

| Mode | Meaning | Crash Behavior |
| --- | --- | --- |
| memory | no Fjall append | run is lost on process crash |
| journaled | append without explicit fsync barrier | OS/page-cache durability only |
| group_commit | batched persist barrier | durable after batch barrier completes |
| strict | persist barrier for each critical event | strongest local durability, highest latency |

Default policy target:

- `RunAccepted` is durable before acknowledgement.
- `StepStarted` for side-effecting actions is durable before the external effect.
- `StepSucceeded` for side-effecting actions is durable before downstream side effects.
- Pure `save`/`choose` chains may group-commit when replay semantics remain valid.

## Recovery

Current replay returns ordered per-run events. Future recovery will hydrate `RunFrame` state from `RunAccepted` plus ordered step events. Snapshots can be added after the event log is stable; they must never replace immutable event history.
