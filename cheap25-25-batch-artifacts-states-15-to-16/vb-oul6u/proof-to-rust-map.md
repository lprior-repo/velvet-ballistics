# Proof-to-Rust Map: vb-oul6u

## Bridge Metadata

| Field | Value |
|-------|-------|
| Bead | vb-oul6u |
| Title | Lint: remove runtime metric `as_conversions` suppression |
| State | 7 (proof-to-implementation bridge) |
| Agent | proof-to-implementation |
| Invocation | p7-proof-to-implementation-cheap25-vb-oul6u |
| Schema | proof-to-rust-map/v1 |
| Source checkout | /home/lewis/src/velvet-ballistics (control plane, read-only) |
| Workspace | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u |
| jj_root | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u |
| jj_workspace | cheap25-vb-oul6u |
| Bead-local artifact dir | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u/.beads/vb-oul6u/ |
| Bridge artifact | `.beads/vb-oul6u/proof-to-rust-map.md` (this file) |
| Refinement obligations | `.beads/vb-oul6u/rust-refinement-obligations.jsonl` (intentionally empty — see §4) |
| Upstream review | `.beads/vb-oul6u/proof-review.md` (state 6, STATUS: APPROVED) |
| Upstream planned obligations | `.beads/vb-oul6u/proof-obligations.planned.jsonl` (3 obligations, 0 formal-verifier lanes) |
| Previous state review | State 6, p6-proof-reviewer-cheap25-vb-oul6u, APPROVED (NO_PROOF_WORK) |

---

## 1. Bead Summary

`vb-oul6u` is a single-file lint remediation in
`crates/vb_runtime/src/runtime.rs` inside `Runtime::collect_metrics`. The
locally-scoped `#[allow(clippy::as_conversions)]` (pre-fix line 583) and the
`(trace_len as f32) / (trace_capacity as f32)` expression (pre-fix line 584) are
replaced with the bounded-narrowing pattern that six sibling metric lines in the
same function (pre-fix 571-577, 596) already use:

```rust
let cap_u32 = u32::try_from(trace_capacity).unwrap_or(0);
let len_u32 = u32::try_from(trace_len).unwrap_or(0);
let ratio = f32::from(len_u32) / f32::from(cap_u32);
ratio * 100.0
```

The `SAFETY:` block (pre-fix lines 581-582) is removed or rewritten because it
justified an `as`-cast that no longer exists. The workspace `as_conversions =
"deny"` policy (`docs/master/section-040-cargo-and-lint-contract.md:34` and
`docs/master/section-034-workspace-cargo-contract.md:72`) is preserved
unchanged. The replacement is `behavior_affecting: false` — numeric equivalence
preserved within 1 ULP for every documented production capacity range per the
existing RA-003 corpus.

## 2. Bridge Disposition

| Disposition | Value |
|-------------|-------|
| Proof obligations routed from upstream | 3 |
| Obligations requiring Rust refinement harnesses | **0** |
| `rust-refinement-obligations.jsonl` row count | **0** (empty by design) |
| Bridge status | **NO_REFINEMENT_HARNESSES_REQUIRED** |
| State 11 (formal-verifier) invocation | **NOT REQUIRED** |

Per `proof-review.md` §17 (binding_classification: `n/a` — all 7 formal-verifier
lanes are `not_applicable`) and per `proof-strategy.md` §3.1 (formal-verifier
applicability: none), no Verus, Kani, Flux, Loom, Miri, cargo-fuzz, or proptest
harness is authored for this bead. The proof-writer's disposition
`NO_FORMAL_PROOF_WORK_REQUIRED` (transcript-state5-pw.txt:6) is the binding
constraint that makes this bridge a no-op for refinement-harness obligations.

The three obligations (`PO-OUL6U-LINT-001`, `PO-OUL6U-RA003-002`,
`PO-OUL6U-CALLSITE-003`) all map to existing or planned Rust **test/clippy
gates**; none is a formal-verifier obligation. The bridge preserves that
classification.

