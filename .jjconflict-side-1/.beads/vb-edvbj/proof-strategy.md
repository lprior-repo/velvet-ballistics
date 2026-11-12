# Proof Strategy — vb-edvbj

**Bead:** vb-edvbj — Runtime: delete fallback that maps unmapped journal events to run failure (P0 bug)
**STRONG-coupled with:** vb-cib14 (must land together)
**Phase:** State 4 — Proof Planning
**Strategy Version:** 1.0.0
**Date:** 2026-07-01

---

## 1. Strategy Summary

**Classification:** P0 persistence / temporal / public-api fix that eliminates a silent
state-machine corruption bug. The buggy fallback in
`StorageRuntimeJournal::storage_event` (chunk_002.rs:295-302) synthesizes
`JournalEvent::RunFailedEvent { run, seq, attempt: 1 }` whenever every per-layer
helper (`run_storage_event`, `action_storage_event`, `boundary_storage_event`) returns
`None`. Today this happens only for `RuntimeJournalEvent::Resumed { run, timestamp }`,
but any future non-exhaustive variant would silently corrupt the run's terminal-class
classification.

**Replacement strategy (Option A — typed error):** Replace the wildcard fallback with
`Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind: &'static str })` where
`event_kind` is the literal variant name. This is a single-file body change in
`chunk_002.rs:270-303` plus six wiring arms across `crates/vb_runtime/src/error/**`
(mod.rs, equality.rs, display.rs, diagnostics.rs).

**Primary Lanes (mandated by bead):**
- **Kani (rust-local, bounded symbolic)** — split-harness over `run_storage_event`,
  `action_storage_event`, `boundary_storage_event`, and the top-level dispatcher.
  Covers I-1 (no fabrication) and I-2 (helper consistency).
- **Verus (rust-local, deductive)** — exec-fn proof binding to production
  `storage_event` body, requires/ensures over the
  `Result<JournalEvent, RuntimeError>` return set, paired with the existing
  `verification/verus/extern_storage_kind_family.rs` mirror (no mirror-type changes
  required for this bead; the gate is re-run).
- **proptest (temporal-replay + cargo test)** — exhaustive enumeration of all 21
  `RuntimeJournalEvent` variants, replay-equivalence assertion (storage-side
  `append_sequenced` returns `Err(UNMAPPED)` and `events_for_run(run)` contains zero
  `RunFailedEvent` records for the input seq), and Strict-profile regression.

**Defense-in-Depth Lanes:**
- **Flux (refinement)** — refines `RuntimeError::diagnostic_code()` return type to a
  finite enum of declared codes (0x2001..=0x2020); guards against accidental
  collision with the pre-existing duplicate 0x201F (H-2) and confirms 0x2020
  registration is unique.
- **source-lint (cargo check + lint gate)** — the `forbid(unsafe_code)` plus
  `no_panic_surface` + `forbid/unwrap.expect.panic.todo.unimplemented.dbg` moon ci
  gate, plus the existing `storage_event_clones_the_event_exactly_once_per_dispatch`
  clone-counter invariant at chunk_002.rs:310-312, enforce the post-fix shape.

**Not Applicable Lanes:**
- **Loom** — `storage_event` is synchronous, `&event`-borrowing, no shared state, no
  async, no Send/Sync boundary. `boundary-map.md` §"Functional core / imperative
  shell" classifies the dispatcher as sync-imperative; no concurrency surface
  exists in this codepath.
- **Miri** — the affected crate sets `#![forbid(unsafe_code)]`. No FFI, no raw
  pointers, no provenance concerns. (`boundary-map.md` §"Unsafe" row.)
- **cargo-fuzz** — no parser/codec/hostile-input boundary. `RuntimeJournalEvent`
  values are constructed by `vb_runtime` itself in shard paths; external
  deserialization goes through serde (chunk_001.rs:14) which is orthogonal to the
  dispatch fix.
- **Flux (refinement on `storage_event`)** — the claim "no fabrication" is a
  value-level invariant, not a refinement type. Flux is used only for the
  diagnostic-code finite-enum refinement (PO-006); not for the dispatcher itself.

**Waiver Candidates:** None. All behavior-affecting claims are bound to Kani/Verus
symbolic-and-deductive proof + proptest property pressure. The H-3 mirror-drift is
verified by re-running `bash scripts/check-verus-production-binding.sh` after the
implementation lands; if drift is detected the gate fails and the merge is blocked
(no waiver path).

---

## 2. Risk Classification

