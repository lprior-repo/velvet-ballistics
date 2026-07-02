# Proof Review — vb-edvbj

**Bead:** vb-edvbj — Runtime: delete fallback that maps unmapped journal events to run failure (P0 bug)
**STRONG-coupled with:** vb-cib14 (must land together)
**Reviewer skill:** proof-reviewer
**Review state:** proof-review
**Review state number:** State 6
**Date:** 2026-07-01
**Reviewer invocation ID:** proof-reviewer-vb-edvbj-state6
**Workdir:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-edvbj`

---

## 1. Provenance

| Field | Value |
|-------|-------|
| `reviewer_skill` | proof-reviewer |
| `reviewer_invocation_id` | proof-reviewer-vb-edvbj-state6 |
| `review_state` | proof-review |
| Independent reviewer (vs writer invocation) | Yes — `proof-reviewer-vb-edvbj-state6` ≠ `proof-writer-vb-edvbj-state5` (verified via `agent-invocation-ledger.jsonl` row 4) |
| Plan-reviewer invocation | `proof-plan-reviewer-vb-edvbj-state4b-p4b` (row 3, schema-valid) |
| `proof-writer-report.md` present | Yes (253 lines) |
| `proof-evidence.md` present | Yes (317 lines) |
| `trusted-base-ledger.jsonl` present | Yes (10 rows, all `acknowledged`) |
| `proof-plan-review.md` present | Yes (172 lines) |

The reviewer invocation is independent of the writer invocation; the writer and reviewer are not the same agent identity. Self-approval is rejected.

## 2. Reviewed Artifacts

| # | Artifact | Path | Classification |
|---|----------|------|----------------|
| 1 | Verus spec | `verification/verus/vb_edvbj_storage_event.rs` | WEAK (extern companion) |
| 2 | Verus extern | `verification/verus/extern_vb_edvbj_storage_event.rs` | WEAK (extern companion) |
| 3 | Verus production_inner mirror | `verification/verus/production_inner/vb_edvbj_storage_event_production.rs` | WEAK (mirror body) |
| 4 | Verus spec | `verification/verus/vb_edvbj_propagation.rs` | WEAK (extern companion) |
| 5 | Verus extern | `verification/verus/extern_vb_edvbj_propagation.rs` | WEAK (extern companion) |
| 6 | Verus production_inner mirror | `verification/verus/production_inner/vb_edvbj_propagation_production.rs` | WEAK (mirror body) |
| 7 | Verus spec | `verification/verus/vb_edvbj_symbolic_code.rs` | WEAK (extern companion) |
| 8 | Verus extern | `verification/verus/extern_vb_edvbj_symbolic_code.rs` | WEAK (extern companion) |
| 9 | Verus production_inner mirror | `verification/verus/production_inner/vb_edvbj_symbolic_code_production.rs` | WEAK (mirror body) |
| 10 | Verus spec (drift gate) | `verification/verus/vb_edvbj_mirror_bind.rs` | WEAK (mirror, drift gate) |
| 11 | Kani harness file (6 harnesses) | `crates/vb_runtime/src/kani_vb_edvbj_storage_event_no_fabricate.rs` | non-vacuous for per-layer helpers; H4 dispatcher has only `cover!` reachability |
| 12 | Kani harness file (2 harnesses) | `crates/vb_runtime/src/kani_vb_edvbj_propagation_strict_gate.rs` | non-vacuous |
| 13 | proptest (10k cases) | `crates/vb_runtime/src/journal/tests/proptest_vb_edvbj_all_21_variants.rs` | PENDING_FORMAL_EXECUTION (vb-cib14 dep) |
| 14 | proptest (1k cases) | `crates/vb_runtime/src/journal/tests/proptest_vb_edvbj_resumed_replay.rs` | PENDING_FORMAL_EXECUTION (vb-cib14 dep) |
| 15 | proptest (10k cases) | `crates/vb_runtime/src/error/tests_diagnostics/proptest_vb_edvbj_diagnostic_code.rs` | PENDING_FORMAL_EXECUTION (vb-cib14 dep) |
| 16 | Flux refinement | `crates/vb_runtime/src/verification/flux/vb_edvbj_diagnostic_code_refinement.rs` | broad `#[trusted]` markers (deviation from `#[extern_spec]` pattern) |

