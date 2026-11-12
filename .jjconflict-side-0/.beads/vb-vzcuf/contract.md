# Contract: vb-vzcuf Fresh Journal Batch Byte Accounting

## Acceptance Contract for Downstream Lanes

The storage layer must enforce an accumulated encoded-byte budget for journal event appends in `JournalWriteBatch`.

### C1 Limit Presence

Every open `JournalWriteBatch` has a non-zero `JournalBatchByteLimit`. The default constructor remains bounded; an explicit API exists to supply a validated limit from runtime/core policy.

### C2 Accounting Definition

`StagedJournalEventBytes` equals the sum of encoded journal event value lengths accepted by `append_event` under the chosen same-batch duplicate policy. The encoded length is the full `Vec<u8>.len()` returned by `encode_record`, not just payload length.

### C3 Admission Boundary

For a candidate with encoded length `n` and current total `t`:

- accept iff checked `t + n` exists and `t + n <= limit`;
- reject iff checked `t + n` overflows or `t + n > limit`.

### C4 Typed Error API

Accumulated byte rejection returns a storage-visible `JournalError` variant distinct from `QueueFull` and `PayloadTooLarge`, carrying attempted/limit or equivalent diagnostic fields.

### C5 No Partial Mutation

On accumulated byte rejection, `inner.len()`, staged event keys, and staged byte total remain unchanged; the rejected key/value is not committed later. The batch remains open/non-aborted.

### C6 Error Separation and Precedence

Guard order is: key, durable duplicate, count, per-record encoding/payload, accumulated byte admission, insertion/update. Downstream tests/proofs must control unrelated guards to assert exact errors.

### C7 Overflow Safety

No accumulated-byte path uses unchecked arithmetic or unchecked casts. Overflow is a typed rejection, not panic/wrap/success.

### C8 Core/Storage Bridge

The existing `vb_core` `max_journal_batch_bytes` policy must either feed the storage limit through a typed bridge or the storage default must be explicitly documented as separate. Core `BudgetError` does not close the storage admission contract.

### C9 Observability

There must be an accessor or equivalent bridge for current staged journal event bytes to support integration tests, proof binding, and diagnostics.

## Non-Goals

- Do not alter per-record payload cap semantics.
- Do not count non-journal-event batch writes in this bead.
- Do not introduce async/concurrent shared counters.
- Do not implement production Rust in this contract state.

## Open Product Question Blocking Full Precision

Same-batch duplicate accounting must be confirmed: conservative append-attempt bytes versus precise distinct-key final bytes. Downstream implementation may choose conservative attempt accounting if documented and tested.
