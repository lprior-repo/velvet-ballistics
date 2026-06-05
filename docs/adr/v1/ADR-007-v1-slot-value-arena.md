# ADR 007 (v1): Slot Values and Value Arena

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Runtime state uses numeric slots and handle-backed arenas. `SlotValue` carries finite scalars or handles to symbol, list, object, and blob arenas. It does not use `HashMap<String, Value>` as hot runtime state.

## Invariants

- Handles are valid only for the owning `ValueStore` lifetime.
- Objects preserve insertion order and use side indexes for lookup.
- Blob size is bounded by resource contracts and v1 envelope limits.
- v1 has no arena garbage collection.

## Consequences

- Long-running deployments need operational management for append-only arenas.
- Cross-store handle reuse is invalid.

## Master Anchors

- Section 11: Hot/Cold Data Layout
- Section 14: Core Rust Types
- Section 48: Value Arena, Handle Lifetime, and Blob Contract