Total artifacts reviewed: **16** (the task description names "12 new proof artifacts"; the actual count after Verus spec/extern/mirror decomposition is 16). All are present, schema-valid, and consistent with the proof-writer-report.md's ownership list.

## 3. Schema Validation

- All 16 artifact files exist on disk with non-trivial content (≥1.3K, smallest is the mirror with 1.3K).
- `proof-writer-report.md` is structurally valid (253 lines, 11 sections, contains obligation matrix and command evidence table).
- `proof-evidence.md` is structurally valid (317 lines, 9 sections, contains raw verifier output).
- `trusted-base-ledger.jsonl` parses as JSONL with 10 rows, each `schema_version = trusted-base-ledger/v1`.
- `proof-plan-review.md` is structurally valid (172 lines, STATUS: APPROVED with explicit findings).

No structural issues.

## 4. Production-Binding Audit (MANDATORY Verus gate)

**Raw `bash scripts/check-verus-production-binding.sh <workdir>` output (rerun 2026-07-01):**
```
================================================================
  Verus production-binding audit (ABSOLUTE — no exceptions)
================================================================
  STRONG (direct crates/ binding): 0
  WEAK (production_inner/ mirror): 75
  VACUUM (no production binding):  0
```

**Script-classified counts:** 0 STRONG, 75 WEAK (which includes the 4 new vb-edvbj specs plus 71 pre-existing WEAK specs), 0 VACUUM.

### 4.1 Honest classification for the 4 vb-edvbj Verus specs

| Obligation | Spec file | Binding mechanism | Script class | Honest count |
|------------|-----------|-------------------|--------------|--------------|
| PO-EDVBJ-001-VERUS | `vb_edvbj_storage_event.rs` | `#[path = "extern_*.rs"]` (spec) → extern → `#[path = "production_inner/*"]` | WEAK (extern companion) | WEAK |
| PO-EDVBJ-005-VERUS | `vb_edvbj_propagation.rs` | same pattern | WEAK (extern companion) | WEAK |
| PO-EDVBJ-007-VERUS | `vb_edvbj_mirror_bind.rs` | direct `#[path = "production_inner/storage_kind_family_production.rs"]` | WEAK (mirror) | WEAK |
| PO-EDVBJ-009-VERUS | `vb_edvbj_symbolic_code.rs` | `#[path = "extern_*.rs"]` → extern → `#[path = "production_inner/*"]` | WEAK (extern companion) | WEAK |

**Honest tally for vb-edvbj:** **0 STRONG, 4 WEAK (1 mirror + 3 extern companions), 0 VACUUM.**

The proof-writer-report (line 47-51) labels the same table as "STRONG-shaped spec → extern → production_inner/". **This label is misleading.** Per the binding script, the specs are WEAK, not STRONG. The writer honestly acknowledged this in §4 of the report ("Honest classification (script-confirmed): all four Verus specs are classified as WEAK_MIRROR by the script") but the per-file labeling in §3.1 still calls them "STRONG-shaped", which is at odds with the script. This is a documentation drift, not a VACUUM violation.

### 4.2 Drift-gate presence (required for WEAK classification)

Each of the 3 companion extern files contains a `prod_methods_drift_check_*` function that forces compile-time resolution of every mirror method, satisfying the drift-detection mechanism required for WEAK classification:

- `extern_vb_edvbj_storage_event.rs:64-114` — `prod_methods_drift_check_mirror()` ✓
- `extern_vb_edvbj_propagation.rs:31-43` — `prod_methods_drift_check_propagation()` ✓
- `extern_vb_edvbj_symbolic_code.rs:29-33` — `prod_methods_drift_check_symbolic_code()` ✓

Drift-gate mechanism verified.

### 4.3 Per-`assume_specification` contract audit

