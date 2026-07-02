# Proof Strategy — vb-09aaz

bead_id: vb-09aaz
title: Storage: abort write batch on index key construction failures (P1)
state: 4 (proof-planner, planning only)
controller: femdation
isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz
current_state: 4
owner_state: 4
handoff_target: proof-plan-reviewer (State 4b), then proof-writer (State 5)

## 1. Scope & Blasted Surface

The bead targets exactly one production method: `JournalWriteBatch::append_event`
at `crates/vb_storage/src/batch/append_event.rs:42-121`. The fix is the G8
IndexKeyConstruction guard at L114-115. All other production sites are
explicitly excluded from modification:

- `crates/vb_storage/src/queue/writer.rs:152-231` and
  `crates/vb_storage/src/queue/writer/stage.rs:31-74` (queued-writer
  single-shot path — REVIEW ONLY, no fix).
- `crates/vb_storage/src/journal/internal.rs:50-79` (direct-path
  `append_unfsynced` — REVIEW ONLY, no fix).
- `crates/vb_storage/src/batch/putters.rs` (already canonical, 28 occurrences
  of `self.aborted = true` — REFERENCE ONLY).
- `crates/vb_storage/src/batch/commit.rs:20-23` (commit short-circuit
  consumer — UNCHANGED).
- `crates/vb_storage/src/error/mod.rs:28-29` (KeyCapacity variant — UNCHANGED,
  no new variant introduced).

The Verus spec blast radius is `verification/verus/vb-vzcuf-PS-008.rs` and
`verification/verus/vb-vzcuf-PS-009.rs`, plus the two production mirrors
`verification/verus/production_inner/vb_vzcuf_PS_008_production.rs` and
`_PS_009_production.rs`. Drift-gate headers in those mirror files force
regeneration on production change.

The test blast radius is a single new test in
`crates/vb_storage/src/batch/t_append_event.rs` mirroring
`batch_index_key_error_aborts_commit` at
`crates/vb_storage/src/batch/t_putters_b.rs:177-209`, plus an optional
proptest extension.

## 2. Risk Classification (before lane selection)

| Risk tag | Severity | Source | Lane assignment |
| --- | --- | --- | --- |
| persistence / partial-write | P1 | hazard-analysis.md#H1 | verus (PS-001/008) + persistence integration (PS-006) |
| rust-local / abort invariant | high | hazard-analysis.md#H2 | verus + proptest (PS-001/002) |
| verifier-binding / spec drift | high | hazard-analysis.md#H3 | verus WEAK_EXTERN mirror update (PS-003) |
| public-api doc drift | low | hazard-analysis.md#H4 | documentation review (PS-004) — non-proof-bearing |
| test coverage gap | high | hazard-analysis.md#H9 | rust-local regression test + proptest (PS-002/005) |
| public-api migration | low | hazard-analysis.md#H11 | api-surface-check (PS-007) — non-behavior-affecting |
| mirror regeneration race | medium | hazard-analysis.md#H12 | check-verus-production-binding + check-production-inner-drift |
| concurrency | none | boundary-map.md#async | not_applicable (PhantomData<*mut FjallJournal>) |
| unsafe / UB | none | boundary-map.md#unsafe | not_applicable (#![forbid(unsafe_code)]) |
| arithmetic overflow | none | workflow-model.md#KeyCapacity | not_applicable (production encoding fits 13 bytes exactly; defensive only) |
| temporal / state machine | none | workflow-model.md#typestates | not_applicable (single-threaded state machine, no TLA+ lane exists) |

## 3. Verifier Lane Selection

### 3.1 Active lanes

| Lane | Justification | Risk addressed |
| --- | --- | --- |
| **verus (WEAK_EXTERN mirror update)** | PS-008 and PS-009 currently model only 7 guards. The G8 fix requires adding a new guard index, new exec input `index_key_ok: bool`, new Err(KeyCapacity) match arm in `assume_specification`, and a new exec wrapper `wrapper_append_event_index_key_error`. The mirror must be regenerated; PS-008 production mirror at L88 declares KeyCapacity unreachable, which becomes reachable once G8 is added. | persistence, rust-local, verifier-binding |
| **rust-local** (proptest + integration test) | The regression test `batch_append_event_index_key_error_aborts_commit` is a contract-parity test that exercises the abort-on-fallible-step invariant on the G8 path. Mirrors `batch_index_key_error_aborts_commit` (t_putters_b.rs:177-209). | persistence, public-api |
| **persistence** (master §49 integration) | End-to-end test that a committed batch with a G8-failed previous event leaves `events_for_run(run).is_empty()`. Uses real Fjall database instance. | persistence / crash-consistency |

