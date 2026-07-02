# Proof-to-Rust Map: vb-cn2v4

## Bridge Metadata

| Field | Value |
|-------|-------|
| Bead | vb-cn2v4 |
| Title | Keys: reject zero RunId in all key encoders (P1 bug) |
| State | 7 (proof-to-implementation bridge) |
| Agent | proof-to-implementation |
| Invocation | femdation:vb-cn2v4:p7:proof-to-implementation:v1 |
| Schema | proof-to-rust-map/v1 |
| Source checkout | `/home/lewis/src/velvet-ballistics` (control plane, read-only) |
| Workspace | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4` |
| JJ workspace | `cheap25-vb-cn2v4` |
| Controller | femdation (parent dispatcher; this is a direct child) |
| Plan invocation | `femdation:vb-cn2v4:p4:planner:v1` |
| Plan-review invocation | `femdation:vb-cn2v4:p4b:reviewer:v1` (state 4b; STATUS: APPROVED) |
| Lane profile | rust-local + kani + verus (per femdation directive) |
| Behavior-affecting classification | `false` (rejection is close-of-gap, not behavior change) |

## Bridge Purpose

This bridge maps the six approved `proof-obligation/v1` rows from
`.beads/vb-cn2v4/proof-obligations.planned.jsonl` (PO-001 through PO-006,
totaling 2 verus + 2 kani + 2 proptest obligations) to concrete Rust
source references, independent behavior tests, separate refinement harness
references, and exact verifier commands. The encoder asymmetry P1 bug
(the encoder emits `Ok` bytes for `RunId(0)` while the decoder rejects the
same `RunId(0)` at `keys.rs:372-374, 381-383, 400-402, 412-414, 423-425`)
is closed by routing every public run-bearing encoder through a new
private helper `require_non_zero_run(run)` that returns
`Err(JournalError::InvalidRunId { run })` iff `run.get() == 0`.

## Production Surface Map

| Surface | Path | Symbol | Role |
|---------|------|--------|------|
| Private guard (NEW) | `crates/vb_storage/src/keys.rs` | `require_non_zero_run` | C2: centralises the `run == 0` rejection (returns `Err(JournalError::InvalidRunId { run })` iff `run.get() == 0`) |
| Private helper (called) | `crates/vb_storage/src/keys.rs:480-496` | `sequenced_run_key` | C2: calls `require_non_zero_run` after the existing `seq.get() == u64::MAX` check |
| Private helper (called) | `crates/vb_storage/src/keys.rs:514-521` | `run_only_key` | C2: calls `require_non_zero_run` first |
| Public encoder | `crates/vb_storage/src/keys.rs:76-78` | `run_header_key` | C1: delegates to `run_only_key(PREFIX_RUN_HEADER, run)` |
| Public encoder | `crates/vb_storage/src/keys.rs:81-83` | `run_event_key` | C1: delegates to `journal_key(run, seq)` |
| Public encoder | `crates/vb_storage/src/keys.rs:86-91` | `run_snapshot_key` | C1: delegates to `sequenced_run_key(PREFIX_RUN_SNAPSHOT, run, seq)` |
| Public encoder | `crates/vb_storage/src/keys.rs:101-122` | `index_status_key` | C1/C4: calls `require_non_zero_run` BEFORE `state.to_u8_checked` |
| Public encoder | `crates/vb_storage/src/keys.rs:125-137` | `index_workflow_key` | C1: calls `require_non_zero_run` first |
| Public encoder | `crates/vb_storage/src/keys.rs:140-155` | `index_action_key` | C1: calls `require_non_zero_run` first |
| Public dispatcher | `crates/vb_storage/src/keys.rs:162-198` | `encode_key_into` | C1: dispatches to the six typed encoders (inherits rejection) |
| Public entry | `crates/vb_storage/src/keys.rs:205-209` | `encode_key` | C1: delegates to `encode_key_into` (inherits rejection) |
| Public entry | `crates/vb_storage/src/keys.rs:524-526` | `run_prefix_key` (pub(crate)) | C1: delegates to `run_prefix(run)` → `run_only_key` (inherits rejection) |
| Defence-in-depth | `crates/vb_storage/src/headers.rs:36-39` | `FjallJournal::run_header` | C4: manual `if run.get() == 0 { Err(InvalidRunId) }` check; redundant once `run_header_key` rejects |
| Production error | `crates/vb_storage/src/error/mod.rs:140-141` | `JournalError::InvalidRunId { run: RunId }` | C3: reused (no new variant) |
| Diagnostic code | `crates/vb_storage/src/error/codes.rs:73` | `INVALID_RUN_ID_CODE = 0x4021` | C3: unchanged |
| Symbolic code | `crates/vb_storage/src/error/codes.rs:250` | `INVALID_RUN_ID` | C3: unchanged |
| Decoder (untouched) | `crates/vb_storage/src/keys.rs:346-434` | `decode_storage_key` | C8: source of truth; rejects `run == 0` at lines 372-374, 381-383, 400-402, 412-414, 423-425 |
| Verus mirror (binding) | `verification/verus/extern_vb_storage_keys.rs:199-204` | `SpecKeyEncodeError` | C7: extends with `InvalidRunId { run: u64 }`; assume_specification clauses on run-bearing mirror fns |
| Verus mirror body | `verification/verus/extern_vb_storage_keys.rs:303-307` | `run_event_key` mirror | C7: returns `Err(SpecKeyEncodeError::InvalidRunId { run })` iff `run == 0` |
| Verus mirror body | `verification/verus/extern_vb_storage_keys.rs:276-301` | `journal_key` mirror | C7: returns `Err(SpecKeyEncodeError::InvalidRunId { run })` iff `run == 0` |
| Verus mirror body | `verification/verus/extern_vb_storage_keys.rs:320-344` | `encode_key` mirror | C7: each run-bearing `SpecStorageKey` arm returns `Err(SpecKeyEncodeError::InvalidRunId { run })` iff `run == 0` |
| Kani harness | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:51-92` | `assert_key_contracts` | C6: split-shape `if/else` distinguishing `run_value == 0` (rejection) from `run_value != 0` (happy) |
| Kani entry | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:111-114` | `vb_eepg_typed_partitioned_ids` | C6: `#[kani::proof]` invoking `assert_key_contracts` |
| Kani symbolic input | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:15-24` | `SymbolicKeyInputs` (kani::Arbitrary) | GOD RULE 1: no hardcoded structural inputs; `run_hi: u16 \| run_lo: u16` → `run_value: u64` |

## Obligation Matrix (Six Rows)

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|-------|-------------------|------------------|-------------------|------------------------|----------|-----------------|------------|
| PO-001-VERUS-MIRROR | `SpecKeyEncodeError` extends with `InvalidRunId { run: u64 }`; assume_specification contracts on run-bearing mirror fns include `requires run != 0; ensures result is Err(SpecKeyEncodeError::InvalidRunId { run }) iff run == 0` | false | `crates/vb_storage/src/keys.rs::require_non_zero_run`, `crates/vb_storage/src/keys.rs::run_header_key`, `crates/vb_storage/src/keys.rs::run_event_key`, `crates/vb_storage/src/keys.rs::journal_key`, `crates/vb_storage/src/keys.rs::index_workflow_key`, `crates/vb_storage/src/keys.rs::index_action_key`, `verification/verus/extern_vb_storage_keys.rs::SpecKeyEncodeError` | `crates/vb_storage/src/keys/tests.rs::run_header_key_with_zero_run_id`, `crates/vb_storage/src/keys/tests.rs::index_status_key_with_zero_values`, `crates/vb_storage/src/keys/tests.rs::index_workflow_key_length`, `crates/vb_storage/src/keys/tests.rs::index_action_key_length`, `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs::run_id_zero_roundtrip` | `verification/verus/extern_vb_storage_keys.rs::SpecKeyEncodeError::InvalidRunId`, `verification/verus/extern_vb_storage_keys.rs::run_event_key`, `verification/verus/extern_vb_storage_keys.rs::journal_key`, `verification/verus/extern_vb_storage_keys.rs::encode_key` | verus | `verus --crate-type=lib --edition=2021 verification/verus/extern_vb_storage_keys.rs` | state 11 |
| PO-002-VERUS-DECODER-SYMMETRY | Mirror body of `encode_key` / `run_event_key` / `journal_key` returns `Err(SpecKeyEncodeError::InvalidRunId { run })` iff `run == 0`; decoder mirror unchanged; production-binding gate + drift gate pass | false | `crates/vb_storage/src/keys.rs::encode_key_into`, `crates/vb_storage/src/keys.rs::encode_key`, `crates/vb_storage/src/keys.rs::decode_storage_key`, `verification/verus/extern_vb_storage_keys.rs::encode_key`, `verification/verus/extern_vb_storage_keys.rs::decode_storage_key` | `crates/vb_storage/src/keys/tests.rs::run_header_key_with_zero_run_id`, `crates/vb_storage/src/keys/tests.rs::run_event_key_length`, `crates/vb_storage/src/keys/tests.rs::index_status_key_rejects_other_state_in_collision_range`, `crates/vb_storage/src/keys/tests.rs::index_status_key_accepts_other_state_above_collision_range`, `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs::encode_exact_length_run_header`, `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs::encode_exact_length_run_event`, `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs::encode_exact_length_index_action` | `verification/verus/extern_vb_storage_keys.rs::encode_key`, `verification/verus/extern_vb_storage_keys.rs::decode_storage_key`, `verification/verus/extern_vb_storage_keys.rs::SpecKeyEncodeError::InvalidRunId` | verus | `verus --crate-type=lib --edition=2021 verification/verus/extern_vb_storage_keys.rs` | state 11 |
| PO-003-KANI-SPLIT-HARNESS | `assert_key_contracts` distinguishes `run_value == 0` rejection path (`matches!(..., Err(InvalidRunId { .. }))`) from `run_value != 0` happy path (byte layout); `kani::cover` reachability for both arms; no `Err(_) => assert!(false)` arms | false | `crates/vb_storage/src/keys.rs::require_non_zero_run`, `crates/vb_storage/src/keys.rs::run_header_key`, `crates/vb_storage/src/keys.rs::run_event_key`, `crates/vb_storage/src/keys.rs::index_workflow_key`, `crates/vb_storage/src/keys.rs::index_action_key`, `crates/vb_storage/src/kani_typed_partitioned_ids.rs::assert_key_contracts` | `crates/vb_storage/src/keys/tests.rs::run_header_key_has_correct_prefix`, `crates/vb_storage/src/keys/tests.rs::run_event_key_encodes_run_and_seq_big_endian`, `crates/vb_storage/src/keys/tests.rs::index_workflow_key_encodes_workflow_and_run`, `crates/vb_storage/src/keys/tests.rs::index_action_key_encodes_action_run_step`, `crates/vb_storage/src/keys/tests.rs::run_header_key_with_zero_run_id` | `crates/vb_storage/src/kani_typed_partitioned_ids.rs::vb_eepg_typed_partitioned_ids`, `crates/vb_storage/src/kani_typed_partitioned_ids.rs::assert_key_contracts` | kani | `cargo kani -j 1 --output-format=regular --harness vb_eepg_typed_partitioned_ids --mem-predicates` | state 11 |
| PO-004-KANI-ORDER-OF-CHECKS | For `index_status_key`, `require_non_zero_run` fires BEFORE `state.to_u8_checked`; for `Other(0..2)` collision path with `RunId(0)` the encoder returns `Err(InvalidRunId)` and never reaches the `IndexStatusStateCollision` check | false | `crates/vb_storage/src/keys.rs::index_status_key`, `crates/vb_storage/src/keys.rs::require_non_zero_run`, `crates/vb_storage/src/types.rs::IndexStatusState::to_u8_checked`, `crates/vb_storage/src/kani_typed_partitioned_ids.rs::index_status_key` | `crates/vb_storage/src/keys/tests.rs::index_status_key_rejects_other_state_in_collision_range` (post-flip), `crates/vb_storage/src/keys/tests.rs::index_status_key_accepts_other_state_above_collision_range` (post-flip), `crates/vb_storage/src/keys/tests.rs::index_status_key_with_zero_values`, `crates/vb_storage/src/keys/tests.rs::index_status_key_encodes_state_timestamp_run` | `crates/vb_storage/src/kani_typed_partitioned_ids.rs::vb_eepg_typed_partitioned_ids` (extended), `crates/vb_storage/src/kani_typed_partitioned_ids.rs::assert_key_contracts` (extended) | kani | `cargo kani -j 1 --output-format=regular --harness vb_eepg_typed_partitioned_ids --mem-predicates` | state 11 |
| PO-005-PROPTEST-PER-PREFIX | `encoder_rejects_zero_run_id_for_every_prefix` per-prefix property test covers all six public encoder entry points with `RunId(0)` returning `Err(JournalError::InvalidRunId { run })`; tests the rejection arm explicitly (non-vacuous) | false | `crates/vb_storage/src/keys.rs::run_header_key`, `crates/vb_storage/src/keys.rs::run_event_key`, `crates/vb_storage/src/keys.rs::run_snapshot_key`, `crates/vb_storage/src/keys.rs::index_status_key`, `crates/vb_storage/src/keys.rs::index_workflow_key`, `crates/vb_storage/src/keys.rs::index_action_key`, `crates/vb_storage/src/keys.rs::require_non_zero_run` | `crates/vb_storage/src/keys/tests.rs::run_header_key_with_zero_run_id`, `crates/vb_storage/src/keys/tests.rs::index_status_key_with_zero_values`, `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs::run_header_key_prefix_is_0x10` (RunId 0 arm), `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs::run_header_key_zero_run_id`, `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs::index_workflow_key_zero_values`, `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs::run_id_zero_roundtrip` | `crates/vb_storage/src/proptests.rs::encoder_rejects_zero_run_id_for_every_prefix` (NEW) | proptest | `PROPTEST_CASES=10000 cargo test --test proptest encoder_rejects_zero_run_id_for_every_prefix --release` | state 11 |
| PO-006-PROPTEST-MUTATION | `mutation_resistance_require_non_zero_run` mutation-resistance proptest asserts that removing the `require_non_zero_run` guard produces `Ok(_)` while the guard-on branch returns `Err(InvalidRunId)`; guards against future removal of the centralisation | false | `crates/vb_storage/src/keys.rs::require_non_zero_run`, `crates/vb_storage/src/keys.rs::run_only_key`, `crates/vb_storage/src/keys.rs::sequenced_run_key`, `crates/vb_storage/src/keys.rs::index_status_key`, `crates/vb_storage/src/keys.rs::index_workflow_key`, `crates/vb_storage/src/keys.rs::index_action_key`, `crates/vb_storage/src/error/mod.rs::JournalError::InvalidRunId` | `crates/vb_storage/src/keys/tests.rs::run_header_key_with_zero_run_id`, `crates/vb_storage/src/keys/tests.rs::run_event_key_length`, `crates/vb_storage/src/keys/tests.rs::run_snapshot_key_length`, `crates/vb_storage/src/keys/tests.rs::index_workflow_key_length`, `crates/vb_storage/src/keys/tests.rs::index_action_key_length`, `crates/vb_storage/src/keys/tests.rs::index_status_key_with_zero_values` | `crates/vb_storage/src/proptests.rs::mutation_resistance_require_non_zero_run` (NEW) | proptest | `PROPTEST_CASES=1000 cargo test --test proptest mutation_resistance_require_non_zero_run --release` | state 11 |

## Contract Clause → Obligation Traceability

| Contract Clause | Obligation IDs | Mapping Status |
|----------------|----------------|----------------|
| C1 (Encoder/Decoder Symmetry) | PO-005-PROPTEST-PER-PREFIX, PO-002-VERUS-DECODER-SYMMETRY | planned |
| C2 (Shared Guard Helper) | PO-006-PROPTEST-MUTATION | planned |
| C3 (Error Reuse) | PO-001-VERUS-MIRROR, PO-005-PROPTEST-PER-PREFIX | planned (no new variant; `JournalError::InvalidRunId { run: RunId }` reused) |
| C4 (Defence-in-Depth) | PO-004-KANI-ORDER-OF-CHECKS, PO-006-PROPTEST-MUTATION | planned (manual check in `headers.rs:36-39` tolerated) |
| C5 (Test Suite Flip — 18 tests) | PO-005-PROPTEST-PER-PREFIX (companion to unit flips) | planned |
| C6 (Kani Harness Split) | PO-003-KANI-SPLIT-HARNESS, PO-004-KANI-ORDER-OF-CHECKS | planned |
| C7 (Verus Mirror Variant) | PO-001-VERUS-MIRROR, PO-002-VERUS-DECODER-SYMMETRY | planned |
| C8 (Decoder Unchanged) | PO-002-VERUS-DECODER-SYMMETRY (decoder mirror unchanged) | planned |
| C9 (Out-of-Scope Surfaces Preserved) | All obligations (no out-of-scope surface touched) | planned |

## Test Suite Flip Coverage (18 Tests, C5)

| Test File | Count | Tests |
|-----------|-------|-------|
| `crates/vb_storage/src/keys/tests.rs` | 11 | `run_header_key_has_correct_prefix`, `run_event_key_length`, `index_status_key_has_correct_prefix`, `index_status_key_length`, `index_workflow_key_length`, `index_action_key_length`, `run_header_key_with_zero_run_id`, `index_status_key_with_zero_values`, `run_prefix_key_is_9_bytes`, `index_status_key_rejects_other_state_in_collision_range` (RunId swap `0→1`), `index_status_key_accepts_other_state_above_collision_range` (RunId swap `0→1`) |
| `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs` | 3 | `encode_exact_length_run_header`, `encode_exact_length_run_event`, `encode_exact_length_index_action` |
| `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs` | 4 | `run_header_key_prefix_is_0x10` (RunId-0 arm), `run_header_key_zero_run_id`, `index_workflow_key_zero_values`, `run_id_zero_roundtrip` |

The 18 test flips are scoped to `test-writer`/`test-planner` (proof-planner Non-Goals per `proof-strategy.md` § Non-Goals). The bridge surfaces them as `behavior_test_refs` for every behavior-affecting obligation, so the test-writer's flips are the executable evidence layer that satisfies PO-005 / PO-006 alongside the unit-flipped tests.

## Out-of-Scope Surface Preservation (C9)

The bridge MUST NOT map any obligation to a surface that C9 lists as preserved:

- `RunId::new` and `RunId::ZERO` constructor invariants — no obligation modifies `vb_core::ids::RunId`
- Recovery diagnostics using `RunId::new(0)` as `NoRecoveryData` placeholder — no obligation touches `recovery/replay/summary/{derive,apply,tests}.rs`
- Workspace tests that build `RunId::new(0)` without reaching a key encoder (`vb_test_runtime_lifecycle_state_behavior.rs`, `integration_runtime_storage_fault_tolerance.rs`, `runtime_version_barrier_tests.rs`, `cancel_kill_lattice_props.rs`, `vb_core_yaml_e2e_chain_contract.rs`) — no obligation covers these
- `contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs` (TLA+ spec mirror using `RunId::Run(0)` as spec placeholder) — out of scope by contract
- `crates/vb_storage/src/proptests.rs::all_key_functions_are_deterministic` (already excludes zero from `run_val`) — preserved
- `crates/vb_storage/src/tests.rs::symbolic_code_table` (already maps `INVALID_RUN_ID`) — preserved

## Obligation-by-Obligation Source Mapping

### PO-001-VERUS-MIRROR (Verus spec surface — extension of `SpecKeyEncodeError`)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-cn2v4-001 |
| Verifier | verus |
| Verus artifact | `verification/verus/extern_vb_storage_keys.rs` (extern companion module) |
| Production target | `crates/vb_storage/src/keys.rs::require_non_zero_run` (NEW private helper) |
| | `crates/vb_storage/src/keys.rs::run_header_key` (L76-78) |
| | `crates/vb_storage/src/keys.rs::run_event_key` (L81-83) |
| | `crates/vb_storage/src/keys.rs::journal_key` (L476-478, delegates to `sequenced_run_key`) |
| | `crates/vb_storage/src/keys.rs::index_workflow_key` (L125-137) |
| | `crates/vb_storage/src/keys.rs::index_action_key` (L140-155) |
| | `verification/verus/extern_vb_storage_keys.rs::SpecKeyEncodeError` (L199-204) |
| Source refs | `verification/verus/extern_vb_storage_keys.rs::SpecKeyEncodeError::InvalidRunId` (NEW variant) |
| | `verification/verus/extern_vb_storage_keys.rs::run_event_key` (L303-307, mirror body) |
| | `verification/verus/extern_vb_storage_keys.rs::journal_key` (L276-301, mirror body) |
| | `verification/verus/extern_vb_storage_keys.rs::encode_key` (L320-344, mirror body) |
| Production-binding mechanism | WEAK_EXTERN (mirror is the project's established `extern_*.rs` companion-module pattern; `scripts/check-verus-production-binding.sh` exempts `extern_*.rs`) |
| Production-binding gate | `scripts/check-verus-production-binding.sh` MUST exit 0 (extern_*.rs files are SKIPPED at L67) |
| Mirror-drift gate | `scripts/check-production-inner-drift.sh` MUST exit 0 (mirror comment block cites the new `require_non_zero_run` helper) |
| Behavior test refs | `crates/vb_storage/src/keys/tests.rs::run_header_key_with_zero_run_id` (post-flip; asserts `Err(JournalError::InvalidRunId { run })`) |
| | `crates/vb_storage/src/keys/tests.rs::index_status_key_with_zero_values` (post-flip; asserts `Err(InvalidRunId)`) |
| | `crates/vb_storage/src/keys/tests.rs::index_workflow_key_length` (post-flip; now `Err` for `RunId(0)`) |
| | `crates/vb_storage/src/keys/tests.rs::index_action_key_length` (post-flip; now `Err` for `RunId(0)`) |
| | `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs::run_id_zero_roundtrip` (post-flip; asserts `Err(InvalidRunId)`) |
| Refinement harness refs | `verification/verus/extern_vb_storage_keys.rs::SpecKeyEncodeError::InvalidRunId` (NEW variant declaration) |
| | `verification/verus/extern_vb_storage_keys.rs::run_event_key` (mirror fn; `assume_specification` contract) |
| | `verification/verus/extern_vb_storage_keys.rs::journal_key` (mirror fn; `assume_specification` contract) |
| | `verification/verus/extern_vb_storage_keys.rs::encode_key` (mirror fn; `assume_specification` contract per run-bearing `SpecStorageKey` variant) |
| Evidence command | `verus --crate-type=lib --edition=2021 verification/verus/extern_vb_storage_keys.rs` |
| Evidence workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4` |
| Evidence artifact | `.evidence/verus/extern_vb_storage_keys_verify.log` |
| Expected evidence | Verus reports `verification results:: N verified, 0 errors` for `extern_vb_storage_keys.rs`; post-check confirms `SpecKeyEncodeError::InvalidRunId { run: u64 }` variant is present; the `assume_specification` clauses on `run_event_key`, `journal_key`, and `encode_key` (per run-bearing `SpecStorageKey` variant) include the `requires run != 0; ensures result is Err(InvalidRunId) iff run == 0` clause; no `external_body`, no `assume(`, no `axiom` in the spec file; `scripts/check-verus-production-binding.sh` exits 0 |
| Mapping status | planned |
| behavior_affecting | false |

