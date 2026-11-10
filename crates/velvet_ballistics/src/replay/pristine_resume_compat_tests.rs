use super::select_pristine_resume;
use vb_core::{RunId, RuntimePolicy, WorkflowDigest, errors::CoreError};
use vb_storage::{EventSeq, JournalEvent, RunInputBinding, RunPolicyPin, compat};

fn prefix(run: RunId) -> Vec<JournalEvent> {
    vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: WorkflowDigest::from_bytes([1; 32]),
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: WorkflowDigest::from_bytes([2; 32]),
            input_digest: RunInputBinding::Digest(WorkflowDigest::from_bytes([3; 32])),
            policy_pin: RunPolicyPin::Pinned(WorkflowDigest::from_bytes([4; 32])),
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
        },
    ]
}

#[test]
fn pristine_resume_accepts_compatibility_tail_and_preserves_exact_rejections() -> Result<(), String>
{
    let run = RunId::new(73);
    let original = CoreError::LifecycleDuplicateRequest {
        code: CoreError::LIFECYCLE_DUPLICATE_REQUEST_CODE,
        context: "compatibility-tail rejection sentinel".into(),
        timestamp: chrono::DateTime::from_timestamp(123, 456).ok_or("timestamp")?,
        bead_id: Some(run),
        command: Some("resume"),
    };
    let decision = compat::decide_boundary(
        compat::artifact_ir_version_inputs(compat::CURRENT_IR_VERSION),
        [1; 32],
        [1; 32],
    );
    let event = JournalEvent::CompatibilityDecisionEvent {
        run,
        seq: EventSeq::new(2),
        decision,
    };
    let mut valid = prefix(run);
    valid.push(event.clone());
    assert_eq!(
        select_pristine_resume(run, original.clone(), Ok(valid.clone())),
        Ok(())
    );
    for field in 0..5 {
        let mut invalid = prefix(run);
        let mut event = event.clone();
        let JournalEvent::CompatibilityDecisionEvent {
            run: owner,
            seq,
            decision,
        } = &mut event
        else {
            return Err("compatibility fixture".into());
        };
        match field {
            0 => *owner = RunId::new(74),
            1 => *seq = EventSeq::new(3),
            2 => decision.boundary = compat::CompatBoundary::ArtifactEnvelope,
            3 => decision.class = compat::CompatibilityClass::Rejected,
            4 => invalid.push(event.clone()),
            _ => return Err("bounded case".into()),
        }
        invalid.push(event);
        assert_eq!(
            select_pristine_resume(run, original.clone(), Ok(invalid)),
            Err(original.clone()),
            "field {field}"
        );
    }
    Ok(())
}
