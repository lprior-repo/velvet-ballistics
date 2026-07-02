# Workflow Model: vb-cn2v4 — Keys reject zero RunId (P1 bug)

## Encoder Rejection State Machine

```text
EncoderEntry(prefix, payload)
  ├─ run_field absent (digest variants: WorkflowSource, CompiledIr, Blob)
  │   └─ validate_other_fields(prefix-specific) ?
  │       └─ emit bytes -> Ok
  │
  └─ run_field present (RunHeader, RunEvent, RunSnapshot,
                       IndexStatus, IndexWorkflow, IndexAction)
      ├─ require_non_zero_run(run)            <-- NEW (this bead)
      │   ├─ run.get() == 0
      │   │   └─ Err(InvalidRunId { run })
      │   │      (no byte emission)
      │   │      (no allocation)
      │   │      (no I/O)
      │   └─ run.get() != 0
      │       └─ validate_other_fields(prefix-specific) ?
      │           ├─ seq == u64::MAX          (RunEvent/RunSnapshot)
      │           │   └─ Err(SequenceOverflow)
      │           ├─ state == Other(v < 3)   (IndexStatus)
      │           │   └─ Err(IndexStatusStateCollision)
      │           └─ all OK
      │               └─ emit bytes -> Ok
```

## Encoder Decision Workflow (per call)

1. Caller invokes the typed encoder with arguments.
2. Encoder runs `require_non_zero_run(run)` first (NEW).
3. Encoder runs prefix-specific validation (e.g.
   `IndexStatusState::to_u8_checked`, `seq != MAX`).
4. Encoder emits bytes only if every check passed.
5. The `?` propagation through `encode_key_into` / `encode_key` /
   `journal_key` / `run_prefix_key` callers inherits the rejection.

## Error Surface Shift at Journal Append Call Sites

After this bead, the **first** typed error for `RunId(0)` events at
the journal append entry points changes from
`JournalError::InvalidEvent` (returned by `JournalEvent::is_valid()`)
to `JournalError::InvalidRunId { run }` (returned by the encoder
guard that runs before `is_valid()`).

The following call sites are affected (their `RunId(0)` error
semantics shift):

| Call site | File:line | Encoder used |
|---|---|---|
| `append_strict` | `crates/vb_storage/src/journal/append.rs:47` | `run_event_key` |
| `append_unfsynced` | `crates/vb_storage/src/journal/internal.rs:55` | `run_event_key` |
| `inject_raw_event` | `crates/vb_storage/src/journal/injection.rs:22` | `run_event_key` |
| `inject_seq_gap` | `crates/vb_storage/src/journal/injection.rs:42` | `run_event_key` |
| `stage_queued_event` | `crates/vb_storage/src/queue/writer/stage.rs:37` | `run_event_key` |
| `append_event` (G1 guard) | `crates/vb_storage/src/batch/append_event.rs:43` | `run_event_key` |
| `stage_pending_action_index_op` (Insert) | `crates/vb_storage/src/batch/action_index.rs:116` | `index_action_key` |
| `stage_pending_action_index_op` (Remove) | `crates/vb_storage/src/batch/action_index.rs:121` | `index_action_key` |
| `put_snapshot` | `crates/vb_storage/src/snapshots.rs:32` | `run_snapshot_key` |
| `snapshot` reader | `crates/vb_storage/src/snapshots.rs:53` | `run_snapshot_key` |
| `put_status_index` | `crates/vb_storage/src/indexes.rs:21` | `index_status_key` |
| `put_workflow_index` | `crates/vb_storage/src/indexes.rs:32` | `index_workflow_key` |
| `put_action_index` | `crates/vb_storage/src/indexes.rs:44` | `index_action_key` |
| `delete_action_index` | `crates/vb_storage/src/indexes.rs:62` | `index_action_key` |
| `put_run_header` | `crates/vb_storage/src/headers.rs:19` | `run_header_key` |
| `run_header` reader | `crates/vb_storage/src/headers.rs:36-39` | `run_header_key` (manual check first) |
| `trim_events_for_run` | `crates/vb_storage/src/trimming/logic.rs:60` | `run_prefix_key` |
| `trim_run_event_log` | `crates/vb_storage/src/trimming/logic.rs:213` | `run_prefix_key` |
| `trim_eligibility_diagnostic` | `crates/vb_storage/src/trimming/logic.rs:246` | `run_prefix_key` |

For `RunId(0)` inputs, these sites now return
`Err(JournalError::InvalidRunId { run })` instead of
`Err(JournalError::InvalidEvent)` (or silently emitting bytes in the
uncovered cases). Behaviour matches the existing
`run_header` manual check (which already returns `InvalidRunId`).

## Decoder Workflow (unchanged, source of truth)

```text
decode_storage_key(bytes)
  ├─ try_key_prefix(bytes)
  ├─ length check
  └─ per-variant:
      ├─ run-bearing variants
      │   └─ run_val == 0 -> Err(KeyDecodeError::InvalidRunId)
      └─ non-run-bearing variants
          └─ decode digest fields
```

The decoder-side `InvalidRunId` at
`keys.rs:372-374, 381-383, 400-402, 412-414, 423-425` is the
invariant the encoder must mirror.

## Test Workflow Flips

