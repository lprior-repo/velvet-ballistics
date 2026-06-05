# ADR 002 (v1): Naming and Workspace Boundary

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Canonical names are fixed by the master document:

| Concept | Canonical spelling |
|---------|--------------------|
| Product | `velvet-ballistics` |
| Binary | `velvet-ballistics` |
| Cargo package | `velvet-ballistics` |
| Rust crate and module prefix | `velvet_ballistics` |
| Bead rig | `velvet-ballistics` |
| Bead database | `velvet_ballistics` |
| Language version | `velvet-ballistics/v1` |

Production code lives under `crates/`. Cross-crate integration tests and benchmarks live under `crates/workspace_tests/`. Fuzz targets live under `fuzz/`. Automation lives under `xtask/`.

## Invariants

- New docs and code do not introduce stale product spellings.
- Rust crate references use actual workspace crate names such as `vb_core`, `vb_yaml`, `vb_validate`, `vb_expr`, `vb_compile`, `vb_storage`, `vb_runtime`, and `vb_ipc`.
- No production code, tests, or benchmarks are added at repository root.

## Drift Status

Known stale product spelling and hyphenated crate names in narrative docs have been reconciled. Future recurrence should be caught by ADR review gates.

## Master Anchors

- Section 1: Naming Contract
- Section 23: Workspace Structure
- Section 34: Workspace Cargo Contract
