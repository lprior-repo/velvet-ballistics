# Boundary Map: vb-cn2v4 — Keys reject zero RunId (P1 bug)

## Pure Core Boundary

Conceptual pure decisions owned by every key encoder entry point:

- `require_non_zero_run(run) -> Result<(), JournalError>` is a
  pure decision over the `RunId` value.
- Per-encoder prefix-specific validations:
  - `sequenced_run_key`: `seq != u64::MAX`.
  - `index_status_key`: `state.to_u8_checked()` rejects
    `Other(v < 3)`.
- Byte-layout emission (big-endian, `ArrayVec`-bounded).

These decisions must remain expressible without I/O, clocks, queues,
Fjall, runtime mutation, or `unsafe`.

## Imperative Encoder Boundary

Files/symbols that participate in the encoder tightening:

### `crates/vb_storage/src/keys.rs` — primary encoder file

| Symbol | File:line | Edit shape |
|---|---|---|
| `require_non_zero_run` (NEW) | n/a | New private helper at the top of the file or near `digest_key`. Body: `if run.get() == 0 { return Err(JournalError::InvalidRunId { run }); } Ok(())`. |
| `run_only_key` | `keys.rs:514-521` | Add `require_non_zero_run(run)?;` as the first statement. |
| `sequenced_run_key` | `keys.rs:480-496` | Add `require_non_zero_run(run)?;` as the second statement (after the existing `seq.get() == u64::MAX` check, but as a guard inside the body — see `workflow-model.md` for the precise ordering). The current `seq.get() == u64::MAX` check stays first. |
| `index_status_key` | `keys.rs:101-122` | Add `require_non_zero_run(run)?;` as the first statement. `to_u8_checked` runs second. |
| `index_workflow_key` | `keys.rs:125-137` | Add `require_non_zero_run(run)?;` as the first statement. |
| `index_action_key` | `keys.rs:140-155` | Add `require_non_zero_run(run)?;` as the first statement. |
| `run_prefix` (private) | `keys.rs:498-500` | Inherits rejection via `run_only_key`. No edit. |
| `run_prefix_key` (pub(crate)) | `keys.rs:524-526` | Inherits rejection via `run_prefix`. No edit. |
| `journal_key` (public alias) | `keys.rs:476-478` | Inherits rejection via `sequenced_run_key`. No edit. |
| `run_header_key` | `keys.rs:76-78` | Inherits rejection via `run_only_key`. No edit. |
| `run_event_key` | `keys.rs:81-83` | Inherits rejection via `journal_key`. No edit. |
| `run_snapshot_key` | `keys.rs:86-91` | Inherits rejection via `sequenced_run_key`. No edit. |
| `encode_key_into` | `keys.rs:162-198` | Inherits rejection via per-arm encoder dispatch. No edit to logic. |
| `encode_key` | `keys.rs:205-209` | Inherits rejection via `encode_key_into`. No edit. |

### `crates/vb_storage/src/headers.rs` — manual check becomes redundant

| Symbol | File:line | Edit shape |
|---|---|---|
| `FjallJournal::run_header` | `headers.rs:36-39` | **Optional:** the manual `if run.get() == 0 { return Err(JournalError::InvalidRunId { run }); }` is now redundant because `run_header_key` rejects the same input. Two contract-compliant shapes: (a) **KEEP** as defence-in-depth (recommended; minimal blast radius), or (b) **REMOVE** the manual check and rely on `run_header_key`. The contract permits both; the implementation agent decides. |
| `FjallJournal::put_run_header` | `headers.rs:19` | No edit; inherits rejection via `run_header_key`. |

### Call sites where error semantics shift (no edit; behaviour changes)

These call sites invoke an encoder BEFORE the event-level semantic
check; after this bead, `RunId(0)` inputs return
`Err(InvalidRunId { run })` instead of `Err(InvalidEvent)` (or
silently emitted bytes):

- `crates/vb_storage/src/journal/append.rs:47` (`append_strict`).
- `crates/vb_storage/src/journal/internal.rs:55` (`append_unfsynced`).
- `crates/vb_storage/src/journal/injection.rs:22` (`inject_raw_event`).
- `crates/vb_storage/src/journal/injection.rs:42` (`inject_seq_gap`).
- `crates/vb_storage/src/queue/writer/stage.rs:37` (`stage_queued_event`).
- `crates/vb_storage/src/batch/append_event.rs:43` (`append_event`
  G1 guard).
- `crates/vb_storage/src/batch/action_index.rs:116, 121`
  (`stage_pending_action_index_op` Insert/Remove).
- `crates/vb_storage/src/snapshots.rs:32, 53` (`put_snapshot`,
  `snapshot` reader).
- `crates/vb_storage/src/indexes.rs:21, 32, 44, 62`
  (`put_status_index`, `put_workflow_index`, `put_action_index`,
  `delete_action_index`).
- `crates/vb_storage/src/trimming/logic.rs:60, 213, 246`
  (`trim_events_for_run`, `trim_run_event_log`,
  `trim_eligibility_diagnostic`).

## Verification Boundary

### Kani harness

