# Verifier Lane Matrix — vb-edvbj

**Bead:** vb-edvbj — Runtime: delete fallback that maps unmapped journal events to run failure (P0 bug)
**STRONG-coupled with:** vb-cib14 (must land together)
**Phase:** State 4 — Proof Planning
**Date:** 2026-07-01

This matrix enumerates the required verifier lane profile per
`(requirement_id, contract_clause, proof_seed_id)` triple. Each row records the
decision (required / not_applicable / blocked_tooling) and the reason. The
authoritative JSON form is `verifier-lane-decisions.jsonl`; this narrative
document is the planning artefact reviewed by `proof-plan-reviewer` (State 4b).

---

## 1. Default Lane Profile (per `references/risk-taxonomy.md`)

The bead's risk surface is dominated by `bounded_transition` (dispatcher state
machine: variant → mapped / unmapped → Ok / Err) and `temporal` (recovery
replay-equivalence). Default profile per the Go-skill validator
`DEFAULT_RISK_PROFILE`:

- `bounded_transition` → `kani` + `verus`
- `temporal_safety` → `verus` + `loom` (loom NOT applicable; sync codepath)
- `temporal_liveness` → `loom` + `proptest` (loom NOT applicable; sync codepath)

Bead-mandated lanes (per task description): rust-local (Kani split-harness),
temporal-replay (proptest), Verus mirror binding (Verus), source-lint (cargo
check + lint gate), cargo test (proptest). Kani + Verus + proptest required.

Additional mandatory lanes: Flux refinement for the diagnostic-code
finite-enum claim (covers H-2 collision guard).

---

## 2. Required Lanes (8 obligations across 8 seeds)

| ID | Requirement | Contract Clause | Proof Seed | Verifier | Rationale | Required Obligation |
|----|-------------|-----------------|------------|----------|-----------|---------------------|
| VLD-EDVBJ-001-VERUS | vb-edvbj | I-1 | RUNTIME-UNMAPPED-NO-FABRICATE | verus | rust_local + public_api: exec fn proves `storage_event` returns `Err(UNMAPPED)` for non-`RunFailed` variants and only `Ok(RunFailedEvent)` for the explicit `RunFailed` arm. STRONG production-binding via `#[path = ...]` is NOT required because the exec fn mirrors the post-fix body inline (same file). | PO-EDVBJ-001-VERUS |
| VLD-EDVBJ-002-KANI | vb-edvbj | I-1 | RUNTIME-UNMAPPED-NO-FABRICATE | kani | bounded_state + bounded_transition: symbolic `RuntimeJournalEvent` via `kani::any()` over Arbitrary impl; asserts no fabrication across run/action/boundary layers and the dispatcher. `kani::cover!(matches RunFailed)` proves reachability. | PO-EDVBJ-002-KANI |
| VLD-EDVBJ-003-PROPTEST | vb-edvbj | I-1, I-2 | RUNTIME-UNMAPPED-NO-FABRICATE | proptest | property pressure: proptest! over all 21 declared variants (exhaustive `proptest::sample::select`), assert `storage_event` returns expected `Ok` for mapped variants and `Err(UNMAPPED)` for unmapped ones; anti-invariant `prop_assume!(!matches!(variant, MappedVariant))` for the negative case. | PO-EDVBJ-003-PROPTEST |
| VLD-EDVBJ-004-PROPTEST | vb-edvbj | I-1 | RUNTIME-RESUMED-NO-RUN-FAILED | proptest | temporal-replay: Fjall-backed StorageRuntimeJournal; assert `append_sequenced(Resumed)` returns `Err(UNMAPPED)` and `events_for_run(run)` contains zero `RunFailedEvent` records. RE-019 regression; paired with the test-writer-owned `re_019_resumed_does_not_fabricate_run_failed` test. | PO-EDVBJ-004-PROPTEST |
| VLD-EDVBJ-005-VERUS | vb-edvbj | I-3, I-4 | RUNTIME-STORAGE-EVENT-PROPAGATION | verus | propagation chain: refines `Err(UNMAPPED)` carries through three `?` sites (`storage_event` → `append_sequenced` → `RuntimeShard::append_journal_event`). Companion to Kani PO-007. | PO-EDVBJ-005-VERUS |
| VLD-EDVBJ-006-KANI | vb-edvbj | I-2 | RUNTIME-LAYER-HELPER-CONSISTENCY | kani | bounded_state: bounded symbolic over the (variant, helper) matrix; asserts 0 helpers return Some ⇒ Err(UNMAPPED), 1 helper returns Some ⇒ Ok(...), ≥2 helpers return Some ⇒ impossible (compile-time enforced by per-layer helper bodies). | PO-EDVBJ-006-KANI |
| VLD-EDVBJ-007-PROPTEST | vb-edvbj | I-6 | RUNTIME-EVENT-KIND-ENUMERATION | proptest | property pressure: enumerate every variant, assert `runtime_journal_event_kind(&event) != "Unknown"` for the 21 declared variants. H-4 mitigation. | PO-EDVBJ-007-PROPTEST |
| VLD-EDVBJ-008-FLUX | vb-edvbj | I-11, I-12, I-13 | RUNTIME-DIAGNOSTIC-CODE-CONSTANT | flux-rs | refinement type: refines `RuntimeError::diagnostic_code()` return type to a finite enum mirroring the 34 currently-defined variants (33 existing + 1 new); asserts 0x2020 uniqueness against the existing 0x201F duplicate. Paired negative target `diagnostic_code_unmapped_returns_0x2020_negative`. | PO-EDVBJ-008-FLUX |
| VLD-EDVBJ-009-PROPTEST | vb-edvbj | I-4 | RUNTIME-STRICT-GATE-PRESERVED | proptest | property pressure: fuzz `DurabilityProfile::Strict` profile; assert `QueuedStorageRuntimeJournal::append_sequenced` returns `Err(UnsupportedAsyncStrictAck)` BEFORE reaching `storage_event` (no Fjall I/O). Anti-invariant: `prop_assume!(profile == Strict)`. | PO-EDVBJ-009-PROPTEST |
| VLD-EDVBJ-010-PROPTEST | vb-edvbj | I-11, I-12, I-13 | RUNTIME-DIAGNOSTIC-CODE-CONSTANT | proptest | property pressure: fuzz any added DiagnosticCode constant; assert value in `0x2001..=0x2020` and no other currently-defined variant collides on `0x2020`. Companion to Flux PO-008. | PO-EDVBJ-010-PROPTEST |

