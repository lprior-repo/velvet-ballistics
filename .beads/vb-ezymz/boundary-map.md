# Boundary Map: CLI Submit Input Validation and Persistence

**Bead:** vb-ezymz
**Product:** velvet-ballistics

---

## Architectural Boundaries

```
┌───────────────────────────────────────────────────────────────┐
│                     IMPERATIVE SHELL                         │
│              crates/velvet_ballistics/src/                   │
│                                                              │
│  submit.rs: cmd_submit()                                     │
│    → reads workflow YAML and input bytes from filesystem     │
│    → calls vb_compile::compile_workflow()                    │
│    → calls run_compiled::map_runtime_inputs()                │
│    → opens FjallJournal                                      │
│    → writes RunInputRecord (NEW)                             │
│    → writes RunHeaderRecord                                  │
│    → appends RunAccepted journal event                       │
│    → returns run_id to user via stdout/JSON                  │
└────────────────────┬──────────────────────────────────────────┘
                     │ calls (pure)
┌────────────────────┴──────────────────────────────────────────┐
│                       PURE CORE                              │
│       crates/vb_core/src/workflow/mod.rs                     │
│       crates/vb_core/src/slot_value.rs                       │
│                                                              │
│  CompiledWorkflow::input_slots() → &[CompiledInputSlot]      │
│  CompiledInputSlot { slot: SlotIdx, kind: InputSlotKind }    │
│  SlotValue enum (runtime value shape)                        │
│  WorkflowDigest (BLAKE3)                                     │
│                                                              │
│  No I/O, no mutation, no panics, no unwrap                   │
└────────────────────┬──────────────────────────────────────────┘
                     │ calls (parse)
┌────────────────────┴──────────────────────────────────────────┐
│                   POSTCARD BOUNDARY                          │
│              crates/velvet_ballistics/src/                   │
│                                                              │
│  run_compiled.rs: map_runtime_inputs()                       │
│    → decodes raw bytes to Box<[SlotValue]>                   │
│    → validates count vs declared slots                       │
│    → validates kind per declared slot                        │
│    → returns Result<InputMapping, InputMappingError>         │
│                                                              │
│  Boundary: raw bytes ↔ typed SlotValue collection            │
└────────────────────┬──────────────────────────────────────────┘
                     │ calls (persist)
┌────────────────────┴──────────────────────────────────────────┐
│                     STORAGE BOUNDARY                         │
│                crates/vb_storage/src/                        │
│                                                              │
│  FjallJournal::put_workflow_source()                         │
│  FjallJournal::put_run_header()                              │
│  FjallJournal::put_run_input() (NEW)                         │
│  FjallJournal::append_strict_batch()                         │
│                                                              │
│  Inputs: RunInputRecord, RunHeaderRecord, WorkflowSourceRecord│
│  Outputs: typed Result<_, JournalError>                      │
│  Side effects: filesystem writes, fsync                      │
│                                                              │
│  Recovery reads via get_run_input() to re-hydrate frames     │
└───────────────────────────────────────────────────────────────┘
```

---

## Boundary Crossing Analysis

### 1. CLI Shell → Pure Core (compile)

- **Entry:** `cmd_submit` calls `vb_compile::compile_workflow(&bytes)`
- **Crossing:** Raw YAML bytes → `CompiledWorkflow`
- **Validation:** Compilation errors reported before any input processing
- **Side effects:** None (pure function)

### 2. Pure Core → Parser Boundary (map inputs)

- **Entry:** `map_runtime_inputs(&compiled, &input_data)`
- **Crossing:** `CompiledWorkflow.input_slots()` + raw postcard bytes → `InputMapping`
- **Validation:** 
  - Decoding must succeed (postcard `from_bytes`)
  - Decoded count must not exceed declared slot count
  - Each value's kind must match declared slot kind
- **Side effects:** None (pure mapping)

### 3. CLI Shell → Storage Boundary (persist)

- **Entry:** `journal.put_workflow_source()`, `put_run_input()`, `put_run_header()`, `append_strict_batch()`
- **Crossing:** In-memory records → persistent Fjall keyspaces
- **Validation:** Storage accepts valid record types, rejects malformed envelopes
- **Side effects:** Filesystem writes, durability syncs per mode

### 4. Storage → Recovery (re-hydrate)

- **Entry:** `hydrate_run_frame(&snapshot, &tail, run)` or `hydrate_run_frame_from_events(&events, run)`
- **Crossing:** Persistent records → in-memory `RunFrame`
- **Validation:** Digest checks, sequence validation, slot range checks
- **Side effects:** None (pure reconstruction)

---

## Input Flow Through Boundaries

```
User provides:
  workflow.yaml + input.vbin (raw postcard bytes)

CLI Shell:
  read_file(workflow.yaml) → bytes
  read_file(input.vbin) → raw_bytes

Pure Core:
  compile_workflow(bytes) → CompiledWorkflow
  CompiledWorkflow.input_slots() → declared slots

Parser Boundary:
  map_runtime_inputs(compiled, raw_bytes) → Result<InputMapping, Error>
  
  If declared slots = 0 and raw_bytes non-empty:
    → InputMappingError::SlotCountExceeded
  
  If declared slots > 0 and raw_bytes non-empty:
    → decode → validate count → validate kinds → InputMapping
  
  If declared slots > 0 and raw_bytes empty:
    → Ok(empty mapping)

Storage Boundary (on success):
  put_workflow_source(record)
  put_run_input(RunInputRecord { run, inputs })
  put_run_header(RunHeaderRecord { run, digest, ... })
  append_strict_batch([RunAccepted { run, seq, workflow }])
  
  → returns run_id to user
```

---

## Recovery Flow Through Boundaries

```
Process restarts, needs to recover run R:

Storage Boundary:
  get_run_header(R) → RunHeaderRecord { run: R, digest: D, ... }
  get_run_input(R) → RunInputRecord { run: R, inputs: mapping }
  load_accepted_artifact(D) → CompiledWorkflow IR
  replay_journal(R) → events (slot writes, state changes)

Pure Core:
  compile_workflow_from_ir(IR) → CompiledWorkflow
  CompiledWorkflow.input_slots() → declared slots

Parser Boundary:
  Validate that persisted mapping still matches declared slots
  (should be identical since workflow is deterministic)

Recovery:
  hydrate_run_frame_from_events(events, R) → base frame
  seed_input_slots(frame, mapping, compiled) → hydrated frame
  → resume execution from hydrated frame
```

---

## Error Propagation

| Stage | Error | Propagates To |
|-------|-------|---------------|
| Compile | `CompileErrors` | CLI exits with `CompileFailed`, no storage writes |
| Input decode | `InputMappingError::DecodeFailed` | CLI exits with `ValidationFailed`, no storage writes |
| Input count | `InputMappingError::SlotCountExceeded` | CLI exits with `ValidationFailed`, no storage writes |
| Input kind | `InputMappingError::KindMismatch` | CLI exits with `ValidationFailed`, no storage writes |
| Input range | `InputMappingError::SlotIndexOutOfRange` | CLI exits with `ValidationFailed`, no storage writes |
| Workflow source write | `JournalError` | CLI exits with `StorageError` (partial state) |
| Run input write | `JournalError` | CLI exits with `StorageError` (partial state) |
| Run header write | `JournalError` | CLI exits with `StorageError` (partial state) |
| Journal append | `JournalError` | CLI exits with `StorageError` (partial state) |

**Atomicity note:** There is no true transaction across storage operations. If a write fails mid-sequence, some partial state may be persisted. This is an existing architectural limitation, not introduced by this bead. The contract specifies that validation errors must prevent any durable writes; write failures after validation are acceptable partial states.
