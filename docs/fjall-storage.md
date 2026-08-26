# Fjall Storage

Fjall is the embedded persistence layer. The runtime core remains in memory; persistence behavior is controlled by durability profile.

## Current Scope

`vb-storage` owns the Fjall journal boundary. Current storage writes compact postcard-encoded `JournalEvent` values into an `events` keyspace.

Current event key:

```text
[0x11 | RunId_u64_be | EventSeq_u64_be] = 17 bytes
```

Big-endian encoding of Fjall numeric key fields preserves ordered replay by run
and sequence. Stored values use the separate 60-byte little-endian record
envelope with a BLAKE3 payload digest and CRC32C header checksum before the
Postcard payload.

## Current Events

```text
RunAccepted { run, seq, workflow }
StepStarted { run, seq, step }
StepSucceeded { run, seq, step, output }
RunFinished { run, seq, result }
```

## Current APIs

```text
append_journaled
append_strict
persist_strict
```

Duplicate `(RunId, EventSeq)` appends are rejected inside the journal instance. Replay requires contiguous per-run sequence numbers.

## Target Keyspaces

```text
workflow_source
compiled_ir
run_header
run_event
run_snapshot
blob
index_status
index_workflow
```

## Durability Profiles

```text
volatile   no Fjall writes during run execution
snapshot   async snapshots/checkpoints
journaled  compact events queued to a Fjall writer with group commit
strict     selected critical events synchronously persisted
```

Strict mode is slower and used when durability matters more than wall-clock latency.
