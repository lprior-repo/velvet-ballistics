---
section: 36
title: "Mandatory Test Coverage"
parent: velvet-ballistics-MASTER.md
---

## 36. Mandatory Test Coverage


**Test naming:** Exact test names are not mandated. Tests must exist that verify the following behaviors. The authoritative test list is the codebase; this section states required coverage areas.

### Core value and ID tests

Required coverage:
- `FiniteF64` accepts finite values, rejects NaN, rejects positive infinity, rejects negative infinity.
- `SlotValue` type names are stable and correct for every variant.
- `SlotValue` text uses symbol or blob handles (no inline strings).
- `ConstValue::to_slot_value` maps every variant; no silent Null fallback.
- `StepBudget` exhaustion returns false without error; remaining reaches zero cleanly.
- `RunFrame` bounds-checked for slots and step states; out-of-bounds returns typed errors.
- Step-state mark methods return errors on invalid step indices.
- `CompiledWorkflow::try_from_parts` rejects invalid PC, invalid edges, invalid tables.

### Parser and validator tests

Required coverage:
- Minimal valid manual and IPC workflows parse successfully.
- HTTP trigger rejected as out-of-core.
- Duplicate keys, anchors, aliases, merge keys, YAML 1.1 ambiguous booleans all rejected.
- Unknown top-level fields and unknown step fields rejected.
- Multiple primitives per step rejected; missing primitive rejected.
- Forward references rejected.
- Control-flow cycles detected and rejected.
- Secret-tainted finish results preserved in `Finished(SlotValue, Taint)`.
- All diagnostics have code, path, span, and message.

### Engine invariant tests

Required coverage:
- Terminal states never transition back to running.
- Failed steps do not become succeeded without error handler.
- Budget exhaustion does not advance PC.
- Missing output slot, const out of bounds, expression stack overflow/underflow, unsupported primitive — all return typed errors.
- `SetConst` never reads unrelated slot zero.
- `Choose` and `ChooseSlot` produce identical results when conditions are pre-materialized.

### Recovery tests

Required coverage:
- Full journal replay reconstructs run state.
- Snapshot plus tail replay reconstructs run state.
- Replay detects divergence with typed error.
- Non-idempotent actions blocked during replay.
- Strict profile persists before ack.
- Journaled profile group commit recovers.
- Corrupt journal record returns typed error.

### IPC tests

Required coverage:
- Bad magic rejected before payload allocation.
- Oversized payload rejected.
- Command roundtrips (submit, cancel, inspect, events).
- Backpressure respected.
- Malformed frames return typed errors.

### Scheduler tests

Required coverage:
- Queue-full returns typed error.
- Run stays on one shard.
- Cancel pending and waiting runs.
- Shutdown drains gracefully or reports remaining.
- Timer resume order deterministic.
- Action completion resumes correct run.
- No task-per-step behavior under load.

### Compile-fail tests

Required coverage:
- Active public macro/schema contracts reject invalid usage at compile time when such contracts exist.
- Generated Rust compile-fail tests are removed with `vb_codegen`.

---
