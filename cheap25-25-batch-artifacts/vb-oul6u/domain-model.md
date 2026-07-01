# Domain Model — vb-oul6u (Lint: remove runtime metric `as_conversions` suppression)

## Ubiquitous Language

| Term | Definition |
|------|------------|
| Trace Ring | Per-shard bounded SPSC ring buffer (`vb_runtime::trace::TraceRing`) used to buffer drainable `TraceEvent` values for one shard. Capacity is fixed at construction via `TraceRing::new(capacity)` and bounded below by `capacity.max(1)`. |
| Trace Ring Capacity | `TraceRing::capacity(&self) -> usize` — the configured maximum number of drainable events. Documented production value: 4096; hard lower bound 1. |
| Trace Ring Pending Length | `TraceRing::pending_len(&self) -> usize` — current number of drainable events buffered in the SPSC ring. Bounded by `capacity()`. |
| Trace Ring Fill Percentage | The ratio `pending_len / capacity` scaled to the range `[0.0, 100.0]`. Exposed as `ShardMetricsSnapshot.trace_ring_fill_pct: f32`. |
| Runtime Metrics Snapshot | Aggregate of per-shard counters plus fleet-wide sums. Type: `RuntimeMetricsSnapshot` (`vb_runtime::counters.rs`). Produced only by `Runtime::collect_metrics(&self)`. |
| Shard Metrics Snapshot | Per-shard slice of the runtime metrics. Includes `trace_ring_fill_pct: f32` as a public field. Type: `ShardMetricsSnapshot` (`vb_runtime::counters.rs:113`). |
| Bounded Narrowing | The transformation of a `usize` value known to fit in `u32` into a `u32` via `u32::try_from(...)`, with a configured fallback (`unwrap_or(0)` to mirror the surrounding metric-collection idiom; `unwrap_or(u32::MAX)` for fields whose sentinel saturation is already established by six sibling lines at `runtime.rs:571-577,596`). |
| Lossless Float Promotion | The conversion `f32::from(u32)` provided by `impl From<u32> for f32`. Exact for every `u32` value in `[0, u32::MAX]` because every `u32` is representable in `f32` mantissa. Never trips a lint (`clippy::as_conversions` only flags `as`-casts). |
| Section 44.20 / 44.21 Standing Violation | The workspace lint policy clause from `docs/master/section-044-backend-ir-interpreter-definition-of-done.md:32` ("Unchecked indexing, slicing, casts, and arithmetic are absent from first-party code") which `as_conversions = "deny"` enforces. |
| Numeric Regression Net | The three pinned tests `trace_ring_fill_pct_f32_f64_bit_exact_for_powers_of_two`, `trace_ring_fill_pct_f32_f64_within_one_ulp_for_general_caps`, and `trace_ring_fill_pct_boundary_values_are_bit_exact` (`vb_runtime/src/trace/tests.rs:1186-1309`) that already prove f32-direct and f64-then-f32 paths agree at every production `trace_capacity` up to `2^20`. |

## Bounded Values

### `TraceRingCapacity` (logical; existing source)

- Source: `TraceRing::new(capacity: usize)` (`vb_runtime/src/trace.rs:26-35`).
- Invariant: `capacity >= 1` (enforced by `capacity.max(1)` before being passed to `rtrb::RingBuffer::new`).
- Documented production range: typical `4096`, hard upper bound via configuration (never exceeds `2^20` per the RA-003 numerical-equivalence tests).
- Accessor: `pub const fn capacity(&self) -> usize`.

### `TraceRingPendingLength` (logical; existing source)

- Source: `TraceRing::consumer.slots()` (`vb_runtime/src/trace.rs:47-49`).
- Invariant: `pending_len() <= capacity()`.
- Accessor: `pub fn pending_len(&self) -> usize`.

### `TraceRingFillRatio` (new logical concept this bead exposes)

- Definition: `trace_len / trace_capacity` computed in `f32` after bounded narrowing to `u32`.
- Domain: when `trace_capacity > 0`, `ratio ∈ [0.0, 1.0]`. When `trace_capacity == 0`, the metric returns `0.0` (sentinel branch).
- Wire form: scaled by `100.0` to a percentage and stored as `trace_ring_fill_pct: f32`.

