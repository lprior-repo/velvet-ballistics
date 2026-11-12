# Trusted Base Plan — vb-edvbj

**Bead:** vb-edvbj — Runtime: delete fallback that maps unmapped journal events to run failure (P0 bug)
**STRONG-coupled with:** vb-cib14 (must land together)
**Phase:** State 4 — Proof Planning
**Date:** 2026-07-01

---

## 1. Purpose

This document identifies every trusted element, assumption, stub, external body,
model bound, or reduction that proof obligations depend on. Each item requires an
entry in the trusted-base ledger before State 12 closure. The plan is keyed to
the `proof-obligation/v1` obligations in `proof-obligations.planned.jsonl` and
the `verifier-lane-decision/v1` rows in `verifier-lane-decisions.jsonl`.

---

## 2. Trusted Base Ledger Entries

### TB-VERUS-001-z3-solver

| Field | Value |
|-------|-------|
| **ID** | TB-VERUS-001-z3-solver |
| **Trusted Kind** | `external_body` |
| **Scope** | PO-EDVBJ-001-VERUS, PO-EDVBJ-005-VERUS, PO-EDVBJ-011-VERUS |
| **Impact** | All three Verus obligations for this bead |
| **Reason** | Verus obligations depend on z3's correctness for discharging the `requires`/`ensures` of `exec_storage_event` and the `?`-propagation chain refinement. If z3 has a soundness bug, the obligations may report SUCCEEDED while the property does not hold. |
| **Behavior Affecting** | true |
| **Compensating Evidence** | Kani PO-EDVBJ-002-KANI and PO-EDVBJ-006-KANI provide CBMC-based cross-check on the same claim; proptest PO-EDVBJ-003-PROPTEST provides randomized pressure; the Verus production-binding gate (PO-EDVBJ-011) re-validates the mirror after implementation. |
| **Owner** | proof-planner |
| **Expiry** | None (z3 is a stable ecosystem dependency) |

### TB-VERUS-002-mirror-bind

| Field | Value |
|-------|-------|
| **ID** | TB-VERUS-002-mirror-bind |
| **Trusted Kind** | `external_body` |
| **Scope** | PO-EDVBJ-001-VERUS, PO-EDVBJ-005-VERUS |
| **Impact** | Verus obligations for `exec_storage_event` and `?`-propagation chain |
| **Reason** | The Verus spec's `exec_storage_event` mirrors the post-fix production body. The mirror depends on the production `StorageRuntimeJournal::storage_event` signature and the per-layer helper signatures remaining unchanged. If the implementation diverges from the spec's body shape (e.g., renames a method or changes a signature), the mirror drift gate must re-run. |
| **Behavior Affecting** | true |
| **Compensating Evidence** | PO-EDVBJ-011-VERUS (mandatory mirror-drift gate re-run); existing `verification/verus/extern_storage_kind_family.rs:670-695` `prod_methods_drift_check_mirror` resolves production method names at compile time; if any rename breaks the gate, the merge is blocked. |
| **Owner** | proof-planner |
| **Expiry** | Re-evaluated on every implementation change |

### TB-VERUS-003-mirror-drift-gate

| Field | Value |
|-------|-------|
| **ID** | TB-VERUS-003-mirror-drift-gate |
| **Trusted Kind** | `stub` |
| **Scope** | PO-EDVBJ-011-VERUS |
| **Impact** | H-3 mitigation; mandatory gate |
| **Reason** | The drift-detection helper `prod_methods_drift_check_mirror` at `verification/verus/extern_storage_kind_family.rs:670-695` resolves production method names at compile time via `#![forbid(unsafe_code)]`-flanked stubs. The drift stub may not capture every drift surface (e.g., a rename inside a match arm). |
| **Behavior Affecting** | true |
| **Compensating Evidence** | The companion `scripts/check-production-inner-drift.sh` runs alongside `check-verus-production-binding.sh`; both must pass. Failure of either gate is a hard blocker. |
| **Owner** | proof-planner |
| **Expiry** | None (script lifecycle) |

### TB-KANI-001-cbmc-backend

