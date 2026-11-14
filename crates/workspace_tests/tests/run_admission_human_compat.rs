#![forbid(unsafe_code)]
//! Current human-readable RunAdmission record roundtrips and obsolete-map rejection.

use serde_json::{Value, from_value, to_value};
use vb_core::budget::AggregateResourceBudget;
use vb_core::capability::CapabilitySet;
use vb_core::ids::RunId;
use vb_core::policy::RuntimePolicy;
use vb_runtime::admission::{
    AcceptedArtifactStore, ArtifactEnvelopeError, RunAdmission, admit_artifact_run,
};
use vb_storage::admission::{AcceptedArtifact, prepare_compiled_artifact};
use vb_storage::{RunInputBinding, RunPolicyPin};

const WORKFLOW_SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: admission_human_compat\nwhen: {manual: {}}\nvars: {sentinel: 1}\nsteps:\n  - id: done\n    finish: {result: '$vars.sentinel'}\n";

struct FixtureStore(AcceptedArtifact);

impl AcceptedArtifactStore for FixtureStore {
    fn load_accepted_artifact(
        &self,
        _digest: vb_core::WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.0.clone())
    }
}

/// The verified fixture builds a structural codec record; roundtrip does not authorize effects.
fn admission() -> Result<RunAdmission, String> {
    let workflow = vb_compile::compile_workflow(WORKFLOW_SOURCE)
        .map_err(|error| format!("test workflow compilation failed: {error}"))?;
    let artifact = prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])
        .map_err(|error| format!("test artifact preparation failed: {error}"))?;
    let artifact_digest = artifact.digest;
    let admitted = admit_artifact_run(
        &FixtureStore(artifact),
        RuntimePolicy::Strict,
        RunId::new(100),
        artifact_digest,
        CapabilitySet::empty(),
    )
    .map_err(|error| format!("test artifact admission failed: {error}"))?;
    let budget = AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| format!("test workflow budget failed: {error}"))?;
    let mut value = to_value(admitted).map_err(|error| format!("serialize admission: {error}"))?;
    object_mut(&mut value)?.insert(
        String::from("input_digest"),
        to_value(RunInputBinding::Digest(workflow.digest()))
            .map_err(|error| format!("serialize input digest: {error}"))?,
    );
    object_mut(&mut value)?.insert(
        String::from("budget"),
        to_value(budget).map_err(|error| format!("serialize admission budget: {error}"))?,
    );
    from_value(value).map_err(|error| format!("deserialize full test admission: {error}"))
}

fn object_mut(value: &mut Value) -> Result<&mut serde_json::Map<String, Value>, String> {
    value
        .as_object_mut()
        .ok_or_else(|| String::from("current admission JSON should be an object"))
}

#[test]
fn current_human_record_roundtrips_all_ten_fields() -> Result<(), String> {
    let original = admission()?;
    let value = to_value(&original).map_err(|e| format!("serialize failed: {e}"))?;
    let fields = [
        "artifact_digest",
        "run_id",
        "granted_capabilities",
        "policy",
        "policy_pin",
        "input_digest",
        "budget",
        "tenant",
        "idempotency_attested",
        "ir_compat_decision",
    ];
    for field in fields {
        if value.get(field).is_none() {
            return Err(format!("current human record omitted field {field}"));
        }
    }

    let decoded: RunAdmission =
        from_value(value).map_err(|e| format!("deserialize failed: {e}"))?;
    assert_eq!(decoded, original);
    assert!(matches!(decoded.input_digest(), RunInputBinding::Digest(_)));
    assert!(decoded.budget().is_some());
    assert!(matches!(decoded.policy_pin(), RunPolicyPin::Pinned(_)));
    Ok(())
}

#[test]
fn current_human_record_roundtrips_bound_input_digest() -> Result<(), String> {
    let original = admission()?;
    let binding = original.input_digest();
    let mut value = to_value(&original).map_err(|e| format!("serialize failed: {e}"))?;
    let binding_value = to_value(binding).map_err(|e| format!("serialize input digest: {e}"))?;
    object_mut(&mut value)?.insert(String::from("input_digest"), binding_value);

    let decoded: RunAdmission =
        from_value(value).map_err(|e| format!("deserialize failed: {e}"))?;
    assert_eq!(decoded.input_digest(), binding);
    let encoded = to_value(&decoded).map_err(|e| format!("re-serialize failed: {e}"))?;
    let roundtripped: RunAdmission =
        from_value(encoded).map_err(|e| format!("second deserialize failed: {e}"))?;
    assert_eq!(roundtripped, decoded);
    assert!(roundtripped.budget().is_some());
    assert!(matches!(roundtripped.policy_pin(), RunPolicyPin::Pinned(_)));
    Ok(())
}

#[test]
fn human_record_without_input_digest_is_rejected() -> Result<(), String> {
    let mut value = to_value(admission()?).map_err(|e| format!("serialize failed: {e}"))?;
    object_mut(&mut value)?.remove("input_digest");

    let decoded: Result<RunAdmission, serde_json::Error> = from_value(value);
    let error = match decoded {
        Ok(_) => return Err(String::from("missing required input_digest was accepted")),
        Err(error) => error,
    };
    assert!(error.to_string().contains("input_digest"));
    Ok(())
}

#[test]
fn human_record_without_idempotency_evidence_is_rejected() -> Result<(), String> {
    let mut value = to_value(admission()?).map_err(|e| format!("serialize failed: {e}"))?;
    object_mut(&mut value)?.remove("idempotency_attested");

    let decoded: Result<RunAdmission, serde_json::Error> = from_value(value);
    let error = match decoded {
        Ok(_) => return Err(String::from("missing idempotency_attested was accepted")),
        Err(error) => error,
    };
    assert!(error.to_string().contains("idempotency_attested"));
    Ok(())
}
