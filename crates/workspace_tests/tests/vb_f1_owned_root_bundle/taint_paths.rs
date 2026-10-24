use vb_core::ids::{ListId, SlotIdx};
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

use super::support::{TestResult, checkpoint, fixture};

#[test]
fn alias_root_independence() -> TestResult<()> {
    // Given: two roots pointing to the same list, one tainted, one clean
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
    let mut cp = checkpoint(8);
    cp.lists.push(Box::from([SlotValue::I64(1)]));
    cp.list_taints.push(Box::from([Taint::Secret]));
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([
        (
            SlotIdx::new(0),
            SlotValue::List(ListId::new(0)),
            Taint::TimeDependent,
        ),
        (
            SlotIdx::new(1),
            SlotValue::List(ListId::new(0)),
            Taint::Clean,
        ),
    ]);

    // When: construct bundle - first strong root must not launder second alias
    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected taint mismatch")?;

    // Then: exact TaintMismatch fields on the clean alias slot
    match err {
        OwnedRootBundleError::TaintMismatch {
            slot,
            expected,
            actual,
        } => {
            assert_eq!(slot, SlotIdx::new(1));
            assert_eq!(expected, Taint::Secret);
            assert_eq!(actual, Taint::Clean);
        }
        other => return Err(format!("expected TaintMismatch, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn transitive_propagation() -> TestResult<()> {
    // Given: list containing another list with Secret taint
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::List,
        }],
        8,
        4,
    )?;
    let mut cp = checkpoint(8);
    cp.lists.push(Box::from([SlotValue::List(ListId::new(1))]));
    cp.lists.push(Box::from([SlotValue::I64(1)]));
    cp.list_taints.push(Box::from([Taint::Clean]));
    cp.list_taints.push(Box::from([Taint::Secret]));

    // When: root declares Clean but list is transitively Secret
    let roots_clean: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([(
        SlotIdx::new(0),
        SlotValue::List(ListId::new(0)),
        Taint::Clean,
    )]);
    let err = OwnedRootBundle::new(roots_clean, cp.clone(), &wf)
        .err()
        .ok_or("expected taint mismatch")?;
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
        other => return Err(format!("expected TaintMismatch, got {:?}", other).into()),
    }

    // Then: same root with correct Secret taint is accepted, full retention
    let roots_secret: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([(
        SlotIdx::new(0),
        SlotValue::List(ListId::new(0)),
        Taint::Secret,
    )]);
    let bundle = OwnedRootBundle::new(roots_secret.clone(), cp.clone(), &wf)?;
    assert_eq!(bundle.roots(), roots_secret.as_ref());
    assert_eq!(bundle.checkpoint(), &cp);
    assert_eq!(bundle.input_slot_count(), 1);
    assert!(bundle.has_root(SlotIdx::new(0)));
    match bundle.root(SlotIdx::new(0)) {
        Some((value, taint)) => {
            assert_eq!(value, SlotValue::List(ListId::new(0)));
            assert_eq!(taint, Taint::Secret);
        }
        None => return Err("root should be Some".into()),
    }
    assert_eq!(bundle.checkpoint().lists.len(), 2);
    assert_eq!(bundle.checkpoint().list_taints.len(), 2);
    assert!(bundle.matches_workflow(&wf));
    Ok(())
}