| Field | Value |
|-------|-------|
| **ID** | TB-KANI-001-cbmc-backend |
| **Trusted Kind** | `external_body` |
| **Scope** | PO-EDVBJ-002-KANI, PO-EDVBJ-006-KANI |
| **Impact** | Kani obligations on the no-fabrication contract and the helper-matrix property |
| **Reason** | Kani obligations depend on CBMC's correctness for bounded symbolic execution of `storage_event` over the 21-variant `RuntimeJournalEvent` enum. If CBMC has a soundness bug, the obligations may report VERIFICATION:- SUCCESSFUL while the property does not hold. |
| **Behavior Affecting** | true |
| **Compensating Evidence** | Verus PO-EDVBJ-001-VERUS provides z3-based deductive proof of the same claim; proptest PO-EDVBJ-003-PROPTEST provides randomized pressure. Three independent verifiers on the same claim. |
| **Owner** | proof-planner |
| **Expiry** | None (cargo-kani is a stable ecosystem dependency) |

### TB-FLUX-001-fixpoint-backend

| Field | Value |
|-------|-------|
| **ID** | TB-FLUX-001-fixpoint-backend |
| **Trusted Kind** | `external_body` |
| **Scope** | PO-EDVBJ-008-FLUX |
| **Impact** | Flux refinement on `diagnostic_code()` finite-enum return type |
| **Reason** | The Flux obligation depends on liquid-fixpoint's correctness for discharging the finite-enum refinement of `diagnostic_code()`. If liquid-fixpoint has a soundness bug, the refinement may report satisfaction while the property does not hold. |
| **Behavior Affecting** | true |
| **Compensating Evidence** | proptest PO-EDVBJ-010-PROPTEST fuzzes the `0x2020` constant against all 33 currently-defined constants and asserts no collision; manual review of `diagnostics.rs` enumerates the constants. |
| **Owner** | proof-planner |
| **Expiry** | None (flux is pinned via nightly) |

### TB-PROPTEST-001-shrink-engine

| Field | Value |
|-------|-------|
| **ID** | TB-PROPTEST-001-shrink-engine |
| **Trusted Kind** | `external_body` |
| **Scope** | PO-EDVBJ-003-PROPTEST, PO-EDVBJ-004-PROPTEST, PO-EDVBJ-007-PROPTEST, PO-EDVBJ-009-PROPTEST, PO-EDVBJ-010-PROPTEST |
| **Impact** | All five proptest obligations |
| **Reason** | proptest obligations depend on the shrink engine's correctness and on the strategy's `proptest::sample::select` enumerating every declared variant at least once across the case budget. |
| **Behavior Affecting** | true |
| **Compensating Evidence** | Each obligation pairs the proptest assertion with a Kani or Verus obligation on the same claim (defense-in-depth). The test-writer-owned `re_019_resumed_does_not_fabricate_run_failed` regression test pins the specific case. |
| **Owner** | proof-planner |
| **Expiry** | None (proptest is a stable ecosystem dependency) |

### TB-PROPTEST-002-fjall-semantics

| Field | Value |
|-------|-------|
| **ID** | TB-PROPTEST-002-fjall-semantics |
| **Trusted Kind** | `trusted` |
| **Scope** | PO-EDVBJ-004-PROPTEST |
| **Impact** | Temporal-replay proptest |
| **Reason** | The replay-equivalence assertion depends on `vb_storage::FjallJournal`'s `append_journaled` / `append_strict` semantics and `events_for_run(run)` correctness. If Fjall mis-records or `events_for_run` mis-filters, the replay evidence is unsound. |
| **Behavior Affecting** | true |
| **Compensating Evidence** | vb_storage's existing test suite verifies Fjall semantics independently; the test-writer-owned `re_019_resumed_does_not_fabricate_run_failed` regression test pins the specific case. |
| **Owner** | proof-planner |
| **Expiry** | None (continuously validated by CI) |

### TB-VB-RUNTIME-001-forbid-unsafe-code

| Field | Value |
|-------|-------|
| **ID** | TB-VB-RUNTIME-001-forbid-unsafe-code |
| **Trusted Kind** | `external_body` |
| **Scope** | All 11 obligations on this bead |
| **Impact** | Source-lint layer; gates Loom and Miri non-applicability claims |
| **Reason** | The `forbid(unsafe_code)` lint at the crate root gates the entire `vb_runtime` source. If the lint is removed or weakened, the non-applicability claims for `loom` and `miri` become void and require re-planning. |
| **Behavior Affecting** | true |
| **Compensating Evidence** | moon ci `lint-src` gate enforces the lint; AGENTS.md §"Engineering Rules" explicitly forbids `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg!`. |
| **Owner** | proof-planner |
| **Expiry** | None (continuously enforced) |