### 3.2 Not-applicable lanes

| Lane | Reason | Evidence |
| --- | --- | --- |
| **kani** (in scope for rust-local bounded model checking, but no new harness) | Existing Kani coverage at `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs` exercises G3 durable-duplicate abort path; reusing that harness style for G8 would require mutating the `SpecJournalWriteBatch` mirror with a new `index_key_ok: bool` argument. The WEAK_EXTERN mirror update is the stronger verification (binds to production) and replaces the need for a parallel Kani harness for this single-guard delta. Kani is not silent; the existing harness continues to cover G3, and the new G8 guard inherits from the Verus mirror's `assume_specification` contract. | codebase-map.md#no-fuzz-no-mutation + boundary-map.md#verifier-boundary |
| **loom** | `JournalWriteBatch` is `!Send + !Sync` via `PhantomData<*mut FjallJournal>` (types.rs:18-21). No cross-thread aliasing possible. | boundary-map.md#async-concurrency-boundary |
| **miri** | `#![forbid(unsafe_code)]` at append_event.rs:1 applies to the entire `vb_storage` crate. No unsafe blocks, no FFI, no raw pointers. | boundary-map.md#unsafe-ffi-boundary |
| **cargo-fuzz** | The G8 fix is a 1-line replacement of `?` with `map_err`. No parser, no codec, no hostile byte boundary at this layer. The defensive `KeyCapacity` is unreachable under nominal `ActionId/RunId/StepIdx` triples. | boundary-map.md#parser-codec-boundary |
| **flux-rs** | No refinement types in the batch layer. The Verus mirror is the primary refinement-style proof; Flux would not add coverage. | boundary-map.md#verifier-boundary |
| **tla-plus** | Lane removed from this skill. Temporal/state-machine behavior is covered by loom + proptest (here, just proptest). | skill SKILL.md — TLA+ removed |

### 3.3 Default-profile lane rationale

The default profile per skill is **verus + kani + flux + proptest** for
behavior-affecting seeds. In this bead:

- **verus**: required (WEAK_EXTERN mirror update; production-binding gate
  mandated by AGENTS.md).
- **kani**: not_applicable. The proof writer will document in the mirror's
  TRUST BOUNDARY section that no Kani harness is added for G8 because the
  WEAK_EXTERN mirror update covers the guard-ordering proof, and adding a
  parallel Kani harness for a single-guard delta would duplicate the
  `SpecJournalWriteBatch` mirror with the new `index_key_ok: bool` argument.
  This is documented in non_applicability_evidence_refs with concrete
  source-line citations.
- **flux-rs**: not_applicable. No refinement types in scope.
- **proptest**: required (PS-005 — arbitrary ActionId/RunId/StepIdx triples).

## 4. Lane Profile Summary

```
verus:        required (WEAK_EXTERN mirror update; PS-001, PS-003, PS-008)
kani:         not_applicable (existing harness covers G3; G8 covered via verus mirror)
flux-rs:      not_applicable (no refinement types in batch layer)
proptest:     required (PS-005)
persistence:  required (PS-006 integration test for master §49)
loom:         not_applicable (PhantomData<*mut FjallJournal>)
miri:         not_applicable (#![forbid(unsafe_code)])
cargo-fuzz:   not_applicable (no parser/codec boundary)
tla-plus:     not_applicable (lane removed)
```

## 5. Production Binding Plan (MANDATORY for Verus)

Every Verus obligation in this plan uses **WEAK_EXTERN** mechanism
(via `#[path = "production_inner/vb_vzcuf_PS_008_production.rs"]` and
`#[path = "extern_vb_vzcuf_PS_008.rs"]`). This is the existing pattern for
PS-008/PS-009. The Verus obligation is bound to:

