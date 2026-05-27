# Domain Model — vb-om21: Journal Tail Scan Fallback Tests

## Scope

Bead `vb-om21` models the storage/recovery contract for reconstructing a run journal tail from Fjall `run_event` keys when stored tail metadata is absent or suspect. This State 3 artifact is a Rust domain/type contract only. It does not implement production code, tests, proof artifacts, proof plans, or reviews.

## Ubiquitous Language

| Term | Definition |
|---|---|
| Run | A durable workflow execution identified by `RunId` (`u64`). |
| Journal event | A Postcard/envelope encoded `JournalEvent` persisted under a `run_event` key. |
| Event sequence | A numeric `EventSeq` (`u64`) embedded in the last 8 bytes of a `run_event` key in big-endian order. |
| Run event key | The Fjall key `[0x11][run_id_u64_be][seq_u64_be]`. Lexicographic order is numeric `(run, seq)` order because fields are big-endian. |
| Run event prefix | The per-run prefix `[0x11][run_id_u64_be]`; scans must be bounded to this prefix. |
| Tail | The next journal sequence after the maximum committed event sequence for a run. Empty prefix tail is zero; one event at seq 0 yields tail one. |
| Tail metadata | Optional durable or caller-supplied metadata claiming a tail sequence. The explored code has no current public tail metadata record, so this is a modeled external/suspect input boundary. |
| Reconstructed tail | Tail derived by scanning durable `run_event` keys and decoding the maximum encoded sequence. |
| Missing journal | A non-empty recovery requirement for a run whose `run_event` prefix is absent. |
| Tail mismatch | A typed recovery failure when suspect/declared tail is below the reconstructed final-key tail. |

## Entities and Value Objects

### Entities

- `FjallJournal`: aggregate root for storage keyspaces, including `run_event`.
- `RunJournal`: logical aggregate of all durable journal events sharing a `RunId` prefix.
- `RecoveryRequest`: command context requesting recovery or tail validation for one `RunId`.

### Value Objects

- `RunEventPrefix`: exactly 9 bytes, constructed only from `RunId` and `PREFIX_RUN_EVENT`.
- `RunEventKey`: exactly 17 bytes, constructed only from `RunId` + `EventSeq` via big-endian encoding.
- `EncodedSequence`: 8 big-endian bytes decoded to `EventSeq` only after key-length and prefix validation.
- `JournalTail`: next append sequence, represented as `EventSeq`; invariant: `tail == 0` for empty prefix, otherwise `tail == max_seq + 1` with overflow typed as failure.
- `ObservedTail`: `Empty`, `Present { max_seq, tail }`, or `MissingPrefixForRequiredRun { run }` depending on scan policy.
- `DeclaredTail`: optional metadata/caller-supplied tail, with no trust until compared to the durable key-derived tail.

## Commands

1. `ScanJournalTail { run }`: derive `ObservedTail` from durable keys for exactly one run prefix.
2. `ValidateTailMetadata { run, declared_tail }`: compare suspect metadata against reconstructed tail.
3. `RecoverWithTailFallback { run, declared_tail? }`: recover using reconstructed tail when metadata is absent, and fail typed when metadata conflicts.

## Events / Outcomes

- `TailReconstructed { run, tail, max_seq? }`
- `TailMetadataAccepted { run, tail }`
- `TailMismatch { run, declared_tail, reconstructed_tail }`
- `MissingJournal { run }`
- `StorageScanFailed { run, source }`
- `TailOverflow { run, max_seq }`

## Invariants

1. Tail scan may observe only keys whose bytes start with `run_prefix_key(run)`.
2. A key from another run must terminate or be excluded from the scan; it must never influence the requested run's tail.
3. `EventSeq` ordering used for max-tail selection is numeric and equivalent to lexicographic order over the big-endian sequence bytes.
4. Empty `run_event` keyspace or empty per-run prefix reconstructs tail zero only for pure tail query semantics.
5. Recovery requiring at least one journal event must return typed `MissingJournal { run }` when the run prefix is absent.
6. Suspect metadata below the reconstructed tail must return typed `TailMismatch`; it must not silently truncate committed events.
7. Matching metadata and reconstructed final key recover without warning.
8. Missing metadata uses reconstructed tail from final key.

## Out of Scope

- Copying Restate implementation details, data model, async architecture, HTTP/JSON paths, or storage layout.
- Distributed replication, quorum, leader election, or cross-server recovery.
- Writing the requested integration tests or production implementation in this state.
