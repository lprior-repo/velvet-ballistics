# Type Contracts — vb-oul6u (Lint: remove runtime metric `as_conversions` suppression)

## Scope

Replace the locally-scoped `#[allow(clippy::as_conversions)]` at `crates/vb_runtime/src/runtime.rs:583` with an idiomatic bounded-narrowing pattern that mirrors the six sibling lines already used by `collect_metrics` for `active_runs`, `queue_depth`, `queue_remaining`, `pending_timers`, `frame_pool_free`, `frame_pool_total`, and `shard_id`. Preserve the public `ShardMetricsSnapshot.trace_ring_fill_pct: f32` field type.

## Newtypes / Smart Constructors

This bead does not introduce any new public newtype. The fix is a localised source-level substitution inside a private branch of `Runtime::collect_metrics`. The only `usize → f32` path that is exercised is through a `u32` intermediate; no public API surface changes.

## Type Contract: `trace_capacity_u32` (local binding)

- **Source expression**: `shard.trace_ring().capacity() : usize`
- **Target type**: `u32`
- **Construction**: `u32::try_from(shard.trace_ring().capacity()).unwrap_or(0)`
- **Justification for narrowing**: `TraceRing::capacity()` returns the value that was passed to `TraceRing::new(capacity)` clamped by `capacity.max(1)`. The documented production configuration bounds capacity to ≤ 4096, far below `u32::MAX`. The fallback `0` is unreachable inside the surrounding `if trace_capacity > 0` guard.
- **Lint posture**: no `as`-cast; uses only `core::convert::TryFrom` and `core::option::Option::unwrap_or`.

## Type Contract: `trace_len_u32` (local binding)

- **Source expression**: `shard.trace_ring().pending_len() : usize`
- **Target type**: `u32`
- **Construction**: `u32::try_from(shard.trace_ring().pending_len()).unwrap_or(0)`
- **Justification for narrowing**: `TraceRing::pending_len()` is bounded by `TraceRing::capacity()` (INV-001 in `domain-model.md`), which itself is bounded by configuration. The fallback `0` is unreachable in practice; choosing `0` (not `u32::MAX`) avoids corrupting the ratio if the invariant were ever broken.
- **Lint posture**: no `as`-cast.

## Type Contract: `ratio` (local binding)

- **Source expressions**: `trace_len_u32 : u32`, `trace_capacity_u32 : u32`
- **Target type**: `f32`
- **Construction**: `f32::from(trace_len_u32) / f32::from(trace_capacity_u32)`
- **Justification for promotion**: `impl From<u32> for f32` is exact for the full `u32` range. The division is performed in `f32` arithmetic to match the field type and the documented `trace_ring_fill_pct` resolution.
- **Lint posture**: zero `as`-casts; only `core::convert::From` is invoked.

## Type Contract: `trace_ring_fill_pct` (kept)

- **Type**: `f32`
- **Scope**: local to the `if trace_capacity > 0 { ... } else { 0.0 }` branch in `Runtime::collect_metrics`.
- **Final expression**: `ratio * 100.0` (inside the guard) or `0.0` (outside the guard).
- **Public exposure**: bound into `ShardMetricsSnapshot.trace_ring_fill_pct: f32` (`vb_runtime/src/counters.rs:113`) and re-declared in `vb_ipc/src/metrics.rs:37`.

## Boolean / Flag Audit

- No new boolean behavior flag is introduced.
- The existing guard `if trace_capacity > 0` is preserved unchanged. (The `bool` represents a domain predicate "trace ring was constructed with a nonzero capacity", not a behavior flag — already idiomatic for a zero-denominator guard.)

## `Option` Lifecycle Audit

- No new `Option<T>` lifecycle state is introduced. The branch is `if guard { value } else { sentinel }`, not `Option<f32>`.

## Stringly-Typed Audit

- No string IDs or string-typed identifiers are introduced or removed.

## Parsing at the Boundary

- The only external input to this code path is the `Runtime` instance, which is constructed by trusted Rust code. No parser boundary is introduced or removed.

## Pure-Core / Imperative-Shell Split

- `Runtime::collect_metrics` is a pure read-only method (only `&self` is borrowed; no mutation). The replacement preserves this property: the new expression is a pure function of `trace_capacity_u32` and `trace_len_u32`.
- No I/O, time, network, storage, randomness, or async boundary is introduced or removed.

## Lint Posture

- `vb_runtime/src/runtime.rs` continues to be governed by `#![forbid(unsafe_code)]` and inherits `as_conversions = "deny"` from the workspace `[lints]` table.
- The replacement expression contains zero `as`-casts and zero `#[allow(clippy::as_conversions)]` attributes.
- The `SAFETY:` comment on lines 581-582 must be removed or rewritten because it justifies an `as`-cast that no longer exists. Recommended rewrite: remove the comment block entirely; the `try_from` calls are self-justifying.

## Errors

This bead does not change the error surface. `Runtime::collect_metrics` returns `RuntimeMetricsSnapshot` directly (no `Result`); the sentinel branch `0.0` replaces any need for error reporting. The numeric fallback `unwrap_or(0)` is a value-preserving choice, not an error path.

## Trait / Impl Boundaries

- `core::convert::TryFrom<usize> for u32` (built-in; no new trait impl needed).
- `core::convert::From<u32> for f32` (built-in; no new trait impl needed).
- No new trait derivations, no new sealed traits, no new associated types.

## Public API Diff

| Symbol | Before | After |
|--------|--------|-------|
| `pub fn Runtime::collect_metrics(&self) -> RuntimeMetricsSnapshot` | Unchanged signature | Unchanged signature |
| `pub trace_ring_fill_pct: f32` (`ShardMetricsSnapshot`) | `f32` | `f32` (frozen) |
| `pub trace_ring_fill_pct: f32` (`vb_ipc::metrics::ShardMetricsSnapshot`) | `f32` | `f32` (frozen) |

No symbol added, removed, renamed, or retyped.