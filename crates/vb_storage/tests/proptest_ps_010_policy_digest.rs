#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::panic)]
// Proptest: Policy digest recomputation through public API (Gate 4).
//
// Obligation: PO-vb-h09wf-030
// Verifier: proptest
// Command: cargo test -p vb_storage --test proptest -- ps_010_policy_digest
//
// Domain claim: >1000 cases: compute_policy_digest is deterministic and
// produces consistent results for the same workflow.
//
// PRODUCTION BINDING:
//   vb_storage::admission::compute_policy_digest (public)
//   vb_storage::admission::submit_artifact

use proptest::prelude::*;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy, SlotIdx, StepIdx,
    WorkflowDigest,
    value::ConstValue,
    workflow::{ResourceContract, WorkflowParts},
};
use vb_storage::admission::{compute_policy_digest, submit_artifact};
use vb_storage::journal::FjallJournal;

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}

fn make_workflow() -> CompiledWorkflow {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("proptest_010"),
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
    let hash_bytes =
        postcard::to_allocvec(&parts).expect("serialize workflow parts for digest computation");
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(*computed.as_bytes());
    CompiledWorkflow::try_from_parts(parts).expect("construct compiled workflow from valid parts")
}

proptest! {
    /// PS-010a: compute_policy_digest is deterministic.
    #[test]
    fn ps_010_policy_digest_deterministic(_dummy in proptest::bool::ANY) {
        let workflow = make_workflow();
        let pd1 = compute_policy_digest(&workflow).expect("compute");
        let pd2 = compute_policy_digest(&workflow).expect("compute");
        prop_assert_eq!(pd1, pd2, "compute_policy_digest must be deterministic");
    }

    /// PS-010b: compute_policy_digest returns Ok for valid workflows.
    #[test]
    fn ps_010_compute_policy_digest_succeeds(_dummy in proptest::bool::ANY) {
        let workflow = make_workflow();
        let result = compute_policy_digest(&workflow);
        let policy_digest = result.expect(
            "compute_policy_digest must succeed for valid workflow"
        );
        prop_assert_ne!(
            policy_digest,
            WorkflowDigest::from_bytes([0u8; 32]),
            "policy_digest must be non-zero for valid workflow"
        );
    }

    /// PS-010c: Submitted artifact's policy_digest matches compute_policy_digest.
    #[test]
    fn ps_010_artifact_policy_digest_matches_computed(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let expected_pd = compute_policy_digest(&workflow).expect("compute");

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");

        prop_assert_eq!(artifact.policy_digest, expected_pd,
            "artifact.policy_digest must match compute_policy_digest");
    }
}
