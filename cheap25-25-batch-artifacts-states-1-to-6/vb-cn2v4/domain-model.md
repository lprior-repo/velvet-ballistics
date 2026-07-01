# Domain Model: vb-cn2v4 — Keys reject zero RunId (P1 bug)

## Scope

- Bead: `vb-cn2v4`
- State: 3 / `rust-contract`
- Feature slice: tighten every typed storage-key encoder in
  `crates/vb_storage/src/keys.rs` to reject `RunId(0)` with the
  existing `JournalError::InvalidRunId { run: RunId }` variant
  (diagnostic code `0x4021`, symbolic code `INVALID_RUN_ID`).
- The encoder/decoder pair is currently asymmetric: encoders accept
  `RunId(0)` and emit all-zero `run_id` bytes, while the decoder
  (`keys.rs:372-374, 381-383, 400-402, 412-414, 423-425`) rejects
  those same bytes via `KeyDecodeError::InvalidRunId`. The encoder
  must be tightened to match.
- Out of scope for this state: production Rust implementation, tests,
  verifier harnesses, proof obligations, and proof review approval.

## Ubiquitous Language

| Term | Meaning | Contract relevance |
|---|---|---|
| RunId | Numeric `u64` identity for a shard-owned run. Newtype at `crates/vb_core/src/ids/mod.rs:24-55`. | `RunId::new(0)` is constructor-valid but **semantically invalid** for any storage key; validation lives in the encoder, not the constructor. |
| Valid RunId (key context) | A `RunId` whose `get() != 0` AND whose value is not reserved for any future sentinel. | The encoder must reject `RunId(0)`. Non-zero values are in-scope keys. |
| Storage key | Fixed-format byte slice with a single-byte prefix followed by big-endian fields. | Nine known prefixes; six of them (RunHeader, RunEvent, RunSnapshot, IndexStatus, IndexWorkflow, IndexAction) carry `run` and inherit the rejection. |
| Key encoder | Pure function `prefix/typed args -> Result<[u8; N], JournalError>`. | Must fail closed for invalid inputs; must not emit bytes the same module refuses to decode. |
| Key decoder | Pure function `[u8] -> Result<StorageKey, KeyDecodeError>`. | Already rejects `run == 0` via `KeyDecodeError::InvalidRunId`. This contract is the source of truth the encoder must mirror. |
| `require_non_zero_run` | Private helper `fn(RunId) -> Result<(), JournalError>`. | Single shared guard for every encoder path; returns `JournalError::InvalidRunId { run }` when `run.get() == 0`. |
| `JournalError::InvalidRunId` | Existing typed error variant carrying the offending `RunId`. Code `0x4021`; symbolic `INVALID_RUN_ID`. | Reuse this variant; do NOT add a new encoder-side variant. |
| `SpecKeyEncodeError::InvalidRunId` | Verus mirror variant for the same condition; field `run: u64`. | Required so the assume_specification contracts can bind to production rejection semantics. |

## Entities and Decisions

### Encoder Decision (pure)

Conceptual pure decision owned by every key encoder entry point:

```text
encode(prefix, payload) -> Result<[u8; N], JournalError>
  - require_non_zero_run(run) ?                // NEW (this bead)
  - validate_other_fields(prefix-specific) ?
  - write_bytes(prefix, payload)
```

### Decision Sequence (after this bead)

1. `require_non_zero_run(run)` returns `Err(InvalidRunId { run })` for `RunId(0)`.
2. Prefix-specific validation (e.g. `IndexStatusState::to_u8_checked`, `seq != MAX`) runs next.
3. Byte emission runs only when every check has passed.

The guard is shared by every typed encoder via the existing delegation
graph:

- `run_header_key(run)` -> `run_only_key(PREFIX_RUN_HEADER, run)`
- `run_event_key(run, seq)` -> `journal_key(run, seq)` -> `sequenced_run_key(PREFIX_RUN_EVENT, run, seq)`
- `run_snapshot_key(run, seq)` -> `sequenced_run_key(PREFIX_RUN_SNAPSHOT, run, seq)`
- `index_status_key(state, ts, run)` -> explicit guard then byte layout (no helper)
- `index_workflow_key(workflow, run)` -> explicit guard then byte layout
- `index_action_key(action, run, step)` -> explicit guard then byte layout
- `encode_key_into` / `encode_key` / `run_prefix_key` -> all inherit the new rejection via their delegates
- `headers.rs::FjallJournal::run_header` -> already has a manual `if run.get() == 0` check; the encoder rejection makes this redundant (KEEP as defence-in-depth per bead text)

## Value Objects

