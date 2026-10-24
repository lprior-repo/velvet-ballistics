use super::support::{TestResult, checkpoint, fixture};
use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

#[test]
fn zero_slot_workflow_cap_mismatch() -> TestResult<()> {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("zero-slot-test"),
        digest: WorkflowDigest::from_bytes([42; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract {
            max_slots: 0,
            max_blob_bytes: 0,
            ..ResourceContract::DEFAULT
        },
        step_names: Box::from([]),
        input_slots: Box::from([]),
    };
    let wf =
        CompiledWorkflow::try_from_parts(parts).map_err(|e| -> Box<dyn std::error::Error> {
            format!("workflow construction failed: {}", e).into()
        })?;

    let roots = Box::from([]);
    let cp = checkpoint(0);
    let result = OwnedRootBundle::new(roots, cp, &wf);
    let err = result.err().ok_or("expected error")?;
    match err {
        OwnedRootBundleError::CapMismatch {
            checkpoint_cap,
            workflow_max_slots,
        } => {
            assert_eq!(checkpoint_cap, 0);
            assert_eq!(workflow_max_slots, 0);
        }
        other => return Err(format!("unexpected error: {:?}", other).into()),
    }

    let roots2 = Box::from([]);
    let cp2 = checkpoint(1);
    let result2 = OwnedRootBundle::new(roots2, cp2, &wf);
    let err2 = result2.err().ok_or("expected error")?;
    match err2 {
        OwnedRootBundleError::CapMismatch {
            checkpoint_cap,
            workflow_max_slots,
        } => {
            assert_eq!(checkpoint_cap, 1);
            assert_eq!(workflow_max_slots, 0);
        }
        other => return Err(format!("unexpected error: {:?}", other).into()),
    }

    Ok(())
}

#[test]
fn arena_limit_65536_symbols() -> TestResult<()> {
    let wf = fixture(
        1,
        vec![vb_core::workflow::CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: vb_core::workflow::InputSlotKind::I64,
        }],
        65535,
        4,
    )?;

    let symbols = vec![Box::<str>::from("s"); 65536];
    let cp = ValueStoreCheckpointV1 {
        symbols,
        lists: Vec::new(),
        list_taints: Vec::new(),
        objects: Vec::new(),
        blobs: Vec::new(),
        max_arena_entries: 65535,
    };

    let roots = Box::from([(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)]);
    let result = OwnedRootBundle::new(roots, cp, &wf);
    let err = result.err().ok_or("expected error")?;
    match err {
        OwnedRootBundleError::ArenaLimit { count, limit } => {
            assert_eq!(count, 65536);
            assert_eq!(limit, 65535);
        }
        other => return Err(format!("unexpected error: {:?}", other).into()),
    }

    Ok(())
}

#[test]
fn empty_roots_checkpoint_accepted() -> TestResult<()> {
    let wf = fixture(1, vec![], 8, 4)?;

    let roots = Box::from([]);
    let cp = checkpoint(8);
    let bundle = OwnedRootBundle::new(roots, cp, &wf)?;

    let original_roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([]);
    assert_eq!(bundle.roots(), original_roots.as_ref());

    let original_cp = checkpoint(8);
    assert_eq!(bundle.checkpoint(), &original_cp);

    assert_eq!(bundle.input_slots(), wf.input_slots());
    assert!(bundle.matches_workflow(&wf));

    Ok(())
}
