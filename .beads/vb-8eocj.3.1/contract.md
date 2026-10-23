# RepeatCheck counter evidence metadata (lowering-only)

Narrow lowering fix: the RepeatCheck node currently has `output: None`, so
generic `emit_slot_evidence` cannot emit `SlotWritten` events for the
attempt counter writes that the `repeat_check` primitive performs on both
the continue and exhaust paths. The counter state is written to the
`attempt_slot` but never surfaced as evidence because the lowering pass
declares no output.

## What changes

`crates/vb_compile/src/mod_compile_lowering/part_04.rs`
- `lower_canonical_repeat`: change the RepeatCheck node's `output` from
  `None` to `Some(attempt_slot)`. The `repeat_check` primitive already
  writes the packed `(max_attempts, current_attempt)` state to this slot
  before each branch decision; declaring it as the node's output enables
  the generic drive-loop evidence emission path.
- All other RepeatCheck fields (`kind`, `attempt_slot`, `done`, `next`)
  remain unchanged. The lowering still emits exactly four nodes with
  identical step offsets, bounds checking, and error handling.

## Why this is correct

- The `repeat_check` primitive writes `attempt_slot` unconditionally on
  both paths (continue to body, or exhaust to done). It does NOT write
  a separate result slot.
- The body result slot (`body_result`) is already the output of
  `RepeatFinish` via the `RepeatFinish { result: body_result }` node.
- Setting `RepeatCheck.output = Some(attempt_slot)` is distinct from
  the body result — it captures the counter progression evidence.
- Under Gate13 semantics, the drive loop calls `emit_slot_evidence` for
  every step; with an output slot, it reads the slot, captures taint,
  and emits a `SlotWritten` event with the exact packed counter value.
- No runtime code changes are required; the primitive already writes
  the slot. This is purely a lowering metadata correction.

## Constraints

- No dummy values, fake IR, or dependency relaxation.
- No changes to `crates/vb_core`, `crates/vb_runtime`, or any test/config/allocator files.
- Only the single lowering file is modified: `crates/vb_compile/src/mod_compile_lowering/part_04.rs`.
- Lowerer width remains four nodes; all `checked_step_offset` bounds are preserved.
- Existing `vb-8eocj` integration tests and frozen19 driver will capture
  counter evidence once the OTHER owner merges their runtime integration.

## Prerequisites for acceptance

- Parent vb-8eocj.3 owner must have landed the runtime/transitions integration
  that reads repeat evidence, or this must be landed together.
- No independent local tests are written here; acceptance requires
  integration with the existing repeat test suite owned by vb-8eocj.3.

## Delivery flow

Contract (this file) -> implementation (lowering hunk) -> tests (owner)
-> code review -> test review. No go-skill or full proof-planning.