| Symbol | File:line | Edit shape |
|---|---|---|
| `assert_key_contracts` | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:43-93` | **MUST** reorganise so the `Err(_) => assert!(false)` arms do not fire for `run_value == 0`. The recommended in-place if/else split (or one of the alternatives in `workflow-model.md` § Kani Harness Workflow) makes the rejection path explicit. |
| `vb_eepg_typed_partitioned_ids` | `kani_typed_partitioned_ids.rs:111-115` | Inherits the new contract from `assert_key_contracts`. No edit to the entry point itself if the split is in-place. |
| `vb_eepg_record_kind_contracts` | `kani_typed_partitioned_ids.rs:117-121` | No edit (unrelated). |
| `vb_eepg_unknown_record_kind_error_contract` | `kani_typed_partitioned_ids.rs:123-129` | No edit (unrelated). |

### Verus mirror

| Symbol | File:line | Edit shape |
|---|---|---|
| `SpecKeyEncodeError` enum | `verification/verus/extern_vb_storage_keys.rs:199-204` | **NEW variant:** `InvalidRunId { run: u64 }`. |
| `run_event_key` mirror | `extern_vb_storage_keys.rs:303-307` | **Body edit:** add `if run == 0 { return Err(SpecKeyEncodeError::InvalidRunId { run }); }` before the `journal_key` delegation. The mirror body is `#[verifier::external]` so the edit is a trusted-base projection; the contract is pinned by the assume_specification in `vb_storage_keys_spec.rs`. |
| `journal_key` mirror | `extern_vb_storage_keys.rs:276-301` | **Body edit:** add the same guard. The existing `seq == u64::MAX` check stays first. |
| `encode_key` mirror | `extern_vb_storage_keys.rs:320-344` | **Body edit:** guard the run-bearing `Ok(...)` arms so they return `Err(SpecKeyEncodeError::InvalidRunId { run })` for `run == 0`. Non-run variants (`WorkflowSource`, `CompiledIr`, `Blob`) remain `Ok(...)` for all inputs. |
| `vb_storage_keys_spec.rs` (assume_specification contracts) | referenced by the binding ledger | Each run-bearing mirror's `assume_specification` clause must include `ensures result is Err(InvalidRunId { run }) iff run == 0`. |

### Verus production-binding gates

- `scripts/check-verus-production-binding.sh` MUST pass:
  - The new `SpecKeyEncodeError::InvalidRunId` variant must bind
    to production `JournalError::InvalidRunId { run }` via either
    (a) `#[path = ".../crates/vb_storage/src/keys.rs"]` direct
    inclusion (STRONG), (b) drift-gated mirror at
    `verification/verus/production_inner/vb_storage_keys_production.rs`
    (WEAK), (c) companion `extern_*.rs` that itself binds to
    production or mirror (WEAK), or (d) explicit `ALLOWED_EXCEPTIONS`
    entry with PO-XXXX reference.
- `scripts/check-production-inner-drift.sh` MUST pass against the
  updated mirror.

## Storage Boundary

The encoder boundary interacts with the storage keyspace as follows:

- `RunHeader` (prefix `0x10`) — no `RunId(0)` rows produced.
- `RunEvent` (prefix `0x11`) — no `RunId(0)` rows produced.
- `RunSnapshot` (prefix `0x12`) — no `RunId(0)` rows produced.
- `IndexStatus` (prefix `0x30`) — no `RunId(0)` rows produced.
- `IndexWorkflow` (prefix `0x31`) — no `RunId(0)` rows produced.
- `IndexAction` (prefix `0x32`) — no `RunId(0)` rows produced.
- Non-run-bearing keyspaces (`WorkflowSource` `0x01`, `CompiledIr`
  `0x02`, `Blob` `0x20`) — unchanged; their encoders do not accept a
  `run` argument.

Existing rows persisted prior to this bead are unaffected: the
decoder already rejected `run == 0` bytes, so no such rows exist in
the keyspace.

## Time Boundary

- The encoder is purely synchronous and free of clocks. The new
  guard adds only an integer compare against `0`. No time
  interactions.

## Concurrency Boundary

- The encoder is purely synchronous and free of locks, atomics,
  channels, or thread-local state. No concurrency interactions.
- Loom schedule exploration is NOT applicable (no shared mutable
  state on the encoder path).

## FFI / Unsafe Boundary

- No FFI calls in the encoder paths.
- No `unsafe` blocks (`#![forbid(unsafe_code)]` at `keys.rs:1` and
  throughout the crate).

## Forbidden Boundary Crossings

- The encoder must not call into Fjall, the runtime, the network,
  the clock, or any I/O boundary.
- The encoder must not allocate a heap `Vec<u8>` for the run-bearing
  return types (the public typed signatures return fixed-size
  arrays; `encode_key` allocates only via the `Vec<u8>` return
  for the 6 run-bearing variants, unchanged from current).
- The Verus mirror must not introduce new trusted-base boundaries
  beyond the existing `#[verifier::external]` decoration on
  `run_event_key`, `journal_key`, and `encode_key`.

## Out-of-Scope Boundaries (Must Not Touch)

- `crates/vb_core/src/ids/mod.rs` — `RunId::new` stays as-is.
- `crates/vb_core/src/ids/parts/chunk_001_custom_types.rs` —
  `RunId::ZERO` stays.
- `crates/vb_storage/src/recovery/replay/summary/{derive,apply,tests}.rs`
  — `NoRecoveryData` placeholder pattern stays.
- Workspace tests that build `RunId::new(0)` without reaching a
  key encoder (listed in `delivery-scope.jsonl` out-of-scope
  section) — unchanged.
- `contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs` —
  TLA+ spec mirror stays (uses `RunId::Run(0)` as spec
  placeholder).