### TB-VB-CORE-001-symbolic-code-registry

| Field | Value |
|-------|-------|
| **ID** | TB-VB-CORE-001-symbolic-code-registry |
| **Trusted Kind** | `external_body` |
| **Scope** | PO-EDVBJ-008-FLUX, PO-EDVBJ-010-PROPTEST |
| **Impact** | Diagnostic-code finite-enum refinement |
| **Reason** | The Flux and proptest obligations on `diagnostic_code()` depend on `vb_core::HasSymbolicCode` registry and `vb_core::errors::CoreError::INTERNAL_INVARIANT_CODE` being correctly registered. The registry is verified by `vb-xi2f.10` (P1) and is upstream of this bead. |
| **Behavior Affecting** | true |
| **Compensating Evidence** | `vb-xi2f.10` (P1) verified the registry bijection and master contract parity (PS-016); the 36-code list from `velvet-ballistics-MASTER.md` §16 is treated as ground truth. |
| **Owner** | proof-planner |
| **Expiry** | None (upstream-verified) |

### TB-EXTERN-STORAGE-KIND-FAMILY-001-mirror-stable

| Field | Value |
|-------|-------|
| **ID** | TB-EXTERN-STORAGE-KIND-FAMILY-001-mirror-stable |
| **Trusted Kind** | `trusted` |
| **Scope** | PO-EDVBJ-011-VERUS (mandatory mirror-drift gate) |
| **Impact** | H-3 mitigation; Verus production-binding |
| **Reason** | The existing Verus mirror `verification/verus/extern_storage_kind_family.rs` (`MirrorJournalEvent`, `MirrorRecordKind`, `MirrorEventSeq`, `MirrorRunId`) is the production-binding anchor for this bead. The mirror is unchanged by this fix (production `JournalEvent::RunResumed` shape unchanged; no new mirror types required). The drift-detection helper at lines 670-695 resolves production method names at compile time. |
| **Behavior Affecting** | true |
| **Compensating Evidence** | Mandatory re-run of `bash scripts/check-verus-production-binding.sh` after implementation lands; if the gate fails, the merge is blocked (this is NOT a waiver path). |
| **Owner** | proof-planner |
| **Expiry** | Re-evaluated on every implementation change |

---

## 3. Assumptions by Obligation

| Obligation | Key Assumptions |
|------------|----------------|
| PO-EDVBJ-001-VERUS | The post-fix body is the source of truth; vb-cib14 lands alongside; RuntimeJournalEvent's 21-variant match is exhaustive at compile time; RuntimeResult<JournalEvent> return type is preserved |
| PO-EDVBJ-002-KANI | RuntimeJournalEvent implements kani::Arbitrary or kani::any() enumerates 21 variants; EventSeq can be generated as kani::any() within the unwind bound; the post-fix body is reachable from the harness via #[cfg(kani)]-aware crate path |
| PO-EDVBJ-003-PROPTEST | proptest@1.5 is available; the 21 declared variants are enumerated in the test fixture (not generated) |
| PO-EDVBJ-004-PROPTEST | vb_storage::FjallJournal is available in test deps; temp-dir Fjall is reset per case; the test-writer-owned `re_019_resumed_does_not_fabricate_run_failed` is added in the same change |
| PO-EDVBJ-005-VERUS | The post-fix body of `append_sequenced` uses `?` verbatim on `storage_event`; no `From<...>` conversion is added for the new variant |
| PO-EDVBJ-006-KANI | The per-layer helpers maintain their existing domain-filter shape; the post-fix dispatcher's match arm structure is unchanged from the pre-fix wildcard |
| PO-EDVBJ-007-PROPTEST | The `runtime_journal_event_kind` helper is added and its match is exhaustive over the 21 declared variants; `proptest::sample::select` produces each declared variant at least once across 10000 cases |
| PO-EDVBJ-008-FLUX | vb_runtime compiles under pinned nightly; flux-check is feature-gated; `diagnostics.rs` gains the new constant and arm; H-2 is acknowledged but NOT modified |
| PO-EDVBJ-009-PROPTEST | `QueuedStorageRuntimeJournal::append_sequenced`'s body at chunk_003.rs:8-16 is preserved (only the storage_event's post-fix body changes, not the Strict-profile gate) |
| PO-EDVBJ-010-PROPTEST | `diagnostics.rs` gains the new constant before this test runs; the 33 currently-defined DiagnosticCode constants are enumerated in the test fixture |
| PO-EDVBJ-011-VERUS | The implementation lands without renaming JournalEvent::RunResumed, MirrorJournalEvent, MirrorRecordKind, MirrorEventSeq, or MirrorRunId; vb-cib14 does not introduce any rename that would break the mirror; no new mirror type is required |

