#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::value::ConstValue;
use vb_core::workflow::{ResourceContract, WorkflowParts};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RunId, SlotIdx, StepIdx,
    WorkflowDigest,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::shard::ShardConfig;
use vb_runtime::{Runtime, RuntimeError};

#[test]
fn submit_artifact_rejects_non_empty_input_for_valid_artifact() -> Result<(), String> {
    let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let storage = Arc::new(
        vb_storage::FjallJournal::open(temp_dir.path(), None).map_err(|error| error.to_string())?,
    );
    let workflow = minimal_workflow()?;
    let artifact = vb_storage::admission::submit_artifact(
        storage.as_ref(),
        &workflow,
        vb_core::RuntimePolicy::Journaled,
    )
    .map_err(|error| error.to_string())?;
    let runtime = Runtime::new_with_journal(
        NonZeroUsize::MIN,
        ShardConfig::default(),
        StorageRuntimeJournal::shared_journaled(Arc::clone(&storage)),
    );

    let input = [1_u8];
    let result = runtime.submit_artifact(RunId::new(99), artifact.digest, &input, &[]);

    assert!(matches!(
        result,
        Err(RuntimeError::UnsupportedOperation { operation })
            if operation == "submit_artifact_input_decode"
    ));
    Ok(())
}

fn minimal_workflow() -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("runtime_submit_artifact_test"),
        digest: WorkflowDigest::from_bytes([0_u8; 32]),
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

    let bytes = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    let computed = blake3::hash(&bytes);
    parts.digest = WorkflowDigest::from_bytes(computed.into());
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}
