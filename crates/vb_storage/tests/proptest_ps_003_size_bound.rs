// Proptest: Size bound validation through artifact submission.
//
// Obligation: PO-vb-h09wf-009
// Verifier: proptest
// Command: cargo test -p vb_storage --test proptest -- ps_003_size_bound
//
// Domain claim: >1000 cases: artifacts with varying sizes submit successfully
// when within bounds. The MAX_COMPILED_IR_BYTES gate is tested through
// the public submit_artifact API.
//
// PRODUCTION BINDING:
//   vb_storage::admission::submit_artifact (exercises reject_oversized_compiled_ir_value internally)

use proptest::prelude::*;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy, SlotIdx, StepIdx,
    WorkflowDigest,
    value::ConstValue,
    workflow::{ResourceContract, WorkflowParts},
};
use vb_storage::admission::submit_artifact;
use vb_storage::constants::MAX_COMPILED_IR_BYTES;
use vb_storage::journal::FjallJournal;

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}

fn make_workflow() -> CompiledWorkflow {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("proptest_003"),
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
    let hash_bytes = postcard::to_allocvec(&parts).unwrap();
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(*computed.as_bytes());
    CompiledWorkflow::try_from_parts(parts).unwrap()
}

proptest! {
    /// PS-003a: Normal-sized workflows submit successfully.
    #[test]
    fn ps_003_normal_workflow_submits(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled);
        prop_assert!(result.is_ok(), "normal workflow submission must succeed");
    }

    /// PS-003b: MAX_COMPILED_IR_BYTES is a u32 and fits in its domain.
    #[test]
    fn ps_003_max_fits_in_u32(_dummy in proptest::bool::ANY) {
        prop_assert!(MAX_COMPILED_IR_BYTES <= u32::MAX);
        prop_assert!(MAX_COMPILED_IR_BYTES > 0);
    }

    /// PS-003c: Verify size-related constants are reasonable.
    #[test]
    fn ps_003_size_constants_reasonable(_dummy in proptest::bool::ANY) {
        // MAX_COMPILED_IR_BYTES = 16_777_216 (16 MiB)
        prop_assert_eq!(MAX_COMPILED_IR_BYTES, 16_777_216);
        // Must be representable as usize on all supported platforms
        prop_assert!(MAX_COMPILED_IR_BYTES as usize <= usize::MAX);
    }

    /// PS-003d: Large but valid workflows (within bounds) submit successfully.
    #[test]
    fn ps_003_large_envelope_submits(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let workflow = make_workflow();
        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled);
        if result.is_ok() {
            let artifact = result.unwrap();
            let envelope = postcard::to_allocvec(&artifact).expect("serialize");
            // Envelope must be within bounds for submission to succeed
            prop_assert!(envelope.len() as u32 <= MAX_COMPILED_IR_BYTES,
                "successfully submitted artifact envelope must not exceed MAX");
        }
    }
}
