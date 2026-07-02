# Contract Specification — vb-oul6u (Lint: remove runtime metric `as_conversions` suppression)

## Context

- **Feature**: Remove the locally-scoped `#[allow(clippy::as_conversions)]` at `crates/vb_runtime/src/runtime.rs:583` and replace `(trace_len as f32) / (trace_capacity as f32)` with a bounded-narrowing pattern that does not trip the workspace `as_conversions = "deny"` lint.
- **Domain terms**: Trace Ring, Trace Ring Fill Percentage, Runtime Metrics Snapshot, Shard Metrics Snapshot, Bounded Narrowing, Lossless Float Promotion, Numeric Regression Net.
- **Assumptions**: Trace ring capacity is bounded by configuration (typical 4096, hard upper bound `2^20` per the RA-003 tests). `TraceRing::new` enforces `capacity >= 1` via `capacity.max(1)`. `f32::from(u32)` is exact for every `u32` value.
- **Open questions**:
  - Should the `SAFETY:` comment at `runtime.rs:581-582` be removed or rewritten? (Resolved: must be removed or rewritten; it justified an `as`-cast that no longer exists.)
  - Should the fallback in `unwrap_or(...)` be `0` or `u32::MAX`? (Resolved: `0`, mirroring the sentinel intent of `trace_capacity == 0 → 0.0`.)
  - Does `moon ci` currently pass with the local allow? (Resolved: yes — the allow is statement-scoped and the deny is file-or-wider.)

## Preconditions

- PRE-001: `TraceRing::capacity() >= 1` (enforced by `TraceRing::new` via `capacity.max(1)`).
- PRE-002: `TraceRing::pending_len() <= TraceRing::capacity()` (enforced by `rtrb::RingBuffer` semantics).
- PRE-003: `ShardMetricsSnapshot.trace_ring_fill_pct` field type is `f32` (frozen at `vb_runtime/src/counters.rs:113` and re-declared at `vb_ipc/src/metrics.rs:37`).
- PRE-004: The replacement expression uses only `core::convert::TryFrom` and `core::convert::From` from `std`; no new crate dependency is added.

## Postconditions

- POST-001: After `Runtime::collect_metrics` returns, every `ShardMetricsSnapshot.trace_ring_fill_pct` is in `[0.0, 100.0]` for the documented capacity range, or `0.0` for `trace_capacity == 0`.
- POST-002: `Runtime::collect_metrics` source contains zero `as`-casts and zero `#[allow(clippy::as_conversions)]` attributes between lines 578 and 588.
- POST-003: The replacement value is bit-identical to `(trace_len as f32) / (trace_capacity as f32) * 100.0` for every `trace_capacity ∈ [1, 2^20]` and every `trace_len ∈ [0, trace_capacity]` (RA-003 test corpus).
- POST-004: `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions` exits `0`.
- POST-005: `xtask forbidden-scan` reports zero `as`-casts in `vb_runtime` production source.
- POST-006: `cargo test -p vb_runtime --lib trace_ring_fill_pct` passes all three existing RA-003 tests.

## Invariants

- INV-001: `trace_ring_fill_pct` field type is `f32` (frozen; unchanged by this bead).
- INV-002: `Runtime::collect_metrics` is a pure read over `&self` (no mutation, no I/O, no time, no network, no storage, no async).
- INV-003: `trace_ring_fill_pct ∈ [0.0, 100.0]` for `trace_capacity > 0`, inclusive of the empty-ring and full-ring boundaries.
- INV-004: The replacement expression uses `u32::try_from(...).unwrap_or(0)` followed by `f32::from(u32)`; the fallback value is `0`, not `u32::MAX`.
- INV-005: The `SAFETY:` comment justifying the original `as`-cast is removed or rewritten; it must not remain attached to a non-`unsafe` block.
- INV-006: The workspace `as_conversions = "deny"` policy remains `deny`; the bead does not weaken it.

## Error Taxonomy

This bead does not introduce or modify any `Result`-returning function. Domain errors are out of scope. Lint, test, and AST-scanner errors are listed in `error-taxonomy.md`.

## Contract Signatures

