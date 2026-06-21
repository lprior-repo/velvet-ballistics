# RA-019: `submit_direct` / `submit_compiled_with_inputs` hard-code `CapabilitySet::empty()`, blocking capability workflows

- **Severity**: Info
- **Category**: correctness (API surface)
- **Location**: `crates/vb_runtime/src/runtime/admission/admission_check.rs:123-154`
- **Confidence**: confirmed

## Description

The public `submit_direct`, `submit_compiled`, and `submit_compiled_with_inputs` methods always pass `CapabilitySet::empty()` into the admission preflight and into the enqueued `ShardCommand`. For any workflow whose accepted artifact declares `required_capabilities` non-empty, `admit_artifact_run` rejects with `CapabilityDenied` (count mismatch), so these public APIs cannot submit capability-gated workflows.

## Evidence

```rust
pub fn submit_direct(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
    let shard = self.shard_for(run)?;
    let _admission_guard = shard.lock_admission()?;
    Self::preflight_direct_admission(shard, run, &workflow, CapabilitySet::empty())?;
    shard.enqueue(ShardCommand::Submit {
        run,
        workflow,
        caps: CapabilitySet::empty(),
    })
}

pub fn submit_compiled_with_inputs(
    &self,
    run: RunId,
    workflow: CompiledWorkflow,
    inputs: Box<[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)]>,
) -> RuntimeResult<()> {
    ...
    Self::preflight_direct_admission(shard, run, &workflow, CapabilitySet::empty())?;
    shard.enqueue(ShardCommand::SubmitWithInputs {
        run,
        workflow,
        inputs,
        caps: CapabilitySet::empty(),
    })
}
```

The only public submit that accepts capabilities is `submit_direct_with_inputs_grants_and_contracts` (lines 158-176). It is also the only one that takes `action_contracts`. So callers with capability-gated workflows MUST also supply action contracts and inputs — there is no `submit_direct_with_grants` shortcut.

## Adversarial Check

One could argue capability-gated workflows always need action contracts (the contract is what binds the capability to the action), so the all-in-one API is the only meaningful one. But the asymmetric API surface is surprising: a caller who has a workflow + caps but no inputs gets a `CapabilityDenied` error from `submit_direct` rather than a "use submit_direct_with_inputs_grants_and_contracts" message. The docstrings on `submit_direct` and `submit_compiled_with_inputs` do not mention the capability constraint.

## Suggested Fix

Either (a) document the constraint on `submit_direct` / `submit_compiled_with_inputs` ("does not support workflows with required capabilities — use `submit_direct_with_inputs_grants_and_contracts`"), or (b) add a `submit_compiled_with_grants(run, workflow, caps)` shortcut that supplies empty inputs and empty action_contracts. Option (a) is the minimum fix.
