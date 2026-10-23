# Contract: vb-8eocj.3.4 — Reduce Hot-Function Decomposition

**Target files:** `reduce.rs`, `reduce_evidence.rs`  
**Objective:** Decompose `reduce` hot path so the main function body is ≤ 25 logical lines (helpers excluded). Preserve exact runtime behavior.

## Behavioral Invariants

| Phase | Writes | Notes |
|-------|--------|-------|
| Start (empty) | accumulator only | then done |
| Start (nonempty) | accumulator, input first item, iterator state (cursor=1) | |
| Next (done) | none | |
| Next (body) | item output, then checked cursor increment + state | |
| Unexpected PC | `None` fallback | |

- Accumulator initial write must precede any input read.
- Error propagation and ordering unchanged.
- No new allocations beyond the existing iterator state.
- Preserve: taint, dedupe, gap, and capacity errors.

## Evidence Requirements

- Start Done → accum only
- Start Body → accum / input / output
- Next Done → empty set
- Next Body → output / iterator
- Unexpected PC → `None` fallback

## Test Requirements

- Bounded focused tests required.
- Frozen suite: 19 tests.
- Source inspection ≠ execution; runtime evidence mandatory.

## Status

- Current counter: 10 / allocator 9 PASS
- Not yet full reduce test acceptance
- Contract recorded after raw draft, before integration acceptance
- No performance claim; no proof planning