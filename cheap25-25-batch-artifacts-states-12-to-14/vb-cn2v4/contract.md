# Contract: vb-cn2v4 — Keys reject zero RunId (P1 bug)

## Acceptance Contract

Downstream states (rust-implementer, test-writer, proof-planner,
proof-writer, proof-reviewer) must implement and verify the
following behavior-affecting requirements.

### C1 Encoder/Decoder Symmetry

For every typed key encoder in
`crates/vb_storage/src/keys.rs` that accepts a `run: RunId`
argument, the encoder MUST return
`Err(JournalError::InvalidRunId { run })` for any input where
`run.get() == 0`. The encoders are:

- `run_header_key(run)` — line 76-78
- `run_event_key(run, seq)` — line 81-83
- `run_snapshot_key(run, seq)` — line 86-91
- `index_status_key(state, timestamp, run)` — line 101-122
- `index_workflow_key(workflow, run)` — line 125-137
- `index_action_key(action, run, step)` — line 140-155

The encoder result MUST be symmetric with the decoder: every byte
sequence the encoder emits as `Ok(bytes)` MUST round-trip through
`decode_storage_key` as `Ok(_)` (and conversely, the decoder's
existing `InvalidRunId` rejection is the source of truth).

### C2 Shared Guard Helper

A single private helper
`fn require_non_zero_run(run: RunId) -> Result<(), JournalError>`
in `crates/vb_storage/src/keys.rs` MUST centralise the rejection.
It MUST return `Err(JournalError::InvalidRunId { run })` iff
`run.get() == 0`. The helper MUST be called by:

- `run_only_key` (private) — line 514-521
- `sequenced_run_key` (private) — line 480-496 (called after the
  existing `seq.get() == u64::MAX` check)
- `index_status_key` — line 101-122 (called before
  `state.to_u8_checked()`)
- `index_workflow_key` — line 125-137 (called first)
- `index_action_key` — line 140-155 (called first)

Public entry points that delegate to the above (`run_header_key`,
`run_event_key`, `run_snapshot_key`, `journal_key`, `encode_key`,
`encode_key_into`, `run_prefix_key`) MUST inherit the rejection
without further edits.

### C3 Error Reuse

The encoder MUST reuse the existing variant
`JournalError::InvalidRunId { run: RunId }`
(`crates/vb_storage/src/error/mod.rs:140-141`). No new
`JournalError` variant may be added. Diagnostic code (`0x4021`,
`error/codes.rs:73`) and symbolic name (`INVALID_RUN_ID`,
`error/codes.rs:250`) are unchanged.

### C4 Manual Check in `headers.rs::run_header`

The manual `if run.get() == 0 { return Err(JournalError::InvalidRunId { run }); }`
check at `crates/vb_storage/src/headers.rs:36-39` is now redundant.
Either of the following contract-compliant shapes is permitted:

- **KEEP** the manual check as defence-in-depth (recommended;
  minimal blast radius).
- **REMOVE** the manual check and rely on `run_header_key`'s new
  rejection.

The decision is owned by the implementation agent and MUST be
documented in the implementation report.

### C5 Test Suite Flip (18 tests)

The following 18 tests MUST be flipped from
`Ok(...)` expectations to
`Err(JournalError::InvalidRunId { run: RunId::new(0) })`
expectations:

In `crates/vb_storage/src/keys/tests.rs` (11 tests):

1. `run_header_key_has_correct_prefix` — line 72-78
2. `run_event_key_length` — line 123-128
3. `index_status_key_has_correct_prefix` — line 190-195
4. `index_status_key_length` — line 214-219
5. `index_workflow_key_length` — line 246-251
6. `index_action_key_length` — line 284-289
7. `run_header_key_with_zero_run_id` — line 468-474
8. `index_status_key_with_zero_values` — line 507-514
9. `run_prefix_key_is_9_bytes` — line 587-592
10. `index_status_key_rejects_other_state_in_collision_range` —
    line 678-708 (replace `RunId::new(0)` with `RunId::new(1)` to
    keep the collision path exercised)
11. `index_status_key_accepts_other_state_above_collision_range` —
    line 710-717 (same `RunId::new(1)` swap)

In `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs`
(3 tests):

12. `encode_exact_length_run_header` — line 340-348
13. `encode_exact_length_run_event` — line 350-358
14. `encode_exact_length_index_action` — line 366-374

In `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs` (4 tests):

15. `run_header_key_prefix_is_0x10` — line 91-107 (Given "RunId with value 0" arm)
16. `run_header_key_zero_run_id` — line 109-125
17. `index_workflow_key_zero_values` — line 205-228
18. `run_id_zero_roundtrip` — line 699-711

### C6 Kani Harness Split

`crates/vb_storage/src/kani_typed_partitioned_ids.rs::assert_key_contracts`
MUST be reorganised so the `Err(_) => assert!(false)` arms do not
fire for `run_value == 0`. The recommended in-place if/else split
explicitly distinguishes the rejection path (`matches!(...,
Err(InvalidRunId { .. }))`) from the happy path
(layout assertions on the Ok bytes). Alternative shapes
(`kani::assume(run_value != 0)` at the top of the harness, or
splitting into two `#[kani::proof]` entry points) are accepted
provided the rejection path is exercised in at least one harness.

### C7 Verus Mirror Variant