| Risk Tag | Seeds | Primary Lane | Rationale |
|----------|-------|--------------|-----------|
| `persistence` | RUNTIME-UNMAPPED-NO-FABRICATE, RUNTIME-RESUMED-NO-RUN-FAILED, RUNTIME-EVENT-KIND-ENUMERATION | Kani + Verus | The bug corrupts durable storage (Fjall `JournalEvent::RunFailedEvent` written for a `Resumed` event). Kani bounds symbolic input over the 21-variant enum; Verus binds the exec fn body and proves no fabrication for non-`RunFailed` variants. |
| `temporal` | RUNTIME-UNMAPPED-NO-FABRICATE, RUNTIME-RESUMED-NO-RUN-FAILED | Verus + proptest (replay) | Recovery depends on correct replay classification; the buggy fallback mis-classifies `Resumed` as `Failed`. proptest exercises the Fjall replay-equivalence assertion; Verus proves the dispatcher contract. |
| `public_api` | RUNTIME-LAYER-HELPER-CONSISTENCY, RUNTIME-STORAGE-EVENT-PROPAGATION, RUNTIME-STRICT-GATE-PRESERVED, RUNTIME-DIAGNOSTIC-CODE-CONSTANT | Kani + Verus + proptest | `RuntimeError` gains a new variant; the contract requires `PartialEq`, `Display`, `DiagnosticCode` arms across five modules. Kani covers field-equality; Verus covers the dispatcher return type; proptest covers all 33 currently-defined variants + the new one. |
| `parser/codec` | RUNTIME-LAYER-HELPER-CONSISTENCY, RUNTIME-EVENT-KIND-ENUMERATION | Kani + proptest | Per-variant dispatch is the core "codec" surface (event → mapped-or-typed-error). Kani covers bounded symbolic input; proptest covers exhaustive enumeration. |
| `verification_artifacts` | RUNTIME-MIRROR-DRIFT-PRESERVATION | Verus production-binding gate | The existing `verification/verus/extern_storage_kind_family.rs` mirror's `prod_methods_drift_check_mirror` (lines 670-695) resolves production method names at compile time. The gate `bash scripts/check-verus-production-binding.sh` MUST pass post-implementation; if it fails the merge is blocked. |
| `telemetry` | RUNTIME-DIAGNOSTIC-CODE-CONSTANT | Flux + proptest | Diagnostic code 0x2020 must be unique in the `0x2001..=0x2020` range and not collide with the pre-existing 0x201F duplicate (H-2). Flux refines `diagnostic_code()` return type to a finite enum. |

---

## 3. Coverage Approach By Seed

### 3.1 RUNTIME-UNMAPPED-NO-FABRICATE (I-1, primary seed)

- **Kani (split-harness):** Symbolic `RuntimeJournalEvent` (kani::any() over the
  21-variant enum via Arbitrary impl) into `storage_event`; assert the post-fix
  contract:
  `Ok(JournalEvent::RunFailedEvent { run, seq, attempt: 1 }) ⇒ event was RuntimeJournalEvent::RunFailed { run }`.
  Reachable via `kani::cover!(event matches RunFailed)` and the
  anti-fabrication `kani::assert!(result matches Err(UNMAPPED) for non-RunFailed
  with all-helpers-None)`. Three harnesses: `kani_run_layer_no_fabricate`,
  `kani_action_layer_no_fabricate`, `kani_boundary_layer_no_fabricate`,
  `kani_dispatch_no_fabricate`.
- **Verus:** exec fn `exec_storage_event(event: &RuntimeJournalEvent, seq: EventSeq)
  -> Result<JournalEvent, RuntimeError>` mirrors the post-fix body, with
  `requires(true)` and `ensures(match result { Ok(e) => e is mapped,
  Err(UnmappedRuntimeJournalEvent{..}) => event unmapped })`. Forbids
  `external_body`/`assume`/`axiom` in command (validator gate).
- **proptest:** proptest! over all 21 declared variants (exact enumeration strategy,
  not Just) — assert `storage_event` returns `Ok(J)` only for explicit mapped
  variants and `Err(UNMAPPED)` for all unmapped ones, with anti-invariant
  `prop_assume!(!matches!(variant, MappedVariant))` for the negative case.

### 3.2 RUNTIME-LAYER-HELPER-CONSISTENCY (I-2)

- **Kani + proptest:** exhaustively generate every (variant, helper) matrix;
  assert 0 helpers return Some ⇒ Err(UNMAPPED), 1 helper returns Some ⇒ Ok(...),
  ≥2 helpers return Some ⇒ impossible (implied by current shape; one helper per
  variant). Harness: `kani_layer_consistency`.

