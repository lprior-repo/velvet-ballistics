# Bead vb-cn2v4 — Codebase Map (State 2 Scout)

- bead_id: vb-cn2v4
- title: Keys: reject zero RunId in all key encoders (P1 bug)
- workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4
- jj workspace root: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4
- captured_at: 2026-07-01 (State 2 scout)

## Problem statement

The key encoders in `crates/vb_storage/src/keys.rs` (run header, run
event, run snapshot, index status, index workflow, index action) accept
`RunId(0)` without validation and emit byte keys whose `run_id` field is
all-zero. The DECODER side already rejects `RunId(0)` via
`KeyDecodeError::InvalidRunId` at `keys.rs:372-374, 381-383, 400-402,
412-414, 423-425`. The encoder/decoder pair is therefore asymmetric: a
caller can encode a key that the same module then refuses to decode.
The asymmetric contract is the P1 bug. Header code at
`crates/vb_storage/src/headers.rs:36-39` already manually rejects
`RunId(0)` for `FjallJournal::run_header`; the fix should generalize
that guard into the typed encoders themselves.

## Scope (crates / files / APIs)

### Production code — REQUIRES edit (encoder fix surface)

Primary file (all encoder entry points):

- `crates/vb_storage/src/keys.rs`
  - `pub fn run_header_key(run: RunId) -> Result<..., JournalError>` — line 76-78 (calls `run_only_key(PREFIX_RUN_HEADER, run)` at line 514-521)
  - `pub fn run_event_key(run: RunId, seq: EventSeq) -> Result<..., JournalError>` — line 81-83 (calls `journal_key` → `sequenced_run_key(PREFIX_RUN_EVENT, ...)` at line 476-496)
  - `pub fn run_snapshot_key(run: RunId, seq: EventSeq) -> Result<..., JournalError>` — line 86-91 (calls `sequenced_run_key(PREFIX_RUN_SNAPSHOT, ...)`)
  - `pub fn index_status_key(state, timestamp, run: RunId) -> Result<..., JournalError>` — line 101-122 (uses `run.get().to_be_bytes()` directly at line 119)
  - `pub fn index_workflow_key(workflow, run: RunId) -> Result<..., JournalError>` — line 125-137 (uses `run.get().to_be_bytes()` at line 134)
  - `pub fn index_action_key(action, run: RunId, step) -> Result<..., JournalError>` — line 140-155 (uses `run.get().to_be_bytes()` at line 150)
  - `pub fn encode_key_into(key: &StorageKey, out: &mut Vec<u8>) -> Result<(), JournalError>` — line 162-198 (dispatches to all six encoders above; will inherit rejection)
  - `pub fn encode_key(key: StorageKey) -> Result<Vec<u8>, JournalError>` — line 205-209 (wraps `encode_key_into`)
  - `pub fn journal_key(run, seq)` — line 476-478 (thin alias; same body)
  - `fn sequenced_run_key(prefix, run, seq)` — line 480-496 (private; primary encoder for sequenced runs)
  - `fn run_prefix(run)` — line 498-500 (private)
  - `fn run_only_key(prefix, run)` — line 514-521 (private; primary encoder for run-only keys)
  - `pub(crate) fn run_prefix_key(run: RunId) -> Result<[u8; 9], JournalError>` — line 524-526 (re-export used by FjallJournal trimming)

Reuse the existing typed error:

- `crates/vb_storage/src/error/mod.rs:140-141`
  - `JournalError::InvalidRunId { run: RunId }` already exists with
    diagnostic code `INVALID_RUN_ID_CODE` (0x4021) and symbolic code
    `INVALID_RUN_ID` (see `crates/vb_storage/src/error/codes.rs:73, 168,
    250`).
  - Existing call site that already emits this error: `crates/vb_storage/src/headers.rs:38` (in `FjallJournal::run_header`).

Suggested implementation pattern (single guard shared by all
encoders; suggested shape — NOT IMPLEMENTED HERE):

```rust
fn require_non_zero_run(run: RunId) -> Result<(), JournalError> {
    if run.get() == 0 {
        return Err(JournalError::InvalidRunId { run });
    }
    Ok(())
}
```