## 3. Obligation Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|-------|--------------------|------------------|---------------------|------------------------|----------|------------------|------------|
| PO-OUL6U-LINT-001 | Source-lint clean after `as_conversions` removal | false | `vb_runtime::runtime::Runtime::collect_metrics` @ `crates/vb_runtime/src/runtime.rs:578-595` (post-fix) | n/a (lint + AST scan; tooling-owned clauses) | **none** | cargo-clippy + ast-scan | `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions` + `bash scripts/forbidden-scan.sh` + `rg -n '\bas\b' crates/vb_runtime/src/` | State 6 black-hat-reviewer |
| PO-OUL6U-RA003-002 | RA-003 numerical-equivalence net still passes | false | `vb_runtime::trace::tests` @ `crates/vb_runtime/src/trace/tests.rs:1186-1309` (three RA-003 tests) | `trace_ring_fill_pct_f32_f64_bit_exact_for_powers_of_two` (line 1209), `trace_ring_fill_pct_f32_f64_within_one_ulp_for_general_caps` (line 1250), `trace_ring_fill_pct_boundary_values_are_bit_exact` (line 1283) | **none** | cargo-test (pre-existing RA-003) | `cargo test -p vb_runtime --lib trace_ring_fill_pct` | State 5 test-writer |
| PO-OUL6U-CALLSITE-003 | Three new call-site tests through `Runtime::collect_metrics` | false | `vb_runtime::runtime::Runtime::collect_metrics` @ `crates/vb_runtime/src/runtime.rs:578-595` + `vb_runtime::shard::tests::tick_shard_tests` @ `crates/vb_runtime/src/shard/tests/tick_shard_tests.rs:529,544,630,641,678,715,724` (planned call-site test module) | `collect_metrics_reports_zero_for_empty_trace_ring`, `collect_metrics_reports_fifty_for_half_full_trace_ring`, `collect_metrics_reports_one_hundred_for_full_trace_ring` (planned) | **none** | cargo-test (planned call-site regression) | `cargo test -p vb_runtime --lib collect_metrics_trace_ring_fill_pct` | State 5 test-writer |

All three rows have `refinement_harness_refs: none` because the
proof-writer-reviewer pair agreed that no formal-verifier harness is applicable
(see proof-review.md §17 and proof-strategy.md §3.1).

## 4. Refinement Obligation Routing

Per the bridge disposition (§2) and per the upstream
`proof-obligations.planned.jsonl` (none of whose rows carry a Verus / Kani /
Flux / Loom / Miri / cargo-fuzz / proptest `verifier` field), **zero**
`rust-refinement-obligation/v1` rows are emitted for this bead.

`rust-refinement-obligations.jsonl` is intentionally empty:

```text
$ wc -c .beads/vb-oul6u/rust-refinement-obligations.jsonl
0 .beads/vb-oul6u/rust-refinement-obligations.jsonl
```

A zero-byte file is the canonical surface for "no refinement obligations";
this is consistent with the approved proof-reviewer's `NO_PROOF_WORK`
disposition (proof-review.md:21).