### 3.3 RUNTIME-RESUMED-NO-RUN-FAILED (I-1, RE-019 specific, temporal-replay)

- **proptest (temporal-replay):** build a Fjall-backed `StorageRuntimeJournal`
  (`journaled(journal)`); call `append_sequenced(Resumed { run, timestamp: 0 },
  EventSeq::new(0))`; assert (a) returns `Err(UNmappedRuntimeJournalEvent {
  event_kind: "Resumed" })`; (b) `events_for_run(run)` contains zero records; (c)
  Fjall contains zero `RunFailedEvent` for that seq. proptest property must
  include `prop_assume!(event matches Resumed)` so the strategy is non-vacuous.
  This is the canonical RE-019 regression test (test-writer owns the test file,
  but the obligation is planned here).

### 3.4 RUNTIME-EVENT-KIND-ENUMERATION (I-6, H-4 mitigation)

- **proptest:** generate every variant via `proptest::sample::select` (not Just),
  assert `event_kind(&event) != "Unknown"` for the 21 declared variants; covers
  H-4 (future-variant mitigation: helper enumerates every variant).
- **Kani:** bounded symbolic over the 21-variant enum; assert the `event_kind`
  helper's match is exhaustive (verified at compile time; runtime `kani::cover`
  proves every arm is reachable).

### 3.5 RUNTIME-DIAGNOSTIC-CODE-CONSTANT (I-11, I-12, I-13)

- **Flux:** refine `RuntimeError::diagnostic_code()` return type to a finite enum
  mirroring the 33 currently-defined variants (plus the new `UnmappedRuntimeJournalEvent`
  → 0x2020). Asserts the constant does not collide with the existing 0x201F
  duplicate (H-2). Paired negative target: `diagnostic_code_unmapped_returns_0x2020_negative`.
- **proptest:** fuzz any added `DiagnosticCode` constant: assert value in
  `0x2001..=0x2020` and no other currently-defined variant collides on `0x2020`.
- **Verus (companion):** refinement on `RuntimeError::symbolic_code()` that bounds
  the result set to `SymbolicCode::INTERNAL_INVARIANT` for the unmapped variant.

### 3.6 RUNTIME-MIRROR-DRIFT-PRESERVATION (I-3, H-3)

- **Verus (production-binding gate):** mandatory re-run of
  `bash scripts/check-verus-production-binding.sh` after implementation lands.
  The existing `verification/verus/extern_storage_kind_family.rs:670-695`
  `prod_methods_drift_check_mirror` is unchanged by this fix (production
  `JournalEvent::RunResumed` shape unchanged; no new mirror types required).
  If the gate fails, the planner's H-3 mitigation requires blocking the merge —
  this is a verifier-binding obligation, not a waiver.

### 3.7 RUNTIME-STORAGE-EVENT-PROPAGATION (I-3, I-4)

- **Kani (propagation harness):** symbolic `RuntimeJournalEvent` into
  `StorageRuntimeJournal::append_sequenced`; assert Err(UNMAPPED) at
  `storage_event` ⇒ Err(UNMAPPED) at `append_sequenced`. Three-caller chain:
  `storage_event` → `append_sequenced` → `RuntimeShard::append_journal_event`.
- **Verus (companion):** refinement on the propagation chain proving
  `Err(UNMAPPED)` is preserved across all three `?` sites without caller-level
  rewriting.

### 3.8 RUNTIME-STRICT-GATE-PRESERVED (I-4)

- **proptest:** fuzz `DurabilityProfile::Strict` profile; assert
  `QueuedStorageRuntimeJournal::append_sequenced` returns
  `Err(UnsupportedAsyncStrictAck)` BEFORE reaching `storage_event` (i.e. before
  any Fjall I/O). proptest property: `prop_assume!(profile == Strict)` then
  assert `matches!(result, Err(UnsupportedAsyncStrictAck))`.

---

## 4. Defense-in-Depth Layering

