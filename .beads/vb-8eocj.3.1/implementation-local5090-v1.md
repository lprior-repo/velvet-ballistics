# vb-8eocj.3.1 Implementation Report (Local5090)

Narrow lowering fix: RepeatCheck node now exposes its attempt counter
slot as the node's declared output, enabling generic slot evidence
emission for the counter writes performed by the `repeat_check` primitive.

## Exact hunk applied

File: `crates/vb_compile/src/mod_compile_lowering/part_04.rs`
Function: `lower_canonical_repeat`
Change: Line 142, `output: None` -> `output: Some(attempt_slot)`

```diff
 builder.push_node(CompiledNode {
     id: check,
-    output: None,
+    output: Some(attempt_slot),
     next: Some(body_step),
     error_slot: None,
     on_error: None,
     kind: CompiledNodeKind::RepeatCheck {
         attempt_slot,
         done,
     },
 });
```

## Why this is the correct fix

The `repeat_check` runtime primitive (`crates/vb_runtime/src/primitives/repeat.rs`)
unconditionally writes the updated packed attempt state to `attempt_slot`
before every branch decision (both continue and exhaust paths):

```rust
let updated = encode_repeat_state(max_attempts, next_attempt)?;
run.write_slot(attempt_slot, SlotValue::I64(updated))?;
```

However, the generic drive-loop evidence emission path
(`crates/vb_runtime/src/engine/drive/actions.rs::emit_slot_evidence`)
uses the node's declared `output` field to determine which slot to
capture. With `output: None`, no `SlotWritten` evidence event is emitted
for the counter progression, even though the slot is actively written.

Setting `output: Some(attempt_slot)` connects the existing runtime write
to the existing evidence collection mechanism. No runtime code changes
are needed; this is purely a lowering metadata correction.

## Gate13 reasoning

Under Gate13 semantics, the drive loop:
1. Executes the step (including the primitive's slot write)
2. Calls `mark_step_after_signal`
3. Calls `emit_slot_evidence` for every step

`emit_slot_evidence` checks `node.output` and, if present, reads the slot
value, captures taint, and emits a `SlotWritten` event. With this change,
every invocation of `repeat_check` — whether it loops back to the body
or routes to done — will emit a `SlotWritten` event containing the exact
packed `(max_attempts, current_attempt)` counter state.

## What remains unchanged

- Lowerer still emits exactly four nodes for a repeat construct
- All `checked_step_offset` bounds checks are preserved
- `attempt_slot` allocation is unchanged
- `body_result` allocation is unchanged
- `RepeatFinish` still uses `output: Some(body_result)`
- No changes to `vb_core`, `vb_runtime`, or any test/config files

## Compilation verified

```bash
cd /home/lewis/src/isoloated/vb-8eocj-repeat-evidence-local5090-v1
RUSTC_WRAPPER="" cargo check -p vb_compile --no-default-features
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.23s
```

## Prerequisites for acceptance

This lowering fix is only observable at runtime when:
1. The OTHER owner's vb-8eocj.3 runtime/transitions integration is landed
   (or landed together)
2. The existing repeat test suite and frozen19 driver are run against the
   combined changes
3. Evidence collectors capture the `SlotWritten` events for the counter
   state on each repeat iteration

## NOT TESTED limitations

- No local runtime tests are executed here (owned by vb-8eocj.3)
- No integration with frozen19 driver (owned by vb-8eocj.3)
- No verification that the OTHER owner's runtime changes are compatible
- Acceptance requires the full vb-8eocj integration test suite to pass
- No moon ci gate run (requires integration with runtime changes)

## Files modified

- `crates/vb_compile/src/mod_compile_lowering/part_04.rs` (1 line)

## Files created

- `.beads/vb-8eocj.3.1/contract.md`
- `.beads/vb-8eocj.3.1/implementation-local5090-v1.md` (this file)

## Delivery status

Contract written -> implementation complete -> awaiting tests/code review
from vb-8eocj.3 owner. This is a WIP candidate, not yet accepted.
