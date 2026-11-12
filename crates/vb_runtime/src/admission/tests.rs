use super::*;
use crate::RuntimeError;
use crate::admission::AlwaysPresentArtifactStore;
use vb_core::ids::WorkflowDigest;
use vb_storage::RunPolicyPin;

/// Builds a minimal compiled workflow for policy digest computation.
fn build_workflow_with_single_action() -> Result<vb_core::workflow::CompiledWorkflow, String> {
    use vb_core::workflow::{CompiledNodeKind, ResourceContract, WorkflowParts};
    let node = vb_core::workflow::node(
        vb_core::ids::StepIdx::ZERO,
        CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(1),
            input: vb_core::ids::SlotIdx::ZERO,
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        name: Box::from("admission-test"),
        digest: WorkflowDigest::from_bytes([0xAB; 32]),
        semantic_source_identity: None,
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        entry: vb_core::ids::StepIdx::ZERO,
        input_slots: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([Box::from("step-0")]),
        symbols_count: 0,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| format!("workflow compilation failed: {e}"))
}

struct FixedAcceptedStore {
    artifact: vb_storage::admission::AcceptedArtifact,
}

impl AcceptedArtifactStore for FixedAcceptedStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.artifact.clone())
    }
}

fn test_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0xAB; 32])
}

#[path = "evidence_test_support.rs"]
pub(super) mod evidence_test_support;
use evidence_test_support::accepted_artifact_with_caps;

include!("tests/tests_admission_core.rs");
include!("tests/tests_admission_budget.rs");
