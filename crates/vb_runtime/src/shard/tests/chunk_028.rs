
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
    use vb_core::ids::RunId;
    use vb_core::value::ConstValue;
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts,
    };
    use vb_core::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};

    let set_const = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::<str>::from("rq_w0_20_resuming_recovery"),
        digest: WorkflowDigest::from_bytes([0x20; 32]),
        nodes: Box::new([set_const, finish]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::Bool(true)]),
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
    assert_eq!(shard.runtime_state_get(run), Some(super::RuntimeState::Running));

    // Manually transition the run to Resuming to simulate the
    // half-completed state left by a crashed handle_resume attempt.
    shard.runtime_state_insert(run, super::RuntimeState::Resuming);

    // Recovery call must succeed (RQ-W0-20) without re-asserting the
    // NotResumable error. The run drives forward and finishes.
    let result = shard.handle_resume(run);
    assert!(
        result.is_ok(),
        "handle_resume must accept Resuming as recoverable; got {result:?}"
    );

    // Run should now be terminal (Completed via finish). The post-resume
    // drive reaches the Finish node and applies DriveFinished, which
    // removes the run from runtime_states.
    assert!(
        shard.runtime_state_get(run).is_none(),
        "resumed run must reach terminal state after recovery drive"
    );
    assert!(
        shard.terminal_runs_contains(run),
        "resumed run must be recorded in terminal_runs"
    );
    Ok(())
}
