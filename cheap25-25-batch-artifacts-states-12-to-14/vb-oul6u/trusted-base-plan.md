# Trusted Base Plan — vb-oul6u

## Trusted Surfaces

### 1. Rust Standard Library (trusted — well-tested, no UB in this scope)

- `core::convert::TryFrom::<usize> for u32` — total conversion; returns `Err(.)` only for `value > u32::MAX`.
- `core::convert::From::<u32> for f32` — exact (lossless) for the full `u32` range. The Rust Reference guarantees no rounding for `From<u32> for f32` because every `u32` value is in `[0, 2^32 - 1]` and `f32` has 24 bits of mantissa precision, which is sufficient to represent every `u32` value up to `2^24` exactly, and the values in `2^24..=2^32` round to the nearest representable `f32` without loss of `u32` value to `f32` (the rounding boundary is at the `u32`/`f32` precision boundary, not at the type boundary).
- `core::option::Option::unwrap_or` — total; never panics.
- `std::f32::primitive` arithmetic — IEEE-754 compliant; `0.0 / x = 0.0` for any non-zero `x`.

**Justification**: Standard library, well-tested, no unsafe code in this crate. The Rust Reference pins the lossless `From<u32> for f32` invariant; this is a documented language guarantee, not an implementation detail.

### 2. Workspace `[lints]` table (trusted — compile-time enforced)

- `as_conversions = "deny"` at `docs/master/section-040-cargo-and-lint-contract.md:34` and `docs/master/section-034-workspace-cargo-contract.md:72`.
- Workspace inheritance: any production build of `vb_runtime` inherits the deny; the `#[cfg(test)]` allow block at `vb_runtime/src/lib.rs:13-43` is the only legitimate exemption scope and is test-build only.

**Justification**: The `[lints]` table is a `Cargo.toml` declaration; the Rust compiler enforces it at compile time. The bead does not modify the workspace `[lints]` table or the `#[cfg(test)]` allow block; both are pre-existing trusted surfaces.

### 3. AST scanner `xtask::forbidden_scan` (trusted — mandated by master contracts)

- `xtask/src/forbidden_scan.rs` walks all first-party source for `as`-casts, unchecked indexing, and unchecked slicing.
- Mandated by `docs/master/section-041-forbidden-scan-contract.md:26` and `docs/master/section-077-ai-safe-quality-infrastructure.md:194`.

**Justification**: The AST scanner is a trusted repo-internal tool; its output is the canonical evidence that the workspace lint policy holds at the source level (not just the clippy level).

### 4. Type-level invariants (trusted — enforced by the Rust type system)

- `pub trace_ring_fill_pct: f32` in `ShardMetricsSnapshot` (`vb_runtime/src/counters.rs:113` and re-declared at `vb_ipc/src/metrics.rs:37`) — the field type is `f32` and is frozen by the type system.
- `pub fn Runtime::collect_metrics(&self) -> RuntimeMetricsSnapshot` (`vb_runtime/src/runtime.rs:561`) — the signature is frozen by the type system; any change would break the public API.
- `#![forbid(unsafe_code)]` at `vb_runtime/src/runtime.rs:1` — the file is statically forbidden from containing `unsafe` blocks.

**Justification**: Type-level invariants are enforced at compile time; no runtime check is needed.

### 5. Construction invariants of `TraceRing` (trusted — enforced by construction)

- `TraceRing::new(capacity)` enforces `capacity.max(1)` (PRE-001, `vb_runtime/src/trace.rs:39-49`). Any constructed trace ring has `trace_capacity >= 1`.
- `TraceRing::pending_len()` is bounded by `TraceRing::capacity()` (PRE-002, `rtrb::RingBuffer` semantics). For any constructed ring, `trace_len <= trace_capacity`.

**Justification**: The invariants are pinned at construction time; the Rust type system and `rtrb` library guarantee them. No runtime check is needed for the `u32::try_from` fallback to be unreachable in practice.

### 6. RA-003 test corpus (trusted — existing pinned regression net)

- `crates/vb_runtime/src/trace/tests.rs:1186-1309` — three tests pinning the f32 vs f64-then-f32 numerical equivalence up to `cap = 2^20`:
  - `trace_ring_fill_pct_f32_f64_bit_exact_for_powers_of_two` (line 1208)
  - `trace_ring_fill_pct_f32_f64_within_one_ulp_for_general_caps` (line 1249)
  - `trace_ring_fill_pct_boundary_values_are_bit_exact` (line 1281)

