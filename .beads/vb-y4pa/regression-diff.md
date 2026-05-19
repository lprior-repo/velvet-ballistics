STATUS: PASS

Primary fix: conditional `jump_to_body` in `helpers.rs:60-69`

Before: unconditional `jump_to(run, body)` called without checking body step state
After: `if current == StepState::Succeeded { run.mark_pending(body)?; }` guard added

No regressions introduced. The fix:
- Only affects `jump_to_body` helper used by loop primitives
- Preserves `Waiting` and `Asking` states (does not reset non-Succeeded steps)
- Enables Succeeded→Pending→Running transition for loop body re-entry
- All vb_runtime tests pass