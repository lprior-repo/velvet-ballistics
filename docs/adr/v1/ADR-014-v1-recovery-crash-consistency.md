# ADR 014 (v1): Recovery and Crash Consistency

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Recovery reconstructs runtime state from accepted artifacts, run headers, snapshots, and journal events. It never reparses YAML for existing runs.

## Invariants

- `RunSubmitted` or `RunAccepted` precedes step and slot records.
- `ActionScheduled` is durable before external side-effect dispatch under strict durability.
- `ActionCompleted` is recorded before frame mutation on resume.
- Final result slots are persisted before terminal run events.
- Unsupported live recovery states fail closed with typed errors.

## Known Risk

The master drift register still marks parts of crash recovery partially resolved. Pending-action hydration and strict acknowledgement behavior need end-to-end evidence before crash-safety claims.

## Master Anchors

- Section 18: Fjall Persistence Behavior
- Section 49: Journal Event Payload Schemas and Crash-Consistency Ordering
- Section 61: Fjall Storage Contract
- Section 67: Architectural Drift Register, DRIFT-2
- Section 68: Durable Execution Architecture Contract
