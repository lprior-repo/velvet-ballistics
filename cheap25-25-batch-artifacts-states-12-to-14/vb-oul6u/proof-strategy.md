# Proof Strategy — vb-oul6u (Lint: remove runtime metric `as_conversions` suppression)

| Field | Value |
|-------|-------|
| bead_id | vb-oul6u |
| state | 4 (Proof Planning) |
| invocation_id | p4-proof-planner-cheap25 |
| source_checkout | /home/lewis/src/velvet-ballistics |
| isolated_workdir | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u |
| jj_root | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u |
| jj_workspace | cheap25-vb-oul6u |
| upstream_main | 2c8ea33c9 |
| captured_at | 2026-07-01 |
| proof_strategy_class | lint-remediation + numeric-equivalence regression net |
| behavior_affecting | false (numeric equivalence preserved within 1 ULP for the documented production capacity range) |

## 1. Bead Summary

`vb-oul6u` is a single-file lint remediation in `crates/vb_runtime/src/runtime.rs:578-588` inside `Runtime::collect_metrics`. The locally-scoped `#[allow(clippy::as_conversions)]` (line 583) and the `(trace_len as f32) / (trace_capacity as f32)` expression (line 584) are replaced with the bounded-narrowing pattern that the six sibling metric lines (571-577, 596) already use:

```rust
let cap_u32 = u32::try_from(trace_capacity).unwrap_or(0);
let len_u32 = u32::try_from(trace_len).unwrap_or(0);
let ratio = f32::from(len_u32) / f32::from(cap_u32);
ratio * 100.0
```

The `SAFETY:` comment block at lines 581-582 is removed (it justified an `as`-cast that no longer exists). The fallback value is `0` (not `u32::MAX`) to preserve the sentinel intent of the outer `if trace_capacity > 0` guard.

## 2. Bead Classification

- **Risk class:** L (lint policy) + N (numeric equivalence); secondary D (documentation), A (public type freeze), B (regression).
- **Behavior-affecting:** **false.** The replacement preserves the observable `f32` metric value bit-identically for every `trace_capacity ∈ [1, 2^20]` and every `trace_len ∈ [0, trace_capacity]` (RA-003 corpus).
- **Formal-verifier applicability:** **none.** No Verus/Kani/Flux/Loom/Miri harness references this code path (`rg -l "trace_ring_fill_pct|collect_metrics" crates/vb_runtime/src/verification/` returns no matches). The replacement is a deterministic function of two `usize` values, fully pinned by the existing RA-003 test corpus.
- **Public API surface:** frozen. `Runtime::collect_metrics(&self) -> RuntimeMetricsSnapshot` signature unchanged; `ShardMetricsSnapshot.trace_ring_fill_pct: f32` field type frozen at `vb_runtime/src/counters.rs:113` and re-declared at `vb_ipc/src/metrics.rs:37`.

## 3. Proof Architecture

### 3.1 Formal Verification Stance

No formal verification (Verus, Kani, Flux, Loom, Miri) is applicable. Each is explicitly declared `not_applicable` in `verifier-lane-decisions.jsonl` with concrete evidence:

| Lane | Reason for `not_applicable` |
|------|-----------------------------|
| Verus | No `proof fn` / `spec fn` in `verification/verus/` references `collect_metrics`, `trace_ring_fill_pct`, `trace_capacity`, or `trace_len`. The replacement is a 5-line expression with no abstraction requiring a separate spec model; a Verus proof would be VACUUM (GOD RULE 2 violation). |
| Kani | No `#[kani::proof]` harness references this code path; the equivalence class is fully covered by the deterministic RA-003 test corpus. |
| Flux | No `#[refined_by]` or `#[spec]` annotation targets the ratio; the input domain is `usize` with no refinement needed. |
| Loom | `Runtime::collect_metrics` is synchronous, takes `&self` only, and has no shared mutable state. No concurrency surface exists to model. |
| Miri | `vb_runtime/src/runtime.rs:1` declares `#![forbid(unsafe_code)]`; the replacement introduces no `unsafe` blocks. |
| cargo-fuzz | The function has no external input boundary; the input domain is the trusted Rust type system. |
| proptest | The equivalence class is exhaustively covered by the three RA-003 tests (`bit-exact_for_powers_of_two`, `within_one_ulp_for_general_caps`, `boundary_values_are_bit_exact`); a proptest harness would add redundant statistical coverage. |

### 3.2 Primary Verification Layers

| Layer | Verifier | Risk Tags | Owner | Seed |
|-------|----------|-----------|-------|------|
| Source lint | `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions` | lint, policy | black-hat-reviewer (State 6) | seed-vb-oul6u-01 + seed-vb-oul6u-05 |
| AST scan | `xtask forbidden-scan` (AST scanner targets `unchecked indexing/slicing/as casts`) | lint, policy, documentation | black-hat-reviewer (State 6) | seed-vb-oul6u-01 + seed-vb-oul6u-05 + seed-vb-oul6u-06 |
| Numeric regression net (existing) | `cargo test -p vb_runtime --lib trace_ring_fill_pct` | numeric_safety, regression_risk | test-writer (State 5) | seed-vb-oul6u-02 |
| Call-site regression (new) | `cargo test -p vb_runtime --lib` with three new assertions in `tick_shard_tests.rs` | regression_risk, numeric_safety | test-writer (State 5) | seed-vb-oul6u-03 |
| IPC wire-format regression (existing) | `cargo test -p vb_ipc` (roundtrip tests in `vb_ipc/src/metrics/tests.rs`) | public_api, regression_risk | black-hat-reviewer (State 6) | seed-vb-oul6u-04 |

