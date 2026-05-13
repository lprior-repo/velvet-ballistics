# Proof Plan Review Input: vb-qi37.2.1

## Contract Summary

Aggregate resource budget model for `vb_core` + `vb_runtime`. Core types: `AggregateResourceBudget`, `AggregateResourceCapacity`, `AggregateResourceUsage`, `AggregateReservation`. Core invariants: checked arithmetic everywhere, capacity comparison is inclusive (equality admits), no unbounded dimensions.

## Risk Profile

| Risk | Classification | Evidence |
|---|---|---|
| Arithmetic overflow (add) | Bounded state, Rust-local | 6 Lean theorems, 4 Kani harnesses, 6 proptest props |
| Arithmetic underflow (sub) | Bounded state, Rust-local | 6 Lean theorems, 4 Kani harnesses, 6 proptest props |
| Capacity comparison wrong | API contract, pure function | Kani + Lean + unit per-dim tests |
| Policy validation wrong | Type-level enforcement | Lean THM-POLICY-EXACT + unit tests |
| Conversion lossy | Numeric refinement | Lean THM-CONV-LOSSLESS + proptest |
| Admission leaks state on reject | Integration correctness | 11 integration tests + Kani harness |
| Release underflows | Lifecycle invariant | Integration + Kani roundtrip |
| Runtime parses JSON/YAML | Performance/perf-only | STATIC-PARSER grep scan |
| Production unsafe code | Holzman governance | STATIC-GOV clippy gate |

## Obligation Matrix (Top 15 by criticality)

| ID | Requirement | Verifier | Artifact | Command | Status |
|---|---|---|---|---|---|
| THM-ADD-SAFETY | POST-003: checked add never overflows | lean | lean-report.md | lake build | required |
| THM-SUB-SAFETY | POST-004: checked sub never underflows | lean | lean-report.md | lake build | required |
| THM-FITS-INCLUSIVITY | INV-004: equality admits, strictly greater rejects | lean | lean-report.md | lake build | required |
| THM-POLICY-EXACT | INV-003: policy limits are absolute ceilings | lean | lean-report.md | lake build | required |
| THM-ADD-SUB-ROUNDTRIP | POST-006: add-then-sub recovers original | lean | lean-report.md | lake build | required |
| THM-CONV-LOSSLESS | POST-001: conversion preserves exact dim values | lean | lean-report.md | lake build | required |
| KANI-ADD-SAFETY | POST-003: symbolic overflow before mutation | kani | formal-verification-report.md | cargo kani | required |
| KANI-SUB-SAFETY | POST-004: symbolic underflow before mutation | kani | formal-verification-report.md | cargo kani | required |
| KANI-FITS-INCLUSIVITY | INV-004: symbolic capacity comparison | kani | formal-verification-report.md | cargo kani | required |
| KANI-ADMISSION | INV-007: admission never ok when usage > capacity | kani | formal-verification-report.md | cargo kani | required |
| INTEG-ADMISSION-EQ | POST-002: equality with capacity admits | integration | formal-verification-report.md | cargo nextest | required |
| INTEG-ADMISSION-REJECT | POST-005/ERR-003: over capacity rejects | integration | formal-verification-report.md | cargo nextest | required |
| INTEG-REJECT-UNCHANGED | POST-007: rejection leaves state unchanged | integration | formal-verification-report.md | cargo nextest | required |
| STATIC-GOV | GOV-001: no unsafe/unwrap/expect/panic/todo/dbg | static | formal-verification-report.md | cargo clippy + moon ci | required |
| STATIC-PARSER | PERF-001: no JSON/YAML/HTTP parsing in runtime core | static | formal-verification-report.md | grep scan + moon ci | required |

## Waivers

| Waiver | Owner | Reason | Compensating Evidence |
|---|---|---|---|
| WAIVER-001 | vb-qi37.2.1 contract synthesizer | Runtime admission involves trait objects, mutable shard state, orthogonal check ordering | 66+ unit/integration tests, Kani admission harness, static scan, manual QA |
| WAIVER-002 | vb-qi37.2.1 contract synthesizer | WholeWorkflowBudget::compute uses mutable collections; full IR in Lean out of scope | 35+ unit/integration tests, proptest invariants, fuzz corpus |

## Discovery Evidence

- `budget.rs`: `#![forbid(unsafe_code)]`, zero forbidden pattern matches
- `admission.rs`: `#![forbid(unsafe_code)]`, zero forbidden pattern matches in aggregate paths
- No `spawn`, `tokio`, `Mutex`, `RwLock` in aggregate arithmetic paths
- `kani::` proofs found: 12 `#[kani::proof]` blocks across budget.rs

## Verification Gate

Before proof writing: run `lake build` (Lean), `cargo kani` (Kani), `cargo nextest -p vb_core aggregate` (unit), `cargo clippy` (static) — all must pass before gauntlet.

## Open Questions (for reviewer)

1. Is Lean the right primary lane for overflow/underflow, or should Kani lead with Lean as cross-check?
2. Are 11 integration tests sufficient for the admission shell, or does INV-007 require a dedicated Kani harness?
3. Should GAUNTLET-PROOF run before or after per-obligation evidence collection?
