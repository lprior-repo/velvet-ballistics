use super::graph_depth::chain;
use super::support::{TestResult, checkpoint, fixture};
use vb_core::ids::{ListId, SlotIdx};
use vb_core::root_bundle::{ArenaNode, OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

#[test]
fn direct_self_cycle_detected() -> TestResult<()> {
    // Given: a checkpoint with list0 containing a reference to itself
    let mut cp = checkpoint(8);
    cp.lists.push(Box::from([SlotValue::List(ListId::new(0))]));
    cp.list_taints.push(Box::from([Taint::Clean]));

    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::List,
        }],
        8,
        4,
    )?;

    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::new([(
        SlotIdx::new(0),
        SlotValue::List(ListId::new(0)),
        Taint::Clean,
    )]);

    // When: building the bundle
    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected cycle error")?;

    // Then: GraphCycle error for list0
    match err {
        OwnedRootBundleError::GraphCycle { node } => {
            assert_eq!(node, ArenaNode::List(ListId::new(0)));
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn shared_dag_longer_later_path_rejected() -> TestResult<()> {
    // Given: chain of 65 lists (0->1->...->64 empty)
    let mut cp = chain(65)?;

    // Make list0 empty (breaks the chain at the start)
    let list0 = cp.lists.get_mut(0).ok_or("list0")?;
    *list0 = Box::from([]);
    let taints0 = cp.list_taints.get_mut(0).ok_or("taints0")?;
    *taints0 = Box::from([]);

    // Make list64 point to list0 (creates 1->2->...->64->0 path, no cycle)
    let list64 = cp.lists.get_mut(64).ok_or("list64")?;
    *list64 = Box::from([SlotValue::List(ListId::new(0))]);
    let taints64 = cp.list_taints.get_mut(64).ok_or("taints64")?;
    *taints64 = Box::from([Taint::Clean]);

    let wf = fixture(
        2,
        vec![
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::List,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(1),
                kind: InputSlotKind::List,
            },
        ],
        128,
        4,
    )?;

    // Roots: slot0->list0 (empty, height 1), slot1->list1 (long path)
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::new([
        (
            SlotIdx::new(0),
            SlotValue::List(ListId::new(0)),
            Taint::Clean,
        ),
        (
            SlotIdx::new(1),
            SlotValue::List(ListId::new(1)),
            Taint::Clean,
        ),
    ]);

    // When: building the bundle
    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected depth error")?;

    // Then: GraphDepth error for list1 at depth 65
    match err {
        OwnedRootBundleError::GraphDepth { node, depth } => {
            assert_eq!(node, ArenaNode::List(ListId::new(1)));
            assert_eq!(depth, 65);
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}
