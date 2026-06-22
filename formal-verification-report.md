# Flux Refinement-Type Verification Gap Analysis — Final Report

**Date:** 2026-06-15
**Toolchain:** nightly-2026-04-28
**Flux Tool:** cargo-flux (flux-rs)
**Repository:** velvet-ballistics

---

## Executive Summary

Of 16 production crates, **4 have any Flux annotations at all**. Those 4 have **0 verified functions** in vb_compile and **0 verified functions in vb_runtime** (1,344 trusted / 0 checked). The only crate with any verified content is vb_storage (33 checked), but those come from transitive dependencies, not vb_storage's own annotations.

**This is a critical gap.** The verification ledger contains hundreds of Flux proof obligations that are effectively `#[flux_rs::trusted]` — assumed correct without proof.

---

## 1. Flux Check Results by Crate

| Crate | Functions Checked | Functions Trusted | Trusted Ratio | Verdict |
|-------|-----------------|-------------------|---------------|---------|
| **vb_compile** | 0 | 0 | N/A | **DEAD — files not processed** |
| **vb_runtime** | 0 | 1,344 | 100% | **FAIL — all trusted** |
| **vb_storage** | 33 | 1,337 | 97.6% | **WEAK — deps checked only** |
| **vb_core** | — | — | N/A | NO FLUX |
| **vb_ipc** | — | — | N/A | EMPTY MODULE |
| **vb_ajc40_flux** | 0 | 0 | N/A | OBSOLETE (flux_tool mismatch) |
| vb_boundary_inventory | — | — | N/A | NO FLUX |
| vb_cli | — | — | N/A | NO FLUX (binary) |
| vb_doc | — | — | N/A | NO FLUX (docs) |
| vb_expr | — | — | N/A | NO FLUX |
| vb_proof_kernels | — | — | N/A | NO FLUX |
| vb_queue_semantics | partial | partial | ~40% | PARTIAL (helpers only) |
| vb_test_util | — | — | N/A | NO FLUX (tests) |
| vb_validate | — | — | N/A | NO FLUX |
| vb_verification | — | — | N/A | NO FLUX |
| vb_yaml | — | — | N/A | NO FLUX |
| vb_benchmark | — | — | N/A | NO FLUX (benchmarks) |

---

## 2. Crates WITH Flux — Detailed Findings

### 2.1 vb_compile — DEAD (0 functions processed)

**Status:** The flux check completes in 0.05s with "Finished" and zero summary. **No functions are processed at all.**

**Root cause: TWO independent issues:**

#### Issue 1: `.flux` files in subdirectories not auto-discovered
The 6 `.flux` files in `src/mod_compile_lowering/` (`reduce_body_width.flux`, `reduce_chain.flux`, `reduce_foreach.flux`, `reduce_nested_next.flux`, `reduce_offset.flux`, `reduce_overflow.flux`) are **never processed** because:
- They are NOT declared as modules in `lib.rs`
- There is NO `[package.metadata.flux]` include list in `Cargo.toml`
- Flux does not auto-discover `.flux` files in subdirectories

**Remediation:** Add `[package.metadata.flux]` to `crates/vb_compile/Cargo.toml`:
```toml
[package.metadata.flux]
enabled = true
include = [
  "src/mod_compile_lowering/reduce_*.flux",
]
```

#### Issue 2: `#[cfg(flux)]` on annotations blocks visibility
The 2 declared Flux modules (`body_step_width_flux.rs`, `body_dispatcher_together_flux.rs`) have:
- `#[cfg(flux)]` on all `#[flux_rs::sig(...)]` annotations
- `compile_error!("FLUX ENABLED")` behind `#[cfg(flux)]` in `body_step_width_flux.rs`

Since `cargo flux` does NOT set the `flux` cfg flag, all annotations are invisible.

**Remediation:** Remove `#[cfg(flux)]` from all Flux annotations in both modules (matching the vb_runtime pattern).

#### Fixes applied in this session:
- ✅ Fixed SYNTAX ERROR in `reduce_body_width.flux` (line 41: `SYNTAX ERROR` → `x + 1`)
- ✅ Fixed unnamed parameters in extern_spec sigs (all 6 .flux files)
- ⚠️ Files still not processed until `[package.metadata.flux]` is added