The following 18 tests must flip from `Ok(...)` expectations to
`Err(JournalError::InvalidRunId { run: RunId::new(0) })` expectations:

### `crates/vb_storage/src/keys/tests.rs` (11 tests)

1. `run_header_key_has_correct_prefix` (line 72-78)
2. `run_event_key_length` (line 123-128)
3. `index_status_key_has_correct_prefix` (line 190-195)
4. `index_status_key_length` (line 214-219)
5. `index_workflow_key_length` (line 246-251)
6. `index_action_key_length` (line 284-289)
7. `run_header_key_with_zero_run_id` (line 468-474) — **explicit
   zero-run-id test; now asserts rejection.**
8. `index_status_key_with_zero_values` (line 507-514) — same.
9. `run_prefix_key_is_9_bytes` (line 587-592)
10. `index_status_key_rejects_other_state_in_collision_range`
    (line 678-708) — replace `RunId::new(0)` with `RunId::new(1)` so
    the `IndexStatusStateCollision` path stays exercised.
11. `index_status_key_accepts_other_state_above_collision_range`
    (line 710-717) — same `RunId::new(0)` -> `RunId::new(1)` swap
    to keep the `Other(v)` non-collision path exercised.

### `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs` (3 tests)

12. `encode_exact_length_run_header` (line 340-348)
13. `encode_exact_length_run_event` (line 350-358)
14. `encode_exact_length_index_action` (line 366-374)

### `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs` (4 tests)

15. `run_header_key_prefix_is_0x10` (line 91-107) — Given "RunId with value 0" arm.
16. `run_header_key_zero_run_id` (line 109-125) — title already
    promises zero-run-id rejection; assert the rejection.
17. `index_workflow_key_zero_values` (line 205-228) — flip.
18. `run_id_zero_roundtrip` (line 699-711) — flip; the title
    describes the OLD asymmetric contract.

## Kani Harness Workflow (split-harness approach)

`crates/vb_storage/src/kani_typed_partitioned_ids.rs`
must be reorganised so that the harness body for the four
typed-partitioned-id encoders distinguishes the zero and non-zero
cases. The recommended shape (chosen for strongest coverage; cheaper
options permitted by proof-planner):

```rust
fn assert_key_contracts(inputs: SymbolicKeyInputs) {
    let run_value = run_raw(inputs);
    let seq_value = seq_raw(inputs);
    // ... build typed ids ...

    if run_value == 0 {
        // Rejection path: every encoder must surface InvalidRunId.
        assert!(matches!(keys::run_header_key(run),
            Err(JournalError::InvalidRunId { .. })));
        assert!(matches!(keys::run_event_key(run, seq),
            Err(JournalError::InvalidRunId { .. })));
        assert!(matches!(keys::index_workflow_key(workflow, run),
            Err(JournalError::InvalidRunId { .. })));
        assert!(matches!(keys::index_action_key(action, run, step),
            Err(JournalError::InvalidRunId { .. })));
    } else {
        // Happy path (unchanged layout assertions).
        match keys::run_header_key(run) { Ok(key) => { /* existing layout */ }, Err(_) => assert!(false) }
        // ...
    }
}
```

Alternative accepted shapes (proof-planner lane decision):

- **(a)** `kani::assume(run_value != 0)` at the top of
  `assert_key_contracts` to scope the harness to the non-zero
  domain (cheapest).
- **(b)** Split into two `#[kani::proof]` entry points:
  `vb_eepg_typed_partitioned_ids_happy` (non-zero) and
  `vb_eepg_typed_partitioned_ids_zero` (rejection).
- **(c)** The in-place if/else split above.

The template at `kani_vb_vzcuf_ps004.rs:151` (already pattern-matches
`JournalError::InvalidRunId { .. }`) shows the existing
`Err(InvalidRunId)` pattern that the new zero-arm should match.

## Verus Mirror Workflow (extern_vb_storage_keys.rs)

```text
SpecKeyEncodeError variants (current):
  IndexStatusStateCollision, SequenceOverflow, KeyCapacity

SpecKeyEncodeError variants (after this bead):
  IndexStatusStateCollision, SequenceOverflow, KeyCapacity,
  InvalidRunId { run: u64 }                              <-- NEW

Per-encoder assume_specification clauses to add:
  for every encoder mirror that takes a `run` argument:
    requires run != 0;
    ensures  result is Err(SpecKeyEncodeError::InvalidRunId { run })
             iff run == 0;
```

The mirror fns that bind to `require_non_zero_run` and need the
clause are listed in `type-contracts.md` § Verus Mirror Type Contract.

## Temporal Invariants

- The encoder rejection is order-stable: `require_non_zero_run`
  fires before any other encoder-internal check, so test and
  production order is consistent.
- The decoder rejection is order-stable: `run_val == 0` is checked
  before `seq_val == MAX` for run-event/run-snapshot keys
  (`keys.rs:381-387`); the encoder mirrors this by checking run
  before seq inside `sequenced_run_key`.
- After this bead, the encoder and decoder are symmetric: a `(prefix,
  run, ...)` tuple is accepted by both sides iff `run != 0` (and
  other prefix-specific preconditions hold). Existing persisted
  rows are unaffected (decoder already rejected them; none exist).