//! RunAdmission binary compatibility tests: nine-field historical records,
//! bound admissions with input digest, carrier error handling, exact
//! consumption, and human JSON roundtrip.

use crate::admission::RunAdmission;
use vb_storage::RunInputBinding;
use vb_storage::RunPolicyPin;

use vb_core::budget::AggregateResourceBudget;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, TenantId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;

fn test_artifact_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x2a; 32])
}

fn test_workflow_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x55; 32])
}

fn test_caps() -> CapabilitySet {
    let mut caps = CapabilitySet::new();
    caps.grant(Capability::new("file", "read"));
    caps
}

fn test_action_id() -> ActionId {
    ActionId::new(42)
}

fn test_tenant_id() -> TenantId {
    TenantId::new("test-tenant".to_string())
}

fn test_budget() -> AggregateResourceBudget {
    AggregateResourceBudget::new(1000, 100, 1000, 1000, 60000).unwrap()
}

/// Serialize using the current codec, decode as historical nine-field
/// record, and verify byte-for-byte compatibility.
fn nine_field_roundtrip(
    input_digest: RunInputBinding,
    budget: Option<AggregateResourceBudget>,
    expected_input_digest: RunInputBinding,
) -> Result<(), String> {
    let admission = RunAdmission {
        artifact_digest: test_artifact_digest(),
        run_id: RunId::new(4242),
        granted_capabilities: test_caps(),
        policy: RuntimePolicy::Strict,
        policy_pin: RunPolicyPin::Unverified,
        input_digest,
        budget,
        tenant: Some(test_tenant_id()),
        idempotency_attested: Box::new([test_action_id()]),
        ir_compat_decision: None,
    };

    let bytes = postcard::to_allocvec(&admission)
        .map_err(|e| format!("failed to serialize: {e}"))?;

    let decoded: RunAdmission = postcard::from_bytes(&bytes)
        .map_err(|e| format!("failed to deserialize: {e}"))?;

    assert_eq!(
        decoded.artifact_digest(),
        admission.artifact_digest(),
        "artifact digest mismatch"
    );
    assert_eq!(decoded.run_id(), admission.run_id(), "run id mismatch");
    assert_eq!(
        decoded.granted_capabilities(),
        admission.granted_capabilities(),
        "granted capabilities mismatch"
    );
    assert_eq!(decoded.policy(), admission.policy(), "policy mismatch");
    assert_eq!(
        decoded.policy_pin(),
        admission.policy_pin(),
        "policy pin mismatch"
    );
    assert_eq!(
        decoded.input_digest(),
        &expected_input_digest,
        "input digest mismatch"
    );
    assert_eq!(
        decoded.budget(),
        admission.budget(),
        "budget mismatch"
    );
    assert_eq!(
        decoded.tenant(),
        admission.tenant(),
        "tenant mismatch"
    );
    assert_eq!(
        decoded.idempotency_attested(),
        admission.idempotency_attested(),
        "idempotency attested mismatch"
    );
    assert_eq!(
        decoded.ir_compat_decision(),
        admission.ir_compat_decision(),
        "ir compat decision mismatch"
    );

    // Re-encode must produce identical bytes
    let re_encoded = postcard::to_allocvec(&decoded)
        .map_err(|e| format!("failed to re-serialize: {e}"))?;
    assert_eq!(
        re_encoded, bytes,
        "re-encoded bytes differ from original"
    );

    Ok(())
}

#[test]
fn legacy_unbound_no_budget_decodes_and_reencodes_identically() -> Result<(), String> {
    nine_field_roundtrip(
        RunInputBinding::LegacyUnbound,
        None,
        RunInputBinding::LegacyUnbound,
    )
}

#[test]
fn legacy_unbound_with_budget_decodes_and_reencodes_identically() -> Result<(), String> {
    nine_field_roundtrip(
        RunInputBinding::LegacyUnbound,
        Some(test_budget()),
        RunInputBinding::LegacyUnbound,
    )
}

#[test]
fn bound_digest_without_budget_roundtrips() -> Result<(), String> {
    nine_field_roundtrip(
        RunInputBinding::Digest(test_workflow_digest()),
        None,
        RunInputBinding::Digest(test_workflow_digest()),
    )
}