| Value object | Invariant |
|---|---|
| `RunId` | Numeric newtype; constructor invariant (no validation in `new`). |
| `RunId::ZERO` | Sentinel constant `RunId(0)`; used as `NoRecoveryData` placeholder, NOT as a key. Out of scope. |
| `ValidRunId` (proof concept) | A `RunId` known non-zero. Encoder output precondition for the `run` field. |
| `SpecKeyEncodeError` (Verus mirror) | Closed enum with `IndexStatusStateCollision`, `SequenceOverflow`, `KeyCapacity`, **NEW `InvalidRunId { run: u64 }`**. |
| `JournalError::InvalidRunId { run: RunId }` | Production typed error. Diagnostic code `0x4021`; symbolic `INVALID_RUN_ID`. Already present. |

## Policies

1. **Encoder/decoder symmetry:** every key encoder that emits a `run`
   byte field must reject `RunId(0)` with the same typed error the
   decoder would surface on those bytes.
2. **Single shared guard:** the rejection is implemented by exactly
   one private helper `require_non_zero_run`; typed encoders call it
   (directly or via `run_only_key` / `sequenced_run_key`).
3. **Explicit check in `index_*` fns:** because `index_status_key`,
   `index_workflow_key`, `index_action_key` bypass the `run_only_key`
   helper, they must call `require_non_zero_run` directly at the top
   of their bodies, before any byte mutation.
4. **Reuse existing error:** the encoder must reuse
   `JournalError::InvalidRunId { run: RunId }`. No new error variant
   is added to `JournalError`; the only new variant is in the
   Verus-mirror `SpecKeyEncodeError`.
5. **Pre-`is_valid()` rejection at call sites:** when journal append
   paths (`append_strict`, `append_unfsynced`, `inject_raw_event`,
   `inject_seq_gap`, `stage_queued_event`, `append_event`,
   `stage_pending_action_index_op`, `put_snapshot`, `snapshot`,
   `put_status_index`, `put_workflow_index`, `put_action_index`,
   `delete_action_index`, `put_run_header`,
   `trim_events_for_run`, `trim_run_event_log`,
   `trim_eligibility_diagnostic`) call the encoder before checking
   event-level semantics, the encoder rejection now fires first.
   For `RunId(0)` inputs, the typed error returned shifts from
   `InvalidEvent` to `InvalidRunId`. This is desirable: it is
   specific, typed, and surfaces the actual cause.

## Domain Decisions

- `RunId::new` stays unchanged. Making the constructor validate would
  ripple through every call site (including `NoRecoveryData`
  placeholders); the bead text asks for the rejection in the encoder,
  not the type. See `crates/vb_core/src/ids/mod.rs:24-55`.
- `RunId::ZERO` stays. Recovery diagnostics use it as a
  "no result" marker at `crates/vb_storage/src/recovery/replay/summary/{derive,apply}.rs`
  and `recovery/replay/summary/tests.rs`. These callers never reach
  a key encoder and remain functional.
- `headers.rs::FjallJournal::run_header`'s manual `if run.get() == 0`
  check (`headers.rs:36-39`) stays as defence-in-depth. After the
  patch it is redundant; removing it is permitted but not required.
  The contract permits both.
- `proptests.rs` `all_key_functions_are_deterministic` (`proptests.rs:170`)
  already restricts `run_val in 1u64..=1000u64`, excluding zero. No
  change required.
- The symbolic-code table at `tests.rs:7670-7680` already maps
  `JournalError::InvalidRunId { .. } => "invalid_run_id"`. No change
  required.
- Proptest-regression files: no pre-staged regressions for this bead.

## Out of Scope (Must Not Touch)

- `crates/vb_core/src/ids/mod.rs` and
  `crates/vb_core/src/ids/parts/chunk_001_custom_types.rs` —
  `RunId::new` and `RunId::ZERO` stay as-is.
- `crates/vb_storage/src/recovery/replay/summary/{derive,apply,tests}.rs`
  — `NoRecoveryData { run: RunId::new(0) }` placeholder pattern.
- Workspace tests that use `RunId::new(0)` as a runtime placeholder
  and never reach a key encoder
  (`vb_test_runtime_lifecycle_state_behavior.rs`,
  `integration_runtime_storage_fault_tolerance.rs`,
  `runtime_version_barrier_tests.rs`,
  `cancel_kill_lattice_props.rs`,
  `vb_core_yaml_e2e_chain_contract.rs`).
- `contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs` —
  TLA+ spec mirror uses `RunId::Run(0)` as a spec placeholder.
- `JournalError::InvalidRunId { run }` definition itself
  (`error/mod.rs:140-141`) — already exists, just reuse it.
- `KeyDecodeError::InvalidRunId` (`error/key_decode.rs:28`) —
  decoder side is correct.
- `tests.rs::symbolic_code_table` (`tests.rs:7670-7680`) — already
  maps `INVALID_RUN_ID`.
- `crates/vb_storage/src/proptests.rs::all_key_functions_are_deterministic`
  — already excludes zero from `run_val`.