## Value Objects

### Bounded `u32` narrowing (idiomatic local pattern)

- Six sibling lines already use `u32::try_from(<usize expression>).unwrap_or(u32::MAX)` for every other metric field (`runtime.rs:571-577,596`).
- This bead extends the pattern to `trace_capacity` and `trace_len`, falling back to `0` (not `u32::MAX`) because:
  - `trace_capacity == 0` is already intercepted by the outer `if trace_capacity > 0` guard and the metric returns `0.0`; the narrowing fallback is therefore never observed in practice.
  - A fallback of `u32::MAX` would corrupt the ratio by making the denominator huge if the guard were ever bypassed; `0` is the closest to a documented sentinel under that hypothetical.
- Type signature in the fix: `let trace_capacity_u32 = u32::try_from(trace_capacity).unwrap_or(0);`

### `f32::from(u32)` lossless promotion

- Origin: `core::convert::From<u32> for f32` (`std::f32::from(u32_value)`).
- Property: every `u32` is exactly representable in `f32` mantissa because `u32::MAX < 2^32 < 2^24 * 2^8` — the mantissa has 24 bits of precision and the exponent can shift to cover the full `u32` range losslessly.
- No runtime cost beyond the implicit type tag; identical generated code to `value as f32` for `u32` inputs on common targets.
- Lint posture: `clippy::as_conversions` does not flag this conversion because it is an `impl From` invocation, not an `as`-cast.

## Entities / Aggregates

- **`Runtime`** (`vb_runtime/src/runtime::Runtime`): owns `self.shards: Vec<Shard>`; only entity that emits `RuntimeMetricsSnapshot`.
- **`Shard`** (`vb_runtime/src/shard::Shard`): owns `trace_ring: TraceRing` and `frame_pool`; produces one `ShardMetricsSnapshot`.
- **`RuntimeMetricsSnapshot`** (`vb_runtime/src/counters.rs:120-133`): immutable aggregate; cannot be mutated post-emit.
- **`ShardMetricsSnapshot`** (`vb_runtime/src/counters.rs`): immutable per-shard view; field `trace_ring_fill_pct: f32` is the only field touched by this bead.

## Forbidden States

1. Any first-party production `as`-cast in `vb_runtime` source. Enforced by `as_conversions = "deny"` at the workspace level and `-D clippy::as_conversions` at the CI gate.
2. An `as_conversions` allow attribute whose scope is broader than a single statement, or whose justification comment does not match the actual conversion performed.
3. A replacement expression for `trace_ring_fill_pct` whose output type is not `f32` (the public `ShardMetricsSnapshot` field type is frozen by `vb_runtime/src/counters.rs:113` and re-declared in `vb_ipc/src/metrics.rs:37`).
4. A replacement expression that is observably different from `(trace_len as f32) / (trace_capacity as f32)` within the documented production capacity range `[1, 2^20]`. The three RA-003 tests pin this range.

## Invariants

- INV-001: `TraceRing::pending_len() <= TraceRing::capacity()` always holds. (Pre-existing; not enforced by this bead.)
- INV-002: `TraceRing::capacity() >= 1` always holds. (Pre-existing; enforced by `capacity.max(1)` in `TraceRing::new`.)
- INV-003: `trace_ring_fill_pct ∈ [0.0, 100.0]` for the documented `trace_capacity` range, inclusive of the empty-ring boundary (`len = 0`) and the full-ring boundary (`len = cap`). Enforced by the guard `if trace_capacity > 0`, the ratio expression, and the multiplication by `100.0`.
- INV-004: After the fix, `Runtime::collect_metrics` contains zero `as`-casts and zero `#[allow(clippy::as_conversions)]` attributes in `vb_runtime/src/runtime.rs:580-588`. Enforced by `rg -n "\bas\b" crates/vb_runtime/src/runtime.rs` returning zero matches in the production source.
- INV-005: The replacement expression is numerically equivalent to the original `(trace_len as f32) / (trace_capacity as f32) * 100.0` to within 1 ULP for every `trace_capacity ∈ [1, 2^20]` and every `trace_len ∈ [0, trace_capacity]`. Enforced by the three RA-003 tests, which already cover this exact equivalence class.