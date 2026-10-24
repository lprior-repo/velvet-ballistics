use super::arenas::{TAINTS, VALUES};
use super::support::{Case, TestResult};
use vb_core::{CoreError, SlotValue, StepIdx, StepState, Taint};
use vb_runtime::runtime::Runtime;

pub(super) fn original_prefix(case: &Case, runtime: &Runtime) -> TestResult {
    let state = runtime.inspect_run_state(case.run)?;
    let current = state.store.checkpoint_v1();
    let original = &case.initial;
    assert_eq!(state.workflow.to_parts(), case.workflow.to_parts());
    arena_prefix(original, &current);
    assert_eq!(
        state.frame.read_slot(case.input)?,
        &SlotValue::List(case.root)
    );
    assert_eq!(state.frame.read_taint(case.input)?, Taint::Secret);
    assert_eq!(state.run_attempt.get(), 1);
    assert_eq!(state.action_contracts, case.artifact.action_contracts);
    assert_eq!(state.action_attempts.as_ref(), &[0; 6]);
    let admission = state
        .admission
        .as_ref()
        .ok_or("historical admission missing")?;
    assert_eq!(admission.artifact_digest(), case.artifact.digest);
    assert_eq!(admission.run_id(), case.run);
    assert_eq!(admission.policy(), vb_core::RuntimePolicy::Strict);
    assert_eq!(
        admission.policy_pin(),
        vb_storage::RunPolicyPin::Pinned(case.artifact.policy_digest)
    );
    assert_eq!(admission.budget(), None);
    assert!(admission.idempotency_attested().is_empty());
    Ok(())
}

pub(super) fn rich_cut(case: &Case, runtime: &Runtime, after: bool) -> TestResult {
    original_prefix(case, runtime)?;
    let state = runtime.inspect_run_state(case.run)?;
    assert_eq!(state.store.list_count(), 6);
    rich_items(case, runtime)?;
    assert_eq!(state.frame.pc(), StepIdx::new(if after { 5 } else { 4 }));
    assert_eq!(state.frame.executed(), if after { 11 } else { 10 });
    assert_eq!(
        state.frame.step_state(StepIdx::new(3))?,
        StepState::Succeeded
    );
    assert_eq!(state.frame.step_state(StepIdx::new(5))?, StepState::Pending);
    assert!(state.collect_states.checkpoint_v1().entries.is_empty());
    assert!(state.collect_states.checkpoint_v1().lineages.is_empty());
    output_shape(case, runtime, after)?;
    Ok(())
}

fn rich_items(case: &Case, runtime: &Runtime) -> TestResult {
    let state = runtime.inspect_run_state(case.run)?;
    assert_eq!(state.store.list(case.root)?, VALUES);
    for (raw, (value, taint)) in VALUES.iter().zip(TAINTS.iter()).enumerate() {
        assert_eq!(
            state
                .store
                .list_item_with_taint(case.root, u32::try_from(raw)?)?,
            (*value, *taint)
        );
    }
    assert_eq!(
        state
            .store
            .list_item_with_taint(vb_core::ListId::new(0), 1)?,
        (SlotValue::I64(1), Taint::DerivedFromSecret)
    );
    Ok(())
}

fn output_shape(case: &Case, runtime: &Runtime, after: bool) -> TestResult {
    let state = runtime.inspect_run_state(case.run)?;
    let collector = case.output(0)?;
    let output = case.output(4)?;
    assert_ne!(collector, output);
    assert_ne!(collector, case.input);
    assert_ne!(output, case.input);
    assert_eq!(
        state.frame.read_slot(collector)?,
        &SlotValue::List(case.root)
    );
    assert_eq!(state.frame.read_taint(collector)?, Taint::Secret);
    if after {
        assert_eq!(
            state.frame.step_state(StepIdx::new(4))?,
            StepState::Succeeded
        );
        assert_eq!(state.frame.read_slot(output)?, &SlotValue::List(case.root));
        assert_eq!(state.frame.read_taint(output)?, Taint::Secret);
    } else {
        assert_eq!(state.frame.step_state(StepIdx::new(4))?, StepState::Pending);
        assert_eq!(
            state.frame.read_slot(output),
            Err(CoreError::SlotUninitialized { slot: output })
        );
    }
    Ok(())
}

fn arena_prefix(
    original: &vb_core::value_store::ValueStoreCheckpointV1,
    current: &vb_core::value_store::ValueStoreCheckpointV1,
) {
    assert_eq!(current.max_arena_entries, original.max_arena_entries);
    assert_eq!(
        current.symbols.get(..original.symbols.len()),
        Some(original.symbols.as_slice())
    );
    assert_eq!(
        current.lists.get(..original.lists.len()),
        Some(original.lists.as_slice())
    );
    assert_eq!(
        current.list_taints.get(..original.list_taints.len()),
        Some(original.list_taints.as_slice())
    );
    assert_eq!(
        current.objects.get(..original.objects.len()),
        Some(original.objects.as_slice())
    );
    assert_eq!(
        current.blobs.get(..original.blobs.len()),
        Some(original.blobs.as_slice())
    );
}