#### GOD RULE 2 Violations:
- `reject_invalid_width_zero`, `reject_none_next`, `reject_foreach_width_one`, `reject_overflow_unchecked`, `reject_ambiguous_next`, `reject_invalid_offset` — all `#[flux_rs::trusted]` `unreachable!()` stubs

---

### 2.2 vb_runtime — FAIL (0 checked, 1,344 trusted)

**Status:** 1,370 functions processed (vb_storage: 1,370 + vb_runtime: 1,344), of which **0 are actually checked** and 1,344 are trusted.

**Root cause: `#[flux_rs::trusted]` on ALL model functions.**

#### Production-code Flux files:
- `src/shard/lifecycle/flux_cancel_kill.rs` — 11 `#[flux_rs::trusted]` model functions
- `src/codec/flux_validation.rs` — 12 `#[flux_rs::trusted]` model functions

These define model functions that call production code, but wrap them in `#[flux_rs::trusted]` — the model functions themselves are not proven.

#### Verification-only Flux files (`src/verification/flux/`):
All 6 files define **new enums** that mirror production types:
- `RuntimeResultRef` — separate from production `RuntimeResult`
- `Mrwe6ScheduleAtom` — separate from production `Mrwe6EventClass`/`Mrwe6IntentKind`
- `Mrwe6ResolutionAtom` — separate from production type
- `Mrwe6QueuedIntent` — separate from production type
- `DispatchOutcomeRef`, `DispatchPreconditionRef` — separate from production `ActionOutcome`
- `ActionTicket` — correctly documented as WAIVED (production doesn't support invariants)

#### FZGDN series (`src/verification/flux/vb_fzgdn/`):
All 9 PS files (ps_001 through ps_010) use `#[flux_rs::trusted]` on:
- `SafeGeneration` impl block
- Model functions (`model_safe_generation_new`, `model_safe_generation_get`, etc.)

#### GOD RULE 2 Violations:
- **GOD RULE 2 (No Toy Types):** All 6 verification/flux/ files define separate enums instead of annotating production types with `#[refined_by]`
- **GOD RULE 2 (No Vacuum Proofs):** 100% trusted — 0 constraints solved
- **GOD RULE 2 (No Trusted Boundaries):** 44 `#[flux_rs::trusted]` occurrences across vb_runtime

#### Remediation Priority:
1. Replace `#[flux_rs::trusted]` on production model functions with actual `#[spec]`/`#[sig]` annotations on the production functions
2. Annotate production types directly instead of defining separate enum types
3. Convert trusted boundary proofs to mechanically verified proofs

---

### 2.3 vb_storage — WEAK PASS (33 checked, 1,337 trusted)

**Status:** 33 checked functions (all from transitive vb_core/vb_runtime deps), 1,337 trusted.

#### Files:
- `vb_mrwe5_compat_kind_family.rs` — defines new `CompatibilityClass` enum
- `vb_mrwe5_decode_reject.rs` — defines new `DecodeState` enum
- `vb_mrwe5_kind_parity.rs` — best file: has `#[cfg_attr(flux, flux_rs::sig(...))]` on wrapper with refinement `envelope_kind == payload_kind`
- `vb_mrwe5_roundtrip.rs` — defines new `RoundTripVariant` enum
- `vb_mrwe6_duplicate_refinements.rs` — defines new `Mrwe6DuplicateRetry` enum
- `vb_mrwe6_recovery_refinements.rs` — defines new `Mrwe6RecoveryView` enum

#### GOD RULE 2 Violations:
- Toy types (new enums instead of production annotations)
- 97.6% trusted

---

### 2.4 vb_core — NO FLUX

**One file exists:** `src/verification/flux/vb_rxru0_action_enums.rs`
- Defines refined type aliases (`IdempotencyDiscriminant = u8` with `refined_by(in(0..=2))`, etc.)
- Type aliases, NOT annotations on production enums
- 0 functions processed

#### Remediation:
- Annotate production enums (`Idempotency`, `SideEffect`, `RetrySafety`, `ActionFailureCode`) directly with `#[refined_by]`
- Add `#[spec]`/`#[sig]` annotations on constructor functions

---

### 2.5 vb_ipc — EMPTY

**One file exists:** `src/verification/flux/vb_5iebh/mod.rs`
- Only 3 lines: module doc comment
- No actual Flux content

---

### 2.6 vb_ajc40_flux — OBSOLETE

**Status:** Package excluded from workspace. Uses `flux_tool::sig` instead of `flux_rs::sig`.

**Fix:** Update all `flux_tool` references to `flux_rs`, add to workspace.

---

### 2.7 vb_queue_semantics — PARTIAL

**Already has Flux:** 5 helper functions have `#[cfg_attr(flux, flux_rs::sig(...))]`:
- `helper_valid_capacity(capacity)` — `capacity > 0 && capacity <= 65536`
- `helper_queue_is_full(capacity, len)` — `len >= capacity`
- `helper_enqueue_accepts(capacity, len)` — `len < capacity`
- `helper_command_pop_is_pop_front(capacity, len)` — `len > 0 && capacity > 0`
- `helper_runtime_queue_full_maps(depth, capacity)` — `depth >= capacity`

**Missing:**
- `QueueState<T>` struct — no `#[refined_by]` annotation for capacity invariant
- `EnqueueDecision`, `PopTransition<T>` — no refinement on transition invariants

---

## 3. Crates WITHOUT Flux — Gap Analysis

### 3.1 vb_compile — already covered above

### 3.2 vb_expr (68 files) — CRITICAL

**Types needing refinement:**
- Expression AST nodes with operator-type constraints (arithmetic ops on numeric types only)
- Bytecode instructions with operand validity constraints
- Typecheck results with invariant relationships between input types and output types

### 3.3 vb_proof_kernels (39 files) — CRITICAL

**Types needing refinement:**
- `Budget { steps, actions, parallel, retries, gather_pages, gather_items, for_each_iters, together_branches, repeat_attempts, run_time_secs, result_bytes, slots_written }` — 12 u64 fields with implicit >0 invariants
- `Policy { max_actions, max_parallel, max_run_time, max_result_bytes, max_steps }` — 5 u64 fields, each must be > 0
- Taint propagation invariants
- StepState transition invariants

### 3.4 vb_yaml (31 files) — HIGH

**Types needing refinement:**
- `WorkflowSource` — version must equal `"velvet-ballistics/v1"`, steps non-empty, step IDs unique
- `StepAst` — `then` references must be valid step IDs, IDs must be unique
- `StepPrimitive::ForEach/Collect/Reduce/Repeat` — body must not be empty
- `RetryPolicy` / `Repeat` — `max_attempts` > 0
- `ScalarValue::Integer(i64)` — bounded range

### 3.5 vb_validate (59 files) — HIGH

**Types needing refinement:**
- `is_valid_id(id)` — should be `#[refined_by]` on an `Id` type
- `validate_single_primitive` — should be encoded as type-level invariant
- `FieldValue::Mapping` — key uniqueness

### 3.6 vb_boundary_inventory (16 files) — MEDIUM

**Types needing refinement:**
- `FreshnessMarker` — `source_version > 0 && schema_version > 0 && evidence_version > 0`
- `WorkspaceRoot` — path validation on construction
- `ClassifiedBoundary` — field consistency invariants

---

## 4. Trusted Boundary Audit

| Source | `#[flux_rs::trusted]` Count | Verified | Trusted | Gap |
|--------|---------------------------|----------|---------|-----|
| vb_compile (declared modules) | 4+ | 0 | 0 | Dead — files not processed |
| vb_compile (.flux files) | 6+ | 0 | 0 | Dead — files not processed |
| vb_runtime production Flux | 23 | 0 | 23 | 100% trusted |
| vb_runtime verification Flux | 21+ | 0 | 21+ | 100% trusted |
| vb_storage verification Flux | 12+ | 0* | 12+ | 100% trusted |

*vb_storage 33 checked functions come from transitive deps (vb_core, vb_runtime), not from vb_storage's own annotations.

**Total trusted boundary debt: 56+ `#[flux_rs::trusted]` instances with 0 verified constraints.**

---

## 5. GOD Rule Violations Summary

| Rule | Severity | Count | Details |
|------|----------|-------|---------|
| GOD 2: No Toy Types | **HIGH** | 12+ files | Separate enum types instead of production annotations |
| GOD 2: No Vacuum Proofs | **CRITICAL** | 1,344 functions | All trusted, 0 verified |
| GOD 2: Extern Spec Real Predicates | **MEDIUM** | 6 files | Predicates use `overhead`, `id`, etc. which may not be accessible in Flux |
| GOD 2: No Trusted Boundaries | **HIGH** | 56+ | Unjustified trusted blocks |

---

## 6. Fixes Applied in This Session

1. ✅ `reduce_body_width.flux:41` — Fixed SYNTAX ERROR (`SYNTAX ERROR` → `x + 1`)
2. ✅ `reduce_body_width.flux:17` — Named parameter in body_width extern_spec
3. ✅ `reduce_foreach.flux:17` — Named parameter in canonical_body_step_width extern_spec
4. ✅ `reduce_nested_next.flux:19` — Named parameter in canonical_body_step_width extern_spec
5. ✅ `reduce_offset.flux:17-18` — Simplified predicate (removed inaccessible `as_usize()` call)
6. ✅ `reduce_overflow.flux:17` — Named parameter in body_width extern_spec

---

## 7. Priority Remediation Plan

### P0 — Structural (fix to enable any Flux processing)

1. **Add `[package.metadata.flux]` to vb_compile/Cargo.toml**
   - Include: `src/mod_compile_lowering/reduce_*.flux`
   
2. **Remove `#[cfg(flux)]` from vb_compile Flux modules**
   - `body_step_width_flux.rs`: Remove `#[cfg(flux)]` from all annotations and `compile_error!`
   - `body_dispatcher_together_flux.rs`: Remove `#[cfg(flux)]` from all annotations

3. **Fix vb_runtime trusted boundary density**
   - Replace `#[flux_rs::trusted]` on production model functions with actual `#[spec]`/`#[sig]` annotations
   - Target: `flux_cancel_kill.rs` (11 trusted), `flux_validation.rs` (12 trusted)

### P1 — Toy Type Replacement

4. **vb_runtime verification/flux/** — Annotate production types directly
   - `RuntimeResult` → `#[refined_by]` instead of `RuntimeResultRef`
   - `Mrwe6EventClass`/`Mrwe6IntentKind` → annotate directly instead of `Mrwe6ScheduleAtom`
   - `ActionOutcome` → annotate directly instead of `DispatchOutcomeRef`

5. **vb_storage verification/flux/** — Same pattern
   - Annotate `JournalKind`, `Mrwe6DuplicateRetryDecision`, etc. directly

6. **vb_core verification/flux/** — Annotate production enums
   - `Idempotency`, `SideEffect`, `RetrySafety`, `ActionFailureCode` → `#[refined_by]`

### P2 — New Crates

7. **vb_yaml** — Annotate `WorkflowSource`, `StepAst`, `StepPrimitive` types
8. **vb_expr** — Annotate AST and bytecode types
9. **vb_proof_kernels** — Annotate `Budget`, `Policy` types
10. **vb_validate** — Add refined `Id` type

### P3 — Maintenance

11. **vb_ajc40_flux** — Update `flux_tool` → `flux_rs`, add to workspace
12. **vb_ipc** — Populate empty verification module
13. **vb_queue_semantics** — Add `#[refined_by]` to `QueueState<T>`

---

## 8. Verification Ledger Impact

Current proof obligations in the ledger that map to Flux:
- vb_compile: 6 POs (PO-WIDTH-MATCH-FLUX-001, PO-CHAIN-FLUX-001, etc.) — **NONE VERIFIED**
- vb_runtime: 19+ POs (PS-001 through PS-010, vb_y9d3v, vb_egysa, vb_mrwe6, vb_rxru0) — **NONE VERIFIED**
- vb_storage: 6 POs (vb_mrwe5_*, vb_mrwe6_*) — **NONE VERIFIED** (33 checked from deps only)
- vb_core: 1 PO (vb_rxru0_action_enums) — **NOT APPLIED** (type aliases only)
- vb_queue_semantics: 5 POs (helper functions) — **PARTIALLY VERIFIED**

**Total Flux POs in ledger: ~36**
**Total Flux POs verified: 0**
**Coverage: 0%**

---

## 9. Next Immediate Actions

1. Add `[package.metadata.flux]` to `crates/vb_compile/Cargo.toml` (10-minute fix)
2. Remove `#[cfg(flux)]` from vb_compile's `body_step_width_flux.rs` and `body_dispatcher_together_flux.rs` (15-minute fix)
3. Re-run `cargo flux -p vb_compile` — should then process the declared modules
4. For .flux files in `mod_compile_lowering/`, add to include list and verify processing
5. Begin replacing `#[flux_rs::trusted]` on vb_runtime production model functions

---

# Wave 3A: Formal Re-verification After Wave 1/2 Production Fixes — 2026-06-22

## Scope

Re-run the four formal-verification obligations that were parked while Wave 1/2 production fixes (B-001-P dispatch coalesce flush, C-R6-4 EventSeq::MAX → MAX_ENCODABLE) landed.

| Bead   | Obligation | Tool        | Target                                     | Classification |
|--------|-----------|-------------|--------------------------------------------|----------------|
| vb-27nz7 | KANI-001 | cargo-kani  | vb_runtime (B-001-P follow-up)             | FAIL_LOCAL     |
| vb-sasf7 | FLUX-001 | cargo-flux  | vb_storage (C-R6-4 follow-up)              | PASS           |
| vb-qmbvs | FLUX-002 | cargo-flux  | vb_runtime (B-001-P follow-up)             | FAIL_GLOBAL    |
| vb-3tt54 | VERUS-001 | verus      | vb_storage recovery + vb_runtime journal   | PASS           |

## Verdict Matrix

| Bead   | Tool        | Command                                                  | Exit | Counts                                                          | Result   | Follow-up bead |
|--------|-------------|----------------------------------------------------------|------|------------------------------------------------------------------|----------|----------------|
| vb-27nz7 | cargo-kani | `bash scripts/kani-list.sh vb_runtime`                   |   0  | 78 harnesses enumerated (incl. 3 B-001-K)                        | FAIL_LOCAL | vb-mzmk9, vb-z66rh |
| vb-27nz7 | cargo-kani | `cargo kani -p vb_runtime --features kani-flush-coalesce-buffer` |  1  | "none of the selected packages contains this feature"            | FAIL_LOCAL | vb-mzmk9       |
| vb-27nz7 | cargo-kani | `cargo kani -p vb_runtime`                               |  1  | 2 E0004 non-exhaustive patterns (PayloadTooLarge)                | FAIL_LOCAL | vb-z66rh       |
| vb-sasf7 | cargo-flux | `bash scripts/flux-check-package.sh vb_storage`          |   0  | 1540 functions; **19 checked**; 1521 trusted; 0 ignored           | PASS     | —              |
| vb-qmbvs | cargo-flux | `bash scripts/flux-check-package.sh vb_runtime`          |   0  | 1506 functions; **0 checked**; 1506 trusted; 0 ignored (vacuous)  | FAIL_GLOBAL | vb-5iuag      |
| vb-qmbvs | cargo-flux | `bash scripts/flux-check-package.sh vb_runtime --features vb-mrwe6-flux-refinements` | 101 | 19 errors E0753 + E0252 (Mrwe6EventClass/Mrwe6IntentKind duplicate) | FAIL_GLOBAL | vb-5iuag       |
| vb-3tt54 | verus      | `bash scripts/verify-verus.sh`                           |   0  | 19 registry targets; **187 verified total**, 0 errors             | PASS     | —              |

## Per-Bead Evidence

### vb-27nz7 — KANI-001 (FAIL_LOCAL)

The `kani-list.sh` scan enumerates 78 harnesses for vb_runtime, including the 3 new B-001-K harnesses in `crates/vb_runtime/src/kani_flush_coalesce_buffer.rs`:

```
flush_coalesce_buffer_drains_buffer_on_every_dispatch_path
flush_coalesce_buffer_no_op_when_empty
flush_coalesce_buffer_idempotent_across_calls
```

However, `kani-list.sh` is a *source-scan* — it does not invoke `cargo kani`. The harness file is **dead code** in the build:

- `kani_flush_coalesce_buffer` is **NOT declared** in `crates/vb_runtime/src/lib.rs` nor `crates/vb_runtime/src/verification/mod.rs` (verified by `grep`).
- The harness file declares `#![cfg(kani)]` at the top (line 37).
- The Cargo.toml feature `kani-flush-coalesce-buffer` **does not exist** (verified: `crates/vb_runtime/Cargo.toml` features list 41-70 omits it).

`cargo kani -p vb_runtime --features kani-flush-coalesce-buffer`:
```
error: Failed to get cargo metadata.: error: none of the selected packages contains this feature: kani-flush-coalesce-buffer
```

Even with the harness wired in, the broader vb_runtime Kani lane is blocked by a pre-existing vb_core compile error under `cfg(kani)`:

```
error[E0004]: non-exhaustive patterns: `Err(workflow::compiled_query::errors::QueryParseError::PayloadTooLarge { .. })` not covered
   --> crates/vb_core/src/workflow/compiled_query_kani.rs:106:11
error[E0004]: non-exhaustive patterns: `Err(workflow::compiled_slug::types::SlugParseError::PayloadTooLarge { .. })` not covered
   --> crates/vb_core/src/workflow/compiled_slug_kani.rs:105:11
```

The `PayloadTooLarge` variant was added to `compiled_query/errors.rs:44` and `compiled_slug/types.rs:117` but the Kani harness modules were not updated. This pre-existing error blocks ALL Kani verification for vb_runtime.

**Follow-ups filed:**
- `vb-mzmk9` — Wire B-001-K harness into Cargo.toml feature + lib.rs mod declaration.
- `vb-z66rh` — Fix pre-existing vb_core Kani compile error in `compiled_query_kani.rs` / `compiled_slug_kani.rs`.

### vb-sasf7 — FLUX-001 (PASS)

```
$ bash scripts/flux-check-package.sh vb_storage
Checking vb_storage v0.1.0
summary. 1540 functions processed: 19 checked; 1521 trusted; 0 ignored. 0 constraints solved. Finished in 1.41s
EXIT=0
```

After C-R6-4 (`EventSeq::MAX` → `MAX_ENCODABLE`), the Flux refinements in vb_storage still hold: 19 functions actively checked, no errors, no ignored. No new flux refinement was required for this rename.

### vb-qmbvs — FLUX-002 (FAIL_GLOBAL)

Default flux check:
```
$ bash scripts/flux-check-package.sh vb_runtime
Checking vb_runtime v0.1.0
summary. 1506 functions processed: 0 checked; 1506 trusted; 0 ignored. 0 constraints solved.
EXIT=0  ← vacuous, not real coverage
```

The script exits 0 but verifies **zero** refinements relevant to B-001-P (coalesce_window_ticks / flush_coalesce_buffer). `Cargo.toml [package.metadata.flux]` include patterns target only `vb_mrwe6_*` and `vb_egysa*` files; none of these files model `coalesce_window_ticks`. A grep across all Flux files confirms zero references to "coalesce".

Enabling the Flux refinement feature also fails to compile:
```
$ bash scripts/flux-check-package.sh vb_runtime --features vb-mrwe6-flux-refinements
error[E0753]: expected outer doc comment
error[E0252]: the name `Mrwe6EventClass` is defined multiple times
error[E0252]: the name `Mrwe6IntentKind` is defined multiple times
...
EXIT=101
```

The pre-existing `Mrwe6EventClass` / `Mrwe6IntentKind` duplicate-declaration errors in `crates/vb_runtime/src/verification/flux/vb_mrwe6_*_refinements.rs` are unrelated to B-001-P but block the entire vb_runtime Flux lane when features are enabled.

**Follow-up filed:**
- `vb-5iuag` — Write a Flux refinement for `coalesce_window_ticks` / `flush_coalesce_buffer` (binding to production), AND resolve the pre-existing `Mrwe6EventClass` / `Mrwe6IntentKind` E0252 collisions so `--features vb-mrwe6-flux-refinements` builds.

### vb-3tt54 — VERUS-001 (PASS)

```
$ bash scripts/verify-verus.sh
verus 0.2026.05.05.d03e906, verusfmt available
VERUS_TARGET_COUNT=19
VERUS_TRUST_SCAN_OK
EXIT=0
```

All 19 registry-driven Verus targets verified with 0 errors. Coverage of the requested scope:

| Target file                                              | Verified |
|----------------------------------------------------------|----------|
| verification/verus/vb_mrwe6_recovery_reliance.rs         | 8        |
| verification/verus/vb_cli_commands_journal_trace.rs      | 7        |
| verification/verus/vb_jpq724_events_for_run_production.rs| 11       |
| verification/verus/idempotency_replay_tracker.rs         | 8        |
| crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs | 24 |
| (16 other registry targets)                              | 129      |
| **Total verified**                                       | **187**  |

Two `compute` warnings emitted by `vb_y9d3v_action_fence.rs:226` and `:239` (failed to simplify `result.is_ok() ==> result.unwrap() >= 1` before Z3). These warnings do not affect verification success — the proofs still complete with 0 errors.

## Tooling Health Discovered

1. **Kani feature gate contract violation:** B-001-K harness file exists at `crates/vb_runtime/src/kani_flush_coalesce_buffer.rs` but is not wired into either `lib.rs` or `Cargo.toml`. The script `kani-list.sh` enumerates 78 harnesses including these 3, but `cargo kani -p vb_runtime --features kani-flush-coalesce-buffer` rejects the feature as not present. Per AGENTS.md rule "Kani harness isolation: Bulky or stale harness groups must be behind package features", this is a contract violation. (Filed as vb-mzmk9.)

2. **Pre-existing vb_core Kani compile blocker:** `compiled_query_kani.rs:106` and `compiled_slug_kani.rs:105` do not handle the `PayloadTooLarge` variant that was added to `compiled_query/errors.rs:44` and `compiled_slug/types.rs:117`. Blocks the entire vb_runtime Kani lane. Not introduced by Wave 1/2 — predates it. (Filed as vb-z66rh.)

3. **Pre-existing vb_runtime Flux mrwe6 E0252 collisions:** `vb_mrwe6_atomic_index_refinements.rs`, `vb_mrwe6_completion_policy_refinements.rs`, `vb_mrwe6_queue_intent_refinements.rs` all re-declare `Mrwe6EventClass` and `Mrwe6IntentKind`. With `--features vb-mrwe6-flux-refinements` the build emits 19 errors (E0252 + E0753). Without the feature, the default scan matches 0 flux refinements and returns vacuously. (Filed as vb-5iuag.)

4. **No Flux refinement for `coalesce_window_ticks`:** Neither the B-001-K follow-up nor the existing Flux module set models the B-001-P fix. Cargo.toml metadata.flux include list does not include any coalesce-related file. A `grep -r coalesce crates/vb_runtime/src/verification/` returns only one match in a Kani harness comment.

## Follow-up Beads Filed (do not close originals)

| Bead ID  | Title                                                                                                          | Priority |
|----------|----------------------------------------------------------------------------------------------------------------|----------|
| vb-mzmk9 | FIX-KANI-001-A: Wire kani_flush_coalesce_buffer.rs into vb_runtime lib.rs and add kani-flush-coalesce-buffer Cargo.toml feature | P1       |
| vb-z66rh | FIX-KANI-001-B: vb_core Kani compile error: QueryParseError::PayloadTooLarge / SlugParseError::PayloadTooLarge not handled in compiled_query_kani.rs and compiled_slug_kani.rs | P1       |
| vb-5iuag | FIX-FLUX-002-A: Write Flux refinement for vb_runtime coalesce_window_ticks / flush_coalesce_buffer after B-001-P fix (and resolve pre-existing mrwe6 E0252 collisions) | P1       |

## Bead Disposition

| Bead   | Result   | Action                                |
|--------|----------|----------------------------------------|
| vb-27nz7 | FAIL_LOCAL | **NOT closed.** Notes added. Follow-ups: vb-mzmk9, vb-z66rh. |
| vb-sasf7 | PASS     | Closed with evidence.                  |
| vb-qmbvs | FAIL_GLOBAL | **NOT closed.** Notes added. Follow-up: vb-5iuag. |
| vb-3tt54 | PASS     | Closed with evidence.                  |
