# Codebase Map — vb-oul6u (lint: remove runtime metric `as_conversions` suppression)

- bead_id: vb-oul6u
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u
- jj_root: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u
- jj_workspace: cheap25-vb-oul6u
- upstream_main: 2c8ea33c9
- captured_at: 2026-07-01 (femdation p2 explore)
- scope_kind: single-file lint remediation in `vb_runtime::runtime::Runtime::collect_metrics`
- related_bead_history: vb-8rldf (RA-003 no-op closure landed in commit `5f9b566d7`; flagged this `as_conversions` allow as a Section 44.20/44.21 standing violation)

## 1. The one offending site

| Field | Value |
| --- | --- |
| Path | `crates/vb_runtime/src/runtime.rs` |
| Lines | 580-588 (single `#[allow(clippy::as_conversions)]` over the ratio assignment) |
| Enclosing fn | `pub fn Runtime::collect_metrics(&self) -> RuntimeMetricsSnapshot` (line 561) |
| Module | `runtime` (re-exported by `lib.rs:82`: `pub mod runtime;`) |
| Crate lint posture | `#![forbid(unsafe_code)]` (`runtime.rs:1`); no file-level as_conversions override |

### Exact code (verbatim)
```rust
let trace_capacity = shard.trace_ring().capacity();      // line 578, type: usize
let trace_len = shard.trace_ring().pending_len();        // line 579, type: usize
let trace_ring_fill_pct = if trace_capacity > 0 {
    // SAFETY: trace_len and trace_capacity are bounded by configuration
    // (typically 4096). Safe lossless narrowing to u32 for metric calculation.
    #[allow(clippy::as_conversions)]                      // line 583 — TARGET
    let ratio = (trace_len as f32) / (trace_capacity as f32);
    ratio * 100.0
} else {
    0.0
};
```

### Why this is the only production `as_conversions` site in `vb_runtime`
- `rg -n "as_conversions" crates/vb_runtime/src/` shows exactly two matches:
  - `crates/vb_runtime/src/runtime.rs:583` (the production allow).
  - `crates/vb_runtime/src/lib.rs:17` — inside a `#[cfg_attr(test, allow(...))]` block at lines 13-43, so the allow is only active under test builds. The runtime.rs allow is required because non-test builds inherit the workspace-level `as_conversions = "deny"` and the CI gate `-D clippy::as_conversions` (see §5).
- All 30+ other `as_conversions` matches repo-wide are test/gate annotations (e.g. `vb_storage/src/error_tests.rs`, `vb_runtime/tests/*_test.rs`) which are inside `#[allow(...)]` blocks under `cfg(test)`/`cfg(kani)` and are out of scope for this bead.

### Surrounding context (lines 569-608 of `runtime.rs`)
- The same loop already uses a documented, idiomatic narrowing pattern for six sibling fields (`active_runs`, `queue_depth`, `queue_remaining`, `pending_timers`, `frame_pool_free`, `frame_pool_total`, `shard_id`):
  ```rust
  let active_runs = u32::try_from(shard.active_run_count()).unwrap_or(u32::MAX);
  ...
  let shard_id = u32::try_from(index).unwrap_or(u32::MAX);
  ```
- The `trace_ring_fill_pct` branch is the lone outlier still using `as`. The natural fix mirrors the local convention (`u32::try_from` + `unwrap_or(...)`) and converts `u32 → f32` via the lossless `From<u32> for f32` impl (`f32::from(u32)`), avoiding the `as` lint.
- An alternative that matches the formulation already pinned by the three `trace_ring_fill_pct_*` tests in `crates/vb_runtime/src/trace/tests.rs` is `usize as f64` → division in `f64` → `as f32` at the very end. The tests prove the f32-direct and f64-then-f32 paths agree at every production capacity.

## 2. Supporting files

