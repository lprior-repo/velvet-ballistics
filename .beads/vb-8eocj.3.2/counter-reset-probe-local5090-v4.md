# vb-8eocj.3.2 Repeat Counter Reset Negative Probe (Local RTX5090)

## Environment
- Machine: RTX5090 local
- Git/JJ root: /home/lewis/src/isoloated/vb-8eocj-repeat-probes-local3090-v1
- Base commit: 9820a5d9d (V15 baseline, already includes vb-8eocj.3.1 fix)
- Test file: crates/workspace_tests/tests/vb_f1_repeat_counter_reset_probe.rs
- Tests run: cargo test --test vb_f1_repeat_counter_reset_probe -- --nocapture

## Results

### Test 1: baseline_repeat_workflow_executes_correctly — PASSED
- Compiled repeat workflow from canonical source
- Executed with 64-step budget
- Signal: Finished with SlotValue::I64(42) — correct
- Body executions (StepSucceeded events): 3 — correct
- Workflow parts unchanged after execution — correct

### Test 2: repeat_check_output_slot_negative_probe — FAILED (observational RED)
- Compiled same baseline repeat workflow
- Found RepeatStart node (StepIdx(1)), body step (StepIdx(2)), RepeatCheck node (StepIdx(3))
- Attempt slot: SlotIdx(1) (owned by RepeatStart output)
- Mutated body node output from Some(SlotIdx(2)) [body_result] to Some(SlotIdx(1)) [attempt_slot]
- Reconstructed workflow via CompiledWorkflow::try_from_parts(mutated_parts)

#### Constructor outcome: ACCEPTED
- No validation error from try_from_parts
- Mutated IR shape accepted as valid

#### Runtime outcome (observational):
- Signal: StepBudgetExhausted (did not finish within 64 steps)
- Body step executions: 31 (expected 3 if working correctly)
- Counter slot writes (SlotWritten events to attempt_slot): 63

#### Interpretation:
The constructor accepts the malformed IR where the body node's output is redirected
to the counter state slot. The runtime executes but exhibits corrupted counter behavior:
31 body executions instead of 3, and the workflow never reaches the RepeatFinish node
because the counter state is overwritten by each body execution.

This proves the pre-fix negative property: the constructor does not validate that
body node output matches the body result slot, and the runtime counter mechanism
is fragile to this class of IR corruption.

## Status
NEGATIVE PROBE COMPLETE — tests written and executed, observations recorded.
No production code changes. No commit/push.