### PO-002-VERUS-DECODER-SYMMETRY (Verus mirror body equals production decoder on `RunId(0)` rejection)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-cn2v4-002 |
| Verifier | verus |
| Verus artifact | `verification/verus/extern_vb_storage_keys.rs` |
| Production target | `crates/vb_storage/src/keys.rs::encode_key_into` (L162-198, dispatches to six typed encoders) |
| | `crates/vb_storage/src/keys.rs::encode_key` (L205-209, delegates to `encode_key_into`) |
| | `crates/vb_storage/src/keys.rs::decode_storage_key` (L346-434, untouched; source of truth) |
| Source refs | `crates/vb_storage/src/keys.rs::encode_key_into` (L162-198) |
| | `crates/vb_storage/src/keys.rs::encode_key` (L205-209) |
| | `crates/vb_storage/src/keys.rs::decode_storage_key` (L346-434) |
| | `verification/verus/extern_vb_storage_keys.rs::encode_key` (L320-344, mirror body) |
| | `verification/verus/extern_vb_storage_keys.rs::decode_storage_key` (L525-657, mirror body; unchanged) |
| Production-binding mechanism | WEAK_EXTERN |
| Behavior test refs | `crates/vb_storage/src/keys/tests.rs::run_header_key_with_zero_run_id` (post-flip) |
| | `crates/vb_storage/src/keys/tests.rs::run_event_key_length` (post-flip) |
| | `crates/vb_storage/src/keys/tests.rs::index_status_key_rejects_other_state_in_collision_range` (post-flip; `RunId::new(1)` swap to keep collision path exercised) |
| | `crates/vb_storage/src/keys/tests.rs::index_status_key_accepts_other_state_above_collision_range` (post-flip; `RunId::new(1)` swap) |
| | `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs::encode_exact_length_run_header` (post-flip; asserts `Err(InvalidRunId)`) |
| | `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs::encode_exact_length_run_event` (post-flip) |
| | `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs::encode_exact_length_index_action` (post-flip) |
| Refinement harness refs | `verification/verus/extern_vb_storage_keys.rs::encode_key` (mirror; per-variant rejection path) |
| | `verification/verus/extern_vb_storage_keys.rs::decode_storage_key` (mirror; unchanged source of truth) |
| | `verification/verus/extern_vb_storage_keys.rs::SpecKeyEncodeError::InvalidRunId` (NEW variant) |
| Evidence command | `verus --crate-type=lib --edition=2021 verification/verus/extern_vb_storage_keys.rs` |
| Evidence artifact | `.evidence/verus/extern_vb_storage_keys_verify.log` |
| Expected evidence | Verus reports `verification results:: N verified, 0 errors`; post-check confirms the mirror body of `encode_key` for each run-bearing `SpecStorageKey` variant (`RunHeader`, `RunEvent`, `RunSnapshot`, `IndexStatus`, `IndexWorkflow`, `IndexAction`) returns `Err(SpecKeyEncodeError::InvalidRunId { run })` when `run == 0`; the mirror body of `run_event_key` and `journal_key` returns the same; the mirror body of `decode_storage_key` is unchanged and continues to surface `SpecKeyDecodeError::InvalidRunId` for `run == 0` bytes; `scripts/check-verus-production-binding.sh` and `scripts/check-production-inner-drift.sh` both exit 0 |
| Mapping status | planned |
| behavior_affecting | false |

