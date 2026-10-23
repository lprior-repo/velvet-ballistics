# Contract: Bounded Owned-Region Body Rearm (`drive/loop_rearm.rs`)

## Problem

`jump_to_body` resets only the body entry step. `owned_continuation_rearm` resets only the loop control latch. Multi-step loop bodies with internal Succeeded nodes retain stale state across iterations, breaking re-execution.

## Solution

New module `drive/loop_rearm.rs` provides `loop_body_rearm`. Replacing the former latch-only rearm, it resets all Succeeded nodes within the validated owned region `[body, control]` to Pending.

## API

```rust
// drive/loop_rearm.rs
pub(super) fn loop_body_rearm(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    control: StepIdx,
    body: StepIdx,
) -> RuntimeEngineResult<()>;
```

Caller: `finish_drive_step` passes the validated plan (seventh argument) to `owned_continuation_rearm`; that wrapper calls this helper instead of the old latch-only mutation, after success evidence/state commit.

## Ownership Validation

The validator constrains nesting topology and strongly pairs RepeatCheck; it does not prove all other Next-to-Start pairings. Runtime validates pairing at rearm time:

1. Scan backward from `control` to find nearest active Start node.
2. Verify `owner.body == body`, `owner.done` matches expected exit, and iterator/accumulator slots align per loop family.
3. Verify `body < control < done`.

If pairing fails, return `Err` without mutation.

## State Transition Rules

Two passes over `[body, control]`:

**Pass 1 (preflight):** Scan all nodes. If any is Waiting, Asking, or Running, return `Err` without mutation (fail-closed).

**Pass 2 (mutation):** For each node:
- Succeeded → Pending
- Pending → Pending (preserved; body entry already reset by `jump_to_body`)
- Failed, Cancelled, Skipped → preserved

## Invariants

1. **Paired ownership**: `body` and `control` proven paired to same Start node before mutation.
2. **Fail-closed**: In-flight states cause `Err` with no mutation.
3. **Selective reset**: Only Succeeded nodes transition; all other states preserved.
4. **Bounded iteration**: Range `[body, control]` is finite and validated; no allocation.
5. **No partial reset**: Invalid state detected in pass 1 prevents any mutation in pass 2.
6. **Pre-latch timing**: Invoked before latch rearm; inflight rejection applies after current control Succeeded.

## Risks

- **Pairing validation incomplete**: Runtime checks must be exhaustive per loop family.
- **In-flight detection**: Must check all three in-flight states.
- **Validator/runtime divergence**: Runtime pairing validation is authoritative.

## Tests

1. Simple loop: body has 2 Succeeded nodes → both reset to Pending
2. Nested loops: inner body reset, outer body untouched
3. In-flight node: body contains Waiting → Err, no mutation
4. Mixed states: Succeeded + Failed + Skipped → only Succeeded reset
5. Body entry already Pending: idempotent
6. Pairing mismatch: wrong body index → Err, no mutation
## Exact pairing and boundaries

Nearest active structured start is selected before control: done/join must extend beyond control. Family, body and done must match. ForEach uses owner.output == iterator_slot; Collect uses owner.output or source fallback == collector_slot; Reduce uses owner.output == iterator_slot plus identical accumulator; Repeat uses owner.output == attempt_slot. Require body < control < done, and reset only inclusive [body, control]. No nodes between control and done or outside this interval are reset. Pairing and state preflight precede any mutation by this helper; primitive slot/PC updates and success evidence already occurred. No new allocation or speed claim.
