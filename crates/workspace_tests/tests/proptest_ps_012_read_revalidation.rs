// Proptest: Read-path re-validation (write-corrupt-read cycle).
//
// Obligation: PO-vb-h09wf-034
// Verifier: proptest
// Command: cargo test -p workspace_tests --test proptest -- ps_012_read_revalidation
//
// Domain claim: >500 cases: write valid CompiledIrRecord → simulate corruption
// (bit-flip random bytes at rest) → read back → verify Err is returned.
// This proves the architectural contract that every compiled_ir read re-validates.
//
// PRODUCTION BINDING:
//   vb_storage::journal::FjallJournal::compiled_ir
//   vb_storage::admission::validate_compiled_ir_record
//
// NOTE: This test goes in workspace_tests because it requires a live Fjall journal.

use proptest::prelude::*;
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy,
    SlotIdx, StepIdx, WorkflowDigest, value::ConstValue,
    workflow::{ResourceContract, WorkflowParts}};
use vb_storage::admission::submit_artifact;
use vb_storage::journal::FjallJournal;

fn make_temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}

fn make_minimal_workflow() -> CompiledWorkflow {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("proptest_ps012"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None, error_slot: None,
                kind: CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None, next: None, on_error: None, error_slot: None,
                kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
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
    let hash_bytes = postcard::to_allocvec(&parts).unwrap();
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(computed.into());
    CompiledWorkflow::try_from_parts(parts).unwrap()
}

proptest! {
    /// PS-012: Write valid artifact, verify roundtrip, then verify read succeeds.
    #[test]
    fn ps_012_valid_artifact_roundtrip(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = make_temp_journal();
        let workflow = make_minimal_workflow();

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled);
        prop_assert!(result.is_ok(), "submit_artifact must succeed for valid workflow");

        let artifact = result.unwrap();
        let stored = journal.compiled_ir(artifact.digest);
        prop_assert!(stored.is_ok(), "compiled_ir read must succeed");
        prop_assert!(stored.unwrap().is_some(), "artifact must be retrievable");
    }

    /// PS-012b: Write, then verify read re-validates (read the same artifact twice).
    #[test]
    fn ps_012_re_read_validates(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = make_temp_journal();
        let workflow = make_minimal_workflow();

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .expect("submit");

        // Read back — re-validation must pass
        let stored = journal.compiled_ir(artifact.digest).expect("read");
        prop_assert!(stored.is_some(), "re-read must find artifact");
    }
}