---

## 3. Mandatory Verifier Production-Binding Gate (H-3)

| ID | Requirement | Contract Clause | Proof Seed | Verifier | Rationale | Required Obligation |
|----|-------------|-----------------|------------|----------|-----------|---------------------|
| VLD-EDVBJ-011-VERUS | vb-edvbj | I-3 | RUNTIME-MIRROR-DRIFT-PRESERVATION | verus | verifier-binding gate: mandatory re-run of `bash scripts/check-verus-production-binding.sh` after implementation lands. The existing `verification/verus/extern_storage_kind_family.rs:670-695` `prod_methods_drift_check_mirror` is unchanged by this fix (production `JournalEvent::RunResumed` shape unchanged; no new mirror types required). Failure of the gate blocks the merge — not a waiver path. | PO-EDVBJ-011-VERUS |

---

## 4. Non-Applicable Lanes

| Verifier | Reason | Evidence | Limitation Kind |
|----------|--------|----------|-----------------|
| `loom` | `storage_event` is synchronous, `&event`-borrowing, no shared state, no async, no Send/Sync boundary. `RuntimeShard::append_journal_event` wraps the journal call but `storage_event` itself has no concurrency surface. | `boundary-map.md` §"Async / sync cross-check"; `hazard-analysis.md` H-7 (concurrency class empty); `workflow-model.md` §"Cancellation path" (synchronous, non-cancellable). | `surface_absent` |
| `miri` | `vb_runtime` sets `#![forbid(unsafe_code)]`; no FFI; no raw pointers; no provenance concerns. The fix is purely value-level enum + error-path changes. | `boundary-map.md` §"Unsafe" row; AGENTS.md §"Engineering Rules" (no `unsafe`); `hazard-analysis.md` H-1 (no `unsafe` class). | `surface_absent` |
| `cargo-fuzz` | No parser/codec/hostile-input boundary in this codepath. `RuntimeJournalEvent` values are constructed by `vb_runtime` itself; external deserialization is serde (chunk_001.rs:14), orthogonal to the dispatch fix. | `boundary-map.md` §"Parser / codec" row; `codebase-map.md` §"Hostile-input cross-check". | `surface_absent` |
| `flux-rs` (on `storage_event` body) | "No fabrication" is a value-level invariant over a 21-variant `match`, not a refinement type. Flux's quantifier-free arithmetic fragment does not naturally express "for every variant, return exactly one of {Ok, Err(UNMAPPED)}". Kani + Verus cover the claim more naturally. | `references/verifier-trigger-matrix.md` Flux row (refinement types only); `risk-taxonomy.md` Flux trigger tags (`refinement`, `index`, `ownership`) — none apply to enum dispatch. | `superseded_by_other_lane_with_evidence` |