| Bridge | Location | Contract | Verdict |
|--------|----------|----------|---------|
| `mirror_storage_event` | vb_edvbj_storage_event.rs:165-171 | `r.is_ok() \|\| r.is_err()` | **TAUTOLOGY** (always true for any `Result<T,E>`) — LETHAL pattern (skill rule §Lethal patterns to reject) |
| `mirror_runtime_journal_event_kind` | vb_edvbj_storage_event.rs:175-180 | `r == spec_event_kind_for_variant(*event)` | non-tautological (mirror body is `#[verifier::external]`, however — Verus treats this assumption as unverified) |
| `mirror_append_sequenced_body` | vb_edvbj_propagation.rs:103-112 | `match storage_event_result { Err(_) => r.is_err(), Ok(_) => true }` | non-tautological (asserts Err preserved); mirror body `#[verifier::external]` makes this an unverified assumption |
| `mirror_queued_strict_append_sequenced` | vb_edvbj_propagation.rs:117-123 | `spec_strict_profile_short_circuits(profile, r)` | non-tautological; mirror body `#[verifier::external]` |
| `mirror_symbolic_code` | vb_edvbj_symbolic_code.rs:106-111 | `spec_symbolic_code_for_unmapped_is_internal_invariant(*err, r)` | non-tautological; mirror body `#[verifier::external]` |
| `mirror_runtime_code` | vb_edvbj_symbolic_code.rs:115-120 | `spec_runtime_code_for_unmapped_is_none(*err, r)` | non-tautological; mirror body `#[verifier::external]` |
| `record_kind_discriminant_check` | vb_edvbj_mirror_bind.rs:104-107 | `r == true \|\| r == false` | **TAUTOLOGY** (always true for any bool) — LETHAL pattern |
| `is_known_record_kind_stub` | vb_edvbj_mirror_bind.rs:112-115 | `r == true \|\| r == false` | **TAUTOLOGY** — LETHAL pattern |

**Tautological contracts: 3 of 8 (`mirror_storage_event`, `record_kind_discriminant_check`, `is_known_record_kind_stub`).**

Per the proof-reviewer skill's `Lethal patterns to reject` section, tautological `assume_specification` contracts are classified as VACUUM. The binding script (syntactic) does not catch tautological contracts; my skill's pattern list (semantic) does.

**binding_classification: WEAK (script) / VACUUM (semantic, for the 3 tautological bridges).** The disagreement is documented in F-001 below.

## 5. Verifier Lane Profile Compliance (Default Rust)

| Lane | Obligation(s) | Verifier tool status (rerun 2026-07-01) | Status |
|------|---------------|---------------------------------------|--------|
| verus | PO-001, 005, 007, 009 | verus 0.2026.05.05.d03e906 in PATH; `verus --crate-type=lib` ran 44 verified, 0 errors across the 4 specs | VERIFIED |
| kani | PO-002, 006 | cargo-kani 0.67.0 in PATH; proof-writer-report claims BLOCKED_TOOLING | NOT VERIFIED at State 5; artifact schema-valid; tool IS installed |
| flux-rs | PO-008 | cargo-flux 4d329f2 (2026-05-23) in PATH; proof-writer-report claims BLOCKED_TOOLING | NOT VERIFIED at State 5; artifact schema-valid; tool IS installed |
| proptest | PO-003, 004, 010 | proptest 1.5 stable in workspace; PENDING_FORMAL_EXECUTION (depends on vb-cib14 variant) | NOT RUN at State 5 |

**Tooling status discrepancy:** the proof-writer-report claims `BLOCKED_TOOLING` for Kani (cargo-kani) and Flux (cargo-flux), but both binaries ARE in `$PATH` and version-printable. The handoffs in §5 of proof-writer-report and §7 of proof-evidence.md are inconsistent with the actual environment. See F-004 below.

## 6. Raw Verifier Output (Re-run 2026-07-01)

| Target | Command | Result |
|--------|---------|--------|
| `verification/verus/vb_edvbj_storage_event.rs` | `verus --crate-type=lib verification/verus/vb_edvbj_storage_event.rs` | `verification results:: 26 verified, 0 errors` |
| `verification/verus/vb_edvbj_propagation.rs` | `verus --crate-type=lib verification/verus/vb_edvbj_propagation.rs` | `verification results:: 10 verified, 0 errors` |
| `verification/verus/vb_edvbj_symbolic_code.rs` | `verus --crate-type=lib verification/verus/vb_edvbj_symbolic_code.rs` | `verification results:: 6 verified, 0 errors` |
| `verification/verus/vb_edvbj_mirror_bind.rs` | `verus --crate-type=lib verification/verus/vb_edvbj_mirror_bind.rs` | `verification results:: 2 verified, 0 errors` |
| `verification/verus/production_inner/vb_edvbj_storage_event_production.rs` | `verus --crate-type=lib …` | `21 verified, 0 errors` |
| `verification/verus/production_inner/vb_edvbj_propagation_production.rs` | same | `6 verified, 0 errors` |
| `verification/verus/production_inner/vb_edvbj_symbolic_code_production.rs` | same | `2 verified, 0 errors` |
| `bash scripts/check-verus-production-binding.sh .` | (passed workdir) | `0 STRONG, 75 WEAK, 0 VACUUM` |