```
Layer 0 (Compile-time, source-lint):
  ├── forbid(unsafe_code) in vb_runtime
  ├── no panic / unwrap / expect / todo / unimplemented / dbg! macro
  └── STORAGE_EVENT_CLONE_COUNT = 1 invariant (chunk_002.rs:310-312)

Layer 1 (Verus deductive, exec-fn binding):
  ├── exec_storage_event requires/ensures contract (no fabrication)
  ├── Verus production-binding gate (H-3 mirror drift detection)
  └── symbolic_code() refinement for UNMAPPED

Layer 2 (Kani bounded symbolic):
  ├── kani_run_layer_no_fabricate
  ├── kani_action_layer_no_fabricate
  ├── kani_boundary_layer_no_fabricate
  ├── kani_dispatch_no_fabricate
  ├── kani_layer_consistency (helper matrix)
  └── kani_append_sequenced_propagation (? chain)

Layer 3 (proptest property pressure):
  ├── proptest_dispatch_all_21_variants (exhaustive enumeration)
  ├── proptest_resumed_temporal_replay (RE-019 replay equivalence)
  ├── proptest_event_kind_enumeration (H-4 mitigation)
  ├── proptest_diagnostic_code_constant (H-2 collision guard)
  └── proptest_strict_profile_gate (I-4 preservation)

Layer 4 (Flux refinement, source-level):
  └── diagnostic_code() finite-enum refinement (paired negative target)

Layer 5 (cargo test, source-lint):
  ├── cargo test --test storage_runtime_journal_maps_cancelled_and_failed_events
  ├── cargo test --test storage_event_clones_the_event_exactly_once_per_dispatch
  └── moon ci :rust-verification-gauntlet (aggregate gate)
```

---

## 5. Obligation Count

| Verifier | Obligations | Seeds Covered |
|----------|-------------|---------------|
| Verus | 2 | RUNTIME-UNMAPPED-NO-FABRICATE, RUNTIME-MIRROR-DRIFT-PRESERVATION, RUNTIME-STORAGE-EVENT-PROPAGATION, RUNTIME-DIAGNOSTIC-CODE-CONSTANT |
| Kani | 3 | RUNTIME-UNMAPPED-NO-FABRICATE (split-harness), RUNTIME-LAYER-HELPER-CONSISTENCY, RUNTIME-STORAGE-EVENT-PROPAGATION |
| proptest | 3 | RUNTIME-RESUMED-NO-RUN-FAILED (temporal-replay), RUNTIME-EVENT-KIND-ENUMERATION, RUNTIME-STRICT-GATE-PRESERVED, RUNTIME-DIAGNOSTIC-CODE-CONSTANT, RUNTIME-UNMAPPED-NO-FABRICATE |
| Flux | 1 | RUNTIME-DIAGNOSTIC-CODE-CONSTANT |
| **Total** | **8** | **All 8 seeds** |

Note: each obligation may cover multiple seeds via `proof_seed_id` references;
the 8 obligations together cover all 8 seeds, satisfying the default risk profile
for `bounded_transition` (kani + verus) plus the additional required lanes
(Verus mirror binding, proptest temporal-replay, Flux refinement).

---

## 6. Non-Applicable Lane Summary

| Lane | Reason | Evidence |
|------|--------|----------|
| **Loom** | `storage_event` is sync, `&event`-borrowing, no shared state, no async, no Send/Sync boundary. Per `boundary-map.md` §"Async / sync cross-check": "no async context switches". | `boundary-map.md` §"Async / sync cross-check"; `workflow-model.md` §"Cancellation path" (no cancellation hazard); `hazard-analysis.md` H-7 (concurrency class empty). |
| **Miri** | `vb_runtime` crate sets `#![forbid(unsafe_code)]`; no FFI; no raw pointers; no provenance concerns. The fix is purely value-level enum + error-path changes. | `boundary-map.md` §"Unsafe" row; AGENTS.md §"Engineering Rules" (no `unsafe`); `hazard-analysis.md` H-1 (no `unsafe` class). |
| **cargo-fuzz** | No parser/codec/hostile-input boundary in this codepath. `RuntimeJournalEvent` values are constructed by `vb_runtime` itself in shard paths. External deserialization is serde (chunk_001.rs:14), orthogonal to the dispatch fix. | `boundary-map.md` §"Parser / codec" row (no external byte boundary on dispatcher); `codebase-map.md` §"Hostile-input cross-check". |
| **Flux (on `storage_event` body)** | "No fabrication" is a value-level invariant, not a refinement type. The dispatcher uses a `match` over `&event` plus an `Err` constructor — Flux's quantifier-free arithmetic fragment does not naturally express "for every variant, the return is exactly this set". Kani + Verus cover the claim more naturally. | `references/verifier-trigger-matrix.md` Flux row: "API precondition refinement" applies to refinement types, not enum dispatchers. |

---

## 7. Trusted Base Summary

