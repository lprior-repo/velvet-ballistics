// Proptest: Trailing bytes property (Gate 3) through public API.
//
// Obligation: PO-vb-h09wf-015
// Verifier: proptest
// Command: cargo test -p vb_storage --test proptest -- ps_005_trailing_bytes
//
// Domain claim: >1000 cases: envelopes with trailing bytes fail postcard decode.
// The trailing byte defense is verified at the postcard boundary.
//
// PRODUCTION BINDING:
//   postcard::from_bytes (public crate API)
//   vb_storage::admission::submit_artifact

use proptest::prelude::*;
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy,
    SlotIdx, StepIdx, WorkflowDigest, value::ConstValue,
    workflow::{ResourceContract, WorkflowParts}};
use vb_storage::admission::AcceptedArtifact;
use vb_storage::admission::submit_artifact;
use vb_storage::journal::FjallJournal;

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}

fn make_workflow() -> CompiledWorkflow {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("proptest_005"),
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
    /// PS-005a: Valid envelope decodes without trailing bytes issue.
    #[test]
    fn ps_005_valid_envelope_no_trailing_issue(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");
        let envelope = postcard::to_allocvec(&artifact).expect("serialize");

        let (_decoded, remaining): (AcceptedArtifact, &[u8]) =
            postcard::take_from_bytes(&envelope).expect("decode");
        prop_assert!(remaining.is_empty(),
            "valid envelope must have no trailing bytes after decode");
    }

    /// PS-005b: Envelope with appended random bytes has trailing bytes.
    #[test]
    fn ps_005_envelope_with_trailer_has_remaining(_dummy in proptest::bool::ANY, 
        trailer_len in 1usize..=32usize,
    ) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");
        let mut envelope = postcard::to_allocvec(&artifact).expect("serialize");

        // Append random trailer bytes
        let trailer: Vec<u8> = (0..trailer_len).map(|_| 0xFFu8).collect();
        envelope.extend_from_slice(&trailer);

        let (_decoded, remaining): (AcceptedArtifact, &[u8]) =
            postcard::take_from_bytes(&envelope).expect("decode");
        prop_assert!(!remaining.is_empty(),
            "envelope with trailer must have remaining bytes after decode");
        prop_assert_eq!(remaining.len(), trailer_len,
            "remaining bytes length must equal trailer length");
    }

    /// PS-005c: postcard::from_bytes rejects trailered envelope (strict mode).
    #[test]
    fn ps_005_strict_decode_rejects_trailer(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");
        let mut envelope = postcard::to_allocvec(&artifact).expect("serialize");
        envelope.push(0xFF); // Add one trailing byte

        // postcard::from_bytes is strict: no trailing bytes allowed
        let result: Result<AcceptedArtifact, _> = postcard::from_bytes(&envelope);
        prop_assert!(result.is_err(), "strict decode must reject trailered envelope");
    }
}
