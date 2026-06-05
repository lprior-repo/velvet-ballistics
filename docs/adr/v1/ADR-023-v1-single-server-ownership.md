# ADR 023 (v1): Single-Server Ownership and Durability Boundary

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

v1 is single-server. One process owns a database path at a time. Fjall file locking enforces exclusive storage ownership.

## Invariants

- No replication, quorum, leader election, or distributed control plane in v1.
- A second process opening the same database path receives a typed error.
- Durability profile selection is explicit.
- Journaled durability is documented with its data-loss window.

## Consequences

- Horizontal scale and HA require future architecture work.
- Local durability can be tested without pretending to be distributed consensus.

## Master Anchors

- Section 18: Fjall Persistence Behavior
- Section 54: Single-Server Ownership and Database Locking
- Section 58: Platform Support
- Section 61: Fjall Storage Contract
- Section 68: Durable Execution Architecture Contract