- production mirror path: `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs`
- extern file path: `verification/verus/extern_vb_vzcuf_PS_008.rs` (analogous for PS-009)
- drift gate: `scripts/check-production-inner-drift.sh` (zero-tolerance)
- production-binding gate: `scripts/check-verus-production-binding.sh` (mandatory)

The drift-gate header in
`verification/verus/production_inner/vb_vzcuf_PS_008_production.rs:7-14` and
`_PS_009_production.rs:5-32` MUST be honored. The proof-writer MUST regenerate
both mirrors as part of the fix.

## 6. Obligation Plan (4-5 obligations)

The plan emits **5 obligations**, all behavior-affecting except
PO-09aaz-005 (which is the public API surface review).

| ID | Verifier | Requirement | Proof seed | Behavior-affecting | Status |
| --- | --- | --- | --- | --- | --- |
| PO-09aaz-001 | verus | C1, C2, C4, C7 (G8 abort invariant + guard precedence + mirror update) | PS-001, PS-003, PS-008 | yes | planned |
| PO-09aaz-002 | rust-local (proptest/integration-test) | C4, C8 (regression test mirrors t_putters_b.rs:177-209) | PS-002 | yes | planned |
| PO-09aaz-003 | proptest | C8 (proptest variant with arbitrary triples) | PS-005 | yes | planned |
| PO-09aaz-004 | persistence (integration-test) | C4, C5 (master §49 crash-consistency) | PS-006 | yes | planned |
| PO-09aaz-005 | rust-local (api-surface-check, doc-review) | C6, C9 (API stability + doc-comment update) | PS-004, PS-007 | no | planned |

PO-09aaz-005 is the only non-behavior-affecting obligation. It exists to
record the API-surface review (signature unchanged, error variant unchanged)
and the doc-comment update (Guard Precedence and Postconditions). It carries
no proof closure requirement — only a documentation gate.

## 7. Execution Plan (downstream)

State 5 (proof-writer) is the next stop. It must:

1. Add the `index_key_ok: bool` exec arg to the
   `SpecJournalWriteBatch::append_event` signature in both PS-008 and PS-009
   production mirrors (regenerate mirrors with G8 enumeration).
2. Add a new match arm for `Err(KeyCapacity)` from G8 in
   `assume_specification` requiring
   `spec_state_preserved_except_aborted(*old(batch), *final(batch))` with
   witness `!index_key_ok`.
3. Add a new exec wrapper `wrapper_append_event_index_key_error` to exercise
   G8 from `verus!` context.
4. Write the regression test in `crates/vb_storage/src/batch/t_append_event.rs`.
5. Write the proptest variant (or extend `proptest_vb_vzcuf_PS_004.rs`).
6. Write the master §49 integration test using a real Fjall instance.
7. Run `bash scripts/verify-verus.sh`,
   `bash scripts/check-verus-production-binding.sh`,
   `bash scripts/check-production-inner-drift.sh`.

## 8. Handoff

- proof-plan-reviewer (State 4b): dispositions each lane decision; may reject
  the lane profile, obligation plan, or production-binding mechanism.
- proof-writer (State 5): authors the proof artifacts per approved plan.
- proof-reviewer (State 6): adversarial review of written proof artifacts.
- formal-verifier (State 12): executes verifier commands and closes ledger.

This planner does NOT claim proof success, approval, or PASS. Disposition
is the reviewer's; closure is the formal verifier's.

## 9. Forbidden / Out-of-Scope

- MUST NOT touch queued-writer path (`queue/writer.rs`, `queue/writer/stage.rs`).
- MUST NOT touch direct-path `append_unfsynced` (`journal/internal.rs`).
- MUST NOT modify `putters.rs` (canonical reference pattern, 28 occurrences).
- MUST NOT modify `commit.rs` (short-circuit consumer).
- MUST NOT introduce new `JournalError` variant; reuse `KeyCapacity` unit
  variant at `error/mod.rs:28-29`.
- MUST NOT change the public API signature of `append_event`, `is_aborted`,
  `commit`.
- MUST NOT add new fields to `JournalWriteBatch` (`types.rs:21-30`).
- MUST NOT write production Rust, tests, or verifier artifacts in this state.