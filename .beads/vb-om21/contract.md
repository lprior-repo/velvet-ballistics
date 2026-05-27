# Contract — vb-om21

## Requirement IDs

| ID | Requirement |
|---|---|
| `REQ-vb-om21-01` | Missing tail metadata must reconstruct tail from the final durable `run_event` key for that run. |
| `REQ-vb-om21-02` | Matching declared tail metadata and final durable key must recover without warning/error. |
| `REQ-vb-om21-03` | Declared/suspect tail below reconstructed durable key tail must return typed `TailMismatch`. |
| `REQ-vb-om21-04` | Recovery-required missing `run_event` prefix must return typed `MissingJournal`. |
| `REQ-vb-om21-05` | Empty keyspace/prefix tail query returns zero tail. |
| `REQ-vb-om21-06` | Single event key at seq 0 reconstructs tail 1. |
| `REQ-vb-om21-07` | Tail scan must be bounded to `[0x11][run_id_u64_be]` and never cross another run prefix. |
| `REQ-vb-om21-08` | Reconstructed tail equals `max(encoded_seq) + 1` using checked arithmetic. |

## Contract Clauses

### `C-vb-om21-prefix-bound`

All tail scans for `run` must derive the prefix using `run_prefix_key(run)` and observe only keys starting with that prefix. The first non-prefix key in an ordered range terminates the scan.

### `C-vb-om21-big-endian-max`

The maximum sequence is selected from bytes `9..17` of validated `run_event` keys and interpreted as `u64::from_be_bytes`. This must be equivalent to numeric max for all valid keys.

### `C-vb-om21-tail-definition`

For no observed keys, query tail is `EventSeq(0)`. For observed keys, reconstructed tail is `checked_add(max_seq, 1)`. Overflow is a typed failure.

### `C-vb-om21-metadata-validation`

Missing metadata never blocks reconstruction. Present metadata is accepted only when it is not below the reconstructed tail. A lower declared value is `TailMismatch` and recovery stops.

### `C-vb-om21-missing-journal`

Recovery modes requiring existing journal data must not convert an absent prefix into successful recovery. They return `MissingJournal { run }`.

### `C-vb-om21-replay-integrity`

Tail reconstruction does not replace replay validation. Decoded journal events still must match run and contiguous expected sequence.

## Implementation Constraints for Later States

- Keep tests in `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` and register the target if the workspace manifest requires explicit `[[test]]` entries.
- Public surface may be a `FjallJournal` query method or recovery helper, but it must expose typed behavior sufficient for integration tests.
- Do not copy Restate source, API names, storage layout, async architecture, or wire protocol.
- Preserve repository rules: no `unsafe`, no panic/unwrap/expect/todo/dbg, no unchecked indexing/slicing/casts/arithmetic, no runtime YAML/JSON/HTTP.

## Open Domain Questions

1. Where will tail metadata live or enter the API? Exploration found no tail metadata record in `RunHeaderRecord`.
2. Should `TailMismatch`/`MissingJournal` be direct `RecoveryError` variants, nested journal errors, or a new tail-specific error converted into recovery? Contract requires structured typed semantics either way.
3. Should pure tail query be public, crate-private, or only observable through recovery APIs? Acceptance tests likely require a public or integration-observable surface.
