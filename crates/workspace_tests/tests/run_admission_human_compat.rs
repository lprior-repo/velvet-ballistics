#![forbid(unsafe_code)]
//! RunAdmission human-readable JSON compatibility tests.
//!
//! These tests use serde_json directly through the public RunAdmission type.
//! They verify that historical nine-field JSON maps decode as LegacyUnbound,
//! that required fields are enforced, that optional fields default to None,
//! and that bound/unbound roundtrips preserve input_digest semantics.

use serde_json::{Value, from_str, from_value, to_string, to_value};
use vb_core::ids::WorkflowDigest;
use vb_runtime::admission::RunAdmission;
use vb_storage::RunInputBinding;

fn test_artifact_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x2a; 32])
}

fn test_workflow_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x55; 32])
}

fn make_unbound_admission() -> RunAdmission {
    RunAdmission::new(
        test_artifact_digest(),
        vb_core::ids::RunId::new(100),
        vb_core::capability::CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Strict,
    )
}

fn make_bound_admission() -> Result<RunAdmission, String> {
    // Build via genuine RunAdmission + serde JSON manipulation.
    // Start with unbound, serialize to Value, insert input_digest as a
    // typed RunInputBinding, then deserialize back.
    let admission = make_unbound_admission();
    let mut json_val =
        to_value(&admission).map_err(|e| format!("failed to serialize genuine admission: {e}"))?;

    let binding = RunInputBinding::Digest(test_workflow_digest());
    let binding_val =
        to_value(&binding).map_err(|e| format!("failed to serialize input binding: {e}"))?;

    let obj = json_val
        .as_object_mut()
        .ok_or_else(|| String::from("admission JSON should be an object"))?;
    obj.insert("input_digest".to_string(), binding_val);

    from_value(json_val).map_err(|e| format!("failed to deserialize bound admission: {e}"))
}

#[test]
fn human_legacy_map_missing_input_digest_decodes_as_unbound() -> Result<(), String> {
    // Genuine RunAdmission serialized to JSON, then have input_digest
    // removed from the JSON map to test legacy nine-field decoding.
    let admission = make_unbound_admission();
    let json_val =
        to_value(&admission).map_err(|e| format!("failed to serialize genuine admission: {e}"))?;

    let mut val = json_val;
    let obj = val
        .as_object_mut()
        .ok_or_else(|| String::from("admission JSON should be an object"))?;
    obj.remove("input_digest");

    let decoded: RunAdmission =
        from_value(val).map_err(|e| format!("failed to deserialize legacy JSON: {e}"))?;
    assert_eq!(decoded, admission, "roundtrip should be identical");
    assert_eq!(
        decoded.input_digest(),
        RunInputBinding::LegacyUnbound,
        "missing input_digest should default to LegacyUnbound"
    );

    Ok(())
}

#[test]
fn human_missing_required_idempotency_field_fails() -> Result<(), String> {
    // Genuine RunAdmission JSON with required field removed.
    let admission = make_unbound_admission();
    let json_val =
        to_value(&admission).map_err(|e| format!("failed to serialize genuine admission: {e}"))?;

    let mut val = json_val;
    let obj = val
        .as_object_mut()
        .ok_or_else(|| String::from("admission JSON should be an object"))?;
    obj.remove("idempotency_attested");

    let result: Result<RunAdmission, serde_json::Error> = from_value(val);
    let err_str = match result {
        Ok(_) => return Err(String::from("missing required field should fail to decode")),
        Err(e) => e.to_string(),
    };
    assert!(
        err_str.contains("idempotency_attested"),
        "error should name missing field idempotency_attested: {err_str}"
    );

    Ok(())
}

#[test]
fn human_missing_optional_tenant_and_ir_context_decodes() -> Result<(), String> {
    // Genuine RunAdmission JSON with optional fields removed.
    let admission = make_unbound_admission();
    let json_val =
        to_value(&admission).map_err(|e| format!("failed to serialize genuine admission: {e}"))?;

    let mut val = json_val;
    let obj = val
        .as_object_mut()
        .ok_or_else(|| String::from("admission JSON should be an object"))?;
    obj.remove("tenant");
    obj.remove("ir_compat_decision");

    let decoded: RunAdmission =
        from_value(val).map_err(|e| format!("failed to deserialize: {e}"))?;
    assert_eq!(decoded, admission, "roundtrip should be identical");
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
    let admission = make_unbound_admission();
    let json = to_string(&admission).map_err(|e| format!("failed to serialize: {e}"))?;
    let json_val: Value = from_str(&json).map_err(|e| format!("failed to parse JSON: {e}"))?;
    assert!(
        json_val.get("input_digest").is_none(),
        "unbound admission should omit input_digest in JSON"
    );
    let decoded: RunAdmission =
        from_str(&json).map_err(|e| format!("failed to deserialize: {e}"))?;
    assert_eq!(decoded, admission, "roundtrip should be identical");
    assert_eq!(
        decoded.input_digest(),
        RunInputBinding::LegacyUnbound,
        "roundtrip should preserve LegacyUnbound"
    );

    Ok(())
}

#[test]
fn human_bound_roundtrip_includes_input_digest() -> Result<(), String> {
    let admission = make_bound_admission()?;
    let json = to_string(&admission).map_err(|e| format!("failed to serialize: {e}"))?;
    let json_val: Value = from_str(&json).map_err(|e| format!("failed to parse JSON: {e}"))?;
    assert!(
        json_val.get("input_digest").is_some(),
        "bound admission should include input_digest in JSON"
    );
    let decoded: RunAdmission =
        from_str(&json).map_err(|e| format!("failed to deserialize: {e}"))?;
    assert_eq!(decoded, admission, "roundtrip should be identical");
    assert_eq!(
        decoded.input_digest(),
        RunInputBinding::Digest(test_workflow_digest()),
        "roundtrip should preserve bound digest"
    );

    Ok(())
}

#[test]
fn human_bound_json_with_explicit_input_digest_decodes() -> Result<(), String> {
    let admission = RunAdmission::new(
        test_artifact_digest(),
        vb_core::ids::RunId::new(202),
        vb_core::capability::CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Strict,
    );
    let json_val =
        to_value(&admission).map_err(|e| format!("failed to serialize admission: {e}"))?;

    let binding = RunInputBinding::Digest(test_workflow_digest());
    let binding_val =
        to_value(&binding).map_err(|e| format!("failed to serialize binding: {e}"))?;

    let mut val = json_val;
    let obj = val
        .as_object_mut()
        .ok_or_else(|| String::from("admission JSON should be an object"))?;
    obj.insert("input_digest".to_string(), binding_val);

    let decoded: RunAdmission =
        from_value(val).map_err(|e| format!("failed to deserialize bound JSON: {e}"))?;
    assert_eq!(
        decoded.input_digest(),
        RunInputBinding::Digest(test_workflow_digest()),
        "explicit input_digest should decode as bound"
    );

    Ok(())
}
