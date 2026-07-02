# Hazard Analysis — vb-oul6u (Lint: remove runtime metric `as_conversions` suppression)

## Purpose

Enumerate every behavioural, lint, numeric, performance, and process hazard associated with the substitution `(trace_len as f32) / (trace_capacity as f32) → f32::from(u32::try_from(...).unwrap_or(0))`. Each hazard is classified by risk category, severity, mitigation, and proof/test lane.

## Hazard Classes Used

- **T** — Temporal / scheduling
- **C** — Concurrency / Send / Sync / race
- **U** — Unsafe / provenance / FFI / raw pointer
- **P** — Parser / codec / hostile input
- **L** — Lint policy / static analysis
- **N** — Numeric / floating-point / lossless conversion
- **B** — Behaviour / regression / observable API drift
- **R** — Performance / release / regression
- **S** — Storage / persistence
- **A** — API surface / public type freeze
- **D** — Documentation / justification comment drift

## Hazard Table

| ID | Class | Hazard | Severity | Mitigation | Lane |
|----|-------|--------|----------|-----------|------|
| H-01 | L | A future patch reintroduces `as`-cast in `vb_runtime` production source and triggers `-D clippy::as_conversions` in CI | High (CI blocker) | `forbidden-scan` AST scanner (`docs/master/section-041-forbidden-scan-contract.md:26`) targets the class; black-hat-reviewer runs it at State 6 | Clippy / AST gate |
| H-02 | L | A future patch widens `#[allow(clippy::as_conversions)]` to a function/module/file scope | High (CI blocker; violates Section 44.21) | Workspace `as_conversions = "deny"` cannot be relaxed; the only legitimate scope is a single statement or `cfg(test)` | Clippy policy |
| H-03 | N | Replacement diverges from `(trace_len as f32) / (trace_capacity as f32)` by more than 1 ULP for some cap ∈ [1, 2^20] | Medium (silent metric drift) | The three RA-003 tests in `vb_runtime/src/trace/tests.rs:1186-1309` already pin the equivalence class and must continue to pass | Rust unit test |
| H-04 | N | Replacement diverges at the boundary (empty ring → 0.0, full ring → 100.0) | Medium (operator-visible drift) | RA-003 test `trace_ring_fill_pct_boundary_values_are_bit_exact` (`trace/tests.rs:1281`); test-writer lane adds `Runtime::collect_metrics` boundary regressions per `delivery-scope.jsonl` row `r03` | Rust unit test |
| H-05 | N | `unwrap_or(0)` fallback produces `0.0 / 0.0 = NaN` if upstream `TraceRing::new` clamp is ever violated | Low (latent; invariant enforced upstream) | Source comment must record the sentinel choice; do not change fallback to `unwrap_or(u32::MAX)` which would silently saturate | Source comment + review |
| H-06 | A | `trace_ring_fill_pct` field type changes from `f32` to `f64` | High (IPC wire format break) | Field type is pinned at `vb_runtime/src/counters.rs:113` and `vb_ipc/src/metrics.rs:37`; any drift must be reflected in both declarations | Out-of-scope; black-hat review |
| H-07 | A | Public signature of `Runtime::collect_metrics` changes | High (downstream consumer break) | Signature is frozen at `runtime.rs:561`; the bead does not modify it | Out-of-scope |
| H-08 | B | Replacement expression produces a value that is structurally identical but observably different in the IPC roundtrip (e.g., `NaN` vs `0.0` for an empty ring) | Medium | IPC roundtrip tests `shard_metrics_with_nan_trace_ring_fill_pct_roundtrip` (`vb_ipc/src/metrics/tests.rs:298`) and `shard_metrics_with_negative_trace_ring_fill_pct_roundtrip` (`vb_ipc/src/metrics/tests.rs:317`) cover the wire-format edge cases | Rust unit test |
| H-09 | B | The replacement overflows at `cap > u32::MAX` | Low (unreachable; documented production max is 4096) | `u32::try_from` falls back to `0`, but `trace_capacity > 0` guard intercepts zero; non-zero overflow would still produce a saturated ratio | Source comment + review |
| H-10 | R | `f32::from(u32)` codegen differs from `value as f32` for `u32` inputs | Negligible (no codegen difference; both go through the same CVT instruction) | Replace is value-equivalent; no runtime cost | Verify by inspection |
| H-11 | D | The `SAFETY:` comment justifying the `as`-cast is left in place after the replacement, becoming a misleading comment | Low | Either delete the comment block or rewrite to describe the `try_from` fallback; lint policy forbids `// SAFETY:` blocks that are not adjacent to `unsafe` | Source cleanup |
| H-12 | L | A test file accidentally copies the old `as`-cast pattern | Low | `vb_runtime/src/lib.rs:13-43` already permits `as_conversions` under `cfg(test)`; if a test under that gate is affected, no regression. Outside the gate, the same clippy deny applies | Clippy gate |
| H-13 | C | A new caller mutates `TraceRing` while `collect_metrics` reads it | Low (pre-existing concern; out of scope) | `collect_metrics` takes `&self`; any concurrent mutation requires `&mut self` and is statically excluded by Rust's borrow checker | Out-of-scope |
| H-14 | T | A time-dependent change to `pending_len` between two `collect_metrics` calls makes the metric inconsistent across calls | Low (idiomatic; out of scope) | Documented in `ShardMetricsSnapshot` docstring ("snapshot"); called per shard in a single loop iteration | Out-of-scope |
| H-15 | U | A `unsafe` block is introduced to "optimize" the narrowing | Low (workspace forbids `unsafe`) | `#![forbid(unsafe_code)]` in `runtime.rs:1`; `AGENTS.md` "Engineering Rules" forbids `unsafe` in first-party code | Out-of-scope |
| H-16 | P | Hostile input is supplied to `collect_metrics` | N/A | Function takes `&self` only; no external input crosses this boundary | Out-of-scope |
| H-17 | R | Performance regression from `try_from` vs `as` | Negligible | `try_from(usize) for u32` is a single conditional; on common hardware it is the same or faster than `as` for the documented capacity range (≤ 4096 fits in `u32` always, so the fall-through is the success path) | Verify by inspection / Criterion if needed |
| H-18 | S | The persisted `RustMetricsSnapshot` (if any) is corrupted by the change | Low | `RuntimeMetricsSnapshot` is in-memory only; IPC serializes a fresh snapshot on demand | Out-of-scope |
| H-19 | L | A reviewer approves the change but does not remove the now-stale `SAFETY:` comment | Low | Black-hat-reviewer must run a final pass on lines 580-588 of `runtime.rs` to confirm comment is rewritten or removed | Black-hat review |
| H-20 | D | The `delivery-scope.jsonl` row `r01` is later read as outdated guidance, recommending the wrong replacement | Low | This contract pins Option A (`u32::try_from + f32::from(u32)`); downstream `proof-to-implementation` and `holzman-rust` agents read this contract | This contract |