`SpecKeyEncodeError` at
`verification/verus/extern_vb_storage_keys.rs:199-204` MUST be
extended with the variant
`SpecKeyEncodeError::InvalidRunId { run: u64 }`. The mirror
bodies of `run_event_key`, `journal_key`, and `encode_key` MUST
return this variant for `run == 0` inputs. The assume_specification
contracts in `vb_storage_keys_spec.rs` MUST include for every
run-bearing mirror fn:

```text
requires run != 0;
ensures  result is Err(SpecKeyEncodeError::InvalidRunId { run })
         iff run == 0;
```

The mirror must remain production-bound (GOD RULE 2): either
`#[path = ".../crates/vb_storage/src/..."]` direct inclusion
(STRONG), a drift-gated mirror at
`verification/verus/production_inner/vb_storage_keys_production.rs`
(WEAK), a companion `extern_*.rs` that itself binds to production
(WEAK), or an explicit `ALLOWED_EXCEPTIONS` row in
`scripts/check-verus-production-binding.sh` with a PO-XXXX
reference. `scripts/check-production-inner-drift.sh` MUST pass.

### C8 Decoder Unchanged

The decoder side remains untouched. `KeyDecodeError::InvalidRunId`
at `crates/vb_storage/src/error/key_decode.rs:28` continues to
reject `run == 0` bytes at `keys.rs:372-374, 381-383, 400-402,
412-414, 423-425`. The encoder tightening closes the asymmetric
gap; the decoder remains the source of truth.

### C9 Out-of-Scope Surfaces Preserved

The following surfaces MUST remain unchanged:

- `RunId::new` and `RunId::ZERO` (constructor invariant).
- Recovery diagnostics using `RunId::new(0)` as a
  `NoRecoveryData` placeholder
  (`recovery/replay/summary/{derive,apply,tests}.rs`).
- Workspace tests that build `RunId::new(0)` without reaching a
  key encoder
  (`vb_test_runtime_lifecycle_state_behavior.rs`,
  `integration_runtime_storage_fault_tolerance.rs`,
  `runtime_version_barrier_tests.rs`,
  `cancel_kill_lattice_props.rs`,
  `vb_core_yaml_e2e_chain_contract.rs`).
- `contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs`
  (TLA+ spec mirror using `RunId::Run(0)` as spec placeholder).
- `crates/vb_storage/src/proptests.rs::all_key_functions_are_deterministic`
  (already excludes zero from `run_val`).
- `crates/vb_storage/src/tests.rs::symbolic_code_table`
  (already maps `INVALID_RUN_ID`).

## Lane Profile (Hint for proof-planner)

The contract intentionally bounds the verification surface to:

- **Rust-local implementation lane:** Verus (`SpecKeyEncodeError`
  variant + assume_specification clauses), Kani (`assert_key_contracts`
  split), Flux (`JournalError::InvalidRunId` is a closed enum
  variant whose code mapping is straightforward), proptest
  (`encoder_rejects_zero_run_id_for_every_prefix` per-prefix
  coverage; mutation test to disable the guard).
- **No concurrency / Loom** (encoder is pure synchronous).
- **No TLA+** (the temporal workflow at risk has no model in this
  bead; master rules remove TLA+ from the lifecycle).
- **No Miri** (no `unsafe`).
- **No fuzz** required by contract; a fuzz target is a natural
  friendly-evidence surface for hostile input but is optional.

## Non-Goals

- No implementation in State 3 (rust-contract).
- No behavior tests or verifier harnesses authored in State 3.
- No broad storage migration beyond tightening the encoder.
- No constructor-invariant change to `RunId::new`.
- No new `JournalError` variant.

## Bridge Pointers for Later States

| Concern | File | Symbol |
|---|---|---|
| Production encoder | `crates/vb_storage/src/keys.rs` | `require_non_zero_run` (NEW), `run_only_key`, `sequenced_run_key`, `index_status_key`, `index_workflow_key`, `index_action_key` |
| Production error type | `crates/vb_storage/src/error/mod.rs` | `JournalError::InvalidRunId { run: RunId }` |
| Production error codes | `crates/vb_storage/src/error/codes.rs` | `INVALID_RUN_ID_CODE = 0x4021`, `INVALID_RUN_ID` |
| Production decoder | `crates/vb_storage/src/keys.rs` | `decode_storage_key` (lines 346-434) |
| Defence-in-depth manual check | `crates/vb_storage/src/headers.rs` | `FjallJournal::run_header` (lines 36-39) |
| Kani harness | `crates/vb_storage/src/kani_typed_partitioned_ids.rs` | `assert_key_contracts`, `vb_eepg_typed_partitioned_ids` |
| Kani template | `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:151` | Existing `Err(InvalidRunId)` pattern |
| Verus mirror | `verification/verus/extern_vb_storage_keys.rs` | `SpecKeyEncodeError` enum, `run_event_key`, `journal_key`, `encode_key` |
| Verus production-binding gate | `scripts/check-verus-production-binding.sh` | ALLOWED_EXCEPTIONS or direct binding |
| Verus drift gate | `scripts/check-production-inner-drift.sh` | mirror drift check |
| Production tests | `crates/vb_storage/src/keys/tests.rs` | 11 tests (see C5) |
| Workspace tests | `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs` | 3 tests (see C5) |
| Workspace tests | `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs` | 4 tests (see C5) |
| Already-correct tests (templates) | `crates/workspace_tests/tests/restate_doctor_key_decode_tests.rs:353-396` | decoder-side `InvalidRunId` examples |
| Proptest (already excludes 0) | `crates/vb_storage/src/proptests.rs:170` | `all_key_functions_are_deterministic` |