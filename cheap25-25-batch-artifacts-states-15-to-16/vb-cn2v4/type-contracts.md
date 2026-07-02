# Type Contracts: vb-cn2v4 — Keys reject zero RunId (P1 bug)

## Desired Type Shape

These are contracts for downstream implementation/proof planning, not
implementation code.

```text
// Private helper, production source
fn require_non_zero_run(run: RunId) -> Result<(), JournalError>;

// Public typed encoder Result semantics (after this bead)
fn run_header_key(run: RunId)
  -> Result<[u8; RUN_ONLY_KEY_BYTES], JournalError>;     // Err(InvalidRunId { run }) iff run.get() == 0

fn run_event_key(run: RunId, seq: EventSeq)
  -> Result<[u8; JOURNAL_KEY_BYTES], JournalError>;      // Err(InvalidRunId { run }) iff run.get() == 0

fn run_snapshot_key(run: RunId, seq: EventSeq)
  -> Result<[u8; JOURNAL_KEY_BYTES], JournalError>;      // Err(InvalidRunId { run }) iff run.get() == 0

fn index_status_key(state: IndexStatusState, timestamp: u64, run: RunId)
  -> Result<[u8; INDEX_STATUS_KEY_BYTES], JournalError>; // Err(InvalidRunId { run }) iff run.get() == 0

fn index_workflow_key(workflow: WorkflowId, run: RunId)
  -> Result<[u8; INDEX_WORKFLOW_KEY_BYTES], JournalError>; // Err(InvalidRunId { run }) iff run.get() == 0

fn index_action_key(action: ActionId, run: RunId, step: StepIdx)
  -> Result<[u8; INDEX_ACTION_KEY_BYTES], JournalError>;  // Err(InvalidRunId { run }) iff run.get() == 0

fn encode_key_into(key: &StorageKey, out: &mut Vec<u8>)
  -> Result<(), JournalError>;                           // inherits rejection

fn encode_key(key: StorageKey)
  -> Result<Vec<u8>, JournalError>;                      // inherits rejection

fn journal_key(run: RunId, seq: EventSeq)
  -> Result<[u8; JOURNAL_KEY_BYTES], JournalError>;      // inherits rejection

pub(crate) fn run_prefix_key(run: RunId)
  -> Result<[u8; 9], JournalError>;                      // inherits rejection
```

## Public API Contracts

| API | Precondition | Success postcondition | Rejection postcondition |
|---|---|---|---|
| `run_header_key(run)` | None. | Emits `[0x10][run.get().to_be_bytes()]`; `key[1..9] != [0;8]`. | `Err(InvalidRunId { run })` iff `run.get() == 0`. |
| `run_event_key(run, seq)` | None. | Emits `[0x11][run.get().to_be_bytes()][seq.get().to_be_bytes()]`; both fields non-zero/MAX respectively. | `Err(InvalidRunId { run })` if `run.get() == 0`. `Err(SequenceOverflow)` if `seq.get() == u64::MAX`. |
| `run_snapshot_key(run, seq)` | None. | Emits `[0x12][run.get().to_be_bytes()][seq.get().to_be_bytes()]`; `run != 0`. | `Err(InvalidRunId { run })` if `run.get() == 0`. `Err(SequenceOverflow)` if `seq.get() == u64::MAX`. |
| `index_status_key(state, timestamp, run)` | None. | Emits `[0x30][state_byte][timestamp.to_be_bytes()][run.get().to_be_bytes()]`; `run != 0`. | `Err(InvalidRunId { run })` if `run.get() == 0`. `Err(IndexStatusStateCollision)` if `state == Other(v)` with `v < 3`. |
| `index_workflow_key(workflow, run)` | None. | Emits `[0x31][workflow.get().to_be_bytes()][run.get().to_be_bytes()]`; `run != 0`. | `Err(InvalidRunId { run })` if `run.get() == 0`. |
| `index_action_key(action, run, step)` | None. | Emits `[0x32][action.get().to_be_bytes()][run.get().to_be_bytes()][step.get().to_be_bytes()]`; `run != 0`. | `Err(InvalidRunId { run })` if `run.get() == 0`. |
| `encode_key_into` / `encode_key` | `out` clearable for `encode_key_into`. | Same byte layout per variant. | For run-bearing variants: `Err(InvalidRunId { run })` for `RunId(0)`. For non-run-bearing variants (`WorkflowSource`, `CompiledIr`, `Blob`): unchanged. |
| `journal_key(run, seq)` | None. | Same as `run_event_key`; alias. | Same. |
| `run_prefix_key(run)` | None. | Emits `[0x11][run.get().to_be_bytes()]`; `run != 0`. | `Err(InvalidRunId { run })` if `run.get() == 0`. |

## Internal Helper Contracts

### `require_non_zero_run(run: RunId) -> Result<(), JournalError>`

| Condition | Result |
|---|---|
| `run.get() == 0` | `Err(JournalError::InvalidRunId { run })`. |
| `run.get() != 0` | `Ok(())`. |

Pure, total over `RunId`. No allocation. No I/O. No clock. No Fjall.

### `sequenced_run_key(prefix, run, seq)`

Order of checks (top-to-bottom is the explicit binding):

1. `seq.get() == u64::MAX` -> `Err(SequenceOverflow)`. **Unchanged.**
2. **NEW** `require_non_zero_run(run)` -> `Err(InvalidRunId { run })` iff `run.get() == 0`.
3. Emit `[prefix][run.get().to_be_bytes()][seq.get().to_be_bytes()]`.