…called at the top of `run_only_key`, `sequenced_run_key`,
`index_status_key`, `index_workflow_key`, `index_action_key`. Because
`run_header_key`, `run_event_key`, `run_snapshot_key`, `journal_key`,
`encode_key`, `encode_key_into`, `run_prefix` all delegate to those
private functions, the guard propagates to every public entry point
automatically. The `index_*` encoders currently bypass `run_only_key`
and `sequenced_run_key`, so they need an explicit call to
`require_non_zero_run`.

### Production code — REQUIRES edit (existing manual guard becomes redundant)

- `crates/vb_storage/src/headers.rs:36-39`
  - `FjallJournal::run_header` already checks `if run.get() == 0 { return Err(JournalError::InvalidRunId { run }); }` before calling `run_header_key(run)`.
  - After the patch, this manual check becomes redundant: `run_header_key(RunId(0))` itself will return `Err`. The manual check can stay (defence-in-depth) or be removed; the `run_header_key` doc comment at line 34-35 already promises the same contract. Decide in contract plan whether to drop or keep.

### Production code — REQUIRES edit (call sites where error semantics shift)

These call sites invoke key encoders BEFORE event-level semantic checks.
After the patch, a `RunId(0)` event will surface `JournalError::InvalidRunId { run }`
from the encoder rather than `JournalError::InvalidEvent` from
`JournalEvent::is_valid()`:

- `crates/vb_storage/src/journal/append.rs:47` — `append_strict` calls `run_event_key(event.run_id(), event.seq())?` then `event.is_valid()`.
- `crates/vb_storage/src/journal/internal.rs:55` — `append_unfsynced` same pattern.
- `crates/vb_storage/src/journal/injection.rs:22, 42` — `inject_raw_event` and `inject_seq_gap` same pattern.
- `crates/vb_storage/src/queue/writer/stage.rs:37` — `stage_queued_event` same pattern.
- `crates/vb_storage/src/batch/append_event.rs:43` — `append_event` same pattern (G1 guard per C6).
- `crates/vb_storage/src/batch/action_index.rs:116, 121` — `stage_pending_action_index_op` calls `index_action_key(action, run, step)?` for Insert and Remove mutations.
- `crates/vb_storage/src/snapshots.rs:32` — `put_snapshot` calls `run_snapshot_key(snapshot.run, snapshot.seq)?`.
- `crates/vb_storage/src/snapshots.rs:53` — `snapshot` reader calls `run_snapshot_key(run, seq)?`.
- `crates/vb_storage/src/indexes.rs:21, 32, 44, 62` — `put_status_index`, `put_workflow_index`, `put_action_index`, `delete_action_index`.
- `crates/vb_storage/src/headers.rs:19` — `put_run_header` calls `run_header_key(record.run)?` (will inherit new rejection).
- `crates/vb_storage/src/headers.rs:40` — `run_header` reader (manual check stays as-is or is removed).
- `crates/vb_storage/src/trimming/logic.rs:60, 213, 246` — `trim_events_for_run`, `trim_run_event_log`, `trim_eligibility_diagnostic` all call `crate::keys::run_prefix_key(run)?`.

Behaviour change at these sites: a `RunId(0)` event now produces
`Err(JournalError::InvalidRunId { run })` at the encoder call, BEFORE
`is_valid()` is consulted. This is desirable (specific, typed) but the
test suite below encodes the OLD order in some places; the test surface
must be flipped in lockstep with the production change.

### Tests — REQUIRES edit (currently expect Ok; must be flipped to expect Err)

Production tests under `crates/vb_storage/src/keys/tests.rs` (included via `#[path = "keys/tests.rs"]` at `keys.rs:529`):

- line 72-78 `run_header_key_has_correct_prefix` — uses `RunId::new(0)` then `run_header_key(run)?`. Flip to assert `Err(JournalError::InvalidRunId { run: RunId::new(0) })`.
- line 123-128 `run_event_key_length` — uses `RunId::new(0)`. Flip.
- line 190-195 `index_status_key_has_correct_prefix` — uses `RunId::new(0)`. Flip.
- line 214-219 `index_status_key_length` — uses `RunId::new(0)`. Flip.
- line 246-251 `index_workflow_key_length` — uses `RunId::new(0)`. Flip.
- line 284-289 `index_action_key_length` — uses `RunId::new(0)`. Flip.
- line 468-474 `run_header_key_with_zero_run_id` — explicitly tests `RunId::new(0)` encoding to bytes. Flip to assert rejection.
- line 507-514 `index_status_key_with_zero_values` — explicitly tests `RunId::new(0)` encoding. Flip to assert rejection.
- line 587-592 `run_prefix_key_is_9_bytes` — uses `RunId::new(0)`. Flip.
- line 678-708 `index_status_key_rejects_other_state_in_collision_range` — uses `RunId::new(0)` as the run id; this test exercises the `Other(v)` collision path. After the patch, the new `InvalidRunId` rejection will fire first (before the `IndexStatusStateCollision` check). The test's three assertions check `IndexStatusStateCollision`; they must be reworked so that `RunId::new(0)` is replaced with `RunId::new(1)` (or any non-zero id) to keep exercising the collision logic.
- line 710-717 `index_status_key_accepts_other_state_above_collision_range` — same `RunId::new(0)` issue; replace with non-zero run id.

Workspace tests under `crates/workspace_tests/tests/`:

- `fjall_keyspace_manifest_tests.rs:340-348` `encode_exact_length_run_header` — uses `run_header_key(RunId::new(0)).unwrap()`. Flip.
- `fjall_keyspace_manifest_tests.rs:350-358` `encode_exact_length_run_event` — same pattern. Flip.
- `fjall_keyspace_manifest_tests.rs:366-374` `encode_exact_length_index_action` — same. Flip.
- `vb_eepg_bdd_tests.rs:91-107` `run_header_key_prefix_is_0x10` — Given "RunId with value 0", calls `keys::run_header_key(run_id)?`. Flip to assert Err.
- `vb_eepg_bdd_tests.rs:109-125` `run_header_key_zero_run_id` — title says "zero run id" but test asserts success. Flip to assert rejection.
- `vb_eepg_bdd_tests.rs:205-228` `index_workflow_key_zero_values` — same pattern. Flip.
- `vb_eepg_bdd_tests.rs:699-711` `run_id_zero_roundtrip` — asserts "zero RunId must roundtrip correctly"; this is the asymmetry evidence. Flip to assert rejection.

Existing correct contract tests (good templates, should remain):

- `crates/workspace_tests/tests/restate_doctor_key_decode_tests.rs:353-358` `decode_storage_key_returns_invalid_run_id_for_zero_run_header` — decoder side already enforces; OK.
- `crates/workspace_tests/tests/restate_doctor_key_decode_tests.rs:361-367` `decode_storage_key_returns_invalid_run_id_for_zero_run_event` — OK.
- `crates/workspace_tests/tests/restate_doctor_key_decode_tests.rs:369-380` `decode_storage_key_returns_invalid_run_id_for_zero_index_status_run` — OK.
- `crates/workspace_tests/tests/storage_contract_pack_runner.rs:151-168` `invalid_key_prefix_returns_typed_error` — title is misleading (it's about `run_id=0` not key prefix). The body uses `journal.run_header(RunId::new(0))` which is the manual check in `headers.rs:36-39`. After the patch this test still passes via either path (encoder rejection or manual check). Consider renaming the test for clarity.
- `crates/vb_storage/src/tests.rs:7670-7680` — symbolic-code table maps `JournalError::InvalidRunId { .. } => "invalid_run_id"`. Already in place; OK.

### Verification harnesses — REQUIRES edit

Kani (production crate):

- `crates/vb_storage/src/kani_typed_partitioned_ids.rs:43-93` `assert_key_contracts`
  - Lines 56-62: `match keys::run_header_key(run) { Ok(key) => ..., Err(_) => assert!(false) }`. The `Err(_) => assert!(false)` arm will fire whenever Kani samples `run_value = 0`. The harness must be patched to: (a) `kani::assume(run_value != 0)` to keep the property scoped to non-zero inputs, OR (b) explicitly handle the `Err(InvalidRunId)` arm and assert it when `run_value == 0`, OR (c) split into two harnesses: one for the non-zero happy path, one for the zero rejection path. (c) is the strongest option; (a) is the cheapest.
  - Lines 63-70: `run_event_key(run, seq)` — same problem.
  - Lines 71-78: `index_workflow_key(workflow, run)` — same problem.
  - Lines 79-87: `index_action_key(action, run, step)` — same problem.
- `crates/vb_storage/src/kani_typed_partitioned_ids.rs:111-115` — `vb_eepg_typed_partitioned_ids` is the proof entry; covers the four key encoders above.

Existing similar Kani pattern (good template for split-harness approach):

- `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:151` — already pattern-matches `JournalError::InvalidRunId { .. }`.

Verus (verification/verus/):

- `verification/verus/extern_vb_storage_keys.rs:199-204`
  - `SpecKeyEncodeError` enum currently has three variants: `IndexStatusStateCollision`, `SequenceOverflow`, `KeyCapacity`. After the patch, the production encoder may return `JournalError::InvalidRunId { run }` from every encoder path. The Verus mirror must add a `SpecKeyEncodeError::InvalidRunId { run: u64 }` variant.
- `verification/verus/extern_vb_storage_keys.rs:303-307`
  - `pub fn run_event_key(run: u64, seq: u64)` mirror. Body is `#[verifier::external]`. The mirror currently returns `Ok` for any `run != MAX` (it doesn't reject 0). After the patch, the assume_specification contract for this function (see `vb_storage_keys_spec.rs` — MISSING as a standalone file, contract is inlined in extern_vb_storage_keys.rs) must include a clause stating `run == 0` returns `Err(SpecKeyEncodeError::InvalidRunId { run })`.
- `verification/verus/extern_vb_storage_keys.rs:320-344`
  - `encode_key` mirror — same shape; all six `Ok(_)` arms for variants that contain a `run` field must be amended so the type permits `Err(InvalidRunId)` on `run == 0`.

RunId newtype (production):

- `crates/vb_core/src/ids/mod.rs:24-55` — `numeric_id!` macro used for `RunId`. `pub const fn new(value: u64) -> Self` at line 36 accepts `0` (constructor is invariant, no validation).
- `crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:156-171` — `impl RunId { pub const ZERO: Self = Self(0); ... }`. The `ZERO` constant is used as a sentinel/placeholder in `recovery::NoRecoveryData { run: RunId::new(0) }` (see `crates/vb_storage/src/recovery/replay/summary/derive.rs:148`, `apply.rs:90`, and tests at `recovery/replay/summary/tests.rs:290, 300`). These callers use ZERO as a "no result" marker for the recovery diagnostic, NOT as a valid key. They are out of scope for this bead and must remain functional — the encoder fix only touches key construction paths.

## Out-of-scope (do NOT touch)

- `crates/vb_core/src/ids/mod.rs` and `parts/chunk_001_custom_types.rs` — `RunId::new` stays as-is. Making `new` validate would force a constructor ripple through every callsite; the bead text asks for the rejection to live in the encoder, not in the type constructor.
- Recovery diagnostics that use `RunId::new(0)` as a placeholder (`crates/vb_storage/src/recovery/replay/summary/{derive,apply}.rs` and the `NoRecoveryData` tests) — these are domain-level "absent" markers, not key-encoding paths.
- Proptests at `crates/vb_storage/src/proptests.rs:170` — `run_val in 1u64..=1000u64` already excludes 0. No change needed.
- `JournalError::InvalidRunId { run }` definition (`error/mod.rs:140-141`) — already exists with code 0x4021 and symbolic name INVALID_RUN_ID; reuse, do not add a new variant.
- `KeyDecodeError::InvalidRunId` (`error/key_decode.rs:28`) — decoder side is correct; do not touch.

## Risk tags

- **persistence**: Yes. Changes how keys are constructed for every keyspace (run_header, events, snapshot, all three indices). Already-decodeable existing rows are unaffected (decoder was already strict); only the encoder now refuses to produce keys that the decoder would refuse.
- **parser/codec**: Yes. Key encoder/decoder contract tightens.
- **public API**: Yes. `run_header_key`, `run_event_key`, `run_snapshot_key`, `index_status_key`, `index_workflow_key`, `index_action_key`, `encode_key`, `encode_key_into`, `journal_key`, `run_prefix_key` all change Result semantics for `RunId(0)`. Downstream crate callers (only `vb_storage` itself, per current evidence) are listed above.
- **concurrency**: No. Pure synchronous encoding.
- **temporal**: No.
- **unsafe/UB**: No.
- **auth/security**: No.
- **dependency**: No.
- **performance**: No. Single `== 0` integer compare per encoder call.
- **migration**: No. Existing rows (decoder side) unchanged.
- **user-visible behavior**: Yes. CLI consumers calling these encoders with `RunId(0)` now get a typed error instead of silently-emitted bytes.

## Open questions / unknowns

- Whether to keep the manual `if run.get() == 0` check in `headers.rs:36-39` (defence-in-depth) or remove it (now redundant). Recommend KEEP for now: minimal-blast-radius change, the manual check costs nothing, and removing it could subtly change the error-context surface for `run_header`.
- Whether to add a `RunId::try_new(value: u64) -> Result<Self, JournalError>` to the type itself for callers that want to validate at construction. Out of scope for this bead (bead text says "in all key encoding paths"), but flag as a future cleanup opportunity.
- Whether to extend the rejection to `vb_runtime` / `vb_core` callers that build `RunId(0)` and then immediately throw it away (e.g. `crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs:1209, 1259`). Out of scope; those callers never reach a key encoder.

## Recommended downstream owners

- **rust-contract**: Write the domain/type contract for the new `require_non_zero_run` guard, the encoder/decoder symmetry invariant, and the error-code mapping (`INVALID_RUN_ID_CODE`).
- **proof-planner**: Plan Verus updates to `extern_vb_storage_keys.rs::SpecKeyEncodeError` (add `InvalidRunId { run: u64 }`), and Kani harness updates to `kani_typed_partitioned_ids.rs` (split or assume-guard).
- **proof-writer**: Implement the Verus spec additions and Kani harness edits.
- **test-planner**: Plan new BDD tests for "encoder rejects zero RunId" per prefix, plus a parity test that every prefix variant (RunHeader, RunEvent, RunSnapshot, IndexStatus, IndexWorkflow, IndexAction) emits `Err(InvalidRunId)` for `RunId(0)`. Plan mutation-test to disable the new guard and assert that the existing proptests catch it.
- **holzman-rust**: Edit `crates/vb_storage/src/keys.rs` to add the shared guard, edit the listed tests to flip `Ok` expectations to `Err(InvalidRunId)`, edit the listed Kani harness, edit the listed Verus spec mirror, and edit `headers.rs` only if the manual check is to be removed.

## Verification artefacts (existing)

- `crates/vb_storage/src/error/codes.rs:73` — `INVALID_RUN_ID_CODE = 0x4021` constant already present.
- `crates/vb_storage/src/error/codes.rs:168` — `Self::InvalidRunId { .. } => Self::INVALID_RUN_ID_CODE` mapping already present.
- `crates/vb_storage/src/error/codes.rs:250` — `Self::InvalidRunId { .. } => "INVALID_RUN_ID"` symbolic mapping already present.
- `crates/vb_storage/src/proptest-regressions/` — directory exists; check for any pre-existing failure files relevant to keys (empty file list visible; no regressions pre-staged for this bead).
- `crates/workspace_tests/tests/restate_doctor_key_decode_tests.rs:353-396` — three decoder tests already pin the `InvalidRunId` invariant; the encoder is the asymmetric gap that this bead closes.

## Source evidence trail (commands executed)

```text
pwd -P                                     -> /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4
jj root                                    -> /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4
ls crates/vb_storage/src/                  -> confirms keys.rs, headers.rs, snapshots.rs, indexes.rs,
                                              journal/{append,internal,injection,parse,replay,readonly,source}.rs,
                                              batch/{append_event,action_index}.rs,
                                              queue/writer/stage.rs, trimming/logic.rs,
                                              error/{mod,key_decode,codes}.rs,
                                              keys/tests.rs,
                                              kani_typed_partitioned_ids.rs,
                                              kani_vb_vzcuf_ps004.rs
ls verification/verus/                     -> confirms extern_vb_storage_keys.rs (mirror + spec),
                                              extern_vb_vzcuf_PS_001.rs, vb-vzcuf-PS-008.rs
ls contracts/verus/                        -> confirms vb_qi37_16_5_lifecycle_journal_storage.rs
                                              (TLA+ mirror, uses RunId::Run(0) as spec placeholder)
rg -n "run_header_key|run_event_key|run_snapshot_key|index_workflow_key|
      index_action_key|index_status_key|journal_key|run_prefix_key|encode_key|
      encode_key_into" crates/vb_storage/src/   -> 32 hits, all call sites enumerated above
rg -n "InvalidRunId" crates/                  -> 10 hits (5 in decoder, 1 in headers.rs manual check,
                                                3 in error def + code + symbolic, 1 in proptest)
rg -n "RunId::new\(0\)" crates/vb_storage/src/  -> 22 hits, all categorised above (encoder tests vs
                                                   recovery placeholders)
```
