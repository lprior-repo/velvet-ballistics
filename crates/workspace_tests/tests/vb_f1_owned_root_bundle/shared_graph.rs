use vb_core::ids::{ListId, ObjectId, SlotIdx, SymbolId};
use vb_core::root_bundle::{ArenaNode, OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ObjectField;
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

use super::support::{TestResult, checkpoint, fixture};

#[test]
fn shared_cyclic_node_reachable_via_two_roots() -> TestResult<()> {
    // Given: workflow declares two List input slots
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
        8,
        4,
    )?;

    // Given: two lists both containing a reference to list 1 (cycle)
    let mut cp = checkpoint(8);
    cp.lists.push(Box::from([SlotValue::List(ListId::new(1))]));
    cp.list_taints.push(Box::from([Taint::Clean]));
    cp.lists.push(Box::from([SlotValue::List(ListId::new(1))]));
    cp.list_taints.push(Box::from([Taint::Clean]));

    // When: bundle is constructed with both lists as roots
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([
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

    let result = OwnedRootBundle::new(roots, cp, &wf);

    // Then: cycle is detected at the shared node
    let err = result.err().ok_or("expected cycle error")?;
    match err {
        OwnedRootBundleError::GraphCycle { node } => {
            assert_eq!(node, ArenaNode::List(ListId::new(1)));
        }
        other => {
            return Err(format!("wrong error variant: {:?}", other).into());
        }
    }

    Ok(())
}

#[test]
fn object_field_taint_propagation() -> TestResult<()> {
    // Given: workflow declares one Object input slot
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::Object,
        }],
        8,
        4,
    )?;

    // Given: object with a Secret-tainted field
    let mut cp = checkpoint(8);
    cp.symbols.push(Box::from("k"));
    cp.objects.push(Box::from([ObjectField::with_taint(
        SymbolId::new(0),
        SlotValue::I64(7),
        Taint::Secret,
    )]));

    // When: bundle is constructed with Clean root (mismatch)
    let roots_clean: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([(
        SlotIdx::new(0),
        SlotValue::Object(ObjectId::new(0)),
        Taint::Clean,
    )]);

    let result_clean = OwnedRootBundle::new(roots_clean, cp.clone(), &wf);

    // Then: taint mismatch error
    let err = result_clean.err().ok_or("expected taint mismatch error")?;
    match err {
        OwnedRootBundleError::TaintMismatch {
            slot,
            expected,
            actual,
        } => {
            assert_eq!(slot, SlotIdx::new(0));
            assert_eq!(expected, Taint::Secret);
            assert_eq!(actual, Taint::Clean);
        }
        other => {
            return Err(format!("wrong error variant: {:?}", other).into());
        }
    }

    // When: bundle is constructed with Secret root (match)
    let roots_secret: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([(
        SlotIdx::new(0),
        SlotValue::Object(ObjectId::new(0)),
        Taint::Secret,
    )]);

    let bundle = OwnedRootBundle::new(roots_secret.clone(), cp.clone(), &wf)?;

    // Then: bundle accepts and preserves the root
    assert!(bundle.matches_workflow(&wf));
    assert_eq!(bundle.roots(), roots_secret.as_ref());
    assert_eq!(bundle.checkpoint(), &cp);
    assert_eq!(bundle.input_slots(), wf.input_slots());

    Ok(())
}
