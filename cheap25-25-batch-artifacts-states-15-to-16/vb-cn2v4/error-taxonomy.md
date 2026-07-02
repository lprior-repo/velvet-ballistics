# Error Taxonomy: vb-cn2v4 — Keys reject zero RunId (P1 bug)

## Storage Errors

| Domain condition | Existing error | Diagnostic code | Symbolic code | Contract |
|---|---|---|---|---|
| Encoder called with `run.get() == 0` | `JournalError::InvalidRunId { run: RunId }` | `0x4021` (`INVALID_RUN_ID_CODE`, `error/codes.rs:73`) | `INVALID_RUN_ID` (`error/codes.rs:250`) | **MUST** surface for every typed encoder that accepts a `run` argument when `run.get() == 0`. No new `JournalError` variant is permitted. |
| Decoder reads `run_val == 0` | `KeyDecodeError::InvalidRunId` (no payload) | n/a (decoder side; distinct error type) | n/a | Already enforced at `keys.rs:372-374, 381-383, 400-402, 412-414, 423-425`. Source of truth the encoder mirrors. |
| Verus mirror rejects `run == 0` | `SpecKeyEncodeError::InvalidRunId { run: u64 }` (**NEW**) | n/a (Verus-mirror type) | n/a | New variant required in `verification/verus/extern_vb_storage_keys.rs:199-204`. Field type `u64` matches the production `RunId::get()` representation. |

## Error-Semantics Decisions

- The `JournalError::InvalidRunId { run: RunId }` variant already
  carries the offending `RunId` value as a structured field. The
  encoder must propagate the full `RunId` (not just the u64
  primitive) so diagnostic consumers can display it via
  `Display` and `Debug` and so subsequent pattern-matching
  can recover the value.
- The `0x4021` code is already allocated to `INVALID_RUN_ID_CODE`
  and registered in the `CODE_REGISTRY` (per
  `error/codes.rs:73`); no new code is allocated.
- The `INVALID_RUN_ID` symbolic name is already mapped at
  `error/codes.rs:250`; no new symbolic name is registered.
- For `RunId(0)` inputs reaching the journal append path:
  - **Before this bead:** the encoder emits all-zero `run` bytes
    (the caller subsequently sees `JournalError::InvalidEvent` from
    `JournalEvent::is_valid()` if it ever runs).
  - **After this bead:** the encoder returns
    `Err(JournalError::InvalidRunId { run })` at the encoder call
    site, **before** `is_valid()` is consulted. This is desirable
    because the typed error names the precise cause.
- The decoder side keeps emitting `KeyDecodeError::InvalidRunId`
  (no payload); the asymmetry between production error variants
  (encoder carries `RunId`, decoder does not) is unchanged.

## Forbidden Error Patterns

- Adding a new `JournalError` variant for this rejection. The
  existing `InvalidRunId` is the only acceptable variant.
- Returning `Err(JournalError::IndexStatusStateCollision)` for
  `RunId(0)` inputs to `index_status_key`. The new `InvalidRunId`
  rejection fires first; collision logic remains reachable only
  for `RunId != 0` with `Other(v < 3)`.
- Returning `Err(JournalError::SequenceOverflow)` for `RunId(0)`
  inputs to `run_event_key` / `run_snapshot_key`. The new
  `InvalidRunId` rejection fires first; `SequenceOverflow`
  remains reachable only for `RunId != 0` with `seq.get() ==
  u64::MAX`.
- Returning `Err(JournalError::KeyCapacity)` for `RunId(0)` inputs.
  The `ArrayVec` capacity check is unreachable when the guard
  fires first.
- Mapping `JournalError::InvalidRunId { .. }` to a different
  diagnostic code than `0x4021`. The mapping at
  `error/codes.rs:168` (`Self::InvalidRunId { .. } =>
  Self::INVALID_RUN_ID_CODE`) must remain unchanged.
- Mapping `JournalError::InvalidRunId { .. }` to a different
  symbolic code than `INVALID_RUN_ID`. The mapping at
  `error/codes.rs:250` must remain unchanged.

## Forbidden Implementation Patterns

- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or
  `dbg!` in the encoder paths (Holzman Rust rule).
- No unchecked indexing or arithmetic in the encoder paths.
- No `unsafe` in the encoder paths.
- No YAML/JSON/HTTP at the encoder boundary.

## Diagnostic Surface

Production callers that observe `JournalError::InvalidRunId { run }`
from a key encoder can:

1. Pattern-match on the variant and recover `run: RunId`.
2. Call `.diagnostic_code()` to obtain
   `DiagnosticCode(0x4021)`.
3. Call `.symbolic_code()` (via the `HasSymbolicCode` impl at
   `error/codes.rs:271-285`) to obtain
   `SymbolicCode::INVALID_RUN_ID`.
4. Use the existing decoder-test templates at
   `crates/workspace_tests/tests/restate_doctor_key_decode_tests.rs:353-396`
   as documentation references for the contract.

The symbolic-code table at `crates/vb_storage/src/tests.rs:7670-7680`
already maps the variant; no change required for it.

## Reused / Already-Correct Surfaces (no change)

| Surface | Source | Status |
|---|---|---|
| `JournalError::InvalidRunId { run: RunId }` | `crates/vb_storage/src/error/mod.rs:140-141` | Already exists; reuse. |
| `INVALID_RUN_ID_CODE = 0x4021` | `crates/vb_storage/src/error/codes.rs:73` | Already exists. |
| `Self::InvalidRunId { .. } => Self::INVALID_RUN_ID_CODE` | `crates/vb_storage/src/error/codes.rs:168` | Already exists. |
| `Self::InvalidRunId { .. } => "INVALID_RUN_ID"` | `crates/vb_storage/src/error/codes.rs:250` | Already exists. |
| `KeyDecodeError::InvalidRunId` | `crates/vb_storage/src/error/key_decode.rs:28` | Decoder side; no change. |
| `tests.rs::symbolic_code_table` invalid_run_id entry | `crates/vb_storage/src/tests.rs:7670-7680` | Already maps the variant. |
| `proptests.rs::all_key_functions_are_deterministic` | `crates/vb_storage/src/proptests.rs:170` | `run_val in 1..=1000` excludes zero; no change. |