---

## 4. Model Reductions

| Reduction | Obligations | Justification |
|-----------|-------------|---------------|
| RuntimeJournalEvent bounded to 21 declared variants | PO-EDVBJ-002-KANI, PO-EDVBJ-003-PROPTEST, PO-EDVBJ-006-KANI, PO-EDVBJ-007-PROPTEST | The enum is `#[non_exhaustive]` but currently has 21 variants; Kani `unwind=8` covers with margin (per-helper match is shallow); proptest enumerates every declared variant. Future variants require updating the helpers AND the dispatcher's match — covered by H-4 mitigation. |
| EventSeq bounded to [0, 8) | PO-EDVBJ-002-KANI | Kani harness uses EventSeq::new(seq) with `seq ∈ [0, 8)` to exercise the `seq` parameter without unbounded symbolic input. Production EventSeq is u64; the 8-value bound is sufficient because the dispatcher's only use of `seq` is to thread it into the mapped JournalEvent (no arithmetic on seq). |
| DiagnosticCode constant set bounded to 34 currently-defined values | PO-EDVBJ-008-FLUX, PO-EDVBJ-010-PROPTEST | 33 existing constants + 1 new (UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE = 0x2020). Flux refines the return type to a finite enum; proptest fuzzes against the 33 existing constants to assert no collision on 0x2020. Adding a new constant requires updating both the Flux refinement and the proptest fixture. |

---

## 5. Open Trusted-Base Debt

| Item | Owner | Status | Required Resolution By |
|------|-------|--------|----------------------|
| H-2 (pre-existing 0x201F duplicate) — ADMISSION_CAPABILITY_COUNT_MISMATCH_CODE and INTROSPECTION_EPOCH_EXHAUSTED_CODE both register 0x201F | proof-planner | Deferred finding | State 12 — surface as `finding/v1` row with `disposition: blocker` OR open a separate bead |
| TB-VERUS-002-mirror-bind — production signature changes invalidate the Verus spec | proof-writer | Planned | State 5 (re-author spec) or State 11 (re-author if implementation diverges) |
| TB-EXTERN-STORAGE-KIND-FAMILY-001-mirror-stable — vb-cib14 rename breaks the mirror | proof-writer | Planned | State 5 (verify vb-cib14 does not rename JournalEvent::RunResumed) |

---

## 6. Trusted Base Acceptance Gate

All TB-* entries must be:

1. Reviewed by `proof-plan-reviewer` (State 4b) — disposition recorded in `verifier-lane-review/v1`.
2. Acknowledged by `proof-writer` (State 5) — recorded in `trusted-base-ledger/v1` rows.
3. Verified by `proof-reviewer` (State 6) — evidence in `proof-evidence.md`.
4. Closed or documented as residual risk by `formal-verifier` (State 12) — `verification-ledger/v1` rows.

Failure of any TB-* entry's compensating evidence at State 12 forces re-opening
the plan. None of the TB-* entries for this bead are behavior-affecting waivers
(validator gate `E_BEHAVIOR_WAIVER`); they are all external-body / stub / trusted
markers that name a concrete boundary and a concrete compensator.

---

## 7. Cross-Reference

- `proof-strategy.md` — strategy, risk profile, lane rationale.
- `proof-coverage-matrix.md` — obligation-to-clause traceability.
- `proof-obligations.planned.jsonl` — every obligation's `trusted_base_refs` references TB-* IDs from this document.
- `waiver-candidates.jsonl` — none for this bead (validator gate `E_BEHAVIOR_WAIVER`).
- `references/implementation-binding.md` — anti-laundering rules that govern what can be trusted.
- `references/waiver-planning-guide.md` — waiver lifecycle (not invoked for this bead).