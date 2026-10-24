use bytes::Bytes;
use vb_core::ids::{BlobId, ListId, ObjectId, SlotIdx, SymbolId};
use vb_core::root_bundle::{ArenaNode, OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ObjectField;
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

use super::support::{TestResult, checkpoint, fixture};

#[test]
fn object_keys_are_reachable() -> TestResult<()> {
    // Given: workflow declares input slot 0 as Object
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::Object,
        }],
        8,
        4,
    )?;

    // Given: checkpoint has two symbols and one object with two distinct fields
    let mut cp = checkpoint(8);
    cp.symbols.push(Box::from("left"));
    cp.symbols.push(Box::from("right"));
    cp.objects.push(Box::from([
        ObjectField::clean(SymbolId::new(0), SlotValue::I64(1)),
        ObjectField::clean(SymbolId::new(1), SlotValue::I64(2)),
    ]));

    // When: bundle is constructed with object root
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([(
        SlotIdx::new(0),
        SlotValue::Object(ObjectId::new(0)),
        Taint::Clean,
    )]);

    let bundle = OwnedRootBundle::new(roots.clone(), cp.clone(), &wf)?;

    // Then: bundle accepts and preserves roots and checkpoint
    assert!(bundle.matches_workflow(&wf));
    assert_eq!(bundle.roots(), roots.as_ref());
    assert_eq!(bundle.checkpoint(), &cp);
    assert!(bundle.has_root(SlotIdx::new(0)));

    match bundle.root(SlotIdx::new(0)) {
        Some((value, taint)) => {
            assert_eq!(value, SlotValue::Object(ObjectId::new(0)));
            assert_eq!(taint, Taint::Clean);
        }
        None => return Err("root should be Some".into()),
    }

    Ok(())
}

#[test]
fn duplicate_object_key() -> TestResult<()> {
    // Given: workflow declares input slot 0 as Object
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::Object,
        }],
        8,
        4,
    )?;

    // Given: checkpoint has one object with two fields sharing the same key
    let mut cp = checkpoint(8);
    cp.symbols.push(Box::from("key"));
    cp.objects.push(Box::from([
        ObjectField::clean(SymbolId::new(0), SlotValue::I64(1)),
        ObjectField::clean(SymbolId::new(0), SlotValue::I64(2)),
    ]));

    // When: bundle is constructed
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([(
        SlotIdx::new(0),
        SlotValue::Object(ObjectId::new(0)),
        Taint::Clean,
    )]);

    let result = OwnedRootBundle::new(roots, cp, &wf);

    // Then: duplicate key is detected with exact object and key ids
    let err = result.err().ok_or("expected duplicate key error")?;
    match err {
        OwnedRootBundleError::GraphDuplicateObjectKey { object, key } => {
            assert_eq!(object, ObjectId::new(0));
            assert_eq!(key, SymbolId::new(0));
        }
        other => {
            return Err(format!("wrong error variant: {:?}", other).into());
        }
    }

    Ok(())
}

#[test]
fn every_arena_unreachable() -> TestResult<()> {
    // Given: four arena node types, each unreachable from scalar root
    let cases = [
        ArenaNode::Symbol(SymbolId::new(0)),
        ArenaNode::List(ListId::new(0)),
        ArenaNode::Object(ObjectId::new(0)),
        ArenaNode::Blob(BlobId::new(0)),
    ];

    let mut checked = 0_u64;

    for node in cases {
        checked = checked.checked_add(1).ok_or("count overflow")?;

        // Given: workflow declares input slot 0 as I64 (scalar, no arena refs)
        let wf = fixture(
            1,
            vec![CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::I64,
            }],
            8,
            4,
        )?;

        // Given: checkpoint has exactly one arena entry matching this node
        let mut cp = checkpoint(8);
        match &node {
            ArenaNode::Symbol(_) => {
                cp.symbols.push(Box::from("s"));
            }
            ArenaNode::List(_) => {
                cp.lists.push(Box::from([]));
                cp.list_taints.push(Box::from([]));
            }
            ArenaNode::Object(_) => {
                cp.objects.push(Box::from([]));
            }
            ArenaNode::Blob(_) => {
                cp.blobs.push(Bytes::from_static(b"x"));
            }
        }

        // When: bundle is constructed with scalar root
        let roots: Box<[(SlotIdx, SlotValue, Taint)]> =
            Box::from([(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)]);

        let result = OwnedRootBundle::new(roots, cp, &wf);

        // Then: unreachable arena entry is detected with exact node
        let err = result.err().ok_or("expected unreachable error")?;
        match err {
            OwnedRootBundleError::GraphUnreachable { node: actual } => {
                assert_eq!(actual, node);
            }
            other => {
                return Err(format!("wrong error variant: {:?}", other).into());
            }
        }
    }

    assert_eq!(checked, 4);
    Ok(())
}