| Path | Why it matters |
| --- | --- |
| `crates/vb_runtime/src/lib.rs` | Defines the crate-wide `#![allow(...)]` block under `#[cfg(test)]` (line 13-43); contains `clippy::as_conversions` only in the `#[cfg(test)]` arm — so any production allow must be local. |
| `crates/vb_runtime/src/counters.rs:113` | `pub trace_ring_fill_pct: f32` — public field of `ShardMetricsSnapshot`. Constrains the replacement to produce an `f32`. |
| `crates/vb_runtime/src/counters.rs:120-132` | `pub struct RuntimeMetricsSnapshot { pub shards: Vec<ShardMetricsSnapshot>, ... }` — public aggregate. |
| `crates/vb_runtime/src/trace.rs:39-49` | `pub const fn capacity(&self) -> usize`, `pub fn pending_len(&self) -> usize`. `TraceRing::new(capacity)` enforces `capacity.max(1)`. Bounded at construction. |
| `crates/vb_runtime/src/trace/tests.rs:1186-1309` | The RA-003 numerical-equivalence tests already pinned for the bead scope. Three test functions reference exactly this ratio and pin bit-exactness/1-ULP bounds up to cap = 2^20. |
| `crates/vb_runtime/src/shard/tests/tick_shard_tests.rs:529,544,630,641,678,715,724` | Existing `runtime.collect_metrics()` callers. Test config uses `trace_capacity: 16` (line 143). None of these assertions read `trace_ring_fill_pct` directly — only `command_queue_depth`. So replacing the cast cannot regress them. |
| `crates/vb_ipc/src/metrics.rs:37` | Re-declares `pub trace_ring_fill_pct: f32` for IPC serialization. Public-API surface unchanged. |
| `crates/vb_ipc/src/metrics/tests.rs` | Roundtrip tests including `shard_metrics_with_nan_trace_ring_fill_pct_roundtrip` (line 298) and `shard_metrics_with_negative_trace_ring_fill_pct_roundtrip` (line 317). Unaffected by the change. |
| `to-fix/wave4/agent-03-black-hat.md:38` | Names this exact allow as a Section 44.20/44.21 standing violation from the RA-003 no-op closure of `vb-8rldf`. |
| `to-fix/wave3/agent-07-test-reviewer.md:21,53` | Independent test-reviewer corroboration that the RA-003 closure was documentation, not a fix. |

## 3. Public API surface diff (anticipated)

No public symbol touched. The public contract is:
- `Runtime::collect_metrics(&self) -> RuntimeMetricsSnapshot` — signature unchanged.
- `ShardMetricsSnapshot.trace_ring_fill_pct: f32` — type unchanged.
- Observable value at every documented production `trace_capacity` (cap ≤ 2^20, validated by the three RA-003 tests) — unchanged.
- For cap ∈ (2^20, 2^24] — unchanged because both `trace_len` and `trace_capacity` are still ≤ 2^24, so they remain exactly representable in `f32` (the `usize → f32` direct path was already lossless there; only rounding in the *division* itself can move a 1-ULP which is below monitoring resolution).
- For cap > 2^24 — out of scope; `f32` would already have been lossy regardless of which replacement is chosen, and the trace ring is bounded by configuration far below 2^24 (typical 4096, hard ceiling via `capacity.max(1)` used directly by `rtrb`).

## 4. Tests already exercising this path

| Test fn | Location | What it pins |
| --- | --- | --- |
| `trace_ring_fill_pct_f32_f64_bit_exact_for_powers_of_two` | `crates/vb_runtime/src/trace/tests.rs:1208` | Bit-exact equivalence for every power-of-two cap ∈ [1, 2^20]. |
| `trace_ring_fill_pct_f32_f64_within_one_ulp_for_general_caps` | `crates/vb_runtime/src/trace/tests.rs:1249` | 1-ULP bound for every cap ∈ [1, 2^20] at five sample lengths. |
| `trace_ring_fill_pct_boundary_values_are_bit_exact` | `crates/vb_runtime/src/trace/tests.rs:1281` | Bit-exact at empty-ring (len=0) and full-ring (len=cap) boundaries. |
| Indirect call-site smoke tests | `crates/vb_runtime/src/shard/tests/tick_shard_tests.rs` (10 call sites) | Exercise `collect_metrics()` via shard execution; none assert on `trace_ring_fill_pct` numeric value, so they cannot regress from the change. |

### Tests the downstream test-planner / test-writer should add
1. A regression test for the *call site* (`Runtime::collect_metrics`) — none currently asserts on `trace_ring_fill_pct` with a known expected value. Recommend asserting `metrics.shards[0].trace_ring_fill_pct` for: (a) empty trace ring → `0.0`; (b) full trace ring → `100.0`; (c) half-full → `50.0`.
2. A targeted lint gate: `cargo clippy -p vb_runtime --tests -- -D clippy::as_conversions` plus `bash scripts/check-verus-production-binding.sh`-equivalent if production-binding contracts apply (NOT — this is a non-verifier-bearing lint fix).
3. `cargo test -p vb_runtime --lib trace_ring_fill_pct` should still pass and serve as the numerical regression net.

## 5. Policy references (lint / spec)