### 3.3 Proof Obligation Count

3 obligations planned (per user directive and seed coverage), one per applicable verification layer above:

1. **PO-OUL6U-LINT-001** — source-lint clean (clippy `as_conversions` deny + AST forbidden-scan).
2. **PO-OUL6U-RA003-002** — RA-003 numerical-equivalence regression net (three existing tests).
3. **PO-OUL6U-CALLSITE-003** — call-site regression (three new tests at `Runtime::collect_metrics` call sites).

Each obligation is `behavior_affecting: false` (numeric equivalence preserved within 1 ULP, documented production cap ≤ 2^20).

## 4. Replacement Strategy (Pinned)

The contract pins **Option A**: `u32::try_from(...).unwrap_or(0) + f32::from(u32)`, mirroring the six sibling lines. The alternative **Option B** (f64-then-f32 path) is rejected because it also requires `as`-casts (`usize → f64`, `f64 → f32`) that the workspace lint denies.

Forbidden substitutions (per user directive):

- **Forbidden:** `unwrap_or(u32::MAX)` — would silently saturate the ratio and break the sentinel intent of the outer `if trace_capacity > 0` guard.
- **Forbidden:** `as f32` direct cast — violates `as_conversions = "deny"` workspace lint.
- **Forbidden:** Changing `pub trace_ring_fill_pct: f32` to any other numeric type — frozen at `vb_runtime/src/counters.rs:113` and re-declared at `vb_ipc/src/metrics.rs:37`; changing it breaks IPC wire format.

## 5. Trusted Base

Trusted surfaces and assumptions are recorded in `trusted-base-plan.md`. Summary:

- **Type system guarantees:** `u32::try_from(usize)` is total and well-defined; `f32::from(u32)` is exact for the full `u32` range.
- **Construction invariants:** `TraceRing::new(capacity)` enforces `capacity.max(1)`, so `trace_capacity >= 1` for any constructed ring. The documented production configuration bounds capacity to ≤ 4096.
- **Frozen public types:** `Runtime::collect_metrics(&self) -> RuntimeMetricsSnapshot`; `ShardMetricsSnapshot.trace_ring_fill_pct: f32` (both `vb_runtime` and `vb_ipc` declarations).
- **Lint policy:** `as_conversions = "deny"` at the workspace level (`docs/master/section-040-cargo-and-lint-contract.md:34`); the bead does not weaken it.

## 6. Waiver Stance

No behavior-affecting waiver candidates. Seven lanes (Verus, Kani, Flux, Loom, Miri, cargo-fuzz, proptest) are explicitly `not_applicable` for this bead and are recorded as waiver candidates of type `not_applicable` in `waiver-candidates.jsonl` (non-behavior-affecting). The lint-policy preservation lane (INV-006, POST-004) is enforced by the source-lint obligation, not a waiver.

## 7. Risk Residuals

- **H-05 (sentinel preservation):** Resolved by `unwrap_or(0)` fallback. The fallback is unreachable inside the surrounding `if trace_capacity > 0` guard; choosing `0` (not `u32::MAX`) preserves the sentinel intent. Source comment must document this choice.
- **H-09 (overflow at cap > u32::MAX):** Unreachable; documented production max is 4096. `u32::try_from` falls back to `0`, but the outer guard intercepts zero first.
- **H-10 (codegen):** `f32::from(u32)` and `value as f32` produce the same CVT instruction on common hardware; no measurable performance regression expected.
- **H-17 (perf):** `try_from(usize) for u32` is a single conditional; on common hardware, equivalent or faster than `as` for the documented capacity range.

## 8. Handoff

- **State 4b (proof-plan-reviewer):** Reviewer dispositions each lane decision.
- **State 5 (test-writer):** Adds the three new call-site regression tests at `tick_shard_tests.rs:529,544,630,641,678,715,724` (or a sibling test module).
- **State 6 (black-hat-reviewer):** Runs clippy, AST forbidden-scan, comment review, signature freeze verification.
- **State 7 (proof-to-implementation):** Maps the three obligations to Rust source refs and exact evidence commands (see `proof-to-implementation-input.md`).
- **State 12 (formal-verifier):** Executes and closes the ledger. For this bead, formal-verifier is not invoked; the obligations are closed by `cargo clippy`, `xtask forbidden-scan`, and `cargo test`.

## 9. Summary

The bead is a lint-remediation with numeric-equivalence regression net. Three obligations, all `behavior_affecting: false`. Seven formal-verifier lanes are explicitly `not_applicable` with concrete evidence. No behavior-affecting waivers. No formal-proof artifacts to author. The proof strategy is a deterministic test/lint plan, not a formal-verifier plan.