Verus-side evidence is reproducible. The "44 verified, 0 errors" total matches the writer-report's claim. The "verified counts" include auto-derived type/struct verification; the per-criterion semantic claim depends on the contract, which is partial (see §4.3).

The proptest files are PENDING (vb-cib14 dep); the Kani/Flux files are NOT RUN at State 5 (toolchain present but author has not executed them in the workspace).

## 7. Non-Vacuity Audit

### 7.1 Verus (per `proof-writer-report` and direct read)

| Conformance to non-vacuity rules | Status |
|-----------------------------------|--------|
| `proof fn` body is non-trivial (not just `ensures true; {}`) | **FAIL** — all 7 `proof fn` markers across the 4 specs have empty bodies with `ensures true`. They prove nothing. |
| `assume_specification` contract is non-tautological | **PARTIAL** — 5 of 8 bridges have non-tautological contracts; 3 are tautologies (LETHAL pattern) |
| Mirror body is `#[verifier::external]` (treated as opaque by Verus) | YES — all `mirror_storage_event`, `mirror_*_storage_event`, `mirror_runtime_journal_event_kind`, `mirror_append_sequenced_body`, `mirror_queued_strict_append_sequenced`, `mirror_symbolic_code`, `mirror_runtime_code` are `#[verifier::external]`. Verus does NOT verify the mirror body; the contract is an unverified assumption. |
| Drift-detection mechanism via `prod_methods_drift_check_*` | YES — present in all 3 extern files and exercised |
| No `assume(...)`, no `axiom`, no `external_body` | YES — script-trust scan in §8 of proof-evidence.md confirms |

**Verdict:** Verus specs provide **drift detection + documentation surface only**. The semantic verification is NOT performed by Verus at this layer; the 7 trivial `proof fn`s plus 3 tautological `assume_specification` bridges plus `#[verifier::external]` mirror bodies = non-vacuous drift gate only.

### 7.2 Kani (`kani_vb_edvbj_storage_event_no_fabricate.rs`)

- 6 harnesses use `kani::any()` over an `Arbitrary` impl for `RuntimeJournalEvent` that enumerates the 21 declared variants via a match-style generator (NOT hardcoded, NOT `proptest::any`-style direct).
- H1 (`kani_run_layer_no_fabricate`): calls production `run_storage_event` and uses `kani::cover!` for `RunFailed` reachability. **No `kani::assert!` on the post-fix required contract** (lines 134-144). Non-vacuous for per-layer.
- H2 (`kani_action_layer_no_fabricate`): calls production `action_storage_event`, asserts `!matches!(journal_event, RunFailedEvent)`. ✓ non-vacuous.
- H3 (`kani_boundary_layer_no_fabricate`): calls production `boundary_storage_event`, asserts `!matches!(journal_event, RunFailedEvent)`. ✓ non-vacuous.
- H4 (`kani_dispatch_no_fabricate`): **does NOT call top-level `storage_event`** (the production function at `chunk_002.rs:270-303` where the bug lives). Instead, manually inlines the match-arm dispatch in the harness, then uses two `kani::cover!` macros. **No `kani::assert!` on the post-fix return shape.** The harness passes against BOTH the pre-fix (buggy, fabricating `Ok(RunFailedEvent)`) and the post-fix (correct, returning `Err(UnmappedRuntimeJournalEvent)`) source.
- H5 (`kani_layer_consistency`): asserts the helper-matrix `count <= 1`. ✓ non-vacuous.
- H6 (`kani_event_kind_enumeration`): asserts `kind != "Unknown"`. ✓ non-vacuous.

