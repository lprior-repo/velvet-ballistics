# Proof Coverage Matrix — vb-edvbj

**Bead:** vb-edvbj — Runtime: delete fallback that maps unmapped journal events to run failure (P0 bug)
**STRONG-coupled with:** vb-cib14 (must land together)
**Phase:** State 4 — Proof Planning
**Date:** 2026-07-01

---

## 1. Seed → Obligation Traceability

| Proof Seed | Requirement | Contract Clause(s) | Domain Claim | Kani | Verus | proptest | Flux |
|-----------|-------------|--------------------|--------------|------|-------|----------|------|
| RUNTIME-UNMAPPED-NO-FABRICATE | vb-edvbj | I-1 | `storage_event` does not fabricate `Ok(JournalEvent::RunFailedEvent)` for any input other than `RuntimeJournalEvent::RunFailed { run }`. | PO-EDVBJ-002 | PO-EDVBJ-001 | PO-EDVBJ-003 | — |
| RUNTIME-LAYER-HELPER-CONSISTENCY | vb-edvbj | I-2 | For every `RuntimeJournalEvent` variant, exactly one of `run_storage_event` / `action_storage_event` / `boundary_storage_event` returns `Some` OR all three return `None`; the dispatcher returns `Err(UNMAPPED)` in the all-None case. | PO-EDVBJ-006 | — | — | — |
| RUNTIME-RESUMED-NO-RUN-FAILED | vb-edvbj | I-1 | For `RuntimeJournalEvent::Resumed { run, timestamp }`, `storage_event` MUST return `Err(UnmappedRuntimeJournalEvent { event_kind: "Resumed" })` and MUST NOT produce any record of journal effect. | — | — | PO-EDVBJ-004 | — |
| RUNTIME-EVENT-KIND-ENUMERATION | vb-edvbj | I-6 | `runtime_journal_event_kind(&event)` returns one of the 21 declared variant name literals for the 21 declared variants. | — | — | PO-EDVBJ-007 | — |
| RUNTIME-DIAGNOSTIC-CODE-CONSTANT | vb-edvbj | I-11, I-12, I-13 | `RuntimeError::UnmappedRuntimeJournalEvent { .. }.diagnostic_code() == 0x2020`; `runtime_code() == None`; `symbolic_code() == INTERNAL_INVARIANT`. | — | PO-EDVBJ-005 (companion) | PO-EDVBJ-010 | PO-EDVBJ-008 |
| RUNTIME-MIRROR-DRIFT-PRESERVATION | vb-edvbj | I-3, H-3 | The Verus mirror at `verification/verus/extern_storage_kind_family.rs` continues to bind production code after the fix; no drift is introduced. | — | PO-EDVBJ-011 (mandatory gate) | — | — |
| RUNTIME-STORAGE-EVENT-PROPAGATION | vb-edvbj | I-3, I-4 | `Err(UnmappedRuntimeJournalEvent)` propagates via `?` through `storage_event` → `append_sequenced` → `RuntimeShard::append_journal_event` without caller-level rewriting. | PO-EDVBJ-007 | PO-EDVBJ-005 | — | — |
| RUNTIME-STRICT-GATE-PRESERVED | vb-edvbj | I-4 | `QueuedStorageRuntimeJournal::append_sequenced` continues to return `Err(UnsupportedAsyncStrictAck)` for `DurabilityProfile::Strict` BEFORE reaching `storage_event`. | — | — | PO-EDVBJ-009 | — |

---

## 2. Coverage Summary by Verifier

| Verifier | Obligations | Seeds Covered | Key Target |
|----------|------------|---------------|------------|
| **Verus** | PO-EDVBJ-001, PO-EDVBJ-005, PO-EDVBJ-011 | RUNTIME-UNMAPPED-NO-FABRICATE, RUNTIME-STORAGE-EVENT-PROPAGATION, RUNTIME-MIRROR-DRIFT-PRESERVATION, RUNTIME-DIAGNOSTIC-CODE-CONSTANT | `exec_storage_event` no-fabrication contract; `?`-propagation chain; mirror drift gate; `symbolic_code()` companion refinement |
| **Kani** | PO-EDVBJ-002, PO-EDVBJ-006, PO-EDVBJ-007 | RUNTIME-UNMAPPED-NO-FABRICATE, RUNTIME-LAYER-HELPER-CONSISTENCY, RUNTIME-STORAGE-EVENT-PROPAGATION | Symbolic `RuntimeJournalEvent` over the 21-variant enum; helper-matrix; `append_sequenced` propagation |
| **proptest** | PO-EDVBJ-003, PO-EDVBJ-004, PO-EDVBJ-007, PO-EDVBJ-009, PO-EDVBJ-010 | RUNTIME-UNMAPPED-NO-FABRICATE, RUNTIME-RESUMED-NO-RUN-FAILED, RUNTIME-EVENT-KIND-ENUMERATION, RUNTIME-STRICT-GATE-PRESERVED, RUNTIME-DIAGNOSTIC-CODE-CONSTANT | Exhaustive 21-variant enumeration; temporal-replay (RE-019); event_kind completeness; Strict profile gate; 0x2020 collision guard |
| **Flux** | PO-EDVBJ-008 | RUNTIME-DIAGNOSTIC-CODE-CONSTANT | `diagnostic_code()` finite-enum refinement (paired negative target) |

