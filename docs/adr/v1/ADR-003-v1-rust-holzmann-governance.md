# ADR 003 (v1): Rust and Holzmann Governance

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

First-party Rust follows the repository reliability contract:

- Forbid `unsafe` in first-party crates.
- Reject `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, and `dbg`.
- Reject unchecked indexing, slicing, casts, arithmetic, length math, and capacity math.
- Reject ignored `Result` and ignored fallible returns.
- Reject unbounded queues, loops, retries, fanout, buffers, timers, pagination, and expression stacks.
- Keep hot runtime paths free of formatting, dynamic maps, string lookups, and hidden allocation growth.

## Invariants

- User and external failures return typed errors.
- Nightly features remain allowlisted.
- Performance-only nightly features stay in approved locations.
- Dependency unsafe is advisory-governed; first-party unsafe is still forbidden.

## Master Anchors

- Section 2: Non-Negotiable Rust Rules
- Section 3: Holzmann Compliance Matrix
- Section 4: Mandatory Rust Tooling
- Section 7: Nightly Governance
- Section 52: Fallible Allocation and No-Panic Enforcement
- Section 53: Hot/Cold Module Classification
