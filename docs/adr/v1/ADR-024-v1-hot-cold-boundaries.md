# ADR 024 (v1): Hot/Cold Boundaries and No-Async Core

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Hot runtime modules are numeric, bounded, synchronous, and allocation-disciplined. Cold modules own parsing, diagnostics, formatting, maps, allocation-heavy validation, CLI rendering, and recovery tooling.

## Invariants

- Hot modules do not perform YAML, JSON, HTTP, dynamic string lookup, formatted output, or unbounded allocation.
- Core crates do not use async executors or async functions.
- `mio` is the only approved low-level IPC eventing mechanism.
- Runtime actions may block only in bounded worker contexts or return `Suspended`.

## Consequences

- Adapters and rich operator surfaces must stay outside hot runtime core.
- Any async dependency in core crates is architectural drift until proven excluded.

## Master Anchors

- Section 11: Hot/Cold Data Layout
- Section 12: Forbidden Hot-Path APIs
- Section 53: Hot/Cold Module Classification
- Section 62: No-Async Rule
