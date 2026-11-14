use crate::admission::RunAdmission;
use vb_core::ids::RunId;
use vb_core::policy::RuntimePolicy;
use vb_storage::RunInputBinding;

fn test_workflow_digest() -> vb_core::ids::WorkflowDigest {
    vb_core::ids::WorkflowDigest::from_bytes([0x55; 32])
}

fn test_caps() -> vb_core::capability::CapabilitySet {
    vb_core::capability::CapabilitySet::from_grants(Box::new([
        vb_core::capability::Capability::new("test".into(), vb_core::ids::ActionId::new(1)),
    ]))
}

fn test_tenant() -> vb_core::ids::TenantId {
    vb_core::ids::TenantId::new(10)
}

fn test_idempotency_ids() -> Box<[vb_core::ids::ActionId]> {
    Box::new([vb_core::ids::ActionId::new(1)])
}

/// Roundtrip coverage proves structural serialization only, not runtime authority.
fn full_admission(run_id: u64) -> Result<RunAdmission, String> {
    let artifact = accepted_artifact_with_caps(Box::new([vb_core::capability::Capability::new(
        "test".into(),
        vb_core::ids::ActionId::new(1),
    )]))?;
    let decision = real_ir_compat_decision();
    let budget = requested_budget_77_18(200, 5);
    Ok(RunAdmission::with_idempotency_evidence(
        artifact.digest,
        RunId::new(run_id),
        test_caps(),
        RuntimePolicy::Strict,
        test_idempotency_ids(),
    )
    .with_policy_pin(vb_storage::RunPolicyPin::Pinned(artifact.policy_digest))
    .with_input_digest(RunInputBinding::Digest(test_workflow_digest()))
    .with_attached_budget(budget)
    .with_tenant(test_tenant())
    .with_ir_compat_decision(decision))
}

fn real_ir_compat_decision() -> vb_storage::compat::CompatDecision {
    let inputs =
        vb_storage::compat::artifact_ir_version_inputs(vb_storage::compat::CURRENT_IR_VERSION);
    vb_storage::compat::decide_boundary(
        inputs,
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION),
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION),
    )
}

#[test]
fn current_bound_admission_roundtrips_all_ten_fields() -> Result<(), String> {
    let admission = full_admission(5001)?;
    let bytes = postcard::to_allocvec(&admission).map_err(|e| format!("serialize failed: {e}"))?;
    let decoded: RunAdmission =
        postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;

    assert_eq!(decoded, admission);
    assert_eq!(
        decoded.input_digest(),
        RunInputBinding::Digest(test_workflow_digest())
    );
    assert_eq!(decoded.budget(), admission.budget());
    assert!(decoded.budget().is_some());
    assert_eq!(decoded.policy_pin(), admission.policy_pin());
    assert!(matches!(
        decoded.policy_pin(),
        vb_storage::RunPolicyPin::Pinned(_)
    ));
    assert_eq!(decoded.tenant(), Some(test_tenant()));
    assert_eq!(
        decoded.idempotency_attested(),
        test_idempotency_ids().as_ref()
    );
    assert_eq!(decoded.ir_compat_decision(), admission.ir_compat_decision());
    Ok(())
}

#[test]
fn current_bound_admission_with_budget_roundtrips() -> Result<(), String> {
    let admission = full_admission(5002)?;
    let bytes = postcard::to_allocvec(&admission).map_err(|e| format!("serialize failed: {e}"))?;
    let decoded: RunAdmission =
        postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;

    assert_eq!(decoded, admission);
    assert_eq!(decoded.budget(), admission.budget());
    assert!(decoded.budget().is_some());
    Ok(())
}

#[test]
fn obsolete_nine_position_admission_is_rejected() {
    let obsolete_bytes = [
        0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a,
        0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a,
        0x2a, 0x2a, 0x2a, 0x92, 0x21, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let result: Result<RunAdmission, _> = postcard::from_bytes(&obsolete_bytes);
    assert!(
        result.is_err(),
        "obsolete nine-position bytes must fail to decode"
    );
}

#[test]
fn truncated_current_admission_is_rejected() -> Result<(), String> {
    let admission = full_admission(5003)?;
    let bytes = postcard::to_allocvec(&admission).map_err(|e| format!("serialize failed: {e}"))?;
    let truncated = bytes
        .get(..bytes.len().saturating_sub(1))
        .ok_or_else(|| String::from("truncated record slice unavailable"))?;

    let decoded: Result<RunAdmission, _> = postcard::from_bytes(truncated);
    assert!(
        decoded.is_err(),
        "truncated ten-field record must fail to decode"
    );
    Ok(())
}

#[test]
fn streaming_decode_leaves_the_following_owner_bytes() -> Result<(), String> {
    let admission = full_admission(5004)?;
    let mut bytes =
        postcard::to_allocvec(&admission).map_err(|e| format!("serialize failed: {e}"))?;
    bytes.extend_from_slice(&[0xAA, 0xBB]);

    let (decoded, remaining) = postcard::take_from_bytes::<RunAdmission>(&bytes)
        .map_err(|e| format!("take_from_bytes failed: {e}"))?;
    assert_eq!(decoded, admission);
    assert_eq!(remaining, &[0xAA, 0xBB]);
    Ok(())
}