| Trusted Element | Kind | Reason |
|-----------------|------|--------|
| `verus` solver (z3) | external_body | Verus obligations depend on z3's correctness for the `requires`/`ensures` discharge of `exec_storage_event`. |
| `cargo-kani` CBMC backend | external_body | Kani obligations depend on CBMC's correctness for bounded symbolic execution of `storage_event`. |
| `cargo-flux` liquid-fixpoint backend | external_body | Flux obligation depends on liquid-fixpoint for the finite-enum refinement of `diagnostic_code()`. |
| `proptest@1.5` shrink engine | external_body | proptest obligations depend on proptest's strategy/shrink correctness. |
| `vb_storage::FjallJournal` (impl-by-keyword) | trusted | Temporal-replay proptest depends on Fjall's append/lookup semantics; this is independently verified by vb_storage's test suite and the `re_019_resumed_does_not_fabricate_run_failed` regression test (test-writer owns). |
| Existing `extern_storage_kind_family.rs` mirror | trusted | The Verus mirror's drift-detection helper `prod_methods_drift_check_mirror` (lines 670-695) is the production-binding anchor for this bead's Verus obligations. The mirror is unchanged by the fix; the gate is re-run post-implementation. |
| `vb_core::HasSymbolicCode` registry | trusted | The `symbolic_code()` chain (`vb_core::errors::CoreError`, `vb_runtime::error::RuntimeError::symbolic_code`) relies on the symbolic-code registry in `vb_core/src/diagnostic.rs`. The registry is verified by `vb-xi2f.10` (P1) and is upstream of this bead. |

---

## 8. Waiver Candidates

**None.** All behavior-affecting claims are bound to verifier evidence. No
behavior-affecting waiver candidates are emitted (validator gate
`E_BEHAVIOR_WAIVER`). The H-3 mirror-drift is a verifier-binding obligation
(mandatory re-run of `check-verus-production-binding.sh`), not a waiver candidate.

---

## 9. Blockers

None. All required verifier tooling is available in the workspace (per
`scripts/verify-verus.sh`, `scripts/flux-check-package.sh`, `scripts/kani-list.sh`).
The Verus mirror file `verification/verus/extern_storage_kind_family.rs` already
exists and binds production code without drift.

---

## 10. Gap Coverage

| Gap | Covered By |
|-----|-----------|
| Buggy post-dispatch fallback (chunk_002.rs:295-302) | PO-001-KANI, PO-001-VERUS, PO-001-PROPTEST, PO-002-KANI |
| New typed error variant + equality/display/diagnostic wiring | PO-003-PROPTEST, PO-006-FLUX, PO-008-VERUS |
| `event_kind: &'static str` allocation-free invariant | PO-004-PROPTEST, PO-001-VERUS |
| Verus mirror drift preservation (H-3) | PO-005-VERUS (mandatory gate re-run) |
| Strict-profile gate preservation (I-4) | PO-008-PROPTEST |
| `?` propagation chain (I-3) | PO-007-KANI, PO-005-VERUS |
| Diagnostic-code collision guard (H-2 — pre-existing duplicate at 0x201F) | PO-006-FLUX (deferred-finding surfaced via hazard-analysis.md) |

---

## 11. STRONG Coupling with vb-cib14

This bead is STRONG-coupled with vb-cib14 per the bead description. Both beads
must land together. The proof obligations here assume vb-cib14's implementation
landed (or is landing in the same JJ change). Specifically:
- PO-001-VERUS's `exec_storage_event` mirrors the post-fix body, which requires
  vb-cib14's signature changes (if any) to `RuntimeError`.
- PO-003-PROPTEST's exhaustive 21-variant strategy assumes vb-cib14's
  `UnmappedRuntimeJournalEvent` variant is declared in `crates/vb_runtime/src/error/mod.rs`.
- PO-005-VERUS's mirror gate assumes vb-cib14 has not introduced any rename
  of `JournalEvent::RunResumed` (which would break the existing
  `prod_methods_drift_check_mirror`).

If vb-cib14 changes any of these, this bead's obligations must re-plan with the
new artifacts. The proof-plan-reviewer is expected to verify the coupling during
disposition (State 4b).

---

## 12. Handoff

- Submit to `proof-plan-reviewer` (State 4b) with all 7 artifacts under
  `.beads/vb-edvbj/`.
- After approval, `proof-writer` (State 5) authors the Kani harnesses, Verus exec
  fn, Flux refinement, and proptest property tests.
- `proof-reviewer` (State 6) approves the written proof artifacts.
- `proof-to-implementation` (State 7) bridges proof claims to Rust source refs.
- `holzman-rust` (State 11) implements the production fix in `chunk_002.rs` and
  the six error-module files.
- `formal-verifier` (State 12) runs the verifier commands and closes the ledger.