# Boundary Map — vb-oul6u (Lint: remove runtime metric `as_conversions` suppression)

## Purpose

Identify every trust and module boundary that the bead's source change crosses or relies on, so that proof and test plans can target the right layers.

## Boundary Inventory

### 1. Pure-Core Boundary (within `vb_runtime`)

| Side | Module / Function | Notes |
|------|-------------------|-------|
| Inside pure core | `Runtime::collect_metrics` (`vb_runtime/src/runtime.rs:561-618`) | Reads only `&self`. No I/O, time, network, storage, randomness, or async. The fix preserves this property. |

### 2. Type-Definition Boundary

| Side | Symbol | Source |
|------|--------|--------|
| Producer | `pub struct ShardMetricsSnapshot { ..., pub trace_ring_fill_pct: f32, ... }` | `vb_runtime/src/counters.rs:113` |
| Re-declaration | `pub struct vb_ipc::metrics::ShardMetricsSnapshot { ..., pub trace_ring_fill_pct: f32, ... }` | `vb_ipc/src/metrics.rs:37` |
| Wire serializer | `vb_ipc` Postcard roundtrip (`vb_ipc/src/metrics/tests.rs:298, 317`) | Field must remain `f32` for byte-identical wire format |

The bead does **not** cross this boundary — it preserves both field types exactly. Downstream owner must verify the IPC roundtrip tests still pass.

### 3. Bounded-Construction Boundary

| Side | Function | Notes |
|------|----------|-------|
| Source | `TraceRing::new(capacity: usize) -> Self` (`vb_runtime/src/trace.rs:26-35`) | Enforces `capacity.max(1)`. The downstream invariant `capacity >= 1` is the foundation of the fix's narrow-then-promote pattern. |
| Reader | `TraceRing::capacity() -> usize` and `TraceRing::pending_len() -> usize` (`vb_runtime/src/trace.rs:39-49`) | These are the only accessors the bead touches. |

### 4. Lint-Policy Boundary (workspace → crate)

| Side | Source | Notes |
|------|--------|-------|
| Source | `as_conversions = "deny"` in workspace `[lints]` table | `docs/master/section-040-cargo-and-lint-contract.md:34`, `docs/master/section-034-workspace-cargo-contract.md:72` |
| Local exception | `#[cfg_attr(test, allow(... clippy::as_conversions ...))]` in `vb_runtime/src/lib.rs:13-43` | Test-only; out of scope. |
| Removed exception | `#[allow(clippy::as_conversions)]` at `vb_runtime/src/runtime.rs:583` | The bead's target. |
| Gate | `-D clippy::as_conversions` in CI (`docs/master/section-040-ci-gate.md:38`) | The CI gate that produces an error if the allow is reintroduced. |

### 5. AST-Scanner Boundary

| Side | Source | Notes |
|------|--------|-------|
| Source | `xtask forbidden-scan` AST scanner | `docs/master/section-041-forbidden-scan-contract.md:26`, `docs/master/section-077-ai-safe-quality-infrastructure.md:194` |
| Coverage | Walks all `crates/*/src/**` first-party source | Targets `as`-casts in production source regardless of lint scope. |

### 6. Numeric-Regression Boundary (existing test net)

| Side | Source | Notes |
|------|--------|-------|
| Tests | `trace_ring_fill_pct_f32_f64_bit_exact_for_powers_of_two` (`vb_runtime/src/trace/tests.rs:1208`) | Bit-exact equivalence for every power-of-two cap ∈ [1, 2^20] |
| Tests | `trace_ring_fill_pct_f32_f64_within_one_ulp_for_general_caps` (`vb_runtime/src/trace/tests.rs:1249`) | 1-ULP bound for every cap ∈ [1, 2^20] at five sample lengths |
| Tests | `trace_ring_fill_pct_boundary_values_are_bit_exact` (`vb_runtime/src/trace/tests.rs:1281`) | Bit-exact at empty-ring (len=0) and full-ring (len=cap) boundaries |

These three tests pin the numeric equivalence class. The fix must satisfy them by construction (no change to f32-direct computation, only the upstream narrowing changed from `as` to `try_from`).

### 7. Unsafe / FFI Boundary

The bead does not cross this boundary. `vb_runtime` is `#![forbid(unsafe_code)]` and the replacement uses only `core::convert::TryFrom`, `core::convert::From`, and integer arithmetic. No `unsafe`, no FFI, no SIMD, no raw pointers.