## Hazard Summary by Class

| Class | Count | Notable IDs |
|-------|-------|-------------|
| T | 1 | H-14 |
| C | 1 | H-13 |
| U | 1 | H-15 |
| P | 0 | (none — function has no external input boundary) |
| L | 4 | H-01, H-02, H-12, H-19 |
| N | 4 | H-03, H-04, H-05, H-09 |
| B | 2 | H-08, H-09 |
| R | 2 | H-10, H-17 |
| S | 1 | H-18 |
| A | 2 | H-06, H-07 |
| D | 2 | H-11, H-20 |

## Critical Hazards (severity ≥ Medium)

| ID | Class | Description |
|----|-------|-------------|
| H-01 | L | Reintroducing `as`-cast in production source — CI blocker. |
| H-02 | L | Widening the `#[allow]` scope — CI blocker; Section 44.21 standing violation. |
| H-03 | N | Numeric divergence beyond 1 ULP for some cap ∈ [1, 2^20] — silent metric drift. |
| H-04 | N | Boundary drift (0.0 or 100.0 not produced) — operator-visible. |
| H-06 | A | Field type widening — IPC wire format break. |
| H-07 | A | Public signature change — downstream consumer break. |
| H-08 | B | IPC roundtrip drift — observable wire-level change. |

## Mitigation Mapping (which stage owns which mitigation)

| Stage | Owns | Hazards |
|-------|------|---------|
| `holzman-rust` / `functional-rust` (State 4) | Source substitution, comment rewrite, `unwrap_or(0)` fallback | H-05, H-09, H-11, H-15, H-17 |
| `test-writer` / `bdd-enforcer` (State 5) | Add three call-site regressions (empty / half / full) | H-04, H-08 |
| `black-hat-reviewer` (State 6) | Run clippy, AST scanner, comment review, signature freeze verification | H-01, H-02, H-06, H-07, H-11, H-12, H-19 |
| `evidence-packaging` (State 11) | Capture raw command output for clippy, AST scan, three RA-003 tests, three new call-site tests | H-03, H-04 |
| This contract (State 3) | Pin replacement strategy, fallback value, comment policy, field type | H-06, H-07, H-20 |

## Lint-Only Risk Posture

The bead is classified as a **lint-only source change with numeric-equivalence regression net**. No formal-verifier (Verus/Kani/Flux/Loom) is required and none can land proof coverage for this code path. The numeric equivalence is bounded by Rust's core conversion rules (lossless `From<u32> for f32`) and by the existing RA-003 test corpus; no additional state-machine or invariant reasoning is required.