**Kani split-harness plan-vs-actual drift:** the proof-plan-review (§6 of proof-plan-review.md) characterizes PO-EDVBJ-002-KANI as "`kani::any()` ... `kani::cover!` for reachability, `kani::assert!` for post-fix contract". The actual H4 only has `cover!` for reachability; no `assert!`. The plan's intended defense-in-depth cover for the dispatcher is incomplete. The H4 harness does NOT actually exercise or assert on the buggy top-level `storage_event` body; a `cargo kani -p vb_runtime` run would not catch the P0 bug.

### 7.3 Kani (`kani_vb_edvbj_propagation_strict_gate.rs`)

- H1 (`kani_append_sequenced_propagation`): manually inlines the dispatcher match-arms and uses `kani::cover!` for `Resumed` reachability. **No `kani::assert!` on the post-fix return shape.** Compiles but does not catch the post-fix-only typed error path.
- H2 (`kani_queued_strict_gate`): uses `kani::any()` over a `u8` profile byte and asserts `_is_strict_guard_fired` reachability. **No `kani::assert!` that the guard fires for `Strict` BEFORE the storage_event chain.**

Both harnesses are non-vacuous (use `kani::any()`) but neither carries an assertion that the post-fix required contract holds. Same plan-vs-actual drift as §7.2.

### 7.4 proptest (`proptest_vb_edvbj_all_21_variants.rs`)

- 10_000 cases (configured at line 106).
- Strategy: `proptest::sample::select` over the 21 declared variants (lines 60-101).
- Anti-invariant: `prop_assert!` on `Err(UnmappedRuntimeJournalEvent { event_kind })` for `Resumed`, `prop_assert!(result.is_ok())` for others (lines 126-138).
- Uses temp FjallJournal per case (lines 31-41).
- **PENDING_FORMAL_EXECUTION:** the variant `RuntimeError::UnmappedRuntimeJournalEvent` is NOT in production at the time of this review (`crates/vb_runtime/src/error/mod.rs:7-60` lacks the variant). The proptest compiles only under `--features=vb-edvbj-pending` AND after vb-cib14 adds the variant. Currently the proptest is gated behind a feature flag whose enable-time requires the production-side change.

Strategy non-vacuous: YES (select over 21 variants + prop_assert on specific error discriminant). Evidence: NONE at State 5.

### 7.5 proptest (`proptest_vb_edvbj_resumed_replay.rs`)

- 1_000 cases (line 43).
- Strategy: `prop_assume!(matches!(event, Resumed { .. }))` + temp FjallJournal.
- Asserts `Err(UnmappedRuntimeJournalEvent)`, `events_for_run(run).len() == 0`, no `RunFailedEvent` observed at any seq.
- **PENDING_FORMAL_EXECUTION:** same vb-cib14 dep.

### 7.6 proptest (`proptest_vb_edvbj_diagnostic_code.rs`)

- 10_000 cases (line 29).
- Strategy: `_dummy in 0u32..1` (deterministic single case, no proptest pressure on the variant selection).
- Asserts `diagnostic_code() == 0x2020`, no collision against 18 other variants, `runtime_code() == None`, `symbolic_code() == INTERNAL_INVARIANT`.
- **PENDING_FORMAL_EXECUTION:** depends on `crate::RuntimeError::UnmappedRuntimeJournalEvent` existing.

This proptest is a deterministic property test (one case repeated 10k times for shrink-engine reliability), not an enumeration of variants. Strategy non-vacuous for the discrete property claimed (single RuntimeError value). Compiles only with vb-cib14 dep.

### 7.7 Flux (`vb_edvbj_diagnostic_code_refinement.rs`)

- 3 `model_*` functions, **all marked `#[flux_rs::trusted]`** (lines 56, 70, 85). Per flux-rs semantics, `#[trusted]` marks a function as opaque to Flux (its body is not verified).
- The negative target (`model_synthetic_0x201f_does_not_satisfy_0x2020_contract`) is `#[flux_rs::should_fail]`, but since the function is `#[trusted]` the `#[sig]` annotation is also unverified.
- **Deviation from established pattern:** the existing Flux refinement at `crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs:27-34` uses `#[extern_spec]` to refine the production function directly. The new file uses `#[trusted]` local abstract models with no production binding.
- Per skill rule §Lethal Findings: "Flux broad `trusted` / `ignore` or tautological refinements" — broad `#[trusted]` markers on the entire refinement are lethal.
- **BLOCKED_TOOLING** in writer-report; cargo-flux IS installed (version 4d329f2 (2026-05-23) in PATH). The artifact could run; the author has not run it.