| Doc | Pin |
| --- | --- |
| `docs/master/section-040-cargo-and-lint-contract.md:34` | `as_conversions = "deny"` (master lint contract). |
| `docs/master/section-040-ci-gate.md:38` | CI gate: `-D clippy::as_conversions -D clippy::let_underscore_must_use`. |
| `docs/master/section-034-workspace-cargo-contract.md:72` | Workspace `[lints]` table: `as_conversions = "deny"`. |
| `docs/master/section-044-backend-ir-interpreter-definition-of-done.md:32` | Section 44.21: "Unchecked indexing, slicing, casts, and arithmetic are absent from first-party code." |
| `docs/master/section-041-forbidden-scan-contract.md:26` | xtask `forbidden-scan` AST scanner targets `unchecked indexing/slicing/as casts`. |
| `docs/master/section-077-ai-safe-quality-infrastructure.md:194` | AST scanner mandate reaffirmed. |
| `to-fix/wave4/agent-03-black-hat.md` | Prior black-hat review explicitly flagged this `as_conversions` allow as a Section 44.20/44.21 standing violation. |

Net effect of the bead: remove the standing `as_conversions` exemption so first-party `vb_runtime` source has zero `as`-casts in production, satisfying the deny contract and the Section 44.21 "casts absent from first-party code" rule.

## 6. Verification / formal-artifact posture

- No Kani harness references `Runtime::collect_metrics`, `trace_ring_fill_pct`, `trace_capacity`/`trace_len` directly: `rg -l "trace_ring_fill_pct|collect_metrics|trace_len|trace_capacity" crates/vb_runtime/src/verification/` returns no matches.
- No Flux refinement targets the ratio.
- No Verus spec names the field.
- Loom / proptest lanes do not constrain this path.
- Conclusion: zero formal-artifact blast radius; the bead is pure source-lint and a numeric-equivalence regression net. Proof-planner can declare no new proof obligations.

## 7. Downstream owner recommendations

| Stage | Owner | Notes |
| --- | --- | --- |
| Implementation (State 4) | `holzman-rust` / `functional-rust` | Replace `#[allow(clippy::as_conversions)]` with one of the two paths in §1. Recommendation: `u32::try_from(trace_capacity).unwrap_or(0)` + `u32::try_from(trace_len).unwrap_or(0)` + `f32::from(u32)` to mirror the six sibling lines. Multiply by `100.0` at the end. |
| Bridge | `proof-to-implementation` | If proof-planner determines Kani/Flux needed for any contract change — none expected. |
| Tests (State 5) | `test-writer` / `bdd-enforcer` | Add the three call-site regressions in §4. |
| Review (State 6) | `black-hat-reviewer` | Verify zero `as`-casts remain in `vb_runtime` production source via `rg -n "\\bas\\b" crates/vb_runtime/src/`. |
| Proof (formal) | NONE | See §6. |

## 8. Open questions for downstream agents

1. Replacement strategy — pick exactly one and document the choice in the commit:
   - (A) **Local convention (matches lines 571-577, 596)**: `u32::try_from(...).unwrap_or(0)` + `f32::from(u32)`. Cleanest match to surrounding code style.
   - (B) **Numerical-equivalence path (matches existing RA-003 tests)**: `usize as f64` + division in `f64` + `as f32`. Carries a `#[allow(clippy::as_conversions)]` for `usize → f64` too — needs care since `usize → f64` is also a lossless `as` that the lint will catch. Therefore (A) is the lint-clean path.
   - **Recommendation**: choose (A).
2. Should the docstring "SAFETY: ... bounded by configuration (typically 4096). Safe lossless narrowing to u32 for metric calculation." be updated to reflect the explicit `try_from`? (Yes — it currently justifies the `as`, not a `try_from`.)
3. Does `moon ci` currently pass with this `#[allow]` in place? The bead's runbook says lint is zero-tolerance, so the answer must be yes (the allow is locally scoped) but downstream black-hat-reviewer should confirm `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions` exits 0 after the fix.
4. Should `RustMetricsSnapshot` (or related) gain a `Debug` test to surface failure modes? Out of scope for this bead — flag for follow-up.

## 9. Exclusions (NOT in this bead's scope)

- All other `as_conversions` matches in the repo (test annotations, other crates) — explicitly out of scope; they live inside `cfg(test)`/`cfg(kani)` or are not first-party production code.
- `vb_ipc/src/metrics.rs` `trace_ring_fill_pct: f32` declaration — pure field type, no `as`-cast here.
- `docs/ra-003-no-op.md` — historical closure evidence; do NOT modify (records the prior bead-vb-8rldf RA-003 no-op and its reviewer lineage).
- `crates/vb_runtime/src/lib.rs:17` — stays; it gates `#[cfg(test)]` only and is the legitimate test-build permit.
- `as_conversions = "deny"` workspace policy — must remain `deny`; do not weaken.