### PO-003-KANI-SPLIT-HARNESS (Kani harness rejection-path split)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-cn2v4-003 |
| Verifier | kani |
| Kani artifact | `crates/vb_storage/src/kani_typed_partitioned_ids.rs` (split-harness shape) |
| Production target | `crates/vb_storage/src/keys.rs::require_non_zero_run` (NEW private helper) |
| | `crates/vb_storage/src/keys.rs::run_header_key` (L76-78) |
| | `crates/vb_storage/src/keys.rs::run_event_key` (L81-83) |
| | `crates/vb_storage/src/keys.rs::index_workflow_key` (L125-137) |
| | `crates/vb_storage/src/keys.rs::index_action_key` (L140-155) |
| Source refs | `crates/vb_storage/src/kani_typed_partitioned_ids.rs::assert_key_contracts` (L51-92; split `if/else` on `run_value == 0`) |
| | `crates/vb_storage/src/kani_typed_partitioned_ids.rs::vb_eepg_typed_partitioned_ids` (L111-114; `#[kani::proof]` entry) |
| | `crates/vb_storage/src/kani_typed_partitioned_ids.rs::SymbolicKeyInputs` (L15-24; `kani::Arbitrary`; GOD RULE 1) |
| | `crates/vb_storage/src/kani_typed_partitioned_ids.rs::run_raw` (L35-37; `run_hi:u16\|run_lo:u16 → u64`) |
| Behavior test refs | `crates/vb_storage/src/keys/tests.rs::run_header_key_has_correct_prefix` (happy-path unit test, currently passes for `RunId::new(0)`; companion to the rejection arm) |
| | `crates/vb_storage/src/keys/tests.rs::run_event_key_encodes_run_and_seq_big_endian` (happy-path unit test) |
| | `crates/vb_storage/src/keys/tests.rs::index_workflow_key_encodes_workflow_and_run` (happy-path unit test) |
| | `crates/vb_storage/src/keys/tests.rs::index_action_key_encodes_action_run_step` (happy-path unit test) |
| | `crates/vb_storage/src/keys/tests.rs::run_header_key_with_zero_run_id` (post-flip; rejection arm test) |
| Refinement harness refs | `crates/vb_storage/src/kani_typed_partitioned_ids.rs::vb_eepg_typed_partitioned_ids` (proof entry; symbolic input + assertion on split arms) |
| | `crates/vb_storage/src/kani_typed_partitioned_ids.rs::assert_key_contracts` (split-shape body) |
| Evidence command | `cargo kani -j 1 --output-format=regular --harness vb_eepg_typed_partitioned_ids --mem-predicates` |
| Evidence workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4` |
| Evidence artifact | `.evidence/kani/vb_storage/kani_typed_partitioned_ids_success.log` |
| Expected evidence | Kani reports `VERIFICATION:- SUCCESSFUL` for `vb_eepg_typed_partitioned_ids` with `check_failure=0`; the harness body distinguishes `run_value == 0` (asserts `matches!(..., Err(InvalidRunId { .. }))`) from `run_value != 0` (asserts `Ok(key)` with byte layout); `kani::cover` entries prove the rejection arm is reachable for `run_value == 0` and the happy arm is reachable for `run_value != 0`; the harness uses `kani::Arbitrary` / `kani::any()` for symbolic input (GOD RULE 1 compliant); no `Err(_) => assert!(false)` arms remain |
| Mapping status | planned |
| behavior_affecting | false |

### PO-004-KANI-ORDER-OF-CHECKS (Kani harness order-of-checks for `index_status_key`)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-cn2v4-004 |
| Verifier | kani |
| Kani artifact | `crates/vb_storage/src/kani_typed_partitioned_ids.rs` (extended to cover `run_snapshot_key` and `index_status_key`) |
| Production target | `crates/vb_storage/src/keys.rs::index_status_key` (L101-122; `require_non_zero_run` fires BEFORE `state.to_u8_checked`) |
| | `crates/vb_storage/src/types.rs::IndexStatusState::to_u8_checked` (collision-range check on `Other(0..2)`) |
| Source refs | `crates/vb_storage/src/keys.rs::index_status_key` (L101-122) |
| | `crates/vb_storage/src/keys.rs::require_non_zero_run` (NEW; called first in `index_status_key`) |
| | `crates/vb_storage/src/types.rs::IndexStatusState::to_u8_checked` (collision check; called AFTER the new guard) |
| | `crates/vb_storage/src/kani_typed_partitioned_ids.rs::assert_key_contracts` (extended to cover `run_snapshot_key` and `index_status_key`) |
| Behavior test refs | `crates/vb_storage/src/keys/tests.rs::index_status_key_rejects_other_state_in_collision_range` (post-flip; `RunId::new(1)` swap to keep collision path exercised; asserts `Err(IndexStatusStateCollision)` for `Other(0..2)` with non-zero `RunId`) |
| | `crates/vb_storage/src/keys/tests.rs::index_status_key_accepts_other_state_above_collision_range` (post-flip; `RunId::new(1)` swap; asserts `Ok(bytes)` for `Other(>=3)`) |
| | `crates/vb_storage/src/keys/tests.rs::index_status_key_with_zero_values` (post-flip; asserts `Err(InvalidRunId)` for `Other(0)` with `RunId(0)`; demonstrates the new guard fires BEFORE the collision check) |
| | `crates/vb_storage/src/keys/tests.rs::index_status_key_encodes_state_timestamp_run` (happy-path with non-zero `RunId`) |
| Refinement harness refs | `crates/vb_storage/src/kani_typed_partitioned_ids.rs::vb_eepg_typed_partitioned_ids` (extended to cover `run_snapshot_key` and `index_status_key`) |
| | `crates/vb_storage/src/kani_typed_partitioned_ids.rs::assert_key_contracts` (extended with paired test for `IndexStatusState::Other(0)` + `RunId::new(0)` → `Err(InvalidRunId)` and reachability for `Other(0)` + `RunId::new(1)` → `Err(IndexStatusStateCollision)`) |
| Evidence command | `cargo kani -j 1 --output-format=regular --harness vb_eepg_typed_partitioned_ids --mem-predicates` |
| Evidence artifact | `.evidence/kani/vb_storage/kani_typed_partitioned_ids_success.log` |
| Expected evidence | Kani reports `VERIFICATION:- SUCCESSFUL` for `vb_eepg_typed_partitioned_ids` with `check_failure=0`; the harness body for `index_status_key` includes a paired test that constructs `IndexStatusState::Other(0)` (or any `v < 3`) with `run == RunId::new(0)` and asserts the result is `Err(JournalError::InvalidRunId { .. })` (NOT `IndexStatusStateCollision`); `kani::cover` proves the `IndexStatusStateCollision` path is reachable when `run != RunId::new(0)` (e.g., `RunId::new(1)` with `IndexStatusState::Other(0)`); no `Err(_) => assert!(false)` arms remain |
| Mapping status | planned |
| behavior_affecting | false |

### PO-005-PROPTEST-PER-PREFIX (proptest per-prefix rejection coverage)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-cn2v4-005 |
| Verifier | proptest |
| Proptest artifact | `crates/vb_storage/src/proptests.rs` (NEW property test `encoder_rejects_zero_run_id_for_every_prefix`) |
| Production target | `crates/vb_storage/src/keys.rs::run_header_key` (L76-78) |
| | `crates/vb_storage/src/keys.rs::run_event_key` (L81-83) |
| | `crates/vb_storage/src/keys.rs::run_snapshot_key` (L86-91) |
| | `crates/vb_storage/src/keys.rs::index_status_key` (L101-122) |
| | `crates/vb_storage/src/keys.rs::index_workflow_key` (L125-137) |
| | `crates/vb_storage/src/keys.rs::index_action_key` (L140-155) |
| | `crates/vb_storage/src/keys.rs::require_non_zero_run` (NEW private helper) |
| Source refs | `crates/vb_storage/src/keys.rs::run_header_key` |
| | `crates/vb_storage/src/keys.rs::run_event_key` |
| | `crates/vb_storage/src/keys.rs::run_snapshot_key` |
| | `crates/vb_storage/src/keys.rs::index_status_key` |
| | `crates/vb_storage/src/keys.rs::index_workflow_key` |
| | `crates/vb_storage/src/keys.rs::index_action_key` |
| | `crates/vb_storage/src/keys.rs::require_non_zero_run` (NEW) |
| | `crates/vb_storage/src/error/mod.rs::JournalError::InvalidRunId` (L140-141) |
| Behavior test refs | `crates/vb_storage/src/keys/tests.rs::run_header_key_with_zero_run_id` (post-flip; companion unit test asserting `Err(InvalidRunId)`) |
| | `crates/vb_storage/src/keys/tests.rs::index_status_key_with_zero_values` (post-flip) |
| | `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs::run_header_key_prefix_is_0x10` (RunId-0 arm; post-flip) |
| | `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs::run_header_key_zero_run_id` (post-flip) |
| | `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs::index_workflow_key_zero_values` (post-flip) |
| | `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs::run_id_zero_roundtrip` (post-flip) |
| Refinement harness refs | `crates/vb_storage/src/proptests.rs::encoder_rejects_zero_run_id_for_every_prefix` (NEW proptest; iterates over the six encoders and asserts `matches!(result, Err(JournalError::InvalidRunId { run }))` for `run == RunId::new(0)`) |
| Evidence command | `PROPTEST_CASES=10000 cargo test --test proptest encoder_rejects_zero_run_id_for_every_prefix --release` |
| Evidence workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4` |
| Evidence artifact | `.evidence/proptest/vb_storage/encoder_rejects_zero_run_id_for_every_prefix_pass.log` |
| Expected evidence | `cargo test` reports `test result: ok. 1 passed; 0 failed` for `encoder_rejects_zero_run_id_for_every_prefix`; the proptest body iterates over the six encoder entry points and asserts `matches!(result, Err(JournalError::InvalidRunId { run }))` for `run == RunId::new(0)`; the strategy uses `prop::strategy::Just` to feed the `run == 0` literal (non-vacuous; tests the rejection arm explicitly); the anti-invariant asserts the result is NOT `Err(JournalError::InvalidEvent)` (the prior error surface) and NOT `Err(JournalError::KeyCapacity)` (a different encoder failure); the proptest runs alongside the 18 test flips (C5) as the property-test companion |
| Mapping status | planned |
| behavior_affecting | false |

