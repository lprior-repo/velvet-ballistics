# Boundary Map: vb-vzcuf

## Pure Core Boundary

Pure admission logic should be factored so proofs can bind to production code:

```text
admit_journal_event_bytes(staged, encoded_len, limit)
  -> Accepted(new_total) | Rejected(accumulated-byte error)
```

This helper must use checked conversion/addition and exact `<=` comparison. It should not access Fjall, encode payloads, allocate, or inspect global state.

## Imperative Storage Shell

`JournalWriteBatch::append_event` remains the imperative shell:

- builds keys;
- checks durable Fjall keyspace for duplicates;
- checks operation count;
- calls `encode_record`;
- calls pure admission helper;
- mutates `OwnedWriteBatch` and byte accumulator only after success.

## Parser/Codec Boundary

`encode_record` is the boundary that translates a typed `JournalEvent` into encoded bytes and enforces the per-record payload cap. Accumulated byte accounting must consume the returned `Vec<u8>.len()` and must not reimplement postcard payload sizing.

## Core/Storage Policy Boundary

`vb_core::ResourceContract::max_journal_batch_bytes` and `WholeWorkflowBudget::max_journal_batch_bytes` are core policy values. Storage needs a typed bridge:

```text
core u32 max_journal_batch_bytes -> JournalBatchByteLimit -> JournalWriteBatch limits
```

Core `BudgetError::JournalBatchBytesExceeded` is not a substitute for storage `JournalError`; either bridge errors explicitly or keep them separate with documented mapping.

## Async/Concurrency Boundary

`JournalWriteBatch` is intentionally `!Send + !Sync`. Byte accounting state must remain inside the same non-send batch handle. No atomics, locks, or shared mutable byte counters are needed for this bead.

## Persistence Boundary

Fjall `OwnedWriteBatch` is the durable mutation boundary. Rejected accumulated-byte candidates must never be inserted into the `OwnedWriteBatch`, so a later `commit` cannot persist them.

## Unsafe/FFI Boundary

No unsafe or FFI is required. Miri/unsafe proof seeds are not primary unless implementation introduces unsafe, which would violate repository rules.

## Public API Boundary

Likely public or crate-visible additions:

- limit value object or limits object;
- batch constructor/factory with limits;
- staged journal event byte accessor;
- `JournalError` accumulated-budget variant.

These are behavior-affecting and must be reflected in tests, proof bridge, and release/API notes.
