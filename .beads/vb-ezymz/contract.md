# Rust Contract: CLI Submit Input Validation and Persistence (vb-ezymz)

**Bead:** vb-ezymz
**Product:** velvet-ballistics
**Language version:** velvet-ballistics/v1
**Type:** Bug fix — admission and persistence defect

---

## Ubiquitous Language

- **submit:** CLI command that compiles a YAML workflow, validates and maps runtime inputs against declared input slots, persists run metadata and validated inputs, and returns a `run_id`.
- **input slot:** A declared entry point on a compiled workflow (`CompiledInputSlot`) that pairs a `SlotIdx` with an `InputSlotKind`.
- **input mapping:** A collection of `(SlotIdx, SlotValue)` pairs produced by decoding and validating raw postcard bytes against a workflow's declared input slots.
- **malformed input:** Raw bytes that fail postcard decoding into `Box<[SlotValue]>`.
- **durable acceptance:** The point at which run metadata and validated inputs are persisted to the storage layer such that process restart can recover them.
- **recovery hydration:** The process of reconstructing a live `RunFrame` from persistent storage records, including slot values seeded by input mapping.

---

## Entities and Value Objects

### Entities

| Name | Type | Lifecycle |
|------|------|-----------|
| Run | `RunId` (u64) | Created on submit, terminal on finish/fail/cancel |

### Value Objects

| Name | Fields | Invariants |
|------|--------|------------|
| `CompiledInputSlot` | `slot: SlotIdx`, `kind: InputSlotKind` | `slot` in range `[0, slot_count)`, `kind` in `InputSlotKind` enum |
| `InputMapping` | `Box<[(SlotIdx, SlotValue)]>` | Each `SlotIdx` unique, each in declared input slots, value kind matches declared kind |
| `InputMappingError` | Tagged error variants | Discriminates malformed, kind-mismatch, duplicate, out-of-range, count-exceeded |

### Commands

| Name | Input | Output |
|------|-------|--------|
| `cmd_submit(workflow_path, input_bin_path, db_path, durability, output)` | Paths, durability mode, output format | `ExitCode`, side effects: storage writes, JSON or text response |
| `map_runtime_inputs(compiled, input_bytes)` | `CompiledWorkflow`, raw bytes | `Result<Box<[(SlotIdx, SlotValue)]>, InputMappingError>` |

### Policies

- Input bytes must be decoded and validated against declared input slots **before** any durable write of run header or `RunAccepted` journal event.
- If validation fails, no partial durable state is left.
- If the workflow declares no input slots, non-empty input bytes are considered a **SlotCountExceeded** error (user provided inputs where none are declared).
- Empty input bytes on a workflow with no input slots is valid and produces an empty `InputMapping`.
- Empty input bytes on a workflow with declared input slots is valid and produces an empty `InputMapping` (user omitted optional inputs).

---

## State Transitions

```
User provides workflow + input bytes
  → compile YAML to CompiledWorkflow (existing)
  → validate/map raw input bytes against compiled.input_slots() (NEW)
  → on mapping error: report error, exit, no durable writes
  → on mapping success: persist workflow source (existing)
  → persist validated InputMapping (NEW)
  → persist RunHeaderRecord (existing)
  → persist RunAccepted journal event (existing)
  → return run_id to user
```

---

## Durable Acceptance Atomicity

Durable acceptance requires the following to be persisted before returning `run_id`:

1. **Workflow source** (`put_workflow_source`) — already done
2. **Run header** (`put_run_header`) — already done
3. **Validated InputMapping** (NEW: `put_run_input` or equivalent)
4. **RunAccepted journal event** (`append_strict_batch`) — already done

Under `DurabilityMode::Strict`, items 1-4 are synced. Under `DurabilityMode::Journaled`, items are queued. Under `DurabilityMode::None`, no persistence occurs.

**No compiled IR is stored for submit:** submit is an admission command, not execution. The workflow is compiled in memory, its source is stored, and its digest is bound to the run header. Runtime execution (if initiated separately) will re-compile from source or accept the artifact.

---

## Process Reopen Behavior

When the velvet-ballistics process restarts and needs to recover a submitted run:

1. Recovery reads the `RunHeaderRecord` to get `run_id` and `compiled_digest`.
2. Recovery loads the compiled workflow from the artifact store by digest (or re-compiles from source).
3. Recovery reads the persisted `InputMapping` for the run.
4. Recovery hydrates a `RunFrame` using the `InputMapping` to seed input slots via `seed_input_slots`.
5. Recovery continues execution from the recovered frame state.

The persisted `InputMapping` is essential: without it, recovery cannot reconstruct the user's original input values, and the run cannot be resumed from a pre-execution state.

---

## Bounded Implementation Changes

### Files Modified

| File | Change |
|------|--------|
| `crates/velvet_ballistics/src/submit.rs` | Call `map_runtime_inputs` after compile; on success, pass mapping to persistence layer; on error, exit before durable writes |
| `crates/velvet_ballistics/src/app_impl.rs` | Export `InputMappingError` variants for CLI error handling |
| `crates/velvet_ballistics/src/run_compiled.rs` | Fix `map_runtime_inputs` to error on non-empty input bytes when `input_slots` is empty |

### Files Added

| File | Purpose |
|------|---------|
| `crates/vb_storage/src/inputs.rs` (new) | `RunInputRecord` type and `put_run_input`/`get_run_input`/`remove_run_input` operations on `FjallJournal` |
| `crates/vb_storage/src/records.rs` (add) | `RunInputRecord` struct: `{ run: RunId, inputs: Box<[(SlotIdx, SlotValue)]> }` |

### Functions Added

| Module | Function | Signature |
|--------|----------|-----------|
| `vb_storage::inputs` | `put_run_input` | `fn put_run_input(&self, record: &RunInputRecord) -> Result<(), JournalError>` |
| `vb_storage::inputs` | `get_run_input` | `fn get_run_input(&self, run: RunId) -> Result<Option<RunInputRecord>, JournalError>` |
| `vb_storage::inputs` | `remove_run_input` | `fn remove_run_input(&self, run: RunId) -> Result<(), JournalError>` |
| `vb_core::slot_value` | `input_mapping_empty` | `fn input_mapping_empty(inputs: &[(SlotIdx, SlotValue)]) -> bool` |
| `vb_storage::recovery::hydrate` | (extend `hydrate_run_frame`) | Accept optional `inputs` parameter for slot seeding |

### No Changes To

- Workflow YAML format
- Compiled IR format
- Runtime execution loop
- IPC protocol
- Existing run/run-compiled commands' input handling

---

## Hazards

- **Malformed bytes accepted:** If `map_runtime_inputs` returns `Ok` for malformed bytes, the user believes submission succeeded but the run cannot execute. Fixed by strict decoding.
- **No-input workflow given bytes:** If a workflow has no input slots, non-empty bytes must be an error, not silently ignored.
- **Recovery input drift:** If recovery does not re-apply the persisted `InputMapping`, the resumed run has different initial slot values than originally submitted.
- **Duplicate slot mapping:** If raw bytes decode to duplicate `SlotIdx` entries, only the last survives; this should be detected and rejected.
- **Kind mismatch:** If a value's runtime kind does not match the declared `InputSlotKind`, the mismatch is detected at submit time, not execution time.
