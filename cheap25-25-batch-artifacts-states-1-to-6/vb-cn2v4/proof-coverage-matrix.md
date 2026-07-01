# Proof Coverage Matrix: vb-cn2v4 — Keys reject zero RunId (P1 bug)

This matrix is the planner's narrative map of every contract clause to
the obligations that prove it. It is the bridge between the contract
(`contract.md`, `type-contracts.md`) and the obligations
(`proof-obligations.planned.jsonl`).

## Coverage by Contract Clause

| CC | Clause title | Seed(s) | Primary obligation(s) | Companion obligation(s) | Behavior-affecting |
|---|---|---|---|---|---|
| **C1** | Encoder/Decoder Symmetry (encoder rejects RunId(0)) | `vb-cn2v4-seed-001` | `PO-001-VERUS-MIRROR` (unbounded Verus) | `PO-003-KANI-SPLIT-HARNESS` (bounded symbolic) | `false` |
| **C1** | Encoder/Decoder Symmetry (per-prefix property) | `vb-cn2v4-seed-001` | `PO-005-PROPTEST-PER-PREFIX` (rust-local property) | `PO-006-PROPTEST-MUTATION` (mutation resistance) | `false` |
| **C2** | Shared Guard Helper (require_non_zero_run) | `vb-cn2v4-seed-002` | `PO-005-PROPTEST-PER-PREFIX` (call-graph coverage) | `PO-006-PROPTEST-MUTATION` (guard not removable) | `false` |
| **C3** | Error Reuse (no new variant) | `vb-cn2v4-seed-006` | `PO-005-PROPTEST-PER-PREFIX` (asserts exact `InvalidRunId` variant) | (covered by `tests.rs::symbolic_code_table`) | `false` |
| **C4** | Manual Check in `headers.rs::run_header` (defence-in-depth) | (no seed) | (no obligation; both shapes contract-compliant) | (no obligation) | `false` |
| **C5** | Test Suite Flip (18 tests) | `vb-cn2v4-seed-007` | `PO-005-PROPTEST-PER-PREFIX` (rust-local layer) | (test-writer owns the 18 flips; proof covers per-prefix property) | `false` |
| **C6** | Kani Harness Split | `vb-cn2v4-seed-004` | `PO-003-KANI-SPLIT-HARNESS` (split into rejection + happy) | `PO-004-KANI-ORDER-OF-CHECKS` (order-of-checks for index_status_key) | `false` |
| **C7** | Verus Mirror Variant (`SpecKeyEncodeError::InvalidRunId`) | `vb-cn2v4-seed-005` | `PO-001-VERUS-MIRROR` (mirror + assume_specification) | `PO-002-VERUS-DECODER-SYMMETRY` (mirror body + drift gate) | `false` |
| **C8** | Decoder Unchanged (encoder/decoder symmetry) | `vb-cn2v4-seed-003` | `PO-002-VERUS-DECODER-SYMMETRY` (unbounded) | `PO-003-KANI-SPLIT-HARNESS` (bounded) | `false` |
| **C9** | Out-of-Scope Surfaces Preserved | (no seed) | (no obligation; out-of-scope surfaces are not touched) | (no obligation) | `false` |
| workflow | Error Surface Shift at Journal Append Call Sites | `vb-cn2v4-seed-008` | `PO-005-PROPTEST-PER-PREFIX` (per-prefix property covers the 19 call sites via the public encoder entry points) | (no obligation) | `false` |

## Coverage by Risk

| Risk class | Required by risk_tags | Lanes used | Obligations |
|---|---|---|---|
| `rejection` | `rejection`, `rust_local`, `parser_codec` | `verus` + `kani` + `proptest` | `PO-001`, `PO-002`, `PO-003`, `PO-004`, `PO-005`, `PO-006` |
| `equality` (round-trip) | `round_trip`, `invariant` | `verus` (primary) + `kani` (companion) | `PO-002`, `PO-003` |
| `parse_canonicalization` | `parser_codec` | `verus` (parse spec) | `PO-001`, `PO-002` |
| `panic_freedom` (implicit) | (none in seed) | (proptest implicitly covers) | `PO-005` |

## Coverage by Production Source Symbol

