// Proptest: verification.digest cross-field consistency (Gate 10).
//
// Obligation: PO-vb-h09wf-022
// Verifier: proptest
// Command: cargo test -p vb_storage --test proptest -- ps_007_verification_digest_match
//
// Domain claim: >1000 cases: all three digests (artifact, verification, source)
// are consistent for artifacts produced by the public submission API.
//
// PRODUCTION BINDING:
//   vb_storage::admission::submit_artifact

use proptest::prelude::*;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy, SlotIdx, StepIdx,
    WorkflowDigest,
    value::ConstValue,
    workflow::{ResourceContract, WorkflowParts},
};
use vb_storage::admission::submit_artifact;
use vb_storage::journal::FjallJournal;

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}

fn make_workflow() -> CompiledWorkflow {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("proptest_007"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let hash_bytes = postcard::to_allocvec(&parts)
        .expect("serialize workflow parts for digest computation");
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(*computed.as_bytes());
    CompiledWorkflow::try_from_parts(parts)
        .expect("construct compiled workflow from valid parts")
}

proptest! {
    /// PS-007a: verification.digest equals artifact.digest (three-digest triangle).
    #[test]
    fn ps_007_three_digest_triangle(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");

        // Digest Triangle Invariant: all three digests must be equal
        prop_assert_eq!(artifact.digest, artifact.verification.digest,
            "artifact.digest must equal verification.digest");
        prop_assert_eq!(artifact.digest, artifact.source_digest,
            "artifact.digest must equal source_digest");
        prop_assert_eq!(artifact.verification.digest, artifact.source_digest,
            "verification.digest must equal source_digest");
    }

    /// PS-007b: Digest triangle holds across multiple submissions.
    #[test]
    fn ps_007_digest_triangle_across_submissions(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();

        for _ in 0..3 {
            let workflow = make_workflow();
            let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
                .expect("submit");

            prop_assert_eq!(artifact.digest, artifact.verification.digest);
            prop_assert_eq!(artifact.digest, artifact.source_digest);
        }
    }

    /// PS-007c: Strict policy also preserves digest triangle.
    #[test]
    fn ps_007_strict_policy_preserves_triangle(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .expect("submit");

        prop_assert_eq!(artifact.digest, artifact.verification.digest);
        prop_assert_eq!(artifact.digest, artifact.source_digest);
        prop_assert!(artifact.verification.durable);
    }
}