### PO-006-PROPTEST-MUTATION (mutation-resistance proptest for `require_non_zero_run`)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-cn2v4-006 |
| Verifier | proptest |
| Proptest artifact | `crates/vb_storage/src/proptests.rs` (NEW property test `mutation_resistance_require_non_zero_run`) |
| Production target | `crates/vb_storage/src/keys.rs::require_non_zero_run` (NEW private helper; called by `run_only_key`, `sequenced_run_key`, `index_status_key`, `index_workflow_key`, `index_action_key`) |
| Source refs | `crates/vb_storage/src/keys.rs::require_non_zero_run` (NEW; centralised guard) |
| | `crates/vb_storage/src/keys.rs::run_only_key` (L514-521; call site) |
| | `crates/vb_storage/src/keys.rs::sequenced_run_key` (L480-496; call site) |
| | `crates/vb_storage/src/keys.rs::index_status_key` (L101-122; call site, fires before `state.to_u8_checked`) |
| | `crates/vb_storage/src/keys.rs::index_workflow_key` (L125-137; call site) |
| | `crates/vb_storage/src/keys.rs::index_action_key` (L140-155; call site) |
| | `crates/vb_storage/src/error/mod.rs::JournalError::InvalidRunId` (L140-141) |
| Behavior test refs | `crates/vb_storage/src/keys/tests.rs::run_header_key_with_zero_run_id` (post-flip; companion unit test for `run_only_key` call site) |
| | `crates/vb_storage/src/keys/tests.rs::run_event_key_length` (post-flip; companion unit test for `sequenced_run_key` call site) |
| | `crates/vb_storage/src/keys/tests.rs::run_snapshot_key_length` (post-flip; companion unit test for `sequenced_run_key` call site) |
| | `crates/vb_storage/src/keys/tests.rs::index_workflow_key_length` (post-flip; companion unit test for `index_workflow_key` call site) |
| | `crates/vb_storage/src/keys/tests.rs::index_action_key_length` (post-flip; companion unit test for `index_action_key` call site) |
| | `crates/vb_storage/src/keys/tests.rs::index_status_key_with_zero_values` (post-flip; companion unit test for `index_status_key` call site) |
| Refinement harness refs | `crates/vb_storage/src/proptests.rs::mutation_resistance_require_non_zero_run` (NEW proptest; constructs a guard-on closure (calls `require_non_zero_run` first) and a guard-off closure (does NOT call `require_non_zero_run`), and asserts the guard-on branch returns `Err(InvalidRunId)` for `run == RunId::new(0)` while the guard-off branch returns `Ok(_)`; this proves the guard is necessary and the proptest catches its removal) |
| Evidence command | `PROPTEST_CASES=1000 cargo test --test proptest mutation_resistance_require_non_zero_run --release` |
| Evidence workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4` |
| Evidence artifact | `.evidence/proptest/vb_storage/mutation_resistance_require_non_zero_run_pass.log` |
| Expected evidence | `cargo test` reports `test result: ok. 1 passed; 0 failed` for `mutation_resistance_require_non_zero_run`; the proptest body constructs the two closures (guard-on, guard-off) and asserts the divergent behaviour; this proptest acts as a regression test: if a future change removes the guard, the proptest fails because the guard-on and guard-off branches produce the same `Ok(_)` |
| Mapping status | planned |
| behavior_affecting | false |

## Implementation Task Summary for States 8-10 (Holzman-Rust; surfaced for downstream)

The following production source changes are required before the bridge's
`refinement_harness_refs` can be executed and the `behavior_test_refs`
can be flipped. These are owned by `rust-implementer` at State 10 (not
this bridge agent); they are surfaced here for downstream visibility.

### Task 1 — Add `require_non_zero_run` private helper (C2)

- **File**: `crates/vb_storage/src/keys.rs` (insert near top of private helpers, after `digest_key`)
- **Code** (Holzman-Rust; no `unwrap`/`expect`/`panic`):
  ```rust
  /// Returns `Err(JournalError::InvalidRunId { run })` iff `run.get() == 0`.
  /// Centralises the rejection so every run-bearing encoder inherits it.
  fn require_non_zero_run(run: RunId) -> Result<(), JournalError> {
      if run.get() == 0 {
          Err(JournalError::InvalidRunId { run })
      } else {
          Ok(())
      }
  }
  ```
- **Affected RROs**: RRO-001..006 (all six obligations bind to this helper)

### Task 2 — Insert `require_non_zero_run(run)?` into five private call sites (C2)

- **File**: `crates/vb_storage/src/keys.rs`
- **Insertion sites**:
  - `run_only_key` (L514-521): insert `require_non_zero_run(run)?;` as the first statement
  - `sequenced_run_key` (L480-496): insert `require_non_zero_run(run)?;` AFTER the existing `if seq.get() == u64::MAX { return Err(SequenceOverflow); }` check
  - `index_status_key` (L101-122): insert `require_non_zero_run(run)?;` BEFORE the `state.to_u8_checked()?` call (order-of-checks invariant; PO-004)
  - `index_workflow_key` (L125-137): insert `require_non_zero_run(run)?;` as the first statement
  - `index_action_key` (L140-155): insert `require_non_zero_run(run)?;` as the first statement
- **Inherited rejection**: `run_header_key`, `run_event_key`, `run_snapshot_key`, `journal_key`, `encode_key_into`, `encode_key`, `run_prefix_key` automatically inherit the rejection through their existing call-graph to the private helpers

### Task 3 — Defence-in-depth decision (C4)

- **File**: `crates/vb_storage/src/headers.rs:36-39`
- **Decision**: KEEP the manual `if run.get() == 0 { return Err(JournalError::InvalidRunId { run }); }` check in `FjallJournal::run_header` as defence-in-depth (minimal blast radius; no removal required). Document the decision in the implementation report.

### Task 4 — Extend `SpecKeyEncodeError` with `InvalidRunId { run: u64 }` (C7)

- **File**: `verification/verus/extern_vb_storage_keys.rs:199-204`
- **Insertion**:
  ```rust
  #[derive(Clone, Copy)]
  pub enum SpecKeyEncodeError {
      IndexStatusStateCollision,
      SequenceOverflow,
      KeyCapacity,
      InvalidRunId { run: u64 },  // NEW; mirrors JournalError::InvalidRunId
  }
  ```
- **Spec file**: when `verification/verus/vb_storage_keys_spec.rs` is created at State 5, attach `assume_specification` contracts to the run-bearing mirror fns (`run_event_key`, `journal_key`, `encode_key`) with the clause:
  ```text
  requires run != 0;
  ensures  result is Err(SpecKeyEncodeError::InvalidRunId { run })
           iff run == 0;
  ```

### Task 5 — Kani split-harness shape (C6)

- **File**: `crates/vb_storage/src/kani_typed_partitioned_ids.rs:51-92`
- **Change**: replace each `match keys::xxx_key(...) { Ok(key) => { ... } Err(_) => assert!(false) }` with a split `if/else` on `run_value == 0`:
  - If `run_value == 0`: assert `matches!(result, Err(JournalError::InvalidRunId { .. }))`
  - If `run_value != 0`: assert `Ok(key)` with byte-layout assertions (existing)
- **Reachability**: add `kani::cover!(run_value == 0)` and `kani::cover!(run_value != 0)` to prove both arms are reachable
- **Extension for PO-004**: cover `run_snapshot_key` and `index_status_key` (the two encoders not in the current `assert_key_contracts` body)

### Task 6 — Proptest additions (C5 companion)

- **File**: `crates/vb_storage/src/proptests.rs`
- **NEW property test**: `encoder_rejects_zero_run_id_for_every_prefix` (PO-005)
- **NEW property test**: `mutation_resistance_require_non_zero_run` (PO-006)

### Task 7 — 18-test flip (C5; owned by test-writer, not this bridge)

- **Files**: `crates/vb_storage/src/keys/tests.rs` (11), `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs` (3), `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs` (4)
- **Pattern**: flip each test's expectation from `Ok(...)` to `Err(JournalError::InvalidRunId { run: RunId::new(0) })`; for the two `index_status_key_rejects_other_state_in_collision_range` / `index_status_key_accepts_other_state_above_collision_range` tests, swap `RunId::new(0)` → `RunId::new(1)` to keep the collision path exercised

## Handoff for proof-reviewer

The following artifacts form the complete bridge output:

| Artifact | Path | Purpose |
|----------|------|---------|
| proof-to-rust-map.md | `.beads/vb-cn2v4/proof-to-rust-map.md` | Human-readable obligation-to-source mapping (this file) |
| rust-refinement-obligations.jsonl | `.beads/vb-cn2v4/rust-refinement-obligations.jsonl` | Machine-readable RRO rows (schema: `rust-refinement-obligation/v1`) |
| proof-to-rust-review.md | `.beads/vb-cn2v4/proof-to-rust-review.md` | Reviewer disposition (STATUS: APPROVED) |
| agent-invocation-ledger.jsonl | `.beads/vb-cn2v4/agent-invocation-ledger.jsonl` | Updated with seq N+1 entry (this bridge) |

## Unresolved Mapping Gaps

| Gap ID | Description | Impacted RROs |
|--------|-------------|---------------|
| GAP-VB-CN2V4-001 | `verification/verus/vb_storage_keys_spec.rs` does not exist; the plan contract references it for `assume_specification` clauses. The proof-writer (State 5) must create it with the `requires run != 0; ensures result is Err(InvalidRunId) iff run == 0` clause on the run-bearing mirror fns. | RRO-001, RRO-002 |
| GAP-VB-CN2V4-002 | `verification/verus/extern_vb_storage_keys.rs:47` references `production_inner/vb_storage_keys_production.rs` which does not exist. The proof-writer must either create the drift-detection stub or remove the `#[path]` inclusion. | RRO-001, RRO-002 |
| GAP-VB-CN2V4-003 | `require_non_zero_run` does not yet exist in `crates/vb_storage/src/keys.rs`. The rust-implementer (State 10) must add it as a private helper and insert calls into the five call sites. | RRO-001..006 (all six obligations) |
| GAP-VB-CN2V4-004 | The 18 unit-test flips and the two new proptests are owned by test-writer/test-planner (proof-planner Non-Goals; bridge surfaces them as `behavior_test_refs`). | RRO-005, RRO-006 |
| GAP-VB-CN2V4-005 | `kani_typed_partitioned_ids.rs::assert_key_contracts` still uses the `Err(_) => assert!(false)` arms; the Kani split-harness shape (PO-003, PO-004) is required before the Kani command can succeed without spurious counterexamples. | RRO-003, RRO-004 |

## Closure Path

| State | Action |
|-------|--------|
| State 8 (test-planning) | Reference `behavior_test_refs` in each RRO row for test scenario planning; plan the 18 unit-test flips per C5 |
| State 9 (test-writing) | Flip the 18 unit tests; add `encoder_rejects_zero_run_id_for_every_prefix` and `mutation_resistance_require_non_zero_run` proptests |
| State 10 (implementation) | Implement Tasks 1-3 (Holzman-Rust: add `require_non_zero_run` helper; insert into five call sites; document defence-in-depth decision) |
| State 11 (formal-verifier) | Implement Tasks 4-5 (extend `SpecKeyEncodeError`; reorganise Kani split-harness); run the Verus + Kani + proptest commands; close the ledger |
| State 12 (closure) | All six RRO rows must transition from `mapping_status: planned` to `mapping_status: verified` |