| Production symbol | Source path:line | Covered by obligation(s) |
|---|---|---|
| `require_non_zero_run` (NEW) | `crates/vb_storage/src/keys.rs::require_non_zero_run` (NEW, planned) | `PO-001-VERUS-MIRROR` (assume_specification target), `PO-005-PROPTEST-PER-PREFIX` (call-graph), `PO-006-PROPTEST-MUTATION` (guard not removable) |
| `run_only_key` (private) | `crates/vb_storage/src/keys.rs:514-521` | `PO-001-VERUS-MIRROR` (mirror body), `PO-005-PROPTEST-PER-PREFIX` (via `run_header_key`, `run_prefix_key`) |
| `sequenced_run_key` (private) | `crates/vb_storage/src/keys.rs:480-496` | `PO-001-VERUS-MIRROR` (mirror body), `PO-005-PROPTEST-PER-PREFIX` (via `run_event_key`, `run_snapshot_key`) |
| `run_header_key` (public) | `crates/vb_storage/src/keys.rs:76-78` | `PO-003-KANI-SPLIT-HARNESS` (split-harness), `PO-005-PROPTEST-PER-PREFIX` (per-prefix) |
| `run_event_key` (public) | `crates/vb_storage/src/keys.rs:81-83` | `PO-001-VERUS-MIRROR` (assume_specification), `PO-003-KANI-SPLIT-HARNESS`, `PO-005-PROPTEST-PER-PREFIX` |
| `run_snapshot_key` (public) | `crates/vb_storage/src/keys.rs:86-91` | `PO-003-KANI-SPLIT-HARNESS` (extension), `PO-005-PROPTEST-PER-PREFIX` |
| `index_status_key` (public) | `crates/vb_storage/src/keys.rs:101-122` | `PO-004-KANI-ORDER-OF-CHECKS` (order-of-checks), `PO-005-PROPTEST-PER-PREFIX` |
| `index_workflow_key` (public) | `crates/vb_storage/src/keys.rs:125-137` | `PO-001-VERUS-MIRROR` (assume_specification), `PO-003-KANI-SPLIT-HARNESS`, `PO-005-PROPTEST-PER-PREFIX` |
| `index_action_key` (public) | `crates/vb_storage/src/keys.rs:140-155` | `PO-001-VERUS-MIRROR` (assume_specification), `PO-003-KANI-SPLIT-HARNESS`, `PO-005-PROPTEST-PER-PREFIX` |
| `encode_key_into` (public) | `crates/vb_storage/src/keys.rs:162-198` | `PO-001-VERUS-MIRROR` (encode_key mirror), `PO-005-PROPTEST-PER-PREFIX` (per-prefix) |
| `encode_key` (public) | `crates/vb_storage/src/keys.rs:205-209` | `PO-001-VERUS-MIRROR` (assume_specification), `PO-005-PROPTEST-PER-PREFIX` |
| `journal_key` (public) | `crates/vb_storage/src/keys.rs:476-478` | `PO-001-VERUS-MIRROR` (assume_specification), `PO-005-PROPTEST-PER-PREFIX` (via `run_event_key`) |
| `run_prefix_key` (public crate) | `crates/vb_storage/src/keys.rs:524-526` | `PO-005-PROPTEST-PER-PREFIX` (per-prefix) |
| `SpecKeyEncodeError::InvalidRunId { run: u64 }` (NEW) | `verification/verus/extern_vb_storage_keys.rs::SpecKeyEncodeError` (NEW variant) | `PO-001-VERUS-MIRROR` (variant), `PO-002-VERUS-DECODER-SYMMETRY` (mirror body) |
| `JournalError::InvalidRunId { run: RunId }` (existing) | `crates/vb_storage/src/error/mod.rs:140-141` | (production source; not an obligation target; bound via production_binding) |
| `KeyDecodeError::InvalidRunId` (existing, unchanged) | `crates/vb_storage/src/error/key_decode.rs:28` | (decoder side; C8 source-of-truth; not an obligation target) |

## Coverage by Production Call Site

The 19 call sites enumerated in `workflow-model.md#Error-Surface-Shift-at-Journal-Append-Call-Sites` all reach the typed key encoders that the proof obligations cover. The call sites are NOT individually targeted by the obligations (the obligations target the encoder entry points); the call sites inherit the rejection through `?` propagation. This is the contract's "shared guard helper" shape (C2).

| Call site | File:line | Encoder used | Inherited via |
|---|---|---|---|
| `append_strict` | `crates/vb_storage/src/journal/append.rs:47` | `run_event_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `append_unfsynced` | `crates/vb_storage/src/journal/internal.rs:55` | `run_event_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `inject_raw_event` | `crates/vb_storage/src/journal/injection.rs:22` | `run_event_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `inject_seq_gap` | `crates/vb_storage/src/journal/injection.rs:42` | `run_event_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `stage_queued_event` | `crates/vb_storage/src/queue/writer/stage.rs:37` | `run_event_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `append_event` (G1) | `crates/vb_storage/src/batch/append_event.rs:43` | `run_event_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `stage_pending_action_index_op` (Insert) | `crates/vb_storage/src/batch/action_index.rs:116` | `index_action_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `stage_pending_action_index_op` (Remove) | `crates/vb_storage/src/batch/action_index.rs:121` | `index_action_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `put_snapshot` | `crates/vb_storage/src/snapshots.rs:32` | `run_snapshot_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `snapshot` (reader) | `crates/vb_storage/src/snapshots.rs:53` | `run_snapshot_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `put_status_index` | `crates/vb_storage/src/indexes.rs:21` | `index_status_key` | `PO-004-KANI-ORDER-OF-CHECKS`, `PO-005-PROPTEST-PER-PREFIX` |
| `put_workflow_index` | `crates/vb_storage/src/indexes.rs:32` | `index_workflow_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `put_action_index` | `crates/vb_storage/src/indexes.rs:44` | `index_action_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `delete_action_index` | `crates/vb_storage/src/indexes.rs:62` | `index_action_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `put_run_header` | `crates/vb_storage/src/headers.rs:19` | `run_header_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `run_header` (reader) | `crates/vb_storage/src/headers.rs:36-39` | `run_header_key` (manual check first) | `PO-005-PROPTEST-PER-PREFIX` (manual check + encoder) |
| `trim_events_for_run` | `crates/vb_storage/src/trimming/logic.rs:60` | `run_prefix_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `trim_run_event_log` | `crates/vb_storage/src/trimming/logic.rs:213` | `run_prefix_key` | `PO-005-PROPTEST-PER-PREFIX` |
| `trim_eligibility_diagnostic` | `crates/vb_storage/src/trimming/logic.rs:246` | `run_prefix_key` | `PO-005-PROPTEST-PER-PREFIX` |

## Coverage by Test Surface (C5)

The 18 test flips in `contract.md#C5` are the test-writer's scope.
The proof obligations are the rust-local / formal layer that
complements (not replaces) the flipped tests:

- 11 tests in `crates/vb_storage/src/keys/tests.rs` — covered
  indirectly by `PO-005-PROPTEST-PER-PREFIX` (the per-prefix
  property test exercises the same encoders the flipped tests
  exercise; the rust-local layer and the formal layer share the
  same encoder entry points).
- 3 tests in `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs` — same
  coverage as above.
- 4 tests in `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs` — same
  coverage as above.

The proptest `PO-005-PROPTEST-PER-PREFIX` is the rust-local
companion to the flipped tests. The Kani harness
`PO-003-KANI-SPLIT-HARNESS` and the Verus mirror
`PO-001-VERUS-MIRROR` are the formal companions.

## Coverage by Kani Harness Group

| Harness | File:line | Covered by |
|---|---|---|
| `assert_key_contracts` | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:43-93` | `PO-003-KANI-SPLIT-HARNESS` (split into rejection + happy arms) |
| `vb_eepg_typed_partitioned_ids` (entry) | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:111-115` | `PO-003-KANI-SPLIT-HARNESS` (entry unchanged; body reorganised) |
| `assert_record_kind_contract` (unrelated) | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:96-108` | (out of scope; unchanged) |
| `vb_eepg_record_kind_contracts` (unrelated) | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:118-121` | (out of scope; unchanged) |

The existing `kani_vb_vzcuf_ps004.rs:151` already pattern-matches
`Err(JournalError::InvalidRunId { .. })` and is the template for
the new rejection arm. It is NOT a target of this bead (it is the
template, not a contract clause); it is preserved as-is.

## Coverage by Verus Mirror

| Mirror symbol | File:line | Covered by |
|---|---|---|
| `SpecKeyEncodeError::InvalidRunId { run: u64 }` (NEW) | `verification/verus/extern_vb_storage_keys.rs:199-204` (NEW variant) | `PO-001-VERUS-MIRROR` (variant added) |
| `journal_key` mirror | `verification/verus/extern_vb_storage_keys.rs:276-301` | `PO-001-VERUS-MIRROR` (assume_specification clause), `PO-002-VERUS-DECODER-SYMMETRY` (body returns `Err(InvalidRunId)` for `run == 0`) |
| `run_event_key` mirror | `verification/verus/extern_vb_storage_keys.rs:303-307` | `PO-001-VERUS-MIRROR` (assume_specification clause), `PO-002-VERUS-DECODER-SYMMETRY` (body returns `Err(InvalidRunId)` for `run == 0`) |
| `encode_key` mirror | `verification/verus/extern_vb_storage_keys.rs:320-344` | `PO-001-VERUS-MIRROR` (assume_specification clause for each run-bearing variant), `PO-002-VERUS-DECODER-SYMMETRY` (body returns `Err(InvalidRunId)` for `run == 0` in each run-bearing arm) |

The mirror body of `journal_key` and `run_event_key` is
`#[verifier::external]`; the assume_specification clauses are the
verified surface. The mirror body of `encode_key` is also
`#[verifier::external]`; the per-arm `assume_specification`
clauses are the verified surface.

## Behavior-Affecting Summary

| Obligation | `behavior_affecting` | Justification |
|---|---|---|
| `PO-001-VERUS-MIRROR` | `false` | Rejection close-of-gap; decoder already enforces. |
| `PO-002-VERUS-DECODER-SYMMETRY` | `false` | Rejection close-of-gap; decoder already enforces. |
| `PO-003-KANI-SPLIT-HARNESS` | `false` | Kani harness split is a refactor; no new behaviour. |
| `PO-004-KANI-ORDER-OF-CHECKS` | `false` | Order-of-checks is a structural invariant; no new behaviour. |
| `PO-005-PROPTEST-PER-PREFIX` | `false` | Per-prefix property test; rejection is the close-of-gap. |
| `PO-006-PROPTEST-MUTATION` | `false` | Mutation-resistance proptest; no production behaviour change. |

All six obligations carry `behavior_affecting: false` per
femdation directive. No `E_BEHAVIOR_WAIVER` concerns arise.

## Out-of-Scope Coverage (Preservation Invariants)

The proof plan does NOT add obligations that touch these surfaces:

- `RunId::new` and `RunId::ZERO` constructor invariants
  (C9; preserved by every obligation's `RunId::new(0)` literal
  input).
- `recovery/replay/summary/{derive,apply,tests}.rs`
  (C9; no obligation targets these files).
- Workspace tests that build `RunId::new(0)` without reaching
  an encoder
  (C9; these tests are not in any obligation's target set).
- TLA+ spec mirror `RunId::Run(0)` placeholder
  (C9; TLA+ removed per proof-planner SKILL.md).
- Proptest `all_key_functions_are_deterministic`
  (C9; already excludes zero from `run_val`).
- `tests.rs::symbolic_code_table`
  (C9; already maps `INVALID_RUN_ID` to `invalid_run_id`).

The proptest `PO-005-PROPTEST-PER-PREFIX` and the Kani harness
`PO-003-KANI-SPLIT-HARNESS` exercise only the production
encoders; the out-of-scope surfaces are not reached.

## Self-Audit

- [x] Every contract clause (C1-C9) has a primary obligation or
      an explicit "no obligation" justification.
- [x] Every production source symbol in C1 has at least one
      obligation target.
- [x] Every public encoder entry point (`run_header_key`,
      `run_event_key`, `run_snapshot_key`, `index_status_key`,
      `index_workflow_key`, `index_action_key`) is covered by
      `PO-005-PROPTEST-PER-PREFIX`.
- [x] Every public wrapper (`encode_key`, `encode_key_into`,
      `journal_key`, `run_prefix_key`) is covered by
      `PO-005-PROPTEST-PER-PREFIX` (via the per-prefix property
      test) and `PO-001-VERUS-MIRROR` (via the mirror).
- [x] Every Kani harness group affected by the contract change
      is covered by `PO-003-KANI-SPLIT-HARNESS` and
      `PO-004-KANI-ORDER-OF-CHECKS`.
- [x] Every Verus mirror symbol affected by the contract change
      is covered by `PO-001-VERUS-MIRROR` and
      `PO-002-VERUS-DECODER-SYMMETRY`.
- [x] All obligations carry `behavior_affecting: false` per
      femdation directive.
- [x] Out-of-scope surfaces are NOT targeted by any obligation
      (preservation invariant).
- [x] Six obligations total (within the 5-7 femdation
      envelope).
