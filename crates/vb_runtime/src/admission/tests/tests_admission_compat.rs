// RunAdmission binary compatibility tests: nine-field historical records,
// bound admissions with input digest, and carrier error handling.

use crate::admission::RunAdmission;
use vb_storage::RunInputBinding;

use vb_core::ids::RunId;
use vb_core::policy::RuntimePolicy;

fn test_artifact_digest() -> vb_core::ids::WorkflowDigest {
    vb_core::ids::WorkflowDigest::from_bytes([0x2a; 32])
}

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

/// Independent old nine-field DTO matching main's exact historical layout order.
/// This is the authoritative byte oracle for legacy compatibility. Its Serde is
/// derived independently from RunAdmission, so it cannot silently follow the
/// new codec's decisions.
#[derive(serde::Serialize, serde::Deserialize)]
struct OldNineFieldDto {
    artifact_digest: vb_core::ids::WorkflowDigest,
    run_id: RunId,
    granted_capabilities: vb_core::capability::CapabilitySet,
    policy: RuntimePolicy,
    policy_pin: vb_storage::RunPolicyPin,
    budget: Option<vb_core::budget::AggregateResourceBudget>,
    tenant: Option<vb_core::ids::TenantId>,
    idempotency_attested: Box<[vb_core::ids::ActionId]>,
    ir_compat_decision: Option<Box<vb_storage::compat::CompatDecision>>,
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

fn make_old_dto(
    budget: Option<vb_core::budget::AggregateResourceBudget>,
    decision: Option<Box<vb_storage::compat::CompatDecision>>,
) -> OldNineFieldDto {
    OldNineFieldDto {
        artifact_digest: test_artifact_digest(),
        run_id: RunId::new(4242),
        granted_capabilities: test_caps(),
        policy: RuntimePolicy::Strict,
        policy_pin: vb_storage::RunPolicyPin::Unverified,
        budget,
        tenant: Some(test_tenant()),
        idempotency_attested: test_idempotency_ids(),
        ir_compat_decision: decision,
    }
}

fn make_new_admission(
    budget: Option<vb_core::budget::AggregateResourceBudget>,
    decision: Option<vb_storage::compat::CompatDecision>,
) -> RunAdmission {
    let base = RunAdmission::with_idempotency_evidence(
        test_artifact_digest(),
        RunId::new(4242),
        test_caps(),
        RuntimePolicy::Strict,
        test_idempotency_ids(),
    );
    let admission = match budget {
        Some(b) => base.with_attached_budget(b),
        None => base,
    };
    let admission = admission.with_tenant(test_tenant());
    match decision {
        Some(d) => admission.with_ir_compat_decision(d),
        None => admission,
    }
}

/// Compare new RunAdmission bytes against the old DTO byte-for-byte.
/// For LegacyUnbound with None budget, they must be identical.
fn legacy_none_budget_bytes_match_old() -> Result<(), String> {
    let decision = real_ir_compat_decision();
    let old = make_old_dto(None, Some(Box::new(decision)));
    let old_bytes =
        postcard::to_allocvec(&old).map_err(|e| format!("old DTO serialize failed: {e}"))?;

    let new = make_new_admission(None, Some(decision));

    let new_bytes = postcard::to_allocvec(&new)
        .map_err(|e| format!("new RunAdmission serialize failed: {e}"))?;

    assert_eq!(
        old_bytes, new_bytes,
        "new LegacyUnbound None-budget bytes must equal old DTO bytes"
    );

    // Decode old bytes through new codec
    let decoded: RunAdmission =
        postcard::from_bytes(&old_bytes).map_err(|e| format!("decode old bytes failed: {e}"))?;

    assert_eq!(
        decoded, new,
        "decoded old bytes must equal new RunAdmission value"
    );

    // Re-encode decoded bytes
    let re_encoded =
        postcard::to_allocvec(&decoded).map_err(|e| format!("re-encode failed: {e}"))?;

    assert_eq!(
        old_bytes, re_encoded,
        "re-encoded decoded bytes must equal original old DTO bytes"
    );

    Ok(())
}

/// Compare new RunAdmission bytes against the old DTO byte-for-byte for Some budget.
fn legacy_some_budget_bytes_match_old() -> Result<(), String> {
    let decision = real_ir_compat_decision();
    let budget = requested_budget_77_18(100, 0);
    let old = make_old_dto(Some(budget), Some(Box::new(decision)));
    let old_bytes =
        postcard::to_allocvec(&old).map_err(|e| format!("old DTO serialize failed: {e}"))?;

    let new = make_new_admission(Some(budget), Some(decision));

    let new_bytes = postcard::to_allocvec(&new)
        .map_err(|e| format!("new RunAdmission serialize failed: {e}"))?;

    assert_eq!(
        old_bytes, new_bytes,
        "new LegacyUnbound Some-budget bytes must equal old DTO bytes"
    );

    // Decode old bytes through new codec
    let decoded: RunAdmission =
        postcard::from_bytes(&old_bytes).map_err(|e| format!("decode old bytes failed: {e}"))?;

    assert_eq!(
        decoded, new,
        "decoded old bytes must equal new RunAdmission value"
    );

    // Re-encode decoded bytes
    let re_encoded =
        postcard::to_allocvec(&decoded).map_err(|e| format!("re-encode failed: {e}"))?;

    assert_eq!(
        old_bytes, re_encoded,
        "re-encoded decoded bytes must equal original old DTO bytes"
    );

    Ok(())
}

#[test]
fn legacy_unbound_none_budget_bytes_identical_to_old() -> Result<(), String> {
    legacy_none_budget_bytes_match_old()
}

#[test]
fn legacy_unbound_some_budget_bytes_identical_to_old() -> Result<(), String> {
    legacy_some_budget_bytes_match_old()
}

#[test]
fn legacy_nine_field_bytes_decode_as_legacy_unbound() -> Result<(), String> {
    // Original legacy nine-field record from the baseline fixture.
    let legacy_bytes = [
        0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a,
        0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a,
        0x2a, 0x2a, 0x92, 0x21, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let decoded: RunAdmission = postcard::from_bytes(&legacy_bytes)
        .map_err(|e| format!("failed to deserialize legacy bytes: {e}"))?;

    assert_eq!(
        decoded.input_digest(),
        RunInputBinding::LegacyUnbound,
        "legacy bytes should decode as LegacyUnbound"
    );
    assert_eq!(
        decoded.run_id(),
        RunId::new(4242),
        "run id should be preserved"
    );

    // Re-encoding must reproduce the original bytes exactly
    let re_encoded =
        postcard::to_allocvec(&decoded).map_err(|e| format!("failed to re-serialize: {e}"))?;
    assert_eq!(
        re_encoded, legacy_bytes,
        "re-encoded legacy bytes differ from original"
    );

    Ok(())
}

#[test]
fn bound_digest_none_budget_roundtrips_full_context() -> Result<(), String> {
    let decision = real_ir_compat_decision();
    let admission = RunAdmission::with_idempotency_evidence(
        test_artifact_digest(),
        RunId::new(5001),
        test_caps(),
        RuntimePolicy::Strict,
        test_idempotency_ids(),
    )
    .with_input_digest(RunInputBinding::Digest(test_workflow_digest()))
    .with_tenant(test_tenant())
    .with_ir_compat_decision(decision);

    let bytes = postcard::to_allocvec(&admission).map_err(|e| format!("serialize failed: {e}"))?;

    let decoded: RunAdmission =
        postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;

    assert_eq!(
        decoded, admission,
        "full bound None-budget admission must roundtrip"
    );
    assert_eq!(
        decoded.input_digest(),
        RunInputBinding::Digest(test_workflow_digest()),
        "input digest must be preserved"
    );
    assert_eq!(decoded.budget(), None, "budget must remain None");
    assert_eq!(
        decoded.tenant(),
        Some(test_tenant()),
        "tenant must be preserved"
    );
    assert_eq!(
        decoded.ir_compat_decision().is_some(),
        true,
        "IR compat decision must be preserved"
    );

    Ok(())
}

#[test]
fn bound_digest_some_budget_roundtrips_full_context() -> Result<(), String> {
    let decision = real_ir_compat_decision();
    let budget = requested_budget_77_18(200, 5);
    let admission = RunAdmission::with_idempotency_evidence(
        test_artifact_digest(),
        RunId::new(5002),
        test_caps(),
        RuntimePolicy::Strict,
        test_idempotency_ids(),
    )
    .with_input_digest(RunInputBinding::Digest(test_workflow_digest()))
    .with_attached_budget(budget)
    .with_tenant(test_tenant())
    .with_ir_compat_decision(decision);

    let bytes = postcard::to_allocvec(&admission).map_err(|e| format!("serialize failed: {e}"))?;

    let decoded: RunAdmission =
        postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;

    assert_eq!(
        decoded, admission,
        "full bound Some-budget admission must roundtrip"
    );
    assert_eq!(
        decoded.input_digest(),
        RunInputBinding::Digest(test_workflow_digest()),
        "input digest must be preserved"
    );
    assert_eq!(decoded.budget(), Some(budget), "budget must be preserved");

    Ok(())
}

#[test]
fn invalid_carrier_discriminant_three_errors() -> Result<(), String> {
    // Build a valid admission, decode it to find the carrier position,
    // then craft a record with invalid carrier discriminant 3.
    let admission = RunAdmission::with_idempotency_evidence(
        test_artifact_digest(),
        RunId::new(102),
        test_caps(),
        RuntimePolicy::Strict,
        test_idempotency_ids(),
    );
    let good_bytes =
        postcard::to_allocvec(&admission).map_err(|e| format!("failed to serialize: {e}"))?;

    // Decode the five prefix fields to find the carrier position.
    let mut remaining = good_bytes.as_slice();
    let (_ad_digest, rem1) = postcard::take_from_bytes::<vb_core::ids::WorkflowDigest>(remaining)
        .map_err(|e| format!("parse artifact_digest: {e}"))?;
    remaining = rem1;

    let (_run_id, rem2) =
        postcard::take_from_bytes::<RunId>(remaining).map_err(|e| format!("parse run_id: {e}"))?;
    remaining = rem2;

    let (_caps, rem3) = postcard::take_from_bytes::<vb_core::capability::CapabilitySet>(remaining)
        .map_err(|e| format!("parse capabilities: {e}"))?;
    remaining = rem3;

    let (_policy, rem4) = postcard::take_from_bytes::<RuntimePolicy>(remaining)
        .map_err(|e| format!("parse policy: {e}"))?;
    remaining = rem4;

    let (_pin, carrier_and_rest) = postcard::take_from_bytes::<vb_storage::RunPolicyPin>(remaining)
        .map_err(|e| format!("parse policy_pin: {e}"))?;

    // Carrier position is where carrier_and_rest begins in the original bytes.
    // Compute carrier position with checked arithmetic.
    let carrier_pos = good_bytes
        .len()
        .checked_sub(carrier_and_rest.len())
        .ok_or_else(|| String::from("carrier position arithmetic overflow"))?;

    // Carrier discriminant is the first byte of what remains.
    let mut bad_bytes = good_bytes.clone();
    let slot = bad_bytes
        .get_mut(carrier_pos)
        .ok_or_else(|| String::from("carrier position out of bounds"))?;
    *slot = 0x03;

    let result: Result<RunAdmission, _> = postcard::from_bytes(&bad_bytes);
    assert!(
        result.is_err(),
        "invalid carrier discriminant should error, not decode as legacy"
    );

    Ok(())
}

#[test]
fn truncated_carrier_payload_errors() -> Result<(), String> {
    // Write a bound carrier (discriminant 2) but truncate after the
    // carrier discriminant, omitting the digest and budget payload.
    let admission = RunAdmission::with_idempotency_evidence(
        test_artifact_digest(),
        RunId::new(103),
        test_caps(),
        RuntimePolicy::Strict,
        test_idempotency_ids(),
    )
    .with_input_digest(RunInputBinding::Digest(test_workflow_digest()));

    let good_bytes =
        postcard::to_allocvec(&admission).map_err(|e| format!("failed to serialize: {e}"))?;

    // Decode the five prefix fields to find the carrier position.
    let mut remaining = good_bytes.as_slice();
    let (_ad_digest, rem1) = postcard::take_from_bytes::<vb_core::ids::WorkflowDigest>(remaining)
        .map_err(|e| format!("parse artifact_digest: {e}"))?;
    remaining = rem1;

    let (_run_id, rem2) =
        postcard::take_from_bytes::<RunId>(remaining).map_err(|e| format!("parse run_id: {e}"))?;
    remaining = rem2;

    let (_caps, rem3) = postcard::take_from_bytes::<vb_core::capability::CapabilitySet>(remaining)
        .map_err(|e| format!("parse capabilities: {e}"))?;
    remaining = rem3;

    let (_policy, rem4) = postcard::take_from_bytes::<RuntimePolicy>(remaining)
        .map_err(|e| format!("parse policy: {e}"))?;
    remaining = rem4;

    let (_pin, carrier_and_rest) = postcard::take_from_bytes::<vb_storage::RunPolicyPin>(remaining)
        .map_err(|e| format!("parse policy_pin: {e}"))?;

    // Carrier position is where carrier_and_rest begins in the original bytes.
    let carrier_pos = good_bytes
        .len()
        .checked_sub(carrier_and_rest.len())
        .ok_or_else(|| String::from("carrier position arithmetic overflow"))?;
    // Compute truncation length with checked arithmetic.
    let trunc_len = carrier_pos
        .checked_add(1)
        .ok_or_else(|| String::from("truncation length arithmetic overflow"))?;
    // Truncate to just past the carrier discriminant byte.
    let truncated = good_bytes
        .get(..trunc_len)
        .ok_or_else(|| String::from("truncation slice out of bounds"))?
        .to_vec();

    let result: Result<RunAdmission, _> = postcard::from_bytes(&truncated);
    assert!(result.is_err(), "truncated carrier payload should error");

    Ok(())
}

#[test]
fn take_from_bytes_consumes_exact_record_length() -> Result<(), String> {
    let admission = RunAdmission::with_idempotency_evidence(
        test_artifact_digest(),
        RunId::new(100),
        test_caps(),
        RuntimePolicy::Strict,
        test_idempotency_ids(),
    );
    let bytes = postcard::to_allocvec(&admission).map_err(|e| format!("serialize failed: {e}"))?;

    let (decoded, remaining) = postcard::take_from_bytes::<RunAdmission>(&bytes)
        .map_err(|e| format!("take_from_bytes failed: {e}"))?;

    assert_eq!(
        remaining.len(),
        0,
        "take_from_bytes must consume the entire record"
    );
    assert_eq!(
        decoded, admission,
        "take_from_bytes must decode the full record"
    );

    Ok(())
}

#[test]
fn from_bytes_permits_trailing_bytes() -> Result<(), String> {
    let admission = RunAdmission::with_idempotency_evidence(
        test_artifact_digest(),
        RunId::new(101),
        test_caps(),
        RuntimePolicy::Strict,
        test_idempotency_ids(),
    );
    let bytes = postcard::to_allocvec(&admission).map_err(|e| format!("serialize failed: {e}"))?;

    // Add trailing bytes
    let mut with_trailing = bytes.clone();
    with_trailing.push(0xAA);
    with_trailing.push(0xBB);

    let decoded: RunAdmission = postcard::from_bytes(&with_trailing)
        .map_err(|e| format!("from_bytes with trailing failed: {e}"))?;

    assert_eq!(
        decoded, admission,
        "from_bytes must decode correctly despite trailing bytes"
    );

    Ok(())
}
