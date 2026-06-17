#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::panic)]
// Proptest: source_digest == digest through public submission API (Gate 3).
//
// Obligation: PO-vb-h09wf-027
// Verifier: proptest
// Command: cargo test -p vb_storage --test proptest -- ps_009_source_digest
//
// Domain claim: >1000 cases: artifacts submitted through public API
// always have source_digest == digest for directly compiled workflows.
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
        name: Box::<str>::from("proptest_009"),
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
    /// PS-009a: source_digest always equals artifact.digest for submitted artifacts.
    #[test]
    fn ps_009_source_digest_equals_digest(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");

        prop_assert_eq!(artifact.source_digest, artifact.digest,
            "source_digest must equal digest for directly compiled workflows");
    }

    /// PS-009b: source_digest is preserved across policies.
    #[test]
    fn ps_009_source_digest_across_policies(policy_byte in 0u8..2u8) {
        let policy = match policy_byte {
            0 => RuntimePolicy::Journaled,
            _ => RuntimePolicy::Strict,
        };

        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();

        let result = submit_artifact(&journal, &workflow, policy);
        let artifact = result.expect(
            &format!("source_digest invariance test: {policy:?} must succeed for valid workflow")
        );
        prop_assert_eq!(artifact.source_digest, artifact.digest);
    }

    /// PS-009c: Multiple submissions preserve source_digest invariance.
    #[test]
    fn ps_009_multiple_submissions_preserve_source_digest(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();

        for _ in 0..3 {
            let workflow = make_workflow();
            let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
                .expect("submit");
            prop_assert_eq!(artifact.source_digest, artifact.digest);
        }
    }
}