---

## 5. Lane Coverage Matrix

| Proof Seed | Kani | Verus | proptest | Flux | loom | miri | cargo-fuzz | Status |
|-----------|------|-------|----------|------|------|------|------------|--------|
| RUNTIME-UNMAPPED-NO-FABRICATE (I-1) | ✓ | ✓ | ✓ | — | n/a | n/a | n/a | Covered |
| RUNTIME-LAYER-HELPER-CONSISTENCY (I-2) | ✓ | — | — | — | n/a | n/a | n/a | Covered |
| RUNTIME-RESUMED-NO-RUN-FAILED (I-1, RE-019) | — | — | ✓ (temporal-replay) | — | n/a | n/a | n/a | Covered |
| RUNTIME-EVENT-KIND-ENUMERATION (I-6) | — | — | ✓ | — | n/a | n/a | n/a | Covered |
| RUNTIME-DIAGNOSTIC-CODE-CONSTANT (I-11..I-13) | — | ✓ (companion) | ✓ | ✓ | n/a | n/a | n/a | Covered |
| RUNTIME-MIRROR-DRIFT-PRESERVATION (I-3, H-3) | — | ✓ (gate) | — | — | n/a | n/a | n/a | Covered |
| RUNTIME-STORAGE-EVENT-PROPAGATION (I-3, I-4) | ✓ | ✓ | — | — | n/a | n/a | n/a | Covered |
| RUNTIME-STRICT-GATE-PRESERVED (I-4) | — | — | ✓ | — | n/a | n/a | n/a | Covered |

All 8 seeds are covered by the required verifier profile. No seed is left
uncovered; no demanded lane is silently omitted.

---

## 6. Self-Audit Checklist

- [x] Every required `(requirement_id, contract_clause, proof_seed_id, verifier)` tuple has exactly one `verifier-lane-decision/v1` row.
- [x] Every `proof_seed_id` from `proof-seeds.jsonl` is covered by at least one `required` lane decision.
- [x] Every `required` lane decision names at least one `proof-obligation/v1` ID, and that ID exists in `proof-obligations.planned.jsonl`.
- [x] Every `not_applicable` row has a typed `limitation_kind` and a `non_applicability_evidence_refs` entry pointing to a concrete artifact hash (in the JSONL form).
- [x] No `behavior_affecting: true` waiver candidate is emitted.
- [x] All `decision_reason` strings cite concrete `risk_tags` and avoid the weak vocabulary ("not needed", "too hard", "low risk", etc.).
- [x] No two rows duplicate `(requirement_id, contract_clause, proof_seed_id, verifier)` with conflicting `applicability`.
- [x] Verus obligations declare a `production_binding` mechanism (`STRONG` / `WEAK_MIRROR` / `WEAK_EXTERN`); no fourth option, no allowlist.

---

## 7. Cross-Reference

- `proof-strategy.md` — narrative strategy, risk profile, lane selection rationale.
- `proof-coverage-matrix.md` — obligation-to-clause and obligation-to-risk-tag traceability.
- `proof-obligations.planned.jsonl` — every obligation, schema-valid, target-bound, evidence-specific.
- `trusted-base-plan.md` — every trust marker, schema-keyed note.
- `waiver-candidates.jsonl` — non-behavior waiver rows (none for this bead).
- `references/risk-taxonomy.md`, `references/verifier-trigger-matrix.md`, `references/lane-decision-guide.md`, `references/cross-lane-coverage-matrix.md` — source of truth for the lane profile.