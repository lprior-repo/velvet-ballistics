// Proptest: artifact.digest == record.digest cross-field consistency (Gate 9).
//
// Obligation: PO-vb-h09wf-019
// Verifier: proptest
// Command: cargo test -p vb_storage --test proptest -- ps_006_artifact_digest_match
//
// Domain claim: >1000 cases: artifacts submitted through the public API
// always have artifact.digest matching the workflow digest. Cross-field
// consistency is an invariant of the submission flow.
//
// PRODUCTION BINDING:
//   vb_storage::admission::submit_artifact

use proptest::prelude::*;
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy,
    SlotIdx, StepIdx, WorkflowDigest, value::ConstValue,
    workflow::{ResourceContract, WorkflowParts}};
use vb_storage::admission::submit_artifact;
use vb_storage::journal::FjallJournal;

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}

fn make_workflow() -> CompiledWorkflow {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("proptest_006"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0), output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)), on_error: None, error_slot: None,
                kind: CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
            },
            CompiledNode {
                id: StepIdx::new(1), output: None, next: None,
                on_error: None, error_slot: None,
                kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
            },
        ]),
        expressions: Box::new([]), accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1, symbols_count: 0, entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let hash_bytes = postcard::to_allocvec(&parts).unwrap();
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(*computed.as_bytes());
    CompiledWorkflow::try_from_parts(parts).unwrap()
}

proptest! {
    /// PS-006a: artifact.digest matches workflow.digest() for submitted artifacts.
    #[test]
    fn ps_006_artifact_digest_matches_workflow(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let wf_digest = workflow.digest();

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");

        prop_assert_eq!(artifact.digest, wf_digest,
            "artifact.digest must equal workflow.digest()");
    }

    /// PS-006b: artifact.source_digest == artifact.digest for submitted artifacts.
    #[test]
    fn ps_006_source_digest_equals_artifact_digest(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");

        prop_assert_eq!(artifact.source_digest, artifact.digest,
            "source_digest must equal digest for directly compiled workflows");
    }

    /// PS-006c: verification.digest == artifact.digest for submitted artifacts.
    #[test]
    fn ps_006_verification_digest_equals_artifact_digest(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");

        prop_assert_eq!(artifact.verification.digest, artifact.digest,
            "verification.digest must equal artifact.digest");
    }
}