### 8. Storage Boundary

Not crossed. `Runtime::collect_metrics` does not read or write to any persistent store.

### 9. Network Boundary

Not crossed. No socket, no IPC, no HTTP, no serialization-deserialization in the touched code path.

### 10. Time / Randomness Boundary

Not crossed. The replacement is a pure function of two `usize` values.

### 11. Async Boundary

Not crossed. `collect_metrics` is synchronous; no `async`, no `Future`, no `Stream`, no tokio/loom/spawn surface.

### 12. Verifier-Bearing Boundary

Not crossed. `rg -l "trace_ring_fill_pct|collect_metrics|trace_len|trace_capacity" crates/vb_runtime/src/verification/` returns zero matches. There is no Kani, Flux, Verus, Loom, or proptest artifact bound to this code path. The bead is pure source-lint and numeric-equivalence regression; proof-planner can declare no new proof obligations.

## Boundary Diagram (ASCII)

```
                ┌──────────────────────────────────────────────────────────┐
                │  Workspace Lint Policy: as_conversions = "deny"         │
                │  docs/master/section-040-cargo-and-lint-contract.md:34  │
                └─────────────────────────┬────────────────────────────────┘
                                          │ inherits
                                          ▼
   ┌────────────────────────────────────────────────────────────────────┐
   │ vb_runtime/src/runtime.rs                                          │
   │   fn collect_metrics(&self) -> RuntimeMetricsSnapshot {            │
   │     ...                                                            │
   │     let trace_capacity = shard.trace_ring().capacity();  // usize │
   │     let trace_len     = shard.trace_ring().pending_len(); // usize │
   │     ┌─────────────────────────────────────────────┐                │
   │     │ if trace_capacity > 0 {                      │  ← THIS BEAD  │
   │     │   let cap = u32::try_from(trace_capacity)    │                │
   │     │            .unwrap_or(0);                    │                │
   │     │   let len = u32::try_from(trace_len)         │                │
   │     │            .unwrap_or(0);                    │                │
   │     │   let ratio = f32::from(len) / f32::from(cap);│                │
   │     │   ratio * 100.0                              │                │
   │     │ } else { 0.0 }                               │                │
   │     └─────────────────────────────────────────────┘                │
   │     ...                                                            │
   │   }                                                                 │
   └────────────────────────────────────────────────────────────────────┘
                                          │ produces
                                          ▼
   ┌────────────────────────────────────────────────────────────────────┐
   │ vb_runtime/src/counters.rs:113                                     │
   │   pub trace_ring_fill_pct: f32      (ShardMetricsSnapshot field)   │
   └────────────────────────────────────────────────────────────────────┘
                                          │ re-declared in
                                          ▼
   ┌────────────────────────────────────────────────────────────────────┐
   │ vb_ipc/src/metrics.rs:37                                            │
   │   pub trace_ring_fill_pct: f32      (IPC re-declaration)           │
   └────────────────────────────────────────────────────────────────────┘
                                          │ tested by
                                          ▼
   ┌────────────────────────────────────────────────────────────────────┐
   │ vb_runtime/src/trace/tests.rs:1186-1309                            │
   │   trace_ring_fill_pct_f32_f64_bit_exact_for_powers_of_two          │
   │   trace_ring_fill_pct_f32_f64_within_one_ulp_for_general_caps      │
   │   trace_ring_fill_pct_boundary_values_are_bit_exact                │
   └────────────────────────────────────────────────────────────────────┘
```

## Crossing Rules

- The bead crosses the **Lint-Policy Boundary** (removing a local allow) and the **Numeric-Regression Boundary** (verifying the substitution preserves equivalence).
- The bead does not cross the **Type-Definition Boundary** (`f32` field type is frozen).
- The bead does not cross any **Verus/Kani/Flux/Loom/proptest/fuzz** verifier-bearing boundary (none exists for this code path).

## Module-Level Dependency Graph (within scope)

```
runtime.rs::Runtime::collect_metrics
   ├── counters.rs::RuntimeMetricsSnapshot, ShardMetricsSnapshot
   ├── shard::Shard::trace_ring()
   │      └── trace::TraceRing::capacity(), TraceRing::pending_len()
   └── (no other dependencies)
```

No new module-level dependencies are introduced. No `Cargo.toml` change is required (per `delivery-scope.jsonl` row `r11`).