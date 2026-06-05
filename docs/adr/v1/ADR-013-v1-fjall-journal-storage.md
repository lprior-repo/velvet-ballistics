# ADR 013 (v1): Fjall Journal and Storage Contract

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Fjall is the embedded persistence layer. Postcard encodes internal durable records. Keys use prefix bytes and big-endian numeric IDs for ordered scans. Record bodies use the master-defined envelope and digest/checksum rules.

## Invariants

- Runtime storage does not use string keys on hot paths.
- Journal appends reject duplicate `(RunId, EventSeq)` entries.
- Strict writes persist with `SyncAll` where required.
- Journaled writes have an explicit data-loss window.
- Cross-keyspace atomicity must use batch semantics when the implementation relies on all-or-nothing behavior.

## Master Anchors

- Section 18: Fjall Persistence Behavior
- Section 49: Journal Event Payload Schemas and Crash-Consistency Ordering
- Section 61: Fjall Storage Contract
