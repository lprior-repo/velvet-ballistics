use super::*;
use crate::artifact::{compile_verified_artifact, verify_compiled_artifact};
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::workflow::node;
use vb_core::{CompiledWorkflow, SlotIdx, WorkflowDigest, WorkflowParts};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: retry_plan\nwhen: { manual: {} }\nsteps:\n  - id: make\n    set: { output: answer, value: \"42\" }\n  - id: done\n    finish: { result: answer }\n";

fn checked(
    wait: bool,
    timeout: u64,
    name: &str,
) -> Result<VerifiedCompilation, Box<dyn std::error::Error>> {
    let seed = compile_verified_artifact(SOURCE, &[])?;
    let mut parts = seed.workflow().to_parts();
    parts.name = name.into();
    let mut nodes = vec![parts.nodes.first().ok_or("missing seed node")?.clone()];
    let mut index = 1u16;
    if wait {
        nodes.push(node(
            StepIdx::new(index),
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
            None,
            Some(StepIdx::new(2)),
        ));
        index = index.checked_add(1).ok_or("fixture step overflow")?;
    }
    for action in 0..2u16 {
        let next = index.checked_add(1).ok_or("fixture step overflow")?;
        nodes.push(node(
            StepIdx::new(index),
            CompiledNodeKind::Do {
                action: ActionId::new(action),
                input: SlotIdx::new(0),
            },
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(next)),
        ));
        index = next;
    }
    nodes.push(node(
        StepIdx::new(index),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    ));
    parts.nodes = nodes.into_boxed_slice();
    parts.slot_count = 2;
    parts.step_names = Box::new([]);
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    let mut contracts = Vec::new();
    for action in 0..2u16 {
        contracts.push(ActionContract {
            id: ActionId::new(action),
            name: ActionName::new(format!("plan_{action}"))?,
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 64,
            max_output_bytes: 64,
            timeout_ms: timeout,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        });
    }
    Ok(verify_compiled_artifact(&workflow, &contracts)?)
}
fn policy(owner: u16, action: u16, total: u16) -> Result<RetryPolicyRecord, RetryPlanningError> {
    RetryPolicyRecord::new(StepIdx::new(owner), ActionId::new(action), total)
}
fn golden(context: &VerifiedCompilation) -> Vec<u8> {
    // Independent fixed wire words; digest values come from the checked provider.
    let mut bytes = vec![1, 255, 86, 66, 82, 84, 82, 89, 1, 0, 1, 0];
    bytes.extend_from_slice(&context.payload_digest().as_bytes());
    bytes.extend_from_slice(&context.action_contract_digest().as_bytes());
    bytes.extend_from_slice(&[2, 0, 1, 0, 3, 0]);
    bytes
}
fn replace(bytes: &mut [u8], offset: usize, value: u16) -> TestResult {
    let end = offset.checked_add(2).ok_or("fixture offset overflow")?;
    bytes
        .get_mut(offset..end)
        .ok_or("fixture missing word")?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}
fn refusal(context: &VerifiedCompilation, bytes: &[u8], error: RetryPlanningError) {
    assert_eq!(decode_retry_planning(context, bytes).err(), Some(error));
}

#[test]
fn independent_wire_golden_and_legacy_decoder_separation() -> TestResult {
    let context = checked(false, 1000, "retry_plan")?;
    let bytes = golden(&context);
    assert_eq!(bytes.len(), 82);
    assert_eq!(encode_retry_planning(&context, &[policy(2, 1, 3)?])?, bytes);
    let view = decode_retry_planning(&context, &bytes)?;
    assert!(std::ptr::eq(view.compilation(), &context));
    assert_eq!(view.policies(), &[policy(2, 1, 3)?]);
    let entry = view.policies().first().ok_or("missing policy")?;
    assert_eq!(
        (entry.owner(), entry.action(), entry.total_attempts()),
        (StepIdx::new(2), ActionId::new(1), 3)
    );
    let legacy: WorkflowParts = postcard::from_bytes(context.payload())?;
    assert_eq!(
        CompiledWorkflow::try_from_parts(legacy)?.to_parts(),
        context.workflow().to_parts()
    );
    assert!(postcard::from_bytes::<WorkflowParts>(&bytes).is_err());
    refusal(&context, context.payload(), RetryPlanningError::Magic);
    Ok(())
}

#[test]
fn strict_header_and_representable_count_bounds() -> TestResult {
    let context = checked(false, 1000, "retry_plan")?;
    let valid = golden(&context);
    assert!(decode_retry_planning(&context, &valid).is_ok());
    for length in 0..valid.len() {
        let bytes = valid.get(..length).ok_or("missing prefix")?;
        assert!(decode_retry_planning(&context, bytes).is_err());
    }
    let mut bytes = valid.clone();
    *bytes.first_mut().ok_or("missing magic")? = 2;
    refusal(&context, &bytes, RetryPlanningError::Magic);
    for version in [0, 2, u16::MAX] {
        let mut bytes = valid.clone();
        replace(&mut bytes, 8, version)?;
        refusal(&context, &bytes, RetryPlanningError::Generation(version));
    }
    for count in [0, 5, u16::MAX] {
        let mut bytes = valid.clone();
        replace(&mut bytes, 10, count)?;
        let length = usize::from(count)
            .checked_mul(6)
            .and_then(|n| n.checked_add(76))
            .ok_or("fixture length overflow")?;
        bytes.resize(length, 0);
        refusal(&context, &bytes, RetryPlanningError::Count);
    }
    let mut bytes = valid.clone();
    bytes.push(0);
    refusal(&context, &bytes, RetryPlanningError::Length);
    let mut bytes = valid;
    replace(&mut bytes, 10, 2)?;
    refusal(&context, &bytes, RetryPlanningError::Length);
    assert_eq!(
        encode_retry_planning(&context, &[]),
        Err(RetryPlanningError::Count)
    );
    Ok(())
}

#[test]
fn bindings_are_independent_and_policy_edits_are_not_authentication() -> TestResult {
    let context = checked(false, 1000, "retry_plan")?;
    let payload_before = context.payload().to_vec();
    let contracts_before = context.contracts().to_vec();
    let bytes = golden(&context);
    let other_payload = checked(false, 1000, "different_name")?;
    assert_ne!(context.payload_digest(), other_payload.payload_digest());
    assert_eq!(
        context.action_contract_digest(),
        other_payload.action_contract_digest()
    );
    refusal(&other_payload, &bytes, RetryPlanningError::Binding);
    let other_contracts = checked(false, 1001, "retry_plan")?;
    assert_eq!(context.payload_digest(), other_contracts.payload_digest());
    assert_ne!(
        context.action_contract_digest(),
        other_contracts.action_contract_digest()
    );
    refusal(&other_contracts, &bytes, RetryPlanningError::Binding);
    for offset in [12, 44] {
        let mut changed = bytes.clone();
        let byte = changed.get_mut(offset).ok_or("missing digest")?;
        *byte ^= 1;
        refusal(&context, &changed, RetryPlanningError::Binding);
    }
    let mut changed = bytes;
    replace(&mut changed, 80, 2)?;
    assert_eq!(
        decode_retry_planning(&context, &changed)?.policies(),
        &[policy(2, 1, 2)?]
    );
    assert_eq!(context.payload(), payload_before);
    assert_eq!(context.contracts(), contracts_before);
    Ok(())
}

#[test]
fn policy_bounds_and_owner_action_identity_are_enforced() -> TestResult {
    let context = checked(false, 1000, "retry_plan")?;
    for total in [1, 10] {
        let record = policy(2, 1, total)?;
        assert_eq!(
            decode_retry_planning(&context, &encode_retry_planning(&context, &[record])?)?
                .policies(),
            &[record]
        );
    }
    for total in [0, 11, u16::MAX] {
        assert_eq!(
            policy(2, 1, total),
            Err(RetryPlanningError::TotalAttempts(total))
        );
        let mut bytes = golden(&context);
        replace(&mut bytes, 80, total)?;
        refusal(&context, &bytes, RetryPlanningError::TotalAttempts(total));
    }
    for (owner, action) in [(0, 1), (4, 1), (u16::MAX, 1), (2, 0), (2, u16::MAX)] {
        let mut bytes = golden(&context);
        replace(&mut bytes, 76, owner)?;
        replace(&mut bytes, 78, action)?;
        refusal(
            &context,
            &bytes,
            RetryPlanningError::Owner(StepIdx::new(owner)),
        );
        assert_eq!(
            encode_retry_planning(&context, &[policy(owner, action, 3)?]),
            Err(RetryPlanningError::Owner(StepIdx::new(owner)))
        );
    }
    let ordered = [policy(1, 0, 1)?, policy(2, 1, 10)?];
    let valid = encode_retry_planning(&context, &ordered)?;
    assert_eq!(
        decode_retry_planning(&context, &valid)?.policies(),
        &ordered
    );
    for records in [
        [policy(2, 1, 10)?, policy(1, 0, 1)?],
        [policy(1, 0, 1)?, policy(1, 0, 1)?],
    ] {
        assert_eq!(
            encode_retry_planning(&context, &records),
            Err(RetryPlanningError::Owners)
        );
    }
    let mut duplicate = valid.clone();
    replace(&mut duplicate, 82, 1)?;
    replace(&mut duplicate, 84, 0)?;
    refusal(&context, &duplicate, RetryPlanningError::Owners);
    let mut reversed = valid;
    replace(&mut reversed, 76, 2)?;
    replace(&mut reversed, 78, 1)?;
    replace(&mut reversed, 82, 1)?;
    replace(&mut reversed, 84, 0)?;
    refusal(&context, &reversed, RetryPlanningError::Owners);
    Ok(())
}

#[test]
fn genuinely_checked_wait_profile_is_refused_before_planning() -> TestResult {
    let context = checked(true, 1000, "retry_plan")?;
    let record = policy(3, 1, 3)?;
    assert_eq!(
        encode_retry_planning(&context, &[record]),
        Err(RetryPlanningError::Profile)
    );
    let mut bytes = golden(&context);
    replace(&mut bytes, 76, 3)?;
    refusal(&context, &bytes, RetryPlanningError::Profile);
    Ok(())
}

#[test]
fn bounded_single_byte_mutations_either_refuse_or_reencode_exactly() -> TestResult {
    let context = checked(false, 1000, "retry_plan")?;
    let golden = golden(&context);
    for offset in 0..golden.len() {
        for value in 0..=u8::MAX {
            let mut bytes = golden.clone();
            *bytes.get_mut(offset).ok_or("missing mutation byte")? = value;
            if let Ok(view) = decode_retry_planning(&context, &bytes) {
                assert_eq!(encode_retry_planning(&context, view.policies())?, bytes);
            }
        }
    }
    Ok(())
}

#[test]
fn checked_historical_retry_check_profile_is_refused_by_both_planning_paths() -> TestResult {
    let ordinary = checked(false, 1000, "retry_plan")?;
    let record = policy(2, 1, 3)?;
    assert!(encode_retry_planning(&ordinary, &[record]).is_ok());
    assert!(decode_retry_planning(&ordinary, &golden(&ordinary)).is_ok());
    // Base426 ordinal30, then policy_slot/body/exhausted declaration order.
    // Historical fixed bytes, not produced by the current variant encoder.
    let historical_kind: CompiledNodeKind = postcard::from_bytes(&[30, 0, 2, 2])?;
    assert_eq!(
        historical_kind,
        CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(0),
            body: StepIdx::new(2),
            exhausted: StepIdx::new(2),
        }
    );
    let mut parts = ordinary.workflow().to_parts();
    *parts.nodes.get_mut(1).ok_or("missing retry position")? = node(
        StepIdx::new(1),
        historical_kind,
        None,
        Some(StepIdx::new(2)),
    );
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    // Replacing the action-0 Do leaves that contract unreferenced, and the
    // compiler rejects orphan contracts; keep only the action the legacy
    // RetryCheck body still reaches.
    let legacy_contracts: Vec<ActionContract> = ordinary
        .contracts()
        .iter()
        .filter(|contract| contract.id == ActionId::new(1))
        .cloned()
        .collect();
    let legacy = verify_compiled_artifact(&workflow, &legacy_contracts)?;
    assert!(
        matches!(legacy.workflow().node(record.owner()).map(|node| &node.kind),
        Some(CompiledNodeKind::Do { action, .. }) if *action == record.action())
    );
    assert_eq!(
        encode_retry_planning(&legacy, &[record]),
        Err(RetryPlanningError::Profile)
    );
    // Independent golden words retain a real Do owner and both checked bindings.
    refusal(&legacy, &golden(&legacy), RetryPlanningError::Profile);
    Ok(())
}