**Total obligations:** 11 (3 Verus + 3 Kani + 5 proptest + 1 Flux; some
obligations cover multiple seeds via shared `proof_seed_id` references).

**Required by task description:** 5–8 obligations. We plan 11 to ensure
defense-in-depth per `references/defense-depth-matrix.md`; each obligation is
narrow-scoped to a single seed-and-verifier pair so the planner's coverage is
traceable. The bead's `proof-plan-reviewer` may consolidate pairs if the
reviewer deems them redundant.

---

## 3. Requirement Coverage

| Requirement ID | Contract Clause(s) | Seeds | Obligations | Status |
|----------------|--------------------|-------|-------------|--------|
| vb-edvbj | I-1 | RUNTIME-UNMAPPED-NO-FABRICATE, RUNTIME-RESUMED-NO-RUN-FAILED | PO-EDVBJ-001, PO-EDVBJ-002, PO-EDVBJ-003, PO-EDVBJ-004 | Covered |
| vb-edvbj | I-2 | RUNTIME-LAYER-HELPER-CONSISTENCY | PO-EDVBJ-006 | Covered |
| vb-edvbj | I-3 | RUNTIME-STORAGE-EVENT-PROPAGATION, RUNTIME-MIRROR-DRIFT-PRESERVATION | PO-EDVBJ-005, PO-EDVBJ-007, PO-EDVBJ-011 | Covered |
| vb-edvbj | I-4 | RUNTIME-STORAGE-EVENT-PROPAGATION, RUNTIME-STRICT-GATE-PRESERVED | PO-EDVBJ-005, PO-EDVBJ-007, PO-EDVBJ-009 | Covered |
| vb-edvbj | I-6 | RUNTIME-EVENT-KIND-ENUMERATION | PO-EDVBJ-007 (proptest half) | Covered |
| vb-edvbj | I-11, I-12, I-13 | RUNTIME-DIAGNOSTIC-CODE-CONSTANT | PO-EDVBJ-005, PO-EDVBJ-008, PO-EDVBJ-010 | Covered |

---

## 4. Risk Tag Coverage

| Risk Tag | Seeds | Primary Lanes |
|----------|-------|---------------|
| `persistence` | RUNTIME-UNMAPPED-NO-FABRICATE, RUNTIME-RESUMED-NO-RUN-FAILED | Kani + Verus + proptest (replay) |
| `temporal` | RUNTIME-UNMAPPED-NO-FABRICATE, RUNTIME-RESUMED-NO-RUN-FAILED | Verus + proptest (replay); loom n/a |
| `public_api` | RUNTIME-LAYER-HELPER-CONSISTENCY, RUNTIME-STORAGE-EVENT-PROPAGATION, RUNTIME-STRICT-GATE-PRESERVED, RUNTIME-DIAGNOSTIC-CODE-CONSTANT | Kani + Verus + proptest + Flux |
| `parser/codec` | RUNTIME-LAYER-HELPER-CONSISTENCY, RUNTIME-EVENT-KIND-ENUMERATION | Kani + proptest |
| `recovery` | RUNTIME-RESUMED-NO-RUN-FAILED | proptest (temporal-replay) |
| `telemetry` | RUNTIME-DIAGNOSTIC-CODE-CONSTANT | Flux + proptest |
| `verification_artifacts` | RUNTIME-MIRROR-DRIFT-PRESERVATION | Verus (mandatory gate) |

---

## 5. Forbidden-State Coverage

The contract's forbidden post-fix states (from `contract.md` §"Forbidden post-fix states"):

| Forbidden State | Forbidden By | Covered By |
|-----------------|--------------|------------|
| `Ok(JournalEvent::RunFailedEvent)` produced from a non-`RunFailed` `RuntimeJournalEvent`. | I-1, I-2 | PO-EDVBJ-001, PO-EDVBJ-002, PO-EDVBJ-003, PO-EDVBJ-006 |
| `Ok(JournalEvent::RunFailedEvent)` produced silently without operator-visible error logging when `storage_event` returns `Err`. | I-1, I-3 | PO-EDVBJ-001, PO-EDVBJ-002, PO-EDVBJ-005 |
| Successful dispatch write to Fjall of any `JournalEvent` whose variant does not correspond to a mapped `RuntimeJournalEvent` arm. | I-1, I-3 | PO-EDVBJ-001, PO-EDVBJ-002, PO-EDVBJ-004, PO-EDVBJ-005 |

---

## 6. Non-Applicable Lane Coverage

