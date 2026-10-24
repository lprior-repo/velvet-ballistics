use vb_core::ids::{ListId, ObjectId, SlotIdx, SymbolId};
use vb_core::root_bundle::{ArenaNode, OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ObjectField;
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

use super::support::{TestResult, checkpoint, fixture};

#[test]
fn key_reuse_across_objects_accepted() -> TestResult<()> {
    // Given: workflow declares two Object input slots
    let wf = fixture(
        2,
        vec![
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::Object,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(1),
                kind: InputSlotKind::Object,
            },
        ],
        8,
        4,
    )?;

    // Given: two objects each with one field using the same key symbol
    let mut cp = checkpoint(8);
    cp.symbols.push(Box::from("key"));
    cp.objects.push(Box::from([ObjectField::clean(
        SymbolId::new(0),
        SlotValue::I64(1),
    )]));
    cp.objects.push(Box::from([ObjectField::clean(
        SymbolId::new(0),
        SlotValue::I64(2),
    )]));

    // When: bundle is constructed with both objects as roots
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([
        (
            SlotIdx::new(0),
            SlotValue::Object(ObjectId::new(0)),
            Taint::Clean,
        ),
        (
            SlotIdx::new(1),
            SlotValue::Object(ObjectId::new(1)),
            Taint::Clean,
        ),
    ]);

    let bundle = OwnedRootBundle::new(roots.clone(), cp.clone(), &wf)?;

    // Then: bundle accepts and preserves both roots
    assert!(bundle.matches_workflow(&wf));
    assert_eq!(bundle.roots(), roots.as_ref());
    assert_eq!(bundle.checkpoint(), &cp);
    assert!(bundle.has_root(SlotIdx::new(0)));
    assert!(bundle.has_root(SlotIdx::new(1)));

    match bundle.root(SlotIdx::new(0)) {
        Some((value, taint)) => {
            assert_eq!(value, SlotValue::Object(ObjectId::new(0)));
            assert_eq!(taint, Taint::Clean);
        }
        None => return Err("root 0 should be Some".into()),
    }

    match bundle.root(SlotIdx::new(1)) {
        Some((value, taint)) => {
            assert_eq!(value, SlotValue::Object(ObjectId::new(1)));
            assert_eq!(taint, Taint::Clean);
        }
        None => return Err("root 1 should be Some".into()),
    }

    Ok(())
}

#[test]
fn indirect_mixed_cycle_rejected() -> TestResult<()> {
    // Given: workflow declares one List input slot
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::List,
        }],
        8,
        4,
    )?;

    // Given: list contains object, object field references list (cycle)
    let mut cp = checkpoint(8);
    cp.symbols.push(Box::from("key"));
    cp.lists
        .push(Box::from([SlotValue::Object(ObjectId::new(0))]));
    cp.list_taints.push(Box::from([Taint::Clean]));
    cp.objects.push(Box::from([ObjectField::clean(
        SymbolId::new(0),
        SlotValue::List(ListId::new(0)),
    )]));

    // When: bundle is constructed with list as root
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([(
        SlotIdx::new(0),
        SlotValue::List(ListId::new(0)),
        Taint::Clean,
    )]);

    let result = OwnedRootBundle::new(roots, cp, &wf);

    // Then: cycle is detected with exact list node
    let err = result.err().ok_or("expected cycle error")?;
    match err {
        OwnedRootBundleError::GraphCycle { node } => {
            assert_eq!(node, ArenaNode::List(ListId::new(0)));
        }
        other => {
            return Err(format!("wrong error variant: {:?}", other).into());
        }
    }

    Ok(())
}