## 8. Trust Marker Scan

`bash rg` scan over the 4 spec files:
```
$ rg -n 'assume\(|\baxiom\b|external_body' verification/verus/vb_edvbj_*.rs
(no matches)
```

No `assume(...)`, `axiom`, or `external_body` markers. The `#[verifier::external]` markers in production_inner mirrors are the documented production-binding mechanism (not in the "forbidden" list).

`trusted-base-ledger.jsonl` has 10 rows, all `acknowledged`. No unledgered trust markers; no pending dispositions.

## 9. Bridge to Implementation

The bead is STRONG-coupled with vb-cib14. The proof-writer-report §9 documents the coupling surface:
- PO-EDVBJ-001-VERUS assumes vb-cib14 lands `UnmappedRuntimeJournalEvent` in `RuntimeError` AND replaces the buggy fallback in `chunk_002.rs:295-302`.
- PO-EDVBJ-003-PROPTEST's exhaustive 21-variant strategy assumes vb-cib14's variant is declared.
- PO-EDVBJ-007-VERUS's mirror gate assumes vb-cib14 has not renamed `JournalEvent::RunResumed`.

The mandatory re-run of `bash scripts/check-verus-production-binding.sh` after vb-cib14 lands is the documented gate. Acknowledged.

## 10. Findings (12 total — see proof-findings.jsonl)