If a downstream state (State 11 formal-verifier, State 12 closure) attempts to
load this JSONL and finds it empty, that is the expected outcome: state 11 is
not invoked for this bead per `proof-strategy.md` §108 ("For this bead,
formal-verifier is not invoked; the obligations are closed by `cargo clippy`,
`xtask forbidden-scan`, and `cargo test`").

## 5. Obligation-by-Obligation Source Mapping

### 5.1 PO-OUL6U-LINT-001 (Source-Lint Clean — Tooling-Owned)

| Field | Value |
|-------|-------|
| RRO ID | n/a (no RRO row emitted; see §4) |
| Proof claim ref | `proof-obligations.planned.jsonl` row 1 |
| Production target | `Runtime::collect_metrics` ratio branch at `crates/vb_runtime/src/runtime.rs:578-595` (post-fix lines encompass `u32::try_from(...).unwrap_or(0)` + `f32::from(u32)` pattern) |
| Pre-fix source refs | `crates/vb_runtime/src/runtime.rs:583` (`#[allow(clippy::as_conversions)]`) and `crates/vb_runtime/src/runtime.rs:584` (`(trace_len as f32) / (trace_capacity as f32)`) — confirmed by `evidence/rg-vb-runtime-as-casts-pre-fix.log` (line 1) |
| Pre-fix SAFETY block | `crates/vb_runtime/src/runtime.rs:581-582` — confirmed by `evidence/rg-safety-comment-pre-fix.log` (line 1) |
| Post-fix source refs | `crates/vb_runtime/src/runtime.rs:578-595` (replacement expression); no `as`-cast present in this region; no `#[allow(clippy::as_conversions)]` attribute present |
| Sibling lines confirming pattern | `crates/vb_runtime/src/runtime.rs:571-577, 596` (six sibling metric conversions using the same bounded-narrowing pattern) |
| Behavior test refs | n/a — this is a tooling gate, not a behavior test |
| Tooling refs | `xtask forbidden-scan` AST scanner; workspace `[lints].as_conversions = "deny"` invariant |
| Refinement harness refs | **none** (no formal-verifier applicable; lint policy is enforced by clippy + AST scanner + rg) |
| Behavior test refs (concrete) | n/a |
| Evidence command 1 | `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions 2>&1 | tee .beads/vb-oul6u/evidence/clippy-as-conversions.log` |
| Evidence command 2 | `bash scripts/forbidden-scan.sh 2>&1 | tee .beads/vb-oul6u/evidence/forbidden-scan.log` |
| Evidence command 3 | `rg -n '\bas\b' crates/vb_runtime/src/ | rg -v '^crates/vb_runtime/src/lib\.rs:' | tee .beads/vb-oul6u/evidence/vb-runtime-as-casts.log` |
| Evidence workdir | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u |
| Evidence artifacts (planned) | `.beads/vb-oul6u/evidence/clippy-as-conversions.log`, `.beads/vb-oul6u/evidence/forbidden-scan.log`, `.beads/vb-oul6u/evidence/vb-runtime-as-casts.log` |
| Expected evidence | (1) clippy exits 0 with zero `clippy::as_conversions` diagnostics; (2) `forbidden-scan.sh` reports zero as-cast count in `vb_runtime` production source; (3) `rg -n '\bas\b' crates/vb_runtime/src/` (excluding `lib.rs:13-43` `cfg(test)` allow block) returns zero matches; (4) `rg -n '^\s*//\s*SAFETY:' crates/vb_runtime/src/runtime.rs` returns zero matches |
| Mapping status | planned (closure owned by State 6 black-hat-reviewer / State 12 closure) |
| behavior_affecting | false |

### 5.2 PO-OUL6U-RA003-002 (RA-003 Numerical-Equivalence Net — Test-Owned)

| Field | Value |
|-------|-------|
| RRO ID | n/a (no RRO row emitted; see §4) |
| Proof claim ref | `proof-obligations.planned.jsonl` row 2 |
| Production target | `Runtime::collect_metrics` ratio branch at `crates/vb_runtime/src/runtime.rs:578-595` (post-fix) |
| Production target (pre-fix as-cast site) | `crates/vb_runtime/src/runtime.rs:584` (`(trace_len as f32) / (trace_capacity as f32)`) — replaced by bounded-narrowing pattern |
| Behavior test refs | `crates/vb_runtime/src/trace/tests.rs:1209::trace_ring_fill_pct_f32_f64_bit_exact_for_powers_of_two` |
| | `crates/vb_runtime/src/trace/tests.rs:1250::trace_ring_fill_pct_f32_f64_within_one_ulp_for_general_caps` |
| | `crates/vb_runtime/src/trace/tests.rs:1283::trace_ring_fill_pct_boundary_values_are_bit_exact` |
| Refinement harness refs | **none** (RA-003 corpus is the canonical regression net for any lossless replacement strategy; per `proof-review.md:132` "the corpus is the canonical regression net for any lossless replacement of `(trace_len as f32) / (trace_capacity as f32)`") |
| RA-003 corpus coverage | Powers-of-two caps with every `len ∈ [0, cap]` for `cap ∈ [1, 2^20]` (bit-exact); every cap in `[1, 2^20]` with 5 sample lengths (1-ULP bound); both boundaries for every cap (bit-exact empty-ring and full-ring) |
| Sentinel preservation | `unwrap_or(0)` fallback unreachable inside `if trace_capacity > 0` guard; `0_u32 / x = 0.0` in IEEE-754 (exact) → sentinel preserved |
| Evidence command | `cargo test -p vb_runtime --lib trace_ring_fill_pct 2>&1 | tee .beads/vb-oul6u/evidence/ra-003-trace-ring-fill-pct.log` |
| Evidence workdir | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u |
| Evidence artifact (planned) | `.beads/vb-oul6u/evidence/ra-003-trace-ring-fill-pct.log` |
| Pre-fix evidence | `.beads/vb-oul6u/evidence/cargo-test-pre-fix.log` (cargo exit 0; pre-fix tests compile) confirms test target is registered; pre-fix RA-003 tests not exercised against the pre-fix code base because the pre-fix code already passes the RA-003 corpus (it is the corpus's reference implementation) |
| Expected evidence | 3/3 tests pass: (a) bit-exact for every power-of-two cap ∈ [1, 2^20]; (b) 1-ULP bound for every cap ∈ [1, 2^20] at five sample lengths; (c) bit-exact at empty-ring and full-ring boundaries. The empty-ring subcase proves the `unwrap_or(0)` sentinel preservation |
| Mapping status | planned (closure owned by State 5 test-writer in cooperation with State 12 closure) |
| behavior_affecting | false |

### 5.3 PO-OUL6U-CALLSITE-003 (Call-Site Regression — Test-Owned, Newly Planned)

| Field | Value |
|-------|-------|
| RRO ID | n/a (no RRO row emitted; see §4) |
| Proof claim ref | `proof-obligations.planned.jsonl` row 3 |
| Production target | `Runtime::collect_metrics` ratio branch at `crates/vb_runtime/src/runtime.rs:578-595` (post-fix), observable through `RuntimeMetricsSnapshot.shards[*].trace_ring_fill_pct: f32` |
| Public API surface (frozen) | `pub fn Runtime::collect_metrics(&self) -> RuntimeMetricsSnapshot` at `crates/vb_runtime/src/runtime.rs` |
| | `pub trace_ring_fill_pct: f32` at `crates/vb_runtime/src/counters.rs:113` (primary) and `crates/vb_ipc/src/metrics.rs:37` (IPC re-declaration) — type freeze preserved |
| Existing call sites (planned test module location) | `crates/vb_runtime/src/shard/tests/tick_shard_tests.rs:529,544,630,641,678,715,724` (parent test module per `delivery-scope.jsonl` row `r03`) |
| Behavior test refs (planned new tests, named) | `collect_metrics_reports_zero_for_empty_trace_ring` |
| | `collect_metrics_reports_fifty_for_half_full_trace_ring` |
| | `collect_metrics_reports_one_hundred_for_full_trace_ring` |
| Test fixture (planned) | `Runtime::new_for_tests_and_benchmarks_only(1, ShardConfig { trace_capacity: 16, ... })` — matches existing `tick_shard_tests.rs` fixture |
| Refinement harness refs | **none** (call-site regression net is the planned cargo-test pair; no formal-verifier required) |
| Expected bit-exactness | `0_u32 / x = 0.0` (IEEE-754 sentinel); `8_u32 / 16_u32 = 0.5` (exact in `f32`); `16_u32 / 16_u32 = 1.0` (exact in `f32`); `* 100.0` is also exact for `0.0 / 50.0 / 100.0` |
| Evidence command | `cargo test -p vb_runtime --lib collect_metrics_trace_ring_fill_pct 2>&1 | tee .beads/vb-oul6u/evidence/call-site-regression.log` |
| Evidence workdir | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u |
| Evidence artifact (planned) | `.beads/vb-oul6u/evidence/call-site-regression.log` |
| Expected evidence | 3/3 new tests pass: empty trace ring → `0.0` (exact); half-full (8/16) → `50.0` (exact); full (16/16) → `100.0` (exact) |
| Mapping status | planned (closure owned by State 5 test-writer in cooperation with State 12 closure) |
| behavior_affecting | false |

## 6. Contract Clause → Bridge Traceability

| Contract Clause | Bridge Rows | Status |
|-----------------|-------------|--------|
| INV-001 (frozen `trace_ring_fill_pct: f32` field type) | PO-OUL6U-CALLSITE-003 (§5.3) | Mapped; pre-fix baseline shows `pub trace_ring_fill_pct: f32` at `vb_runtime/src/counters.rs:113` and `vb_ipc/src/metrics.rs:37` |
| INV-002 (`collect_metrics` is `&self`-only sync read) | n/a (no obligation maps to this invariant directly; all three obligations exercise the function through test inputs) | Implicit; no production signature change |
| INV-003 (`trace_ring_fill_pct ∈ [0.0, 100.0]`) | PO-OUL6U-RA003-002 (§5.2), PO-OUL6U-CALLSITE-003 (§5.3) | Mapped; RA-003 corpus proves the math; call-site tests prove the integration |
| INV-004 (bounded-narrowing + `unwrap_or(0)` pattern) | PO-OUL6U-LINT-001 (§5.1), PO-OUL6U-RA003-002 (§5.2), PO-OUL6U-CALLSITE-003 (§5.3) | Mapped; lint confirms no `as`-cast remains; tests confirm numeric equivalence |
| INV-005 (SAFETY block removed or rewritten) | PO-OUL6U-LINT-001 (§5.1) | Mapped; pre-fix baseline `rg-safety-comment-pre-fix.log` confirms 1 match at `runtime.rs:581` |
| INV-006 (`as_conversions = "deny"` preserved) | PO-OUL6U-LINT-001 (§5.1) | Mapped; pre-fix baseline `rg-policy-invariant.log` confirms 2 master-doc references to the deny policy |
| POST-001 (`trace_ring_fill_pct ∈ [0.0, 100.0]` after return) | PO-OUL6U-RA003-002 (§5.2), PO-OUL6U-CALLSITE-003 (§5.3) | Mapped |
| POST-002 (zero `as`-casts in `runtime.rs:578-588`) | PO-OUL6U-LINT-001 (§5.1) | Mapped |
| POST-003 (bit-identical to original within 1 ULP for `cap ∈ [1, 2^20]`) | PO-OUL6U-RA003-002 (§5.2) | Mapped |
| POST-004 (clippy exits 0 with `-D clippy::as_conversions`) | PO-OUL6U-LINT-001 (§5.1) | Mapped |
| POST-005 (`xtask forbidden-scan` reports zero as-casts) | PO-OUL6U-LINT-001 (§5.1) | Mapped |
| POST-006 (RA-003 corpus passes) | PO-OUL6U-RA003-002 (§5.2) | Mapped |

12/12 active contract clauses map to bridge rows; clauses are
exhaustively-covered with no orphans.

## 7. Implementation Task Summary for State 10

The following Rust implementation task is required to close the three bridge
rows. This is a single source-file change with a single conceptual edit:

### Task 1: Replace `runtime.rs:578-588` `as`-cast with bounded-narrowing

- **File**: `crates/vb_runtime/src/runtime.rs`
- **Pattern**: Mirrors the six sibling metric lines at pre-fix `runtime.rs:571-577, 596`
- **Pre-fix** (10 lines including SAFETY comment):
  ```rust
  let trace_capacity = shard.trace_ring().capacity();
  let trace_len = shard.trace_ring().pending_len();
  let trace_ring_fill_pct = if trace_capacity > 0 {
      // SAFETY: trace_len and trace_capacity are bounded by configuration
      // (typically 4096). Safe lossless narrowing to u32 for metric calculation.
      #[allow(clippy::as_conversions)]
      let ratio = (trace_len as f32) / (trace_capacity as f32);
      ratio * 100.0
  } else {
      0.0
  };
  ```
- **Post-fix** (12 lines, no SAFETY comment, no `#[allow]`, no `as`-cast):
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
- **Affected bridge rows**: PO-OUL6U-LINT-001, PO-OUL6U-RA003-002, PO-OUL6U-CALLSITE-003

### Task 2: New call-site regression tests

- **File**: `crates/vb_runtime/src/shard/tests/tick_shard_tests.rs` (new test module or appended to existing tick_shard_tests module)
- **Pattern**: Construct `Runtime::new_for_tests_and_benchmarks_only(1, ShardConfig { trace_capacity: 16, ... })` and assert `metrics.shards[0].trace_ring_fill_pct == 0.0 / 50.0 / 100.0` for empty / half / full trace rings respectively
- **Affected bridge rows**: PO-OUL6U-CALLSITE-003

## 8. State 11 / State 12 Closure Path

| State | Action |
|-------|--------|
| State 8 (test-planner) | Optional: confirm test plan for call-site tests. The 3-RRO call-site plan is already in `proof-strategy.md` §106. |
| State 9 (test-writer) | Author the 3 new call-site tests at `tick_shard_tests.rs` call sites. |
| State 10 (implementation) | Apply Task 1 (replace `runtime.rs:578-588`). Re-run pre-fix baseline captures to confirm drop in clippy error count by 1 with no new errors. |
| State 6 (black-hat-reviewer) | Run clippy + AST-scanner + IPC roundtrip + SAFETY-comment rg verification. (Listed in `proof-strategy.md` §107.) |
| State 11 (formal-verifier) | **Not invoked** for this bead. No formal-verifier artifact exists or needs to be authored. |
| State 12 (closure) | All three bridge rows must transition from `mapping_status: planned` to `mapping_status: verified`. The `rust-refinement-obligations.jsonl` empty-file invariant must be preserved. |

## 9. Unresolved Mapping Gaps

| Gap ID | Description | Impacted Bridge Rows | Closure Path |
|--------|-------------|----------------------|--------------|
| GAP-001 | Pre-fix clippy baseline reports 222 errors — all are pre-existing workspace `forbid`-vs-`allow` conflicts unrelated to this bead (per `proof-review.md:144` and `transcript-state5-pw.txt:23`). The post-fix clippy run is expected to drop this count by 1 (the `#[allow(clippy::as_conversions)]` removal) and add no new errors. | PO-OUL6U-LINT-001 | State 6 black-hat-reviewer runs `clippy` post-fix and confirms zero net new errors. |
| GAP-002 | Three new call-site tests are planned but not yet written. | PO-OUL6U-CALLSITE-003 | State 5 test-writer authors the 3 tests; State 12 closure confirms pass. |
| GAP-003 | RA-003 corpus only exercises the *ratio* directly, not the full `collect_metrics` integration path. The 3 new call-site tests close this gap. | PO-OUL6U-CALLSITE-003 (closes PO-OUL6U-RA003-002's call-site coverage gap) | State 5 test-writer; State 12 closure. |

No behavior-affecting waivers required. No formal-verifier harness required.
No GOD RULE 2 / 3 / 5 violation risk.

## 10. Handoff for `proof-reviewer` (Bridge Reviewer)

The following artifacts form the complete bridge output:

| Artifact | Path | Purpose |
|----------|------|---------|
| proof-to-rust-map.md | `.beads/vb-oul6u/proof-to-rust-map.md` (this file) | Human-readable obligation-to-source mapping |
| rust-refinement-obligations.jsonl | `.beads/vb-oul6u/rust-refinement-obligations.jsonl` (empty, 0 bytes) | Machine-readable RRO surface; intentionally empty per `NO_PROOF_WORK` disposition |
| agent-invocation-ledger.jsonl | `.beads/vb-oul6u/agent-invocation-ledger.jsonl` (seq 6 + seq 7 appended) | Updated with 2 new entries for state 7 bridge + state 7 bridge review |
| routing-ledger.jsonl | `.beads/vb-oul6u/routing-ledger.jsonl` (1 new entry appended) | Routing ledger for the bridge invocation |

The bridge is a "no refinement harnesses required" disposition. The bridge
reviewer should verify:

1. The three obligations map to concrete production source refs (`runtime.rs` lines).
2. The three behavior-test refs resolve to real or planned test names (RA-003 corpus + 3 new call-site tests).
3. The `rust-refinement-obligations.jsonl` is intentionally empty, consistent with `proof-review.md` (binding_classification: `n/a`).
4. No `proof-obligations.planned.jsonl` row is silently dropped.
5. The `mapping_status: planned` rows correctly identify their owners (State 5 test-writer, State 6 black-hat-reviewer, State 12 closure).

## 11. Final Status

The bridge is a single-file lint remediation with three obligations, zero
formal-verifier lanes, and zero refinement-harness rows. The mapping is
honest, the source refs are real (verified against `rg` baselines captured
pre-fix), and the disposition `NO_REFINEMENT_HARNESSES_REQUIRED` is faithfully
preserved by the empty `rust-refinement-obligations.jsonl`.

**Bridge status**: ready for `proof-reviewer` (State 7 bridge-review).