- `pub fn Runtime::collect_metrics(&self) -> RuntimeMetricsSnapshot` — signature unchanged.
- `pub trace_ring_fill_pct: f32` (`ShardMetricsSnapshot`) — field unchanged.

## Replacement Expression (Canonical Form)

```rust
let trace_capacity = shard.trace_ring().capacity();
let trace_len = shard.trace_ring().pending_len();
let trace_ring_fill_pct = if trace_capacity > 0 {
    // Bounded narrowing mirrors the six sibling metric lines at runtime.rs:571-577.
    // TraceRing::new clamps capacity to >= 1 and the documented configuration cap
    // (typical 4096) is far below u32::MAX, so the unwrap_or(0) fallback is unreachable.
    // The fallback value is 0 (not u32::MAX) to preserve the sentinel intent of the
    // outer zero-denominator guard.
    let cap_u32 = u32::try_from(trace_capacity).unwrap_or(0);
    let len_u32 = u32::try_from(trace_len).unwrap_or(0);
    let ratio = f32::from(len_u32) / f32::from(cap_u32);
    ratio * 100.0
} else {
    0.0
};
```

## Verifier-Owned Clauses

- **Verus**: None. The bead does not bind any Verus spec to `Runtime::collect_metrics` or `trace_ring_fill_pct`. (`rg -l "trace_ring_fill_pct|collect_metrics" crates/vb_runtime/src/verification/` returns no matches.)
- **Kani**: None. No `#[kani::proof]` harness references this code path.
- **Flux**: None. No `#[refined_by]` or `#[spec]` annotation references this code path.
- **Loom**: None. `collect_metrics` is synchronous and holds `&self`; no interleaving risk.
- **proptest**: None. The replacement is a deterministic function of two `usize` values; the equivalence class is fully pinned by the RA-003 test corpus.
- **cargo-fuzz**: None. The function has no external input boundary.

## Lint / Test / Tooling-Owned Clauses

- **Clippy gate (lint-owned)**: `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions` exits `0` after the fix. Owner: State 6 `black-hat-reviewer`.
- **AST scanner gate (lint-owned)**: `xtask forbidden-scan` reports zero `as`-casts in `vb_runtime` production source. Owner: State 6 `black-hat-reviewer`.
- **Numeric regression net (test-owned)**: `cargo test -p vb_runtime --lib trace_ring_fill_pct` passes all three RA-003 tests. Owner: State 5 `test-writer`.
- **Call-site regression (test-owned)**: New tests assert `metrics.shards[0].trace_ring_fill_pct == 0.0 / 50.0 / 100.0` for empty/half/full trace rings through `Runtime::collect_metrics`. Owner: State 5 `test-writer`. (See `delivery-scope.jsonl` row `r03`.)

## Non-Goals

- Cross-shard metric aggregation changes.
- IPC wire-format changes (`f32` field type is frozen).
- Changes to `Runtime::collect_metrics` signature or return type.
- Changes to any `as`-cast outside `vb_runtime/src/runtime.rs:578-588`.
- Changes to workspace lint policy (`as_conversions = "deny"` is preserved).
- Changes to `TraceRing::new`, `TraceRing::capacity`, or `TraceRing::pending_len` signatures or semantics.
- Formal-verifier artifacts (Kani/Flux/Verus/Loom/proptest) for this code path (none exist; none are needed).

## Cross-References

- `domain-model.md` — ubiquitous language, value objects, entities, forbidden states, invariants.
- `type-contracts.md` — type-level contracts for `trace_capacity_u32`, `trace_len_u32`, `ratio`, `trace_ring_fill_pct`.
- `workflow-model.md` — state transitions and guards for the per-shard metric emission loop.
- `error-taxonomy.md` — lint, test, AST-scanner error catalogue.
- `boundary-map.md` — module, type, lint-policy, AST-scanner, and numeric-regression boundaries.
- `hazard-analysis.md` — twenty-row hazard matrix classified by T/C/U/P/L/N/B/R/S/A/D.
- `proof-seeds.jsonl` — Rust-local implementation seeds; no Verus/Kani/Flux/Loom obligations.
- `traceability-matrix.jsonl` — row-by-row traceability from `delivery-scope.jsonl` rows `r01`–`r16` to contract clauses, hazards, and proof seeds.