use std::error::Error;
use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::workflow::{
    CompiledInputSlot, CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract,
    WorkflowParts,
};

pub(super) type TestResult<T> = Result<T, Box<dyn Error>>;

pub(super) fn fixture(
    slot_count: u16,
    input_slots: Vec<CompiledInputSlot>,
    max_slots: u16,
    max_blob_bytes: u64,
) -> TestResult<CompiledWorkflow> {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("owned-root-bundle-test"),
        digest: WorkflowDigest::from_bytes([42; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract {
            max_slots,
            max_blob_bytes,
            ..ResourceContract::DEFAULT
        },
        step_names: Box::from([]),
        input_slots: input_slots.into_boxed_slice(),
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string().into())
}

pub(super) fn checkpoint(cap: u64) -> ValueStoreCheckpointV1 {
    ValueStoreCheckpointV1 {
        symbols: Vec::new(),
        lists: Vec::new(),
        list_taints: Vec::new(),
        objects: Vec::new(),
        blobs: Vec::new(),
        max_arena_entries: cap,
    }
}