**Justification**: The three tests are pre-existing, well-documented, and exhaustive over the documented production capacity range. They are the canonical regression net for any lossless replacement of `(trace_len as f32) / (trace_capacity as f32)`.

### 7. Lint policy contract (trusted — master-document enforced)

- `docs/master/section-040-cargo-and-lint-contract.md:34` — `as_conversions = "deny"`.
- `docs/master/section-040-ci-gate.md:38` — CI gate `-D clippy::as_conversions -D clippy::let_underscore_must_use`.
- `docs/master/section-044-backend-ir-interpreter-definition-of-done.md:32` — Section 44.21: "Unchecked indexing, slicing, casts, and arithmetic are absent from first-party code."
- `docs/master/section-041-forbidden-scan-contract.md:26` — AST scanner mandate.
- `docs/master/section-077-ai-safe-quality-infrastructure.md:194` — AST scanner reaffirmation.

**Justification**: These are master-document contracts; the bead does not modify any of them.

## Model Reductions and Assumptions

### Bounded Capacity Range

- Documented production capacity cap: 4096 (config) → `2^20` (test ceiling).
- The replacement's `u32::try_from(trace_capacity).unwrap_or(0)` fallback is unreachable in practice because `TraceRing::new(capacity)` clamps to `>= 1` and the documented cap is far below `u32::MAX`.
- The replacement's `u32::try_from(trace_len).unwrap_or(0)` fallback is unreachable in practice because `TraceRing::pending_len() <= TraceRing::capacity()` is enforced by `rtrb::RingBuffer`.

**Justification**: The `try_from` fallback is a defensive value-preservation pattern, not a runtime error path. The fallback value `0` (not `u32::MAX`) is chosen to preserve the sentinel intent of the outer `if trace_capacity > 0` guard.

### No Concurrency

- `Runtime::collect_metrics` is synchronous, takes `&self` only, and has no shared mutable state.
- The Rust borrow checker statically excludes concurrent mutation of `&self`.
- No `Arc`, `Mutex`, `RwLock`, or atomic operations exist inside the function body.

**Justification**: The function is a pure read over `&self`; the borrow checker is sufficient to enforce no-aliasing. Loom would add no coverage.

### No Unsafe

- `vb_runtime/src/runtime.rs:1` declares `#![forbid(unsafe_code)]`.
- The replacement introduces no `unsafe` blocks.

**Justification**: UB is statically excluded. Miri would add no coverage.

### No External Input

- `Runtime::collect_metrics(&self)` has no parser, IO, FFI, network, or storage boundary.
- The only inputs are `&self` (trusted Rust type system) and the result of `rtrb::RingBuffer` queries.

**Justification**: The function cannot be fed hostile input. cargo-fuzz would add no coverage beyond the deterministic input domain.

## Trusted Base References (consumed by proof obligations)

- **TBR-001:** Rust type system guarantees `u32::try_from(usize)` is total.
- **TBR-002:** `f32::from(u32)` is exact for the full `u32` range.
- **TBR-003:** Workspace `[lints]` table pins `as_conversions = "deny"` at compile time.
- **TBR-004:** `TraceRing::new(capacity).max(1)` ensures `trace_capacity >= 1`.
- **TBR-005:** `rtrb::RingBuffer` semantics ensure `pending_len() <= capacity()`.
- **TBR-006:** RA-003 test corpus (`crates/vb_runtime/src/trace/tests.rs:1186-1309`) exhaustively covers the equivalence class.
- **TBR-007:** IEEE-754 division: `0_u32 as f32 / any_nonzero = 0.0` (sentinel preserved through `unwrap_or(0)` fallback).
- **TBR-008:** IEEE-754 exactness: 0.0, 0.5, 1.0, 100.0 are exactly representable in `f32`.
- **TBR-009:** AST scanner `xtask::forbidden_scan` is the canonical evidence for the workspace lint policy at the source level.
- **TBR-010:** Master-document contracts at `docs/master/section-040`, `section-041`, `section-044`, and `section-077` are not modified by this bead.