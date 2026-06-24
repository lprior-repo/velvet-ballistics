# Rust Domain/Type Contract — vb-vbdco (RE-010)

**Bead:** vb-vbdco (duplicate of closed `vb-y71ef`)
**Finding:** `bug-hunt-2026-06-21/findings/runtime-engine/RE-010-evidence-collector-silent-drop.md`
**Source:** `crates/vb_runtime/src/engine/types.rs` (EvidenceCollector),
`crates/vb_runtime/src/engine/drive.rs` (drive-step call sites),
`crates/vb_core/src/errors.rs` (EngineError::EvidenceCapacityExceeded).
**Closure status:** work already merged via `vb-y71ef` (commit
`d8221505b`, merged `5f101f82b`); this contract documents the
**already-shipped** state on `main` rather than a proposed change.

This artifact is intentionally written so that the contract reviewer
can confirm the contract is honored by the production code on `main`,
without proposing new public API surface. No new tests, proofs, or
implementation are introduced by this bead.

## Ubiquitous Language

- **Evidence event** — a single immutable record of a deterministic
  drive-loop observation (`StepStarted`, `StepSucceeded`, `SlotWritten`).
- **Evidence collector** — bounded buffer that accumulates evidence
  events during a drive loop before they are drained to the journal.
- **Capacity** — the maximum number of evidence events the collector
  can hold before it must surface an overflow.
- **Overflow** — the act of attempting to push an event when the
  collector already holds `capacity` events.

## Value Object: `EvidenceCollector`

```text
EvidenceCollector {
    events:   Vec<EvidenceEvent>     // bounded by capacity
    capacity: usize                  // fixed at construction
}
```

The previous `dropped: usize` counter is **deleted**. Overflow is no
longer a silent accounting field — it is a typed error.

## Error Taxonomy

```rust
pub enum EngineError {
    // …existing variants…
    /// Evidence capacity was exceeded during a non-collect push.
    #[error("evidence capacity exceeded: step {step:?} slot {slot:?} capacity {capacity}")]
    EvidenceCapacityExceeded {
        step: StepIdx,
        slot: SlotIdx,
        capacity: usize,
        len: usize,
        required: &'static str,
    },
    // …existing variants…
}
```

A separate `CollectEvidenceCapacityExceeded` variant already exists
for the slot-with-extra path. The two variants are not collapsed:
they differ in the `required` description and the diagnostic-code
bucket.

### Diagnostic Code

`EVIDENCE_CAPACITY_EXCEEDED_CODE = DiagnosticCode::new(0x140E)` is
registered in `CoreError::evidence_capacity_exceeded_code` and
mapped in `engine_error_static_code` to the string code
`"EVIDENCE_CAPACITY_EXCEEDED"`.

## Workflow: `push_*` Contract

```text
push_step_started(step) -> Result<(), EngineError>
push_step_succeeded(step, output) -> Result<(), EngineError>
push_slot_written(slot, value) -> Result<(), EngineError>
push_slot_written_with_taint(slot, value, taint) -> Result<(), EngineError>
```

| Pre-state                          | Action                                | Post-state                                                                                              |
|------------------------------------|---------------------------------------|---------------------------------------------------------------------------------------------------------|
| `events.len() < capacity`          | push                                 | `events.push(event)`; return `Ok(())`                                                                  |
| `events.len() >= capacity` (push) | push                                 | `Err(EngineError::EvidenceCapacityExceeded { step, slot: ZERO, capacity, len, required: <name> })`      |

The `SlotWritten` family uses `slot = <provided>`, `step = ZERO`.
The `StepStarted` and `StepSucceeded` families use `step = <provided>`,
`slot = ZERO`. The `push_slot_written_with_extra` path returns the
pre-existing `CollectEvidenceCapacityExceeded` (with `run_id`) when
the optional `extra` is `Some(_)`.

### Sentinel-Value Discipline

- `step: StepIdx::ZERO` in the slot-only variants is a sentinel that
  means "no step index applies". This is allowed because the variant
  is private to the runtime and `StepIdx::ZERO` is never used as a
  real step index (steps are 1-based in `vb_runtime`).
- `slot: SlotIdx::ZERO` in the step-only variants is the analogous
  sentinel for "no slot applies".

This sentinel discipline keeps the variant signature uniform across
all four push methods without introducing a second error variant per
push flavor.

## Workflow: `drive_deterministic_full` Step Lifecycle

The drive loop composes four observers per step. The RE-011 fix
(commit `3bbfa264d`) sets the canonical order:

1. `run.mark_running(pc).map_err(RuntimeEngineError::Core)?` — commit
   the step state first.
2. `evidence.push_step_started(pc).map_err(RuntimeEngineError::Core)?`
   — emit the StepStarted evidence. If this fails, the step is
   `Running` but no StepStarted was emitted: the caller sees a typed
   `EvidenceCapacityExceeded` and can decide to drain and retry.
3. Execute the node (slot writes happen here; they bypass the
   collector until `emit_slot_evidence`).
4. `emit_slot_evidence(...)` — pushes a `SlotWritten` event (or
   `SlotWritten { extra }` for collect nodes) and propagates the error.
5. `mark_step_after_signal(...)` — commit the post-signal state.
6. On the success signal branch:
   `evidence.push_step_succeeded(...).map_err(RuntimeEngineError::Core)?`.

A `EvidenceCapacityExceeded` raised at step 1 or step 4 propagates
all the way to `drive_deterministic_full`'s return value with no
partial commit: the `RunFrame` is left in a recoverable state and the
caller can drain the partial event buffer, decide whether to abort
the run, or rebuild the collector with a larger capacity.

## Hazards

| Hazard | Where | Mitigation |
|--------|-------|------------|
| Capacity overflow silent drop | `EvidenceCollector::push_*` | All four `push_*` methods return `Result<(), EngineError>`; the only `dropped` field was removed. |
| Drive loop ignoring overflow | `engine/drive.rs` (drive-step helpers) | Every `push_*` call site is followed by `.map_err(RuntimeEngineError::Core)?` so the typed error reaches `drive_deterministic_full`'s caller. |
| Capacity overflow races with step state commit | `engine/drive.rs::finish_drive_step` | RE-011 ordering keeps the `RunFrame` in a recoverable state; the partial `evidence` buffer can be drained on retry. |
| Misclassifying collect vs non-collect overflow | `push_slot_written_with_extra` | Distinct `CollectEvidenceCapacityExceeded` (with `run_id`) and `EvidenceCapacityExceeded` variants; the `extra: Option<CollectPaginationState>` discriminator drives the choice. |
| Sentinel confusion in the error payload | new variant fields | Documented sentinel discipline: `ZERO` means "does not apply". |

## Proof Seeds (carried over from `vb-y71ef`)

| Seed ID | Lane | Property | Source binding |
|---------|------|----------|----------------|
| `seed.re010.capacity_overflow_returns_err` | Kani | For all `c in 0..=8`, pushing `c+1` events of each flavor into a collector of capacity `c` returns `EvidenceCapacityExceeded` on the last push and the buffer remains at length `c`. | `types.rs::push_*` |
| `seed.re010.drive_loop_propagates_overflow` | Proptest | A drive loop with a collector at capacity ends with `Err(EvidenceCapacityExceeded)` and the `RunFrame` is in the pre-success state. | `drive.rs::begin_drive_step`, `drive.rs::finish_drive_step` |
| `seed.re010.zero_capacity_always_errs` | Kani | With capacity 0, every push of every flavor returns `Err(EvidenceCapacityExceeded)` and the buffer stays at length 0. | `types.rs::push_*` |
| `seed.re010.success_path_no_overflow` | Proptest | A drive loop with a collector of capacity >= 3 * step_budget completes without overflow. | `drive.rs::drive_deterministic_full` |
| `seed.re010.error_variant_carries_capacity_and_len` | Kani | `EngineError::EvidenceCapacityExceeded { capacity, len, .. }` matches the collector's `capacity()` and `len()` at the moment of failure. | `types.rs::push_*` |

## Bead Closure Note

`vb-vbdco` is a duplicate of `vb-y71ef`. The contract above documents
what is **already shipped on `main`**, not a proposed change. The
proof seeds in this artifact are evidence that the existing
implementation passes the binding checks; they are not new Kani
obligations introduced by this bead.