| Lane | Seeds Affected | Reason | Evidence |
|------|---------------|--------|----------|
| `loom` | All 8 | Sync codepath; no concurrency surface. | `boundary-map.md` §"Async / sync cross-check"; `hazard-analysis.md` H-7. |
| `miri` | All 8 | `forbid(unsafe_code)`; no FFI; no provenance. | `boundary-map.md` §"Unsafe" row. |
| `cargo-fuzz` | All 8 | No parser/codec/hostile-input boundary on dispatcher. | `boundary-map.md` §"Parser / codec" row. |
| `flux-rs` (on `storage_event`) | RUNTIME-UNMAPPED-NO-FABRICATE | Value-level invariant over 21-variant `match`; not a refinement type. | `references/verifier-trigger-matrix.md` Flux row. |

---

## 7. Defense-in-Depth Layering

```
Layer 0 (Compile-time, source-lint):
  ├── forbid(unsafe_code) in vb_runtime ........................ enforced by moon ci
  ├── no panic / unwrap / expect / todo / unimplemented / dbg! .. enforced by check-panic-surface.sh
  └── STORAGE_EVENT_CLONE_COUNT = 1 invariant ................... chunk_002.rs:310-312

Layer 1 (Verus deductive, exec-fn binding):
  ├── exec_storage_event requires/ensures contract ............... PO-EDVBJ-001
  ├── ?-propagation chain refinement ............................. PO-EDVBJ-005
  └── Verus production-binding gate (H-3 mirror drift) .......... PO-EDVBJ-011 (mandatory)

Layer 2 (Kani bounded symbolic, split-harness):
  ├── kani_run_layer_no_fabricate ............................... PO-EDVBJ-002 (part)
  ├── kani_action_layer_no_fabricate ............................ PO-EDVBJ-002 (part)
  ├── kani_boundary_layer_no_fabricate .......................... PO-EDVBJ-002 (part)
  ├── kani_dispatch_no_fabricate ................................ PO-EDVBJ-002 (part)
  ├── kani_layer_consistency (helper matrix) .................... PO-EDVBJ-006
  └── kani_append_sequenced_propagation (? chain) ............... PO-EDVBJ-007

Layer 3 (proptest property pressure):
  ├── proptest_dispatch_all_21_variants ......................... PO-EDVBJ-003
  ├── proptest_resumed_temporal_replay (RE-019) ................. PO-EDVBJ-004
  ├── proptest_event_kind_enumeration ........................... PO-EDVBJ-007 (proptest half)
  ├── proptest_strict_profile_gate .............................. PO-EDVBJ-009
  └── proptest_diagnostic_code_constant ......................... PO-EDVBJ-010

Layer 4 (Flux refinement, source-level):
  └── diagnostic_code() finite-enum refinement .................. PO-EDVBJ-008
      └── paired negative target: diagnostic_code_unmapped_returns_0x2020_negative

Layer 5 (cargo test, behavior & source-lint):
  ├── cargo test --test storage_event_clones_the_event_exactly_once_per_dispatch
  ├── cargo test --test storage_runtime_journal_maps_cancelled_and_failed_events
  ├── cargo test --test re_019_resumed_does_not_fabricate_run_failed (test-writer owned)
  └── moon ci :rust-verification-gauntlet (aggregate)
```

---

## 8. STRONG Coupling with vb-cib14

This bead is STRONG-coupled with vb-cib14 per the bead description. Both beads
must land together. The proof obligations here assume vb-cib14's implementation
landed (or is landing in the same JJ change). If vb-cib14 changes:

| vb-cib14 Change | Required Replan |
|------------------|-----------------|
| Rename of `JournalEvent::RunResumed` | Rerun `bash scripts/check-verus-production-binding.sh`; if the gate fails, PO-EDVBJ-011 fails and the bead's obligations must re-plan with the new mirror. |
| Change to `RuntimeError` signature | PO-EDVBJ-001 and PO-EDVBJ-005 must re-plan to bind the new signature. |
| Change to `runtime_journal_event_kind` return type | PO-EDVBJ-007 (proptest half) must re-plan to assert the new return. |
| Change to `EventSeq` representation | PO-EDVBJ-002 and PO-EDVBJ-007 must re-plan to bind the new representation. |

The proof-plan-reviewer is expected to verify the coupling during disposition
(State 4b) and reject the plan if vb-cib14's artifacts are not aligned.

---

## 9. Self-Audit Checklist

- [x] Every `(requirement_id, contract_clause, proof_seed_id, verifier)` tuple in the default profile has exactly one lane decision.
- [x] Every required lane decision names at least one `proof-obligation/v1` ID, and that ID exists in `proof-obligations.planned.jsonl`.
- [x] Every behavior-affecting obligation has a paired `rust-refinement-obligation/v1` stub requirement (handled at `proof-to-implementation`, State 7).
- [x] No two rows duplicate `(requirement_id, contract_clause, proof_seed_id, verifier)` with conflicting `applicability`.
- [x] All `not_applicable` rows have a typed `limitation_kind`.
- [x] All `decision_reason` strings cite concrete `risk_tags` and avoid the weak vocabulary.
- [x] Forbidden post-fix states from `contract.md` are explicitly covered by named obligations.
- [x] STRONG coupling with vb-cib14 is documented in the plan; failure of the mirror gate (PO-EDVBJ-011) blocks the merge.