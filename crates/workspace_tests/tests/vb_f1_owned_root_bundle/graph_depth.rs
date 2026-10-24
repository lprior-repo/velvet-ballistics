use super::support::{TestResult, checkpoint, fixture};
use vb_core::ids::{ListId, SlotIdx};
use vb_core::root_bundle::{ArenaNode, OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

pub(super) fn chain(count: u32) -> TestResult<ValueStoreCheckpointV1> {
    let mut cp = checkpoint(128);
    for i in 0..count {
        let next = i.checked_add(1).ok_or("overflow")?;
        let row: Box<[SlotValue]> = if next < count {
            Box::from([SlotValue::List(ListId::new(next))])
        } else {
            Box::from([])
        };
        let taints: Box<[Taint]> = if row.len() == 1 {
            Box::from([Taint::Clean])
        } else {
            Box::from([])
        };
        cp.lists.push(row);
        cp.list_taints.push(taints);
    }
    Ok(cp)
}

#[test]
fn depth_64_accepted() -> TestResult<()> {
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::List,
        }],
        128,
        4,
    )?;
    let cp = chain(64)?;
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::new([(
        SlotIdx::new(0),
        SlotValue::List(ListId::new(0)),
        Taint::Clean,
    )]);
    let saved_roots = roots.clone();
    let saved_cp = cp.clone();
    let bundle = OwnedRootBundle::new(roots, cp, &wf)?;
    assert_eq!(bundle.roots(), saved_roots.as_ref());
    assert_eq!(bundle.checkpoint(), &saved_cp);
    assert!(bundle.matches_workflow(&wf));
    Ok(())
}

#[test]
fn depth_65_rejected() -> TestResult<()> {
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::List,
        }],
        128,
        4,
    )?;
    let cp = chain(65)?;
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::new([(
        SlotIdx::new(0),
        SlotValue::List(ListId::new(0)),
        Taint::Clean,
    )]);
    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected depth error")?;
    match err {
        OwnedRootBundleError::GraphDepth { node, depth } => {
            assert_eq!(node, ArenaNode::List(ListId::new(64)));
            assert_eq!(depth, 65);
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}
