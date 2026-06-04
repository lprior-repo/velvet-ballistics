# Storage and Recovery Defects

## Status Update 2026-06-03

All defects in this file remain live and mapped to open beads: `vb-mrwe.1` through `vb-mrwe.7`. No untracked-bead gap remains for storage/recovery, but none of these rows should be claimed fixed until the corresponding bead is closed.

## P0: Storage envelope does not reject trailing bytes

Evidence:

- `crates/vb_storage/src/codec/payload.rs:56-82` reads the header, computes `payload_end`, verifies digest over `bytes[payload_start..payload_end]`, and returns success without checking that `payload_end == bytes.len()`.

Master violated:

- Section 18: decode must read exactly `payload_len_u32` bytes and verify the payload digest before typed decode.

Impact: A record with valid declared payload and appended garbage can decode successfully, weakening corruption/tamper detection.

Suggested bead: `P0 storage envelope rejects trailing bytes`

## P0: Compiled IR storage does not verify digest before put

Evidence:

- `crates/vb_storage/src/journal/source.rs:18-29` verifies workflow source bytes against the claimed digest.
- `crates/vb_storage/src/journal/source.rs:46-57` stores `CompiledIrRecord` by digest without verifying that `record.ir` hashes to `record.digest`.
- `crates/vb_storage/src/batch.rs:96-107` has the same missing compiled-IR verification in batch writes.

Master violated:

- Section 18: accepted run binds immutably to one compiled workflow digest.
- Section 18: replay checks compiled workflow digest mismatch.
- Section 44 points 13-14.

Impact: Storage APIs can persist forged compiled IR under arbitrary digest keys.

Suggested bead: `P0 reject forged compiled IR digest on direct and batch writes`

## P0: Full digest check omits action ABI and policy digests

Evidence:

- `crates/vb_storage/src/recovery/recover.rs:79-100` documents and implements `DigestCheck::Full` as workflow source plus compiled IR only.
- `crates/vb_storage/src/recovery/recover.rs:103-137` exposes action ABI and policy digest checks as separate caller-supplied functions.

Master violated:

- Section 18: replay checks workflow source digest, compiled workflow digest, action ABI digest, and policy digest.
- Section 44 point 14.

Impact: `DigestCheck::Full` can return `Ok(())` without checking action/policy mismatch unless every caller remembers extra checks.

Suggested bead: `P0 make full digest verification include action ABI and policy evidence`

## P0: Pending action recovery is explicitly unsupported

Evidence:

- `crates/vb_storage/src/recovery/replay/summary.rs:270-274` returns `UnsupportedRecoveryState::pending_actions_unsupported()` whenever pending actions exist.
- Runtime recovery rejects unsupported hydration per subagent inspection.

Master violated:

- Section 18 recovery invariants.
- Section 35 phases 32, 40, and 44.
- Section 44 points 3 and 14.

Impact: Crash recovery cannot hydrate runs suspended on unresolved actions. That is a core durable execution path, not optional polish.

Suggested bead: `P0 hydrate pending actions during runtime recovery`

## P0: StepSucceeded is mapped to SlotWritten record kind

Evidence:

- `crates/vb_storage/src/events.rs:48-58` defines `StepSucceeded` as its own journal event.
- Subagent inspection found `event_kind` maps both `StepSucceeded` and `SlotWrittenEvent` to `RecordKind::SlotWritten`.

Master violated:

- Section 18: record kind IDs and family/kind validation are part of the binary envelope contract.

Impact: Envelope `record_kind` is not semantically congruent with payload variant; kind/payload mismatch can be hidden until Postcard decode.

Suggested bead: `P0 storage record kind parity for StepSucceeded`

## P1: Pending action index keyspace is not maintained by runtime journal path

Evidence:

- Subagent inspection found `put_action_index` exists, but runtime action scheduling appends only a journal event and storage append writes only `run_event`.

Master violated:

- Section 18: `index_action` pending action indexes are required.

Impact: Recovery/inspection cannot rely on `index_action` as an authoritative pending-action index.

Suggested bead: `P1 maintain pending action index from runtime journal writes`

## P1: Journaled writer queue is queue batching, not proven Fjall group commit

Evidence:

- Subagent inspection found `JournalWriterQueue` drains by repeated individual appends and does not use `OwnedWriteBatch` for the batch.

Master violated:

- Section 18 durability profiles.
- Section 35 storage phases.

Impact: Crash behavior for partially flushed journaled batches remains unclear.

Suggested bead: `P1 implement or prove atomic bounded group commit for journaled queue`