### `run_only_key(prefix, run)`

Order of checks:

1. **NEW** `require_non_zero_run(run)` -> `Err(InvalidRunId { run })` iff `run.get() == 0`.
2. Emit `[prefix][run.get().to_be_bytes()]`.

### `index_status_key`, `index_workflow_key`, `index_action_key`

Order of checks:

1. **NEW** `require_non_zero_run(run)` -> `Err(InvalidRunId { run })` iff `run.get() == 0`.
2. `index_status_key`-only: `state.to_u8_checked()?` for `IndexStatusStateCollision`.
3. Emit prefix and field bytes.

The `to_u8_checked` step in `index_status_key` runs only after the
new guard, so the encoder order for `IndexStatusState::Other(0..2)`
with `RunId(0)` is `InvalidRunId` (not `IndexStatusStateCollision`).

## Storage Type Contracts

| Symbol | Contract |
|---|---|
| `JournalError::InvalidRunId { run: RunId }` | Already defined (`error/mod.rs:140-141`); diagnostic code `0x4021` (`error/codes.rs:73`); symbolic `INVALID_RUN_ID` (`error/codes.rs:250`). Reuse, do NOT add new variant. |
| `SpecKeyEncodeError` | Verus mirror at `verification/verus/extern_vb_storage_keys.rs:199-204`. Must add `InvalidRunId { run: u64 }` variant. Field type `u64` mirrors the production `RunId::get()` representation used by `decode_storage_key`. |
| `KeyDecodeError::InvalidRunId` | Already exists at `error/key_decode.rs:28`. Decoder-side invariant is the source of truth the encoder must mirror. |

## Encoder/Decoder Symmetry Invariant

For every byte sequence `B` produced by an encoder:

```
encoder(B_decode_round_trip) == Ok(B)
                iff
decode(B) == Ok(K) for some K
```

Equivalently: every byte sequence accepted by the decoder was
emitted by exactly one (prefix, run, seq, ...) tuple, and every
encoder call that would produce bytes the decoder rejects must
itself reject before emission.

This is the precise form of the P1 bug:

- Today: encoder emits `(prefix, run=0, ...)`; decoder rejects the
  same bytes. Asymmetric.
- After this bead: encoder rejects `run=0` first; symmetric.

## Illegal States to Make Unrepresentable

- Encoder result `Ok(bytes)` where any subsequent `decode_storage_key(bytes)`
  would return `Err(KeyDecodeError::InvalidRunId)`.
- `StorageKey::RunHeader { run: RunId(0) }` produced by an encoder call.
- `StorageKey::RunEvent { run: RunId(0), seq: _ }` produced by an encoder call.
- `StorageKey::RunSnapshot { run: RunId(0), seq: _ }` produced by an encoder call.
- `StorageKey::IndexStatus { run: RunId(0), .. }` produced by an encoder call.
- `StorageKey::IndexWorkflow { run: RunId(0), .. }` produced by an encoder call.
- `StorageKey::IndexAction { run: RunId(0), .. }` produced by an encoder call.

## Forbidden Implementation Shapes

- Adding a new `JournalError` variant for the encoder rejection (reuse `InvalidRunId`).
- Removing the manual `if run.get() == 0` check from
  `crates/vb_storage/src/headers.rs:36-39`. The check is permitted
  to be removed (now redundant) but is permitted to stay (defence-in-depth).
  Either choice is contract-compliant.
- Tightening `RunId::new` to reject zero. Constructor invariant must
  remain unchanged; validation lives in the encoder per bead text.
- Adding `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg!`
  in the encoder paths.
- Using `IndexStatusStateCollision` as the rejection error for
  `RunId(0)` inputs in `index_status_key` (the new
  `InvalidRunId` rejection fires first; tests must be flipped to
  reflect this order).

## Verus Mirror Type Contract (extern_vb_storage_keys.rs)

`SpecKeyEncodeError` at `verification/verus/extern_vb_storage_keys.rs:199-204`
must be extended with:

```text
SpecKeyEncodeError::InvalidRunId { run: u64 }
```

The assume_specification contracts for the following mirror fns must
include a clause: "`run == 0` returns `Err(SpecKeyEncodeError::InvalidRunId { run })`":

- `run_event_key(run, seq)` (mirror of `crates/vb_storage/src/keys.rs:81-83`)
- `journal_key(run, seq)` (mirror of `crates/vb_storage/src/keys.rs:476-478`)
- `encode_key(SpecStorageKey::RunHeader { run })`
- `encode_key(SpecStorageKey::RunEvent { run, seq })`
- `encode_key(SpecStorageKey::RunSnapshot { run, seq })`
- `encode_key(SpecStorageKey::IndexStatus { run, .. })`
- `encode_key(SpecStorageKey::IndexWorkflow { run, .. })`
- `encode_key(SpecStorageKey::IndexAction { run, .. })`

The non-run variants (`WorkflowSource`, `CompiledIr`, `Blob`) remain
unchanged: they cannot carry `run`, so `InvalidRunId` is unreachable.

The mirror drift-gate at
`verification/verus/production_inner/vb_storage_keys_production.rs`
already pins `crates/vb_storage/src/keys.rs::run_event_key` at
line 79-80 of its header comment. The new rejection must surface in
the production comment block too.