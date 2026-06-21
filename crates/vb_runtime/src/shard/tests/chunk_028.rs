
#[test]
fn is_resumable_returns_true_when_state_is_resumable() {
    // Given
    let state = super::RuntimeState::Resumable;

    // When
    let result = state.is_resumable();

    // Then
    assert_eq!(result, true);
}

#[test]
fn is_resumable_returns_false_when_state_cannot_be_resumed() {
    // Given
    let non_resumable_states = [
        super::RuntimeState::Initial,
        super::RuntimeState::Running,
        super::RuntimeState::Resuming,
        super::RuntimeState::Failed,
    ];

    // When / Then
    for state in non_resumable_states {
        assert_eq!(state.is_resumable(), false, "state {state:?} must not be resumable");
    }
}

/// RQ-W0-20: `Shard::handle_resume` must accept `RuntimeState::Resuming`
/// as a recoverable state so a process crash between the journal append
/// and the drive step can be recovered by a subsequent resume attempt.
#[test]
fn handle_resume_recovers_resuming_state_without_reappending() -> Result<(), RuntimeError> {
    use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    // Use a `Do` action that suspends waiting for completion so the run
    // remains in `Running` after the initial tick rather than racing to
    // `Finish` before the recovery transition.
    let action = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::<str>::from("rq_w0_20_resuming_recovery"),
        digest: WorkflowDigest::from_bytes([0x20; 32]),
        nodes: Box::new([action]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        step_names: Box::new([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .map_err(|_| RuntimeError::QueueFull)?;
    let run = RunId::new(0xC0DE_0020);

    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)?;

    shard.enqueue(ShardCommand::Submit {
        run,
        workflow,
        caps: vb_core::capability::CapabilitySet::empty(),
    })?;
    shard.tick()?;
    // The Do action suspends awaiting its completion ticket, so the
    // runtime state machine transitions to `Resumable` (the valid
    // pre-`Resuming` state for a suspended run).
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::RuntimeState::Resumable)
    );

    // Manually transition the run to Resuming to simulate the
    // half-completed state left by a crashed handle_resume attempt.
    shard.runtime_state_insert(run, super::RuntimeState::Resuming);

    // Recovery call must succeed (RQ-W0-20) without re-asserting the
    // NotResumable error. The drive step advances the run forward; the
    // Do action suspends again on the action-completion ticket, so the
    // state machine transitions `Resuming -> Running -> Resumable`.
    let result = shard.handle_resume(run);
    assert!(
        result.is_ok(),
        "handle_resume must accept Resuming as recoverable; got {result:?}"
    );

    // After the recovery drive, the run is still suspended on the Do
    // action and therefore tracked in runtime_states as `Resumable`,
    // awaiting the action-completion ticket.
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::RuntimeState::Resumable),
        "resumed run must re-suspend on the Do action after recovery drive"
    );
    Ok(())
}
