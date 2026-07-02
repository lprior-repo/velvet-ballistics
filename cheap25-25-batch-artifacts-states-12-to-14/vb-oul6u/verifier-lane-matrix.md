# Verifier Lane Matrix — vb-oul6u

Maps each proof seed to its assigned verifier lanes.

| ✅ required | — not applicable (with evidence) |

## Matrix

| Proof Seed ID | Description | Verus | Kani | Flux | Loom | Miri | Proptest | cargo-fuzz | cargo-clippy | AST scan | cargo test (RA-003) | cargo test (call-site) | cargo test (IPC roundtrip) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| seed-vb-oul6u-01 | Zero `as`-casts in `vb_runtime` production source (POST-002) | — | — | — | — | — | — | — | ✅ | ✅ | — | — | — |
| seed-vb-oul6u-02 | Numeric equivalence class within 1 ULP for cap ∈ [1, 2^20] (POST-003) | — | — | — | — | — | — | — | — | — | ✅ | — | — |
| seed-vb-oul6u-03 | `Runtime::collect_metrics` returns 0.0 / 50.0 / 100.0 at empty/half/full trace ring (POST-001) | — | — | — | — | — | — | — | — | — | — | ✅ | — |
| seed-vb-oul6u-04 | IPC wire format `trace_ring_fill_pct: f32` preserved (INV-001) | — | — | — | — | — | — | — | — | — | — | — | ✅ |
| seed-vb-oul6u-05 | Workspace lint policy `as_conversions = "deny"` not weakened (INV-006, POST-004) | — | — | — | — | — | — | — | ✅ | ✅ | — | — | — |
| seed-vb-oul6u-06 | `SAFETY:` comment block at runtime.rs:581-582 removed or rewritten (INV-005) | — | — | — | — | — | — | — | — | ✅ | — | — | — |
| seed-vb-oul6u-07 | `unwrap_or(0)` fallback preserves sentinel intent (INV-004) | — | — | — | — | — | — | — | — | — | ✅ | — | — |

## Verifier Lane Roster

| Lane | Decision | Owner State | Risk Class | Evidence Source |
|------|----------|-------------|-----------|----------------|
| `cargo-clippy` | required (lint lane) | 6 | L | `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions` |
| AST forbidden-scan | required (lint lane) | 6 | L, D | `bash scripts/forbidden-scan.sh` |
| `cargo test` (RA-003 numerical-equivalence regression net) | required (test lane) | 5 | N, B | `cargo test -p vb_runtime --lib trace_ring_fill_pct` |
| `cargo test` (call-site regression) | required (test lane) | 5 | B, N | `cargo test -p vb_runtime --lib trace_ring_fill_pct_call_site` (3 new assertions) |
| `cargo test` (IPC roundtrip) | required (regression lane) | 6 | A, B | `cargo test -p vb_ipc` |
| Verus | not_applicable | n/a | n/a | No Verus spec references this code path |
| Kani | not_applicable | n/a | n/a | No Kani harness references this code path |
| Flux | not_applicable | n/a | n/a | No Flux refinement targets the ratio |
| Loom | not_applicable | n/a | n/a | `collect_metrics` is synchronous, single-threaded, `&self`-only |
| Miri | not_applicable | n/a | n/a | `vb_runtime` is `#![forbid(unsafe_code)]` |
| Proptest | not_applicable | n/a | n/a | RA-003 corpus exhaustively covers the equivalence class |
| cargo-fuzz | not_applicable | n/a | n/a | Function has no external input boundary |

## Non-Applicable Lanes (with Evidence)

| Lane | Proof Seed(s) | Reason | Evidence |
|------|---------------|--------|----------|
| Verus | ALL | No Verus spec in `verification/verus/` references `collect_metrics`, `trace_ring_fill_pct`, `trace_capacity`, or `trace_len`. A Verus proof would be a VACUUM (GOD RULE 2 violation). The replacement is a 5-line deterministic expression; a separate spec model is unjustified. | `rg -l "trace_ring_fill_pct\|collect_metrics" verification/verus/` returns no matches. |
| Kani | ALL | No `#[kani::proof]` harness exists for this code path. The equivalence class is fully covered by the three RA-003 tests in `crates/vb_runtime/src/trace/tests.rs:1186-1309` (powers-of-two bit-exact, 1-ULP bound for general caps, boundary bit-exact). Kani would add redundant coverage. | `rg -l "kani" crates/vb_runtime/src/verification/` returns no matches for this path. |
| Flux | ALL | No `#[refined_by]` or `#[spec]` annotation targets the ratio. The input domain is `usize` with no refinement needed. | `rg -l "refined_by\|flux" crates/vb_runtime/src/runtime.rs` returns no matches. |
| Loom | ALL | `Runtime::collect_metrics` is synchronous, holds `&self` only, and has no shared mutable state. The Rust borrow checker statically excludes concurrent mutation. | `runtime.rs:561` signature is `pub fn collect_metrics(&self)`. No `Arc`, `Mutex`, or atomic operations. |
| Miri | ALL | `vb_runtime/src/runtime.rs:1` declares `#![forbid(unsafe_code)]`. The replacement introduces no `unsafe` blocks. Miri detects UB, not arithmetic semantics. | `runtime.rs:1` is `#![forbid(unsafe_code)]`. |
| Proptest | seed-vb-oul6u-02 | The three RA-003 tests sweep every cap ∈ [1, 2^20] exhaustively, providing stronger coverage than any proptest harness. | `crates/vb_runtime/src/trace/tests.rs:1186-1309` exhaustive loop. |
| cargo-fuzz | ALL | The function has no external input boundary; the only inputs are `&self` (trusted Rust type system). | `runtime.rs:561` signature: `&self -> RuntimeMetricsSnapshot`. |

## Lane Decision Trail

- **4 required lanes** (clippy, AST scan, cargo test RA-003, cargo test call-site) cover all 7 proof seeds.
- **7 not_applicable lanes** (Verus, Kani, Flux, Loom, Miri, Proptest, cargo-fuzz) cover formal-verifier / fuzz lanes that are inapplicable to a lint-only source change.
- **No TLA+ lane.** TLA+ is removed from the verifier profile (per `proof-planner` skill doctrine); temporal properties are out of scope for this single-function synchronous read.

## Legend

- ✅ = Active lane (obligation planned, owner assigned)
- — = Not applicable (with concrete evidence)
- **L** = Lint policy / static analysis
- **N** = Numeric / floating-point / lossless conversion
- **B** = Behaviour / regression / observable API drift
- **A** = API surface / public type freeze
- **D** = Documentation / justification comment drift