#[test]
fn bound_digest_with_budget_roundtrips() -> Result<(), String> {
    nine_field_roundtrip(
        RunInputBinding::Digest(test_workflow_digest()),
        Some(test_budget()),
        RunInputBinding::Digest(test_workflow_digest()),
    )
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
        &RunInputBinding::LegacyUnbound,
        "legacy bytes should decode as LegacyUnbound"
    );
    assert_eq!(decoded.run_id(), RunId::new(4242), "run id should be preserved");

    // Re-encoding must reproduce the original bytes exactly
    let re_encoded = postcard::to_allocvec(&decoded)
        .map_err(|e| format!("failed to re-serialize: {e}"))?;
    assert_eq!(
        re_encoded, legacy_bytes,
        "re-encoded legacy bytes differ from original"
    );

    Ok(())
}

#[test]
fn invalid_carrier_discriminant_three_errors() -> Result<(), String> {
    // Construct a carrier with discriminant 3, which is not a valid variant.
    // This should produce a decode error, not a legacy fallback.
    let mut bytes = [0u8; 40];
    // Fill with enough bytes for artifact_digest (32) + discriminant + padding
    bytes[0..32].copy_from_slice(&[0x2a; 32]);
    bytes[32] = 0x03; // Invalid carrier discriminant

    let result: Result<RunAdmission, _> = postcard::from_bytes(&bytes);
    assert!(
        result.is_err(),
        "invalid carrier discriminant should error, not decode as legacy"
    );

    Ok(())
}

#[test]
fn truncated_carrier_payload_errors() -> Result<(), String> {
    // Carrier discriminant 2 (Bound) requires a digest payload.
    // Provide only the discriminant without the digest.
    let bytes = [
        0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a,
        0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a,
        0x2a, 0x2a, 0x02, // Discriminant for Carrier::Bound, but no digest follows
    ];

    let result: Result<RunAdmission, _> = postcard::from_bytes(&bytes);
    assert!(
        result.is_err(),
        "truncated carrier payload should error"
    );

    Ok(())
}

#[test]
fn take_from_bytes_consumes_exact_record_length() -> Result<(), String> {
    let admission = RunAdmission::new(
        test_artifact_digest(),
        RunId::new(100),
        test_caps(),
        RuntimePolicy::Strict,
    );

    let mut bytes = postcard::to_allocvec(&admission)
        .map_err(|e| format!("failed to serialize: {e}"))?;
    let original_len = bytes.len();

    // Append sentinel suffix that must not be consumed
    bytes.push(0xFF);
    bytes.push(0xFE);

    let (decoded, consumed) = postcard::take_from_bytes::<RunAdmission>(&bytes)
        .map_err(|e| format!("take_from_bytes failed: {e}"))?;

    assert_eq!(
        consumed, original_len,
        "take_from_bytes consumed wrong number of bytes"
    );
    assert_eq!(
        decoded.artifact_digest(),
        admission.artifact_digest(),
        "decoded admission mismatch"
    );

    Ok(())
}

#[test]
fn from_bytes_permits_trailing_bytes() -> Result<(), String> {
    let admission = RunAdmission::new(
        test_artifact_digest(),
        RunId::new(101),
        test_caps(),
        RuntimePolicy::Strict,
    );

    let mut bytes = postcard::to_allocvec(&admission)
        .map_err(|e| format!("failed to serialize: {e}"))?;
    // Append trailing bytes that from_bytes should permit
    bytes.push(0xFF);
    bytes.push(0xFE);

    let decoded: RunAdmission = postcard::from_bytes(&bytes)
        .map_err(|e| format!("from_bytes failed: {e}"))?;

    assert_eq!(
        decoded.artifact_digest(),
        admission.artifact_digest(),
        "decoded admission mismatch with trailing bytes"
    );

    Ok(())
}

#[test]
fn human_legacy_map_missing_input_digest_decodes_as_unbound() -> Result<(), String> {
    // A historical named-map JSON with nine fields and no input_digest.
    let json = r#"{
        "artifact_digest": [42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42],
        "run_id": [4242, 0, 0, 0, 0, 0, 0, 0],
        "granted_capabilities": {"granted": [{"domain":"file","resource":"read"}]},
        "policy": "strict",
        "policy_pin": {"variant":"Unverified"},
        "budget": null,
        "tenant": null,
        "idempotency_attested": [],
        "ir_compat_decision": null
    }"#;

    let decoded: RunAdmission = serde_json::from_str(json)
        .map_err(|e| format!("failed to deserialize legacy JSON: {e}"))?;

    assert_eq!(
        decoded.input_digest(),
        &RunInputBinding::LegacyUnbound,
        "missing input_digest should default to LegacyUnbound"
    );

    Ok(())
}

#[test]
fn human_missing_required_idempotency_field_fails() -> Result<(), String> {
    let json = r#"{
        "artifact_digest": [42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42],
        "run_id": [4242, 0, 0, 0, 0, 0, 0, 0],
        "granted_capabilities": {"granted": [{"domain":"file","resource":"read"}]},
        "policy": "strict",
        "policy_pin": {"variant":"Unverified"},
        "budget": null,
        "tenant": null,
        "ir_compat_decision": null
    }"#;

    let result: Result<RunAdmission, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "missing required idempotency_attested field should fail to decode"
    );

    Ok(())
}

#[test]
fn human_missing_optional_tenant_and_ir_context_decodes() -> Result<(), String> {
    let json = r#"{
        "artifact_digest": [42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42,42],
        "run_id": [4242, 0, 0, 0, 0, 0, 0, 0],
        "granted_capabilities": {"granted": [{"domain":"file","resource":"read"}]},
        "policy": "strict",
        "policy_pin": {"variant":"Unverified"},
        "budget": null,
        "idempotency_attested": []
    }"#;

    let decoded: RunAdmission = serde_json::from_str(json)
        .map_err(|e| format!("failed to deserialize: {e}"))?;

    assert_eq!(decoded.tenant(), None, "missing tenant should be None");
    assert_eq!(
        decoded.ir_compat_decision(),
        None,
        "missing ir_compat_decision should be None"
    );

    Ok(())
}

#[test]
fn human_unbound_roundtrip_omits_input_digest() -> Result<(), String> {
    let admission = RunAdmission::new(
        test_artifact_digest(),
        RunId::new(200),
        test_caps(),
        RuntimePolicy::Strict,
    );

    let json = serde_json::to_string(&admission)
        .map_err(|e| format!("failed to serialize: {e}"))?;

    // The serialized JSON should not contain the input_digest field
    // for unbound admissions (nine-field shape).
    assert!(
        !json.contains("input_digest"),
        "unbound admission should omit input_digest in JSON"
    );

    // Roundtrip should succeed and produce LegacyUnbound
    let decoded: RunAdmission = serde_json::from_str(&json)
        .map_err(|e| format!("failed to deserialize: {e}"))?;

    assert_eq!(
        decoded.input_digest(),
        &RunInputBinding::LegacyUnbound,
        "roundtrip should preserve LegacyUnbound"
    );

    Ok(())
}

#[test]
fn human_bound_roundtrip_includes_input_digest() -> Result<(), String> {
    let mut admission = RunAdmission::new(
        test_artifact_digest(),
        RunId::new(201),
        test_caps(),
        RuntimePolicy::Strict,
    );
    admission.input_digest = RunInputBinding::Digest(test_workflow_digest());

    let json = serde_json::to_string(&admission)
        .map_err(|e| format!("failed to serialize: {e}"))?;

    // Bound admissions should include input_digest
    assert!(
        json.contains("input_digest"),
        "bound admission should include input_digest in JSON"
    );

    // Roundtrip should preserve the digest
    let decoded: RunAdmission = serde_json::from_str(&json)
        .map_err(|e| format!("failed to deserialize: {e}"))?;

    assert_eq!(
        decoded.input_digest(),
        &RunInputBinding::Digest(test_workflow_digest()),
        "roundtrip should preserve bound digest"
    );

    Ok(())
}