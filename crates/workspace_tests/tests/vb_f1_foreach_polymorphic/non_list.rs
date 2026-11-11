use super::*;
use vb_runtime::engine::{
    EvidenceCollector, RetryPolicy, RuntimeEngineError, drive_deterministic_full,
};

pub(crate) fn check() -> TestResult {
    let mut fixture = Fixture::new("$entry")?;
    fixture.seed([Taint::Clean; 3])?;
    let state = fixture.runtime.inspect_run_state(RunId::new(1))?;
    let workflow = state.workflow.clone();
    let contracts = state.action_contracts.clone();
    let (_, _, reply_slot) = *fixture.actions.get(3).ok_or("missing fetch")?;
    fixture.complete(3, V::I64(42), Taint::Secret)?;
    let events = fixture.journal.snapshot()?;
    assert_eq!(
        events
            .iter()
            .filter_map(|event| match event {
                E::RunFailed { run, attempt } => Some((*run, *attempt)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![(RunId::new(1), 1)]
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::RunFinished { .. }))
            .count(),
        0
    );
    let checkpoints = events
        .iter()
        .filter_map(|event| match event {
            E::RetryCheckpoint { payload, .. } => Some(payload),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(checkpoints.len(), 1);
    let checkpoint = vb_runtime::run_checkpoint::decode_checkpoint_payload(
        checkpoints
            .first()
            .ok_or("missing actual failure checkpoint")?,
    )?;
    let artifact = checkpoint.artifact_digest;
    let (mut state, _, failed, _) = checkpoint.restore(workflow, artifact, &contracts)?;
    assert_eq!(state.frame.pc(), failed);
    assert_eq!(*state.frame.read_slot(reply_slot)?, V::I64(42));
    assert_eq!(state.frame.read_taint(reply_slot)?, Taint::Secret);
    let start = state.workflow.node(failed).ok_or("missing failed node")?;
    assert!(matches!(start.kind, K::ForEachStart { .. }));
    // The Runtime's own validated retry checkpoint supplies the exact failed input.
    let result = drive_deterministic_full(
        &state.workflow,
        &mut state.frame,
        &mut vb_core::StepBudget::new(1),
        &mut state.store,
        &state.action_contracts,
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut state.collect_states,
        &CapabilitySet::empty(),
        state.run_attempt.get(),
    );
    let Err(RuntimeEngineError::Core(error)) = result else {
        return Err(format!("expected typed list failure: {result:?}").into());
    };
    assert_eq!(
        error,
        vb_core::CoreError::TypeMismatch {
            expected: "list",
            found: "number"
        }
    );
    Ok(())
}