| ID | Severity | Code | Artifact | Description | Disposition |
|----|----------|------|----------|-------------|-------------|
| F-001 | blocker | E_VERUS_TAUTOLOGICAL_CONTRACT | vb_edvbj_storage_event.rs:170, vb_edvbj_mirror_bind.rs:106,114 | 3 of 8 `assume_specification` bridges use tautological contracts that are always true (`r.is_ok() \|\| r.is_err()`, `r == true \|\| r == false`). Skill pattern `Lethal patterns to reject` classifies as VACUUM. | `fixed_with_evidence` — non-tautological contract required |
| F-002 | blocker | E_KANI_DISPATCHER_NO_ASSERTION | kani_vb_edvbj_storage_event_no_fabricate.rs:188-228 (H4), kani_vb_edvbj_propagation_strict_gate.rs:64-98 (H1) | Kani harnesses for the top-level dispatcher and `?-propagation` use only `kani::cover!` reachability without `kani::assert!` on the post-fix required contract. The harnesses pass against BOTH pre-fix and post-fix code; they do NOT catch the P0 bug. Plan-reviewer explicitly required "`kani::assert!` for post-fix contract". | `fixed_with_evidence` — `kani::assert!` on error-discriminant required |
| F-003 | blocker | E_PROPTEST_PENDING_NO_EVIDENCE | proptest_vb_edvbj_all_21_variants.rs, proptest_vb_edvbj_resumed_replay.rs, proptest_vb_edvbj_diagnostic_code.rs | All 3 proptest files are PENDING_FORMAL_EXECUTION. They reference `crate::RuntimeError::UnmappedRuntimeJournalEvent { event_kind }` (does not exist in production at State 5; gated behind `vb-edvbj-pending` feature). Per skill rule: "Reject `PENDING_FORMAL_EXECUTION` without cheap smoke/typecheck evidence." | `fixed_with_evidence` — at minimum, smoke compile + test pass with vb-cib14 dep or anti-vacuity placeholder required |
| F-004 | blocker | E_FLUX_BROAD_TRUSTED | vb_edvbj_diagnostic_code_refinement.rs | 3 `model_*` functions all marked `#[flux_rs::trusted]` with no `#[extern_spec]` to production. Skill bullet "Flux broad `trusted` / `ignore` or tautological refinements" is lethal. Existing pattern (`vb_y9d3v_action_ticket_refinements.rs:27-50`) uses `#[extern_spec]` directly on production functions. | `fixed_with_evidence` — replace `#[trusted]` local models with `#[extern_spec]` to production, OR document and justify the abstraction in trusted-base-ledger |
| F-005 | blocker | E_VERIFICATION_NOT_RUN_AT_STATE_5 | Kani ×2, Flux ×1 proptest ×3 | PENDING/BLOCKED obligations have no raw verifier output. Per skill rule: "Tooling evidence is not proof evidence." Without raw execution, no behavior-affecting claim is closed. | `fixed_with_evidence` — execute the harness / proptest / flux commands at State 12 with the post-fix variant present |
| F-006 | medium | E_PLAN_VS_ACTUAL_BINDING_CLASSIFICATION_DRIFT | proof-plan-review.md:67-72, proof-writer-report.md:47-51 | Plan-reviewer classified PO-EDVBJ-001/005/009 as STRONG; writer honest-disclosed WEAK_MIRROR fallback in §4 of writer-report; per-file table in §3.1 still labels "STRONG-shaped". The plan artifacts were not updated to reflect the downgrade. | `owner_approved_no_action` — plan-as-document drift; writer-report §4 already contains the honest classification |
| F-007 | medium | E_PROOF_FN_EMPTY_BODY | All 7 `proof fn` markers in the 4 Verus specs | Each `proof fn` body is empty with `ensures true`. They prove nothing by Verus. Combined with `#[verifier::external]` mirror bodies, Verus's actual semantic discharge on this bead is zero. | `owner_approved_no_action` — documented as "proof markers" in writer-report; semantic verification delegated to Kani/proptest |
| F-008 | medium | E_VERUS_EXTERNAL_MIRROR_BODY | production_inner/vb_edvbj_storage_event_production.rs (5 functions), production_inner/vb_edvbj_propagation_production.rs (2 functions), production_inner/vb_edvbj_symbolic_code_production.rs (2 functions) | All 9 production-bound mirror functions are `#[verifier::external]`. Verus treats them as opaque — the `assume_specification` contracts attached in the spec files are unverified. | `owner_approved_no_action` — WEAK (mirror) binding architecture; drift gate via `prod_methods_drift_check_*` is the only Verus-discharged mechanism |
| F-009 | low | E_BLOCKED_TOOLING_REPORT_INACCURATE | proof-writer-report.md:200-204, proof-evidence.md:216-222, 264-270 | Writer-report claims `BLOCKED_TOOLING` for Kani (cargo-kani) and Flux (cargo-flux), but both binaries ARE in `$PATH` (cargo-kani 0.67.0, cargo-flux 4d329f2 2026-05-23). The handoffs to State 12 do not reflect that the toolchains are installed. | `owner_approved_no_action` — State 12 still needs verified execution to discharge; report wording is misleading but not blocking |
| F-010 | low | E_REVIEW_PROVENANCE_INCOMPLETE | agent-invocation-ledger.jsonl | The ledger has only rows for States 1, 2, 4b, 5; the State 4 (proof-planner) invocation row is absent. This is a process gap logged previously in the plan-review (low, owner_approved_no_action). | `owner_approved_no_action` — already tracked; not blocking |
| F-011 | low | E_TRUSTED_BASE_DEFERRED_FINDING | trusted-base-ledger.jsonl:1-10, proof-plan-findings.jsonl | H-2 (pre-existing 0x201F duplicate) is a pre-existing deferred finding. Resolution path documented in trusted-base-plan.md §5 (State 12 surface OR open a separate bead). The Flux negative target (`#[flux_rs::should_fail]`) acknowledges but does not fix the duplicate. | `owner_approved_debt` — tracked for State 12 |
| F-012 | observation | E_VB_EDVBJ_STORAGE_EVENT_NO_POSTFIX_ASSERT | kani_vb_edvbj_storage_event_no_fabricate.rs H4 (lines 188-228) | Even after F-002 is fixed, the H4 harness structure (manually inlined dispatch match-arms) is brittle to production refactors. Recommend replacing the inlined match with a call to `StorageRuntimeJournal::storage_event` (the actual top-level function) with `kani::any()` over `RuntimeJournalEvent`. | `owner_approved_no_action` — F-002 fixes the assertion gap; structural robustness is a follow-up |

**Severity tally:** 5 blocker (F-001, F-002, F-003, F-004, F-005), 3 medium (F-006, F-007, F-008), 3 low (F-009, F-010, F-011), 1 observation (F-012).

## 11. Lethal Finding Summary

The skill explicitly enumerates LETHAL patterns. The following lethal patterns are present:

| Pattern | Skill rule | Evidence |
|---------|-----------|----------|
| `assume_specification` with tautological contract | `Lethal patterns to reject` | F-001 (3 bridges) |
| Kani harness that doesn't catch the post-fix required contract (`cover!` only, no `assert!`) | `tool-specific-lethal-findings.md`: "Kani: assumptions encode result, no cover, arbitrary unwind, hidden stubs" | F-002 (H4 dispatcher, H1 propagation) |
| Flux broad `#[trusted]` without `#[extern_spec]` to production | `tool-specific-lethal-findings.md`: "Flux: broad trusted/ignore, tautological refinement, no invalid-state rejection" | F-004 (3 model functions) |
| `PENDING_FORMAL_EXECUTION` without cheap smoke/typecheck evidence | `## Workflow` rule 9 | F-003, F-005 (3 proptests, 2 Kani harnesses, 1 Flux refinement) |

**5 lethal findings → STATUS: REJECTED.**

## 12. Coupling with vb-cib14

This bead is STRONG-coupled with vb-cib14 (the production-side variant `RuntimeError::UnmappedRuntimeJournalEvent { event_kind }` does not exist yet). The proof-writer-report §9 documents the failure modes. The mandatory re-run of `bash scripts/check-verus-production-binding.sh` after vb-cib14 lands is the documented drift-detection gate. The reviewer acknowledges this coupling but cannot approve proofs whose verification is genuinely PENDING.

## 13. Repair Guidance (Summary)

A full repair guide is in `proof-repair-guide.md`. The minimum to flip STATUS to APPROVED is:

1. **F-001:** Strengthen the 3 tautological `assume_specification` contracts to non-tautological contracts, OR delete the tautological bridges and rely on Kani/proptest. For `mirror_storage_event`, encode the no-fabrication contract explicitly: `match event { RunFailed { .. } => r.is_ok() && /* in set */, _ => r matches Err(UnmappedRuntimeJournalEvent { .. }) }`. For `record_kind_discriminant_check` and `is_known_record_kind_stub`, remove the placeholder bridges (the drift gate is provided by `prod_methods_drift_check_*`, not by these contracts).
2. **F-002:** Replace H4 in `kani_vb_edvbj_storage_event_no_fabricate.rs` with a `kani::assert!` that calls the production `StorageRuntimeJournal::storage_event` directly (not inlining) and asserts the post-fix return shape (`Err(UnmappedRuntimeJournalEvent { .. })` for `Resumed`; `Ok(JournalEvent::RunFailedEvent)` only for `RunFailed`). Same for H1 in `kani_vb_edvbj_propagation_strict_gate.rs` (assert `Err(UNMAPPED)` preserved at `append_sequenced`).
3. **F-003 + F-005:** After vb-cib14 lands the variant, execute `cargo test -p vb_runtime --features=vb-edvbj-pending --release` for the 3 proptests and capture raw output. Gated pre-existence is acceptable ONLY if a documented smoke compile (e.g., `cargo check --features=vb-edvbj-pending`) verifies schema-validity. The 3 proptests must at minimum pass `cargo check --features=vb-edvbj-pending`.
4. **F-004:** Replace `#[flux_rs::trusted]` markers in `vb_edvbj_diagnostic_code_refinement.rs` with `#[extern_spec]` directly on `crate::RuntimeError::diagnostic_code`, `crate::RuntimeError::symbolic_code`, and `crate::RuntimeError::runtime_code`. Add the body model as a Verus spec companion to match.
5. **F-009:** Update proof-writer-report.md to reflect that cargo-kani 0.67.0 and cargo-flux 4d329f2 are installed; State 12 will run, not install.

---

## Summary

5 lethal findings F-001 through F-005; 3 medium + 3 low + 1 observation follow-ups in `proof-findings.jsonl`. The WEAK Verus binding architecture is acceptable per the `bash scripts/check-verus-production-binding.sh` classification (0 STRONG, 75 WEAK, 0 VACUUM), but the skill's semantic pattern list flags 3 tautological contracts, 2 Kani harnesses missing the `kani::assert!` post-fix contract, and 1 Flux refinement with broad `#[trusted]` markers. Verifier execution is required for Kani/Flux/proptest lanes at State 12 after vb-cib14 lands; raw output is the only acceptable closure for the 5 behavior-affecting obligations.

STATUS